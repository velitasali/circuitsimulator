//! Generic USART bit-bang (C++ `UsartModule` / `UartTx` / `UartRx`).
//!
//! Used by scripted MCUs and QEMU ESP32/STM32. MCU-register USART stays in
//! `cs-mcu`.

use super::pin::IoPin;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Parity {
    None = 0,
    Even = 2,
    Odd = 3,
}

impl Parity {
    pub fn from_i32(v: i32) -> Self {
        match v {
            2 => Self::Even,
            3 => Self::Odd,
            _ => Self::None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TxState {
    Stopped,
    Idle,
    Transmit,
    TxEnd,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RxState {
    Stopped,
    Idle,
    Receive,
}

#[derive(Clone, Debug)]
pub struct UsartModule {
    pub baud_rate: i32,
    pub data_bits: u8,
    pub data_mask: u8,
    pub stop_bits: u8,
    pub parity: Parity,
    pub period: u64,
    tx_enabled: bool,
    rx_enabled: bool,
    tx_state: TxState,
    rx_state: RxState,
    tx_buffer: u8,
    tx_data: u8,
    tx_frame: u16,
    tx_frame_size: u8,
    tx_bit: u8,
    pub tx_due: Option<u64>,
    rx_frame: u16,
    rx_frame_size: u8,
    rx_bit: u8,
    rx_start_high: bool,
    rx_fifo: [u16; 2],
    rx_fifo_p: i8,
    pub rx_due: Option<u64>,
}

impl Default for UsartModule {
    fn default() -> Self {
        Self::new()
    }
}

impl UsartModule {
    pub fn new() -> Self {
        let mut u = Self {
            baud_rate: 9600,
            data_bits: 8,
            data_mask: 0xFF,
            stop_bits: 1,
            parity: Parity::None,
            period: 0,
            tx_enabled: false,
            rx_enabled: false,
            tx_state: TxState::Stopped,
            rx_state: RxState::Stopped,
            tx_buffer: 0,
            tx_data: 0,
            tx_frame: 0,
            tx_frame_size: 0,
            tx_bit: 0,
            tx_due: None,
            rx_frame: 0,
            rx_frame_size: 0,
            rx_bit: 0,
            rx_start_high: false,
            rx_fifo: [0; 2],
            rx_fifo_p: -1,
            rx_due: None,
        };
        u.set_baud_rate(9600);
        u
    }

    pub fn set_baud_rate(&mut self, br: i32) {
        self.baud_rate = br.max(1);
        self.period = (1e12 / f64::from(self.baud_rate)).round() as u64;
        if self.period == 0 {
            self.period = 1;
        }
    }

    pub fn set_data_bits(&mut self, b: u8) {
        self.data_bits = b.clamp(5, 9);
        self.data_mask = if self.data_bits >= 8 {
            0xFF
        } else {
            (1u8 << self.data_bits).wrapping_sub(1)
        };
    }

    pub fn set_parity(&mut self, p: Parity) {
        self.parity = p;
    }

    pub fn set_stop_bits(&mut self, n: u8) {
        self.stop_bits = n.max(1);
    }

    pub fn enable_tx(&mut self, en: bool, pins: &mut [IoPin], tx: Option<usize>) {
        self.tx_enabled = en;
        self.tx_state = if en { TxState::Idle } else { TxState::Stopped };
        if !en {
            self.tx_due = None;
        }
        if let Some(i) = tx {
            if let Some(p) = pins.get_mut(i) {
                if en {
                    p.set_pin_mode(super::PinMode::Output);
                    p.set_out_state(true);
                }
            }
        }
    }

    pub fn enable_rx(&mut self, en: bool) {
        self.rx_enabled = en;
        self.rx_state = if en { RxState::Idle } else { RxState::Stopped };
        if !en {
            self.rx_due = None;
        }
        self.rx_start_high = false;
    }

    pub fn is_tx_enabled(&self) -> bool {
        self.tx_enabled
    }

    pub fn is_rx_enabled(&self) -> bool {
        self.rx_enabled
    }

    pub fn sending(&self) -> bool {
        matches!(self.tx_state, TxState::Transmit | TxState::TxEnd)
    }

    /// C++ `UsartModule::sendByte`.
    pub fn send_byte(&mut self, data: u8, now: u64, pins: &mut [IoPin], tx: Option<usize>) {
        self.tx_buffer = data;
        if self.tx_enabled && self.tx_state == TxState::Idle {
            self.start_tx(now, pins, tx);
        }
    }

    fn start_tx(&mut self, now: u64, pins: &mut [IoPin], tx: Option<usize>) {
        self.tx_data = self.tx_buffer;
        self.tx_state = TxState::Transmit;
        let data = self.tx_data & self.data_mask;
        self.tx_frame = u16::from(data) << 1;
        self.tx_frame_size = self.data_bits + 1;
        if self.parity != Parity::None {
            if parity_of(data, self.data_bits, self.parity) {
                self.tx_frame |= 1 << self.tx_frame_size;
            }
            self.tx_frame_size += 1;
        }
        for _ in 0..self.stop_bits {
            self.tx_frame |= 1 << self.tx_frame_size;
            self.tx_frame_size += 1;
        }
        self.tx_bit = 0;
        self.send_bit(now, pins, tx);
    }

    fn send_bit(&mut self, now: u64, pins: &mut [IoPin], tx: Option<usize>) {
        if let Some(i) = tx {
            if let Some(p) = pins.get_mut(i) {
                p.set_out_state(self.tx_frame & 1 != 0);
            }
        }
        self.tx_frame >>= 1;
        self.tx_bit += 1;
        if self.tx_bit == self.tx_frame_size {
            self.tx_state = TxState::TxEnd;
        }
        self.tx_due = Some(now.saturating_add(self.period.max(1)));
    }

    /// Advance TX one bit. Returns `Some(byte)` when a frame finishes (`frameSent`).
    pub fn tx_event(&mut self, now: u64, pins: &mut [IoPin], tx: Option<usize>) -> Option<u8> {
        match self.tx_state {
            TxState::Stopped | TxState::Idle => {
                self.tx_due = None;
                None
            }
            TxState::Transmit => {
                self.send_bit(now, pins, tx);
                None
            }
            TxState::TxEnd => {
                self.tx_state = TxState::Idle;
                if let Some(i) = tx {
                    if let Some(p) = pins.get_mut(i) {
                        p.set_out_state(true);
                    }
                }
                self.tx_due = None;
                Some(self.tx_data)
            }
        }
    }

    pub fn rx_pin_changed(&mut self, now: u64, pins: &[IoPin], rx: Option<usize>) {
        if !self.rx_enabled || self.rx_state != RxState::Idle {
            return;
        }
        let Some(i) = rx else {
            return;
        };
        let Some(p) = pins.get(i) else {
            return;
        };
        let bit = p.last_inp_state();
        if !self.rx_start_high && bit {
            self.rx_start_high = true;
        } else if self.rx_start_high && !bit {
            self.rx_state = RxState::Receive;
            self.rx_frame = 0;
            self.rx_bit = 0;
            self.rx_frame_size =
                self.data_bits + 1 + self.stop_bits + u8::from(self.parity != Parity::None);
            self.rx_due = Some(now.saturating_add((self.period / 2).max(1)));
        }
    }

    /// Advance RX one bit. Returns a received byte when a frame completes.
    pub fn rx_event(&mut self, now: u64, pins: &[IoPin], rx: Option<usize>) -> Option<u8> {
        if self.rx_state != RxState::Receive {
            self.rx_due = None;
            return None;
        }
        let bit = rx
            .and_then(|i| pins.get(i))
            .map(|p| p.last_inp_state())
            .unwrap_or(true);
        if bit {
            if self.rx_bit == 0 {
                self.rx_end();
                return None;
            }
            self.rx_frame += 1 << self.rx_bit;
        }
        self.rx_bit += 1;
        if self.rx_bit == self.rx_frame_size {
            self.rx_frame >>= 1;
            let data = (self.rx_frame as u8) & self.data_mask;
            self.push_rx(u16::from(data));
            self.rx_end();
            Some(data)
        } else {
            self.rx_due = Some(now.saturating_add(self.period.max(1)));
            None
        }
    }

    fn push_rx(&mut self, frame: u16) {
        if self.rx_fifo_p < 0 {
            self.rx_fifo[0] = frame;
            self.rx_fifo_p = 0;
        } else if self.rx_fifo_p == 0 {
            self.rx_fifo[1] = frame;
            self.rx_fifo_p = 1;
        } else {
            self.rx_fifo[1] = frame;
        }
    }

    fn rx_end(&mut self) {
        self.rx_state = RxState::Idle;
        self.rx_bit = 0;
        self.rx_frame = 0;
        self.rx_start_high = false;
        self.rx_due = None;
    }

    pub fn get_rx_data(&mut self) -> u8 {
        if self.rx_fifo_p < 0 {
            return 0;
        }
        let data = (self.rx_fifo[0] as u8) & self.data_mask;
        self.rx_fifo_p -= 1;
        if self.rx_fifo_p == 0 {
            self.rx_fifo[0] = self.rx_fifo[1];
        }
        data
    }

    /// Inject a byte directly into the RX FIFO (e.g. from host serial monitor).
    pub fn inject_rx(&mut self, byte: u8) {
        let data = u16::from(byte & self.data_mask);
        self.push_rx(data);
    }

    pub fn remain(&self, now: u64) -> Option<u64> {
        [self.tx_due, self.rx_due]
            .into_iter()
            .flatten()
            .map(|t| t.saturating_sub(now).max(1))
            .min()
    }

    pub fn tick(
        &mut self,
        now: u64,
        pins: &mut [IoPin],
        tx: Option<usize>,
        rx: Option<usize>,
    ) -> UsartTick {
        let mut out = UsartTick::default();
        if self.tx_due.is_some_and(|t| t <= now) {
            out.frame_sent = self.tx_event(now, pins, tx);
        }
        if self.rx_due.is_some_and(|t| t <= now) {
            out.byte_received = self.rx_event(now, pins, rx);
        }
        out
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct UsartTick {
    pub frame_sent: Option<u8>,
    pub byte_received: Option<u8>,
}

fn parity_of(mut data: u8, bits: u8, par: Parity) -> bool {
    let mut p = false;
    for _ in 0..bits {
        p ^= data & 1 != 0;
        data >>= 1;
    }
    if par == Parity::Odd {
        p = !p;
    }
    p
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn baud_sets_period() {
        let mut u = UsartModule::new();
        u.set_baud_rate(9600);
        assert_eq!(u.period, 104_166_667);
        u.set_baud_rate(115200);
        assert_eq!(u.period, 8_680_556);
    }

    #[test]
    fn tx_drives_start_and_lsb() {
        let mut pins = vec![IoPin::output("TX")];
        let mut u = UsartModule::new();
        u.enable_tx(true, &mut pins, Some(0));
        u.send_byte(0x01, 0, &mut pins, Some(0));
        assert!(!pins[0].get_out_state()); // start bit
        let mut now = 0u64;
        let mut bits = vec![false];
        let mut sent = None;
        for _ in 0..16 {
            now += u.period;
            sent = u.tx_event(now, &mut pins, Some(0)).or(sent);
            bits.push(pins[0].get_out_state());
            if sent.is_some() {
                break;
            }
        }
        assert_eq!(bits[0], false); // start
        assert_eq!(bits[1], true); // LSB of 0x01
        assert_eq!(bits[2], false);
        assert_eq!(sent, Some(0x01));
        assert_eq!(u.tx_state, TxState::Idle);
    }

    #[test]
    fn rx_assembles_byte() {
        let mut pins = vec![IoPin::input("RX")];
        let mut u = UsartModule::new();
        u.enable_rx(true);
        pins[0].get_inp_state(5.0);
        u.rx_pin_changed(0, &pins, Some(0));
        pins[0].get_inp_state(0.0);
        u.rx_pin_changed(1, &pins, Some(0));
        assert_eq!(u.rx_state, RxState::Receive);
        let mut now = 1 + u.period / 2;
        // start already detected; sample 8 data bits of 0x55 = 01010101 LSB first
        // plus stop. After start, first sample is the start-bit centre... C++ samples
        // at period/2 after falling edge then every period. First sample is start (low).
        let sequence = [
            false, // start
            true, false, true, false, true, false, true, false, // 0x55 LSB first
            true,  // stop
        ];
        let mut got = None;
        for b in sequence {
            pins[0].get_inp_state(if b { 5.0 } else { 0.0 });
            got = u.rx_event(now, &pins, Some(0)).or(got);
            now += u.period;
        }
        assert_eq!(got, Some(0x55));
        assert_eq!(u.get_rx_data(), 0x55);
    }
}
