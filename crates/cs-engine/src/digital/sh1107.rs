//! SH1107 128x128 OLED display controller (C++ `Sh1107` + `OledController`).

use super::PinMode;
use super::pin::IoPin;
use super::twi::{TwiModule, TwiPins};

pub const PAGE_ADDR_MODE: u8 = 0;
pub const COLU_ADDR_MODE: u8 = 1;

#[derive(Clone, Debug)]
pub struct Sh1107State {
    pub id: String,
    pub width: usize,
    pub height: usize,
    pub rows: usize,
    pub control_code: u8,
    pub x_offset: bool,
    pub freq_khz: f64,
    pub pin_scl: IoPin,
    pub pin_sda: IoPin,
    pub twi: TwiModule,
    pub ddram: Vec<Vec<u8>>,
    // Addressing state
    pub addr_mode: u8,
    pub addr_x: usize,
    pub addr_y: usize,
    pub start_x: usize,
    pub end_x: usize,
    pub start_y: usize,
    pub end_y: usize,
    // Display config
    pub disp_offset: u8,
    pub ram_offset: u8,
    pub disp_on: bool,
    pub disp_full: bool,
    pub disp_inv: bool,
    pub scan_inv: bool,
    pub remap: bool,
    pub mr: u8,
    // Command protocol state
    pub start: bool,
    pub co: bool,
    pub data: bool,
    pub last_command: u8,
    pub read_bytes: u8,
    pub read_index: u8,
    pub line_mask: u8,
    pub row_mask: u8,
}

impl Sh1107State {
    pub const DEFAULT_ADDRESS: u8 = 0x3C;

    pub fn new(
        id: impl Into<String>,
        width: usize,
        height: usize,
        control_code: u8,
        x_offset: bool,
        freq_khz: f64,
    ) -> Self {
        let id_str = id.into();
        let w = width.clamp(32, 128);
        let h = ((height.clamp(16, 128) / 8) * 8).max(16);
        let rows = h / 8;
        let line_mask = if h > 64 { 0x7F } else { 0x3F };
        let row_mask = if h > 64 { 0x0F } else { 0x07 };

        let mut pin_scl = IoPin::open_collector(format!("{id_str}-PinSck"));
        pin_scl.set_pin_mode(PinMode::OpenCo);
        pin_scl.set_pullup(1e5);
        pin_scl.set_out_state(true);

        let mut pin_sda = IoPin::open_collector(format!("{id_str}-PinSda"));
        pin_sda.set_pin_mode(PinMode::OpenCo);
        pin_sda.set_pullup(1e5);
        pin_sda.set_out_state(true);

        let mut twi = TwiModule::new();
        twi.set_slave(control_code);
        twi.set_freq_khz(freq_khz);

        let ddram = vec![vec![0u8; rows]; w];

        let mut state = Self {
            id: id_str,
            width: w,
            height: h,
            rows,
            control_code,
            x_offset,
            freq_khz,
            pin_scl,
            pin_sda,
            twi,
            ddram,
            addr_mode: PAGE_ADDR_MODE,
            addr_x: 0,
            addr_y: 0,
            start_x: 0,
            end_x: w.saturating_sub(1),
            start_y: 0,
            end_y: rows.saturating_sub(1),
            disp_offset: 0,
            ram_offset: 0,
            disp_on: false,
            disp_full: false,
            disp_inv: false,
            scan_inv: false,
            remap: false,
            mr: 127,
            start: false,
            co: false,
            data: false,
            last_command: 0,
            read_bytes: 0,
            read_index: 0,
            line_mask,
            row_mask,
        };
        state.reset();
        state
    }

    pub fn pin_ids(&self) -> Vec<String> {
        vec![self.pin_scl.id.clone(), self.pin_sda.id.clone()]
    }

    pub fn pins(&self) -> Vec<&IoPin> {
        vec![&self.pin_scl, &self.pin_sda]
    }

    pub fn pins_mut(&mut self) -> Vec<&mut IoPin> {
        vec![&mut self.pin_scl, &mut self.pin_sda]
    }

    pub fn clear_ddram(&mut self) {
        for col in &mut self.ddram {
            col.fill(0);
        }
    }

    pub fn reset(&mut self) {
        self.clear_ddram();
        self.addr_mode = PAGE_ADDR_MODE;
        self.disp_on = false;
        self.disp_full = false;
        self.disp_inv = false;
        self.scan_inv = false;
        self.remap = false;
        self.start_x = 0;
        self.end_x = self.width.saturating_sub(1);
        self.start_y = 0;
        self.end_y = self.rows.saturating_sub(1);
        self.addr_x = 0;
        self.addr_y = 0;
        self.disp_offset = 0;
        self.ram_offset = 0;
        self.mr = 127;
        self.start = false;
        self.co = false;
        self.data = false;
        self.last_command = 0;
        self.read_bytes = 0;
        self.read_index = 0;
        self.twi = TwiModule::new();
        self.twi.set_slave(self.control_code);
        self.twi.set_freq_khz(self.freq_khz);
    }

    pub fn process_command(&mut self, cmd: u8) {
        self.last_command = cmd;
        self.read_index = 0;
        self.read_bytes = 0;

        if cmd < 0x18 {
            if cmd < 0x10 {
                self.addr_x = (self.addr_x & 0xF0) | (cmd as usize & 0x0F);
            } else {
                self.addr_x = (self.addr_x & 0x0F) | (((cmd as usize & 0x07) << 4) & 0x70);
            }
        } else if (0x40..=0x7F).contains(&cmd) {
            self.ram_offset = cmd & self.line_mask;
        } else if (0xB0..=0xBF).contains(&cmd) {
            if self.addr_mode == PAGE_ADDR_MODE {
                self.addr_y = (cmd & self.row_mask) as usize;
            }
        } else {
            match cmd {
                0x20 => self.addr_mode = PAGE_ADDR_MODE,
                0x21 => self.addr_mode = COLU_ADDR_MODE,
                0x81 => self.read_bytes = 1,
                0xA0 => self.remap = false,
                0xA1 => self.remap = true,
                0xA4 => self.disp_full = false,
                0xA5 => self.disp_full = true,
                0xA6 => self.disp_inv = false,
                0xA7 => self.disp_inv = true,
                0xA8 => self.read_bytes = 1,
                0xAE => self.disp_on = false,
                0xAF => self.disp_on = true,
                0xC0 => self.scan_inv = false,
                0xC8 => self.scan_inv = true,
                0xD3 => self.read_bytes = 1,
                0xD5 => self.read_bytes = 1,
                0xD9 => self.read_bytes = 1,
                0xDA => self.read_bytes = 1,
                0xDB => self.read_bytes = 1,
                0xDC => self.read_bytes = 1,
                _ => {}
            }
        }
    }

    pub fn parameter(&mut self, param: u8) {
        self.read_index += 1;
        if self.read_index > self.read_bytes {
            return;
        }
        if self.read_index == self.read_bytes {
            self.read_bytes = 0;
        }

        match self.last_command {
            0xA8 => {
                let mux = param & self.line_mask;
                if mux > 14 {
                    self.mr = mux;
                }
            }
            0xD3 => {
                self.disp_offset = param & self.line_mask;
            }
            0xDC => {
                self.ram_offset = param & self.line_mask;
            }
            _ => {}
        }
    }

    pub fn write_data(&mut self, val: u8) {
        let mut actual_x = self.addr_x;
        if self.x_offset {
            if actual_x >= 96 {
                actual_x -= 96;
            } else {
                actual_x += self.width.saturating_sub(96);
            }
        }

        if actual_x < self.width && self.addr_y < self.rows {
            self.ddram[actual_x][self.addr_y] = val;
        }

        if self.addr_mode == COLU_ADDR_MODE {
            self.addr_y += 1;
            if self.addr_y > self.end_y {
                self.addr_y = self.start_y;
            }
        } else {
            self.addr_x += 1;
            if self.addr_x > self.end_x {
                self.addr_x = self.start_x;
            }
        }
    }

    pub fn hex_bitmap(&self) -> String {
        if !self.disp_on {
            return String::new();
        }

        let mut hex = String::with_capacity(self.rows * self.width * 2);
        use std::fmt::Write;

        for page in 0..self.rows {
            for col in 0..self.width {
                let actual_col = if self.remap {
                    self.width.saturating_sub(1).saturating_sub(col)
                } else {
                    col
                };
                let mut byte = self.ddram[actual_col][page];
                if self.disp_inv {
                    byte = !byte;
                }
                if self.disp_full {
                    byte = 0xFF;
                }
                let _ = write!(hex, "{:02X}", byte);
            }
        }

        hex
    }

    pub fn volt_changed(&mut self, pins: &mut [IoPin], scl_idx: usize, sda_idx: usize) {
        let map = TwiPins {
            scl: Some(scl_idx),
            sda: Some(sda_idx),
        };
        let tick = self.twi.volt_changed(pins, map);
        if tick.stopped {
            self.start = false;
        }
        if tick.need_start_write {
            self.start = true;
            self.co = false;
            self.data = false;
        }
        if let Some(rx) = tick.byte_received {
            if self.start {
                self.co = (rx & 0x80) != 0;
                self.data = (rx & 0x40) != 0;
                self.start = false;
            } else if self.data {
                self.write_data(rx);
            } else if self.read_bytes > 0 {
                self.parameter(rx);
            } else {
                self.process_command(rx);
            }

            if self.co {
                self.start = true;
            }
        }
    }

    pub fn step(&mut self, _circ_time: u64) {
        let mut pins = [self.pin_scl.clone(), self.pin_sda.clone()];
        self.volt_changed(&mut pins, 0, 1);
        self.pin_scl = pins[0].clone();
        self.pin_sda = pins[1].clone();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sh1107_init_and_write() {
        let mut disp = Sh1107State::new("OLED1", 128, 128, 0x3C, true, 100.0);
        assert_eq!(disp.width, 128);
        assert_eq!(disp.height, 128);
        assert_eq!(disp.rows, 16);

        // Turn on display
        disp.process_command(0xAF);
        assert!(disp.disp_on);

        // Write a test byte
        disp.write_data(0xAA);
        let hex = disp.hex_bitmap();
        assert!(!hex.is_empty());
    }
}
