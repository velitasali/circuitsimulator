//! KS0108 128x64 dual-chip parallel graphic LCD controller (C++ `Ks0108`).

use super::pin::IoPin;

pub const WIDTH: usize = 128;
pub const HALF_WIDTH: usize = 64;
#[allow(dead_code)]
pub const HEIGHT: usize = 64;
pub const PAGES: usize = 8;

#[derive(Clone, Debug)]
pub struct Ks0108State {
    pub id: String,
    pub cs_act_low: bool,
    pub pin_rst: IoPin,
    pub pin_cs1: IoPin,
    pub pin_cs2: IoPin,
    pub pin_en: IoPin,
    pub pin_rw: IoPin,
    pub pin_dc: IoPin,
    pub data_pins: Vec<IoPin>,
    pub left_ram: [[u8; HALF_WIDTH]; PAGES],
    pub right_ram: [[u8; HALF_WIDTH]; PAGES],
    pub left_page: usize,
    pub left_col: usize,
    pub right_page: usize,
    pub right_col: usize,
    pub left_start_line: usize,
    pub right_start_line: usize,
    pub disp_on_1: bool,
    pub disp_on_2: bool,
    pub last_en: bool,
}

impl Ks0108State {
    pub fn new(id: impl Into<String>, cs_act_low: bool) -> Self {
        let id_str = id.into();
        let make_pin = |suffix: &str| {
            let mut p = IoPin::input(format!("{id_str}-{suffix}"));
            p.set_levels(5.0, 0.0);
            p.set_thresholds(2.5, 2.5);
            p.set_pullup(1e5);
            p
        };

        let pin_rst = make_pin("PinRst");
        let pin_cs1 = make_pin("PinCs1");
        let pin_cs2 = make_pin("PinCs2");
        let pin_en = make_pin("PinEn");
        let pin_rw = make_pin("PinRW");
        let pin_dc = make_pin("PinDC");

        let data_pins = (0..8).map(|i| make_pin(&format!("dataPin{i}"))).collect();

        let mut st = Self {
            id: id_str,
            cs_act_low,
            pin_rst,
            pin_cs1,
            pin_cs2,
            pin_en,
            pin_rw,
            pin_dc,
            data_pins,
            left_ram: [[0u8; HALF_WIDTH]; PAGES],
            right_ram: [[0u8; HALF_WIDTH]; PAGES],
            left_page: 0,
            left_col: 0,
            right_page: 0,
            right_col: 0,
            left_start_line: 0,
            right_start_line: 0,
            disp_on_1: false,
            disp_on_2: false,
            last_en: false,
        };
        st.reset();
        st
    }

    pub fn pin_ids(&self) -> Vec<String> {
        let mut ids = vec![
            self.pin_rst.id.clone(),
            self.pin_cs1.id.clone(),
            self.pin_cs2.id.clone(),
            self.pin_en.id.clone(),
            self.pin_rw.id.clone(),
            self.pin_dc.id.clone(),
        ];
        for d in &self.data_pins {
            ids.push(d.id.clone());
        }
        ids
    }

    pub fn pins(&self) -> Vec<&IoPin> {
        let mut p = vec![
            &self.pin_rst,
            &self.pin_cs1,
            &self.pin_cs2,
            &self.pin_en,
            &self.pin_rw,
            &self.pin_dc,
        ];
        for d in &self.data_pins {
            p.push(d);
        }
        p
    }

    pub fn pins_mut(&mut self) -> Vec<&mut IoPin> {
        let mut p = vec![
            &mut self.pin_rst,
            &mut self.pin_cs1,
            &mut self.pin_cs2,
            &mut self.pin_en,
            &mut self.pin_rw,
            &mut self.pin_dc,
        ];
        for d in &mut self.data_pins {
            p.push(d);
        }
        p
    }

    pub fn reset(&mut self) {
        self.disp_on_1 = false;
        self.disp_on_2 = false;
        self.left_page = 0;
        self.left_col = 0;
        self.right_page = 0;
        self.right_col = 0;
        self.left_start_line = 0;
        self.right_start_line = 0;
        self.clear_ram();
    }

    pub fn clear_ram(&mut self) {
        for page in &mut self.left_ram {
            page.fill(0);
        }
        for page in &mut self.right_ram {
            page.fill(0);
        }
    }

    pub fn process_command(&mut self, cmd: u8, cs1: bool, cs2: bool) {
        if (cmd & 0xFE) == 0x3E {
            let on = (cmd & 0x01) != 0;
            if cs1 {
                self.disp_on_1 = on;
            }
            if cs2 {
                self.disp_on_2 = on;
            }
        } else if (cmd & 0xC0) == 0x40 {
            // Set Y address (column 0..63)
            let col = (cmd & 0x3F) as usize;
            if cs1 {
                self.left_col = col;
            }
            if cs2 {
                self.right_col = col;
            }
        } else if (cmd & 0xF8) == 0xB8 {
            // Set X address (page 0..7)
            let page = (cmd & 0x07) as usize;
            if cs1 {
                self.left_page = page;
            }
            if cs2 {
                self.right_page = page;
            }
        } else if (cmd & 0xC0) == 0xC0 {
            // Set Z address (start line 0..63)
            let line = (cmd & 0x3F) as usize;
            if cs1 {
                self.left_start_line = line;
            }
            if cs2 {
                self.right_start_line = line;
            }
        }
    }

    pub fn write_data(&mut self, val: u8, cs1: bool, cs2: bool) {
        if cs1 {
            if self.left_page < PAGES && self.left_col < HALF_WIDTH {
                self.left_ram[self.left_page][self.left_col] = val;
                self.left_col = (self.left_col + 1) & 0x3F;
            }
        }
        if cs2 {
            if self.right_page < PAGES && self.right_col < HALF_WIDTH {
                self.right_ram[self.right_page][self.right_col] = val;
                self.right_col = (self.right_col + 1) & 0x3F;
            }
        }
    }

    pub fn hex_bitmap(&self) -> String {
        if !self.disp_on_1 && !self.disp_on_2 {
            return String::new();
        }

        let mut hex = String::with_capacity(PAGES * WIDTH * 2);
        use std::fmt::Write;

        for page in 0..PAGES {
            for col in 0..HALF_WIDTH {
                let b = if self.disp_on_1 {
                    self.left_ram[page][col]
                } else {
                    0
                };
                let _ = write!(hex, "{:02X}", b);
            }
            for col in 0..HALF_WIDTH {
                let b = if self.disp_on_2 {
                    self.right_ram[page][col]
                } else {
                    0
                };
                let _ = write!(hex, "{:02X}", b);
            }
        }

        hex
    }

    pub fn step(&mut self, _circ_time: u64) {
        if !self.pin_rst.inp_state() {
            self.reset();
            return;
        }

        let en = self.pin_en.inp_state();
        if !en && self.last_en {
            // Falling edge (write cycle)
            self.last_en = false;
            let write = !self.pin_rw.inp_state();
            if write {
                let mut val = 0u8;
                for (i, p) in self.data_pins.iter().enumerate() {
                    if p.inp_state() {
                        val |= 1 << i;
                    }
                }

                let mut cs1 = self.pin_cs1.inp_state();
                let mut cs2 = self.pin_cs2.inp_state();
                if !cs1 && !cs2 {
                    cs2 = true;
                }
                if self.cs_act_low {
                    cs1 = !cs1;
                    cs2 = !cs2;
                }

                let is_data = self.pin_dc.inp_state();
                if is_data {
                    self.write_data(val, cs1, cs2);
                } else {
                    self.process_command(val, cs1, cs2);
                }
            }
        } else if en {
            self.last_en = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ks0108_dual_half_write() {
        let mut disp = Ks0108State::new("GLCD1", false);
        assert_eq!(disp.left_page, 0);
        assert_eq!(disp.left_col, 0);

        // Turn on display for both halves
        disp.process_command(0x3F, true, true);
        assert!(disp.disp_on_1);
        assert!(disp.disp_on_2);

        // Write to left half
        disp.write_data(0xA5, true, false);
        assert_eq!(disp.left_ram[0][0], 0xA5);
        assert_eq!(disp.left_col, 1);

        let hex = disp.hex_bitmap();
        assert!(!hex.is_empty());
        assert!(hex.starts_with("A5"));
    }
}
