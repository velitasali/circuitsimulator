//! SD Card SPI Mode Protocol Controller (C++ `SdCard`).
//!
//! Emulates a standard SD/SDHC card accessed over SPI bus in SPI Mode 0 (CPOL=0, CPHA=0).
//! Supports CMD0, CMD8, CMD9, CMD10, CMD12, CMD16, CMD17, CMD24, CMD55, ACMD41, CMD58.

use std::collections::VecDeque;
use std::fs::{self, OpenOptions};
use std::io::{Seek, SeekFrom, Write};
use std::mem;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum SdWriteState {
    Idle,
    WaitingForToken,
    ReceivingData,
    ReceivingCrc(u8),
}

#[derive(Clone, Debug)]
pub struct SdCardSpi {
    pub file: String,
    pub card_inserted: bool,
    pub memory: Vec<u8>,
    pub dirty: bool,

    // Bus signals
    pub cs_high: bool,
    pub last_sck: bool,
    pub do_high: bool,
    pub do_driven: bool,

    // Protocol state
    pub spi_mode: bool,
    pub in_idle: bool,
    pub high_capacity: bool,
    pub block_len: usize,
    pub app_cmd: bool,

    // SPI byte receiver
    in_bit_count: u8,
    in_byte: u8,
    cmd_bytes: [u8; 6],
    cmd_idx: usize,
    in_command: bool,

    // SPI byte transmitter
    tx_queue: VecDeque<u8>,
    tx_byte: u8,
    tx_bit_idx: u8,

    // Block write state
    pub write_state: SdWriteState,
    pub write_lba: u64,
    pub write_buf: Vec<u8>,
}

impl Default for SdCardSpi {
    fn default() -> Self {
        Self::new("")
    }
}

impl SdCardSpi {
    pub fn new(file: impl Into<String>) -> Self {
        let file = file.into();
        let memory = if !file.is_empty() {
            fs::read(&file).unwrap_or_else(|_| vec![0u8; 1024 * 1024])
        } else {
            vec![0u8; 1024 * 1024]
        };
        Self {
            file,
            card_inserted: true,
            memory,
            dirty: false,
            cs_high: true,
            last_sck: false,
            do_high: true,
            do_driven: false,
            spi_mode: false,
            in_idle: false,
            high_capacity: false,
            block_len: 512,
            app_cmd: false,
            in_bit_count: 0,
            in_byte: 0,
            cmd_bytes: [0; 6],
            cmd_idx: 0,
            in_command: false,
            tx_queue: VecDeque::new(),
            tx_byte: 0xFF,
            tx_bit_idx: 0,
            write_state: SdWriteState::Idle,
            write_lba: 0,
            write_buf: Vec::new(),
        }
    }

    /// Read a 512-byte logical block from the backing storage.
    pub fn read_sector(&self, lba: u64, out: &mut [u8; 512]) {
        let offset = (lba as usize) * 512;
        if offset < self.memory.len() {
            let n = (self.memory.len() - offset).min(512);
            out[..n].copy_from_slice(&self.memory[offset..offset + n]);
            out[n..].fill(0);
        } else {
            out.fill(0);
        }
    }

    /// Write a logical block to the backing storage.
    pub fn write_sector(&mut self, lba: u64, data: &[u8]) {
        let offset = (lba as usize) * 512;
        let end = offset + data.len();
        if end > self.memory.len() {
            self.memory.resize(end, 0);
        }
        self.memory[offset..end].copy_from_slice(data);
        self.dirty = true;
        if !self.file.is_empty() {
            if let Ok(mut f) = OpenOptions::new().create(true).write(true).open(&self.file) {
                if f.seek(SeekFrom::Start(offset as u64)).is_ok() {
                    let _ = f.write_all(data);
                }
            }
        }
    }

    /// Process pin updates on CS, SCK, and MOSI.
    ///
    /// Returns whether `PinDO` level or drive state changed.
    pub fn step(&mut self, cs_high: bool, sck_high: bool, di_high: bool) -> bool {
        let old_do_high = self.do_high;
        let old_do_driven = self.do_driven;

        if cs_high {
            if self.do_driven {
                self.do_driven = false;
            }
            self.cs_high = true;
            self.last_sck = sck_high;
            self.in_bit_count = 0;
            self.in_command = false;
            self.cmd_idx = 0;
            self.write_state = SdWriteState::Idle;
            self.tx_queue.clear();
            self.tx_byte = 0xFF;
            self.tx_bit_idx = 0;
            self.do_high = true;
            return old_do_high != self.do_high || old_do_driven != self.do_driven;
        }

        // Card is selected (CS low)
        let cs_just_asserted = self.cs_high;
        self.cs_high = false;
        if self.spi_mode && !self.do_driven {
            self.do_driven = true;
        }

        if cs_just_asserted {
            self.tx_bit_idx = 0;
            self.do_high = (self.tx_byte & 0x80) != 0;
        }

        let rising_sck = sck_high && !self.last_sck;
        let falling_sck = !sck_high && self.last_sck;
        self.last_sck = sck_high;

        if rising_sck {
            // Master drives MOSI, SD card samples on rising SCK
            self.in_byte = (self.in_byte << 1) | u8::from(di_high);
            self.in_bit_count += 1;
            if self.in_bit_count >= 8 {
                self.in_bit_count = 0;
                let rx = self.in_byte;
                self.on_rx_byte(rx);
            }
        }

        if falling_sck {
            // SD card shifts next MISO bit out on falling SCK
            self.advance_tx_bit();
        }

        old_do_high != self.do_high || old_do_driven != self.do_driven
    }

    fn advance_tx_bit(&mut self) {
        self.tx_bit_idx += 1;
        if self.tx_bit_idx >= 8 {
            self.tx_bit_idx = 0;
            self.tx_byte = self.tx_queue.pop_front().unwrap_or(0xFF);
        }
        self.do_high = (self.tx_byte & (0x80 >> self.tx_bit_idx)) != 0;
    }

    fn queue_response(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.tx_queue.push_back(b);
        }
    }

    fn on_rx_byte(&mut self, byte: u8) {
        match self.write_state {
            SdWriteState::WaitingForToken => {
                if byte == 0xFE {
                    self.write_state = SdWriteState::ReceivingData;
                    self.write_buf.clear();
                } else if byte == 0xFD {
                    self.write_state = SdWriteState::Idle;
                }
                return;
            }
            SdWriteState::ReceivingData => {
                self.write_buf.push(byte);
                if self.write_buf.len() >= self.block_len {
                    self.write_state = SdWriteState::ReceivingCrc(0);
                }
                return;
            }
            SdWriteState::ReceivingCrc(crc_count) => {
                if crc_count == 0 {
                    self.write_state = SdWriteState::ReceivingCrc(1);
                } else {
                    let buf = mem::take(&mut self.write_buf);
                    self.write_sector(self.write_lba, &buf);
                    // Data response token: 0x05 (Data accepted)
                    self.queue_response(&[0x05, 0xFF]);
                    self.write_state = SdWriteState::Idle;
                }
                return;
            }
            SdWriteState::Idle => {}
        }

        if !self.in_command {
            // SD command starts with bit 7 = 0 and bit 6 = 1 (0x40..0x7F)
            if (byte & 0xC0) == 0x40 {
                self.in_command = true;
                self.cmd_bytes[0] = byte;
                self.cmd_idx = 1;
            }
        } else {
            self.cmd_bytes[self.cmd_idx] = byte;
            self.cmd_idx += 1;
            if self.cmd_idx >= 6 {
                self.in_command = false;
                self.cmd_idx = 0;
                self.execute_command();
            }
        }
    }

    fn execute_command(&mut self) {
        let cmd = self.cmd_bytes[0] & 0x3F;
        let arg = u32::from_be_bytes([
            self.cmd_bytes[1],
            self.cmd_bytes[2],
            self.cmd_bytes[3],
            self.cmd_bytes[4],
        ]);

        match cmd {
            0 => {
                // CMD0: GO_IDLE_STATE
                self.spi_mode = true;
                self.do_driven = true;
                self.in_idle = true;
                self.app_cmd = false;
                self.write_state = SdWriteState::Idle;
                self.queue_response(&[0xFF, 0x01]);
            }
            8 => {
                // CMD8: SEND_IF_COND
                let check_pattern = (arg & 0xFF) as u8;
                let voltage = ((arg >> 8) & 0x0F) as u8;
                let r1 = if self.in_idle { 0x01 } else { 0x00 };
                self.queue_response(&[0xFF, r1, 0x00, 0x00, voltage, check_pattern]);
            }
            55 => {
                // CMD55: APP_CMD
                self.app_cmd = true;
                let r1 = if self.in_idle { 0x01 } else { 0x00 };
                self.queue_response(&[0xFF, r1]);
            }
            41 => {
                // ACMD41: SD_SEND_OP_COND
                if self.app_cmd {
                    self.app_cmd = false;
                    if (arg & 0x4000_0000) != 0 {
                        self.high_capacity = true;
                    }
                    self.in_idle = false;
                    self.queue_response(&[0xFF, 0x00]);
                } else {
                    self.queue_response(&[0xFF, 0x04]);
                }
            }
            1 => {
                // CMD1: SEND_OP_COND
                self.in_idle = false;
                self.queue_response(&[0xFF, 0x00]);
            }
            58 => {
                // CMD58: READ_OCR
                let r1 = if self.in_idle { 0x01 } else { 0x00 };
                let b0 = if self.high_capacity { 0xC0 } else { 0x80 };
                self.queue_response(&[0xFF, r1, b0, 0xFF, 0x80, 0x00]);
            }
            16 => {
                // CMD16: SET_BLOCKLEN
                if !self.in_idle {
                    self.block_len = (arg as usize).clamp(1, 2048);
                    self.queue_response(&[0xFF, 0x00]);
                } else {
                    self.queue_response(&[0xFF, 0x01]);
                }
            }
            17 => {
                // CMD17: READ_SINGLE_BLOCK
                let lba = if self.high_capacity {
                    arg as u64
                } else {
                    (arg / 512) as u64
                };
                let mut sector = [0u8; 512];
                self.read_sector(lba, &mut sector);

                let mut resp = Vec::with_capacity(518);
                resp.push(0xFF);
                resp.push(0x00);
                resp.push(0xFF);
                resp.push(0xFE);
                let len = self.block_len.min(512);
                resp.extend_from_slice(&sector[..len]);
                if self.block_len > 512 {
                    resp.resize(resp.len() + (self.block_len - 512), 0);
                }
                resp.push(0xFF);
                resp.push(0xFF);
                self.queue_response(&resp);
            }
            24 => {
                // CMD24: WRITE_SINGLE_BLOCK
                let lba = if self.high_capacity {
                    arg as u64
                } else {
                    (arg / 512) as u64
                };
                self.write_lba = lba;
                self.write_state = SdWriteState::WaitingForToken;
                self.queue_response(&[0xFF, 0x00]);
            }
            9 => {
                // CMD9: SEND_CSD
                let mut resp = Vec::with_capacity(22);
                resp.push(0xFF);
                resp.push(0x00);
                resp.push(0xFE);
                resp.extend_from_slice(&[
                    0x40, 0x0E, 0x00, 0x32, 0x5B, 0x59, 0x00, 0x00, 0x7F, 0x80, 0x7F, 0x80, 0x0A,
                    0x40, 0x00, 0x92,
                ]);
                resp.push(0xFF);
                resp.push(0xFF);
                self.queue_response(&resp);
            }
            10 => {
                // CMD10: SEND_CID
                let mut resp = Vec::with_capacity(22);
                resp.push(0xFF);
                resp.push(0x00);
                resp.push(0xFE);
                resp.extend_from_slice(&[
                    0x03, b'C', b'S', b'S', b'D', b'0', b'1', 0x10, 0x12, 0x34, 0x56, 0x78, 0x00,
                    0x26, 0x01, 0x00,
                ]);
                resp.push(0xFF);
                resp.push(0xFF);
                self.queue_response(&resp);
            }
            12 => {
                // CMD12: STOP_TRANSMISSION
                self.write_state = SdWriteState::Idle;
                self.queue_response(&[0xFF, 0x00]);
            }
            _ => {
                self.queue_response(&[0xFF, 0x04]);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn send_byte(spi: &mut SdCardSpi, b: u8) -> u8 {
        let mut rx = 0u8;
        for i in 0..8 {
            let di = (b & (0x80 >> i)) != 0;
            // Rising edge: sample MOSI
            spi.step(false, true, di);
            // Master samples MISO during clock high
            if spi.do_high {
                rx |= 0x80 >> i;
            }
            // Falling edge: advance MISO
            spi.step(false, false, di);
        }
        rx
    }

    fn send_cmd(spi: &mut SdCardSpi, cmd: u8, arg: u32, crc: u8) {
        let arg_bytes = arg.to_be_bytes();
        send_byte(spi, 0x40 | cmd);
        send_byte(spi, arg_bytes[0]);
        send_byte(spi, arg_bytes[1]);
        send_byte(spi, arg_bytes[2]);
        send_byte(spi, arg_bytes[3]);
        send_byte(spi, crc);
    }

    #[test]
    fn test_sd_spi_cmd0_and_cmd8() {
        let mut spi = SdCardSpi::default();
        assert!(!spi.do_driven);

        // Wake up / select
        spi.step(false, false, true);
        assert!(spi.do_high);

        // Send CMD0
        send_cmd(&mut spi, 0, 0, 0x95);
        assert!(spi.do_driven);
        assert!(spi.spi_mode);

        // Turnaround byte
        let r0 = send_byte(&mut spi, 0xFF);
        assert_eq!(r0, 0xFF);

        // R1: 0x01 (in idle state)
        let r1 = send_byte(&mut spi, 0xFF);
        assert_eq!(r1, 0x01);

        // Send CMD8 (check pattern 0xAA, voltage 0x1)
        send_cmd(&mut spi, 8, 0x1AA, 0x87);

        // Read R7 response: Turnaround (0xFF), R1 (0x01), 0x00, 0x00, 0x01, 0xAA
        let r = send_byte(&mut spi, 0xFF);
        assert_eq!(r, 0xFF);
        let r1 = send_byte(&mut spi, 0xFF);
        assert_eq!(r1, 0x01);
        let b1 = send_byte(&mut spi, 0xFF);
        assert_eq!(b1, 0x00);
        let b2 = send_byte(&mut spi, 0xFF);
        assert_eq!(b2, 0x00);
        let b3 = send_byte(&mut spi, 0xFF);
        assert_eq!(b3, 0x01);
        let b4 = send_byte(&mut spi, 0xFF);
        assert_eq!(b4, 0xAA);
    }

    #[test]
    fn test_sd_spi_acmd41_and_read_write() {
        let mut spi = SdCardSpi::default();
        spi.step(false, false, true);

        // CMD0
        send_cmd(&mut spi, 0, 0, 0x95);
        send_byte(&mut spi, 0xFF);
        assert_eq!(send_byte(&mut spi, 0xFF), 0x01);

        // CMD55
        send_cmd(&mut spi, 55, 0, 0x65);
        send_byte(&mut spi, 0xFF);
        assert_eq!(send_byte(&mut spi, 0xFF), 0x01);

        // ACMD41 with HCS (bit 30)
        send_cmd(&mut spi, 41, 0x4000_0000, 0x77);
        send_byte(&mut spi, 0xFF);
        assert_eq!(send_byte(&mut spi, 0xFF), 0x00); // 0x00 = ready!
        assert!(spi.high_capacity);

        // Write block 2: CMD24
        send_cmd(&mut spi, 24, 2, 0x00);
        send_byte(&mut spi, 0xFF);
        assert_eq!(send_byte(&mut spi, 0xFF), 0x00); // R1 success

        // Send Data Token 0xFE
        send_byte(&mut spi, 0xFE);
        // Send 512 bytes data
        let mut payload = [0u8; 512];
        for (i, b) in payload.iter_mut().enumerate() {
            *b = (i % 251) as u8;
            send_byte(&mut spi, *b);
        }
        // Send 2 dummy CRC bytes
        send_byte(&mut spi, 0x12);
        send_byte(&mut spi, 0x34);

        // Read data response token: 0x05 (Data accepted)
        assert_eq!(send_byte(&mut spi, 0xFF), 0x05);

        // Now read back block 2: CMD17
        send_cmd(&mut spi, 17, 2, 0x00);
        send_byte(&mut spi, 0xFF);
        assert_eq!(send_byte(&mut spi, 0xFF), 0x00); // R1
        assert_eq!(send_byte(&mut spi, 0xFF), 0xFF); // turnaround
        assert_eq!(send_byte(&mut spi, 0xFF), 0xFE); // Start token!

        // Read 512 bytes
        let mut readback = [0u8; 512];
        for b in &mut readback {
            *b = send_byte(&mut spi, 0xFF);
        }
        assert_eq!(readback, payload);
    }
}
