//! Generic SPI shifter (C++ `SpiModule`).

use super::PinMode;
use super::clock::{ClkState, Clocked};
use super::pin::IoPin;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpiMode {
    Off = 0,
    Master = 1,
    Slave = 2,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct SpiPins {
    pub mosi: Option<usize>,
    pub miso: Option<usize>,
    pub clk: Option<usize>,
    pub ss: Option<usize>,
}

#[derive(Clone, Debug)]
pub struct SpiModule {
    pub mode: SpiMode,
    pub clock_period: u64,
    pub lsb_first: bool,
    pub use_ss: bool,
    enabled: bool,
    toggle_sck: bool,
    sample_edge: ClkState,
    lead_edge: ClkState,
    tail_edge: ClkState,
    out_bit: u8,
    in_bit: u8,
    bit_count: u8,
    pub sr_reg: u8,
    pub tx_reg: u8,
    pub data_reg: u8,
    pub due: Option<u64>,
    clocked: Clocked,
}

impl Default for SpiModule {
    fn default() -> Self {
        Self::new()
    }
}

impl SpiModule {
    pub fn new() -> Self {
        Self {
            mode: SpiMode::Off,
            clock_period: 500_000, // 1 MHz half-period default
            lsb_first: false,
            use_ss: true,
            enabled: false,
            toggle_sck: false,
            sample_edge: ClkState::Rising,
            lead_edge: ClkState::Rising,
            tail_edge: ClkState::Falling,
            out_bit: 0x80,
            in_bit: 1,
            bit_count: 0,
            sr_reg: 0,
            tx_reg: 0,
            data_reg: 0,
            due: None,
            clocked: Clocked::default(),
        }
    }

    pub fn set_cpol_cpha(&mut self, cpol: bool, cpha: bool) {
        self.lead_edge = if cpol {
            ClkState::Falling
        } else {
            ClkState::Rising
        };
        self.tail_edge = if cpol {
            ClkState::Rising
        } else {
            ClkState::Falling
        };
        self.sample_edge = if cpol == cpha {
            ClkState::Rising
        } else {
            ClkState::Falling
        };
    }

    pub fn set_mode(&mut self, mode: SpiMode, pins: &mut [IoPin], map: SpiPins) {
        if mode == self.mode {
            return;
        }
        self.mode = mode;
        match mode {
            SpiMode::Off => {
                self.due = None;
                self.enabled = false;
            }
            SpiMode::Master => {
                if map.mosi.is_none() || map.miso.is_none() || map.clk.is_none() {
                    self.mode = SpiMode::Off;
                    return;
                }
                if let Some(i) = map.mosi {
                    if let Some(p) = pins.get_mut(i) {
                        p.set_pin_mode(PinMode::Output);
                        p.set_out_state(true);
                    }
                }
                if let Some(i) = map.clk {
                    if let Some(p) = pins.get_mut(i) {
                        p.set_pin_mode(PinMode::Output);
                    }
                }
            }
            SpiMode::Slave => {
                if map.mosi.is_none() || map.clk.is_none() {
                    self.mode = SpiMode::Off;
                    return;
                }
                if let Some(i) = map.miso {
                    if let Some(p) = pins.get_mut(i) {
                        p.set_pin_mode(PinMode::Output);
                    }
                }
            }
        }
    }

    pub fn start_transaction(&mut self, now: u64, pins: &mut [IoPin], map: SpiPins) {
        self.reset_sr();
        self.due = None;
        if self.sample_edge == self.lead_edge {
            self.step(now, pins, map);
        } else if self.mode == SpiMode::Master {
            self.keep_clocking(now);
        }
    }

    fn reset_sr(&mut self) {
        self.tx_reg = self.sr_reg;
        self.bit_count = 0;
        if self.lsb_first {
            self.out_bit = 1;
            self.in_bit = 1 << 7;
        } else {
            self.out_bit = 1 << 7;
            self.in_bit = 1;
        }
    }

    fn keep_clocking(&mut self, now: u64) {
        self.toggle_sck = true;
        self.due = Some(now.saturating_add(self.clock_period.max(1)));
    }

    fn data_out(&self, map: SpiPins) -> Option<usize> {
        match self.mode {
            SpiMode::Master => map.mosi,
            SpiMode::Slave => map.miso,
            SpiMode::Off => None,
        }
    }

    fn data_in(&self, map: SpiPins) -> Option<usize> {
        match self.mode {
            SpiMode::Master => map.miso,
            SpiMode::Slave => map.mosi,
            SpiMode::Off => None,
        }
    }

    fn step(&mut self, now: u64, pins: &mut [IoPin], map: SpiPins) {
        if self.mode == SpiMode::Master {
            self.keep_clocking(now);
        }
        if self.clocked.clk_state == self.sample_edge {
            self.bit_count = self.bit_count.saturating_add(1);
            if let Some(i) = self.data_in(map) {
                if pins.get(i).is_some_and(|p| p.last_inp_state()) {
                    self.sr_reg |= self.in_bit;
                }
            }
        } else {
            if self.bit_count == 8 {
                self.end_transaction(pins, map);
            }
            if let Some(i) = self.data_out(map) {
                if let Some(p) = pins.get_mut(i) {
                    p.set_out_state(self.sr_reg & self.out_bit != 0);
                }
            }
            if self.lsb_first {
                self.sr_reg >>= 1;
            } else {
                self.sr_reg <<= 1;
            }
        }
    }

    pub fn end_transaction(&mut self, pins: &mut [IoPin], map: SpiPins) {
        if self.mode == SpiMode::Master {
            if let Some(i) = self.data_out(map) {
                if let Some(p) = pins.get_mut(i) {
                    p.set_out_state(true);
                }
            }
            self.data_reg = self.sr_reg;
            self.due = None;
        } else {
            self.reset_sr();
        }
    }

    /// Master clock event. Returns true when a byte finishes.
    pub fn run_event(&mut self, now: u64, pins: &mut [IoPin], map: SpiPins) -> bool {
        if self.mode != SpiMode::Master {
            return false;
        }
        if !self.toggle_sck {
            return false;
        }
        if let Some(i) = map.clk {
            if let Some(p) = pins.get_mut(i) {
                p.set_out_state(!p.get_out_state());
                self.clocked.clk_state = if p.get_out_state() {
                    ClkState::Rising
                } else {
                    ClkState::Falling
                };
            }
        }
        self.toggle_sck = false;
        if self.bit_count == 8 {
            self.end_transaction(pins, map);
            true
        } else {
            self.step(now, pins, map);
            false
        }
    }

    /// Slave SCK / SS change. Returns true when a byte finishes.
    pub fn volt_changed(&mut self, now: u64, pins: &mut [IoPin], map: SpiPins) -> bool {
        if self.mode != SpiMode::Slave {
            return false;
        }
        let clk_level = map
            .clk
            .and_then(|i| pins.get(i))
            .map(|p| p.last_inp_state());
        self.clocked.update(clk_level);
        let mut enabled = true;
        if self.use_ss {
            if let Some(i) = map.ss {
                enabled = !pins.get(i).is_some_and(|p| p.last_inp_state());
            }
        }
        if enabled != self.enabled {
            if enabled && !self.enabled {
                self.reset_sr();
                if let Some(i) = self.data_out(map) {
                    if let Some(p) = pins.get_mut(i) {
                        p.set_out_state(self.sr_reg & self.out_bit != 0);
                    }
                }
            } else {
                self.sr_reg = 0;
            }
            self.enabled = enabled;
        }
        if !enabled {
            return false;
        }
        if matches!(self.clocked.clk_state, ClkState::High | ClkState::Low) {
            return false;
        }
        let before = self.bit_count;
        self.step(now, pins, map);
        before < 8 && self.bit_count == 8
    }

    pub fn remain(&self, now: u64) -> Option<u64> {
        self.due.map(|t| t.saturating_sub(now).max(1))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pins() -> (Vec<IoPin>, SpiPins) {
        let pins = vec![
            IoPin::output("MOSI"),
            IoPin::input("MISO"),
            IoPin::output("SCK"),
            IoPin::output("SS"),
        ];
        let map = SpiPins {
            mosi: Some(0),
            miso: Some(1),
            clk: Some(2),
            ss: Some(3),
        };
        (pins, map)
    }

    #[test]
    fn master_shifts_eight_bits() {
        let (mut pins, map) = pins();
        let mut s = SpiModule::new();
        s.clock_period = 1_000;
        s.set_mode(SpiMode::Master, &mut pins, map);
        s.sr_reg = 0xA5;
        s.start_transaction(0, &mut pins, map);
        let mut now = 0u64;
        let mut done = false;
        for _ in 0..32 {
            if let Some(d) = s.remain(now) {
                now += d;
            } else {
                break;
            }
            done = s.run_event(now, &mut pins, map) || done;
        }
        assert!(done);
        assert_eq!(s.data_reg, 0); // MISO idle low
        assert_eq!(s.tx_reg, 0xA5);
    }
}
