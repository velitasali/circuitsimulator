//! AIP31068 5x8 dot-matrix character LCD with I2C interface (C++ `Aip31068_i2c`).
//!
//! Wraps `Hd44780State` with an I2C slave (`TwiModule`) at default address `0x3E`.

use super::PinMode;
use super::hd44780::Hd44780State;
use super::pin::IoPin;
use super::twi::{TwiModule, TwiPins};

#[derive(Clone, Debug)]
pub struct Aip31068State {
    pub id: String,
    pub rows: usize,
    pub cols: usize,
    pub control_code: u8,
    pub freq_khz: f64,
    pub pin_scl: IoPin,
    pub pin_sda: IoPin,
    pub twi: TwiModule,
    pub hd44780: Hd44780State,
    pub control_byte: u8,
    pub phase: u8,
}

impl Aip31068State {
    pub const DEFAULT_ADDRESS: u8 = 0x3E;

    pub fn new(
        id: impl Into<String>,
        rows: usize,
        cols: usize,
        control_code: u8,
        freq_khz: f64,
    ) -> Self {
        let id_str = id.into();
        let rows = rows.clamp(1, 4);
        let cols = cols.clamp(8, 20);

        let mut pin_scl = IoPin::open_collector(format!("{id_str}-PinSCL"));
        pin_scl.set_pin_mode(PinMode::OpenCo);
        pin_scl.set_pullup(1e5);
        pin_scl.set_out_state(true);

        let mut pin_sda = IoPin::open_collector(format!("{id_str}-PinSDA"));
        pin_sda.set_pin_mode(PinMode::OpenCo);
        pin_sda.set_pullup(1e5);
        pin_sda.set_out_state(true);

        let mut twi = TwiModule::new();
        twi.set_slave(control_code);
        twi.set_freq_khz(freq_khz);

        let hd44780 = Hd44780State::new(&id_str, rows, cols);

        Self {
            id: id_str,
            rows,
            cols,
            control_code,
            freq_khz,
            pin_scl,
            pin_sda,
            twi,
            hd44780,
            control_byte: 0,
            phase: 0,
        }
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

    pub fn reset(&mut self) {
        self.control_byte = 0;
        self.phase = 0;
        self.twi = TwiModule::new();
        self.twi.set_slave(self.control_code);
        self.twi.set_freq_khz(self.freq_khz);
        self.hd44780.clear_lcd();
    }

    pub fn hex_bitmap(&self) -> String {
        self.hd44780.hex_bitmap()
    }

    pub fn lines(&self) -> Vec<String> {
        self.hd44780.lines()
    }

    pub fn volt_changed(&mut self, pins: &mut [IoPin], scl_idx: usize, sda_idx: usize) {
        let map = TwiPins {
            scl: Some(scl_idx),
            sda: Some(sda_idx),
        };
        let tick = self.twi.volt_changed(pins, map);
        if tick.stopped {
            self.phase = 0;
        }
        if tick.need_start_write {
            self.phase = 0;
        }
        if let Some(rx) = tick.byte_received {
            if self.phase == 0 {
                self.control_byte = rx;
                self.phase = 1;
            } else {
                let rs = (self.control_byte & 0x40) != 0;
                if rs {
                    self.hd44780.write_data(rx);
                } else {
                    self.hd44780.process_command(rx);
                }
                if (self.control_byte & 0x80) != 0 {
                    self.phase = 0;
                }
            }
        }
    }

    pub fn step(&mut self, circ_time: u64) {
        let mut pins = [self.pin_scl.clone(), self.pin_sda.clone()];
        self.volt_changed(&mut pins, 0, 1);
        self.pin_scl = pins[0].clone();
        self.pin_sda = pins[1].clone();
        self.hd44780.step(circ_time);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aip31068_init_and_commands() {
        let mut disp = Aip31068State::new("LCD1", 2, 16, 0x3E, 100.0);
        assert_eq!(disp.rows, 2);
        assert_eq!(disp.cols, 16);
        assert_eq!(disp.control_code, 0x3E);

        // Turn display on: Command 0x0C (Display ON, cursor OFF, blink OFF)
        disp.hd44780.process_command(0x0C);
        assert!(disp.hd44780.disp_on);

        // Write 'H', 'e', 'l', 'l', 'o'
        for b in b"Hello" {
            disp.hd44780.write_data(*b);
        }

        let lines = disp.lines();
        assert_eq!(lines.len(), 2);
        assert!(lines[0].starts_with("Hello"));
        assert!(!disp.hex_bitmap().is_empty());
    }
}
