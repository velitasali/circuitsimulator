//! PCD8544 (Nokia 5110) 84x48 SPI monochrome LCD controller (C++ `Pcd8544`).

use super::pin::IoPin;

pub const WIDTH: usize = 84;
#[allow(dead_code)]
pub const HEIGHT: usize = 48;
pub const PAGES: usize = 6;

#[derive(Clone, Debug)]
pub struct Pcd8544State {
    pub id: String,
    pub contrast: u8,
    pub bias: u8,
    pub pin_rst: IoPin,
    pub pin_cs: IoPin,
    pub pin_dc: IoPin,
    pub pin_si: IoPin,
    pub pin_scl: IoPin,
    pub ram: [[u8; WIDTH]; PAGES],
    pub addr_x: usize,
    pub addr_y: usize,
    pub in_buf: u8,
    pub in_bit: usize,
    pub last_scl: bool,
    pub pd: bool,
    pub v_mode: bool,
    pub h_mode: bool,
    pub d_bit: bool,
    pub e_bit: bool,
}

impl Pcd8544State {
    pub fn new(id: impl Into<String>, contrast: u8, bias: u8) -> Self {
        let id_str = id.into();
        let make_pin = |suffix: &str| {
            let mut p = IoPin::input(format!("{id_str}-{suffix}"));
            p.set_levels(5.0, 0.0);
            p.set_thresholds(2.5, 2.5);
            p.set_pullup(1e5);
            p
        };

        let pin_rst = make_pin("PinRst");
        let pin_cs = make_pin("PinCs");
        let pin_dc = make_pin("PinDc");
        let pin_si = make_pin("PinSi");
        let pin_scl = make_pin("PinScl");

        let mut st = Self {
            id: id_str,
            contrast,
            bias,
            pin_rst,
            pin_cs,
            pin_dc,
            pin_si,
            pin_scl,
            ram: [[0u8; WIDTH]; PAGES],
            addr_x: 0,
            addr_y: 0,
            in_buf: 0,
            in_bit: 0,
            last_scl: false,
            pd: true,
            v_mode: false,
            h_mode: false,
            d_bit: false,
            e_bit: false,
        };
        st.reset();
        st
    }

    pub fn pin_ids(&self) -> Vec<String> {
        vec![
            self.pin_rst.id.clone(),
            self.pin_cs.id.clone(),
            self.pin_dc.id.clone(),
            self.pin_si.id.clone(),
            self.pin_scl.id.clone(),
        ]
    }

    pub fn pins(&self) -> Vec<&IoPin> {
        vec![
            &self.pin_rst,
            &self.pin_cs,
            &self.pin_dc,
            &self.pin_si,
            &self.pin_scl,
        ]
    }

    pub fn pins_mut(&mut self) -> Vec<&mut IoPin> {
        vec![
            &mut self.pin_rst,
            &mut self.pin_cs,
            &mut self.pin_dc,
            &mut self.pin_si,
            &mut self.pin_scl,
        ]
    }

    pub fn reset(&mut self) {
        self.in_buf = 0;
        self.in_bit = 0;
        self.addr_x = 0;
        self.addr_y = 0;
        self.pd = true;
        self.v_mode = false;
        self.h_mode = false;
        self.d_bit = false;
        self.e_bit = false;
        self.last_scl = false;
    }

    pub fn clear_ram(&mut self) {
        for page in &mut self.ram {
            page.fill(0);
        }
    }

    pub fn increment_pointer(&mut self) {
        if self.v_mode {
            self.addr_y += 1;
            if self.addr_y >= PAGES {
                self.addr_y = 0;
                self.addr_x += 1;
            }
            if self.addr_x >= WIDTH {
                self.addr_x = 0;
            }
        } else {
            self.addr_x += 1;
            if self.addr_x >= WIDTH {
                self.addr_x = 0;
                self.addr_y += 1;
            }
            if self.addr_y >= PAGES {
                self.addr_y = 0;
            }
        }
    }

    pub fn write_data(&mut self, data: u8) {
        if self.addr_y < PAGES && self.addr_x < WIDTH {
            self.ram[self.addr_y][self.addr_x] = data;
            self.increment_pointer();
        }
    }

    pub fn process_command(&mut self, cmd: u8) {
        if (cmd & 0xF8) == 0x20 {
            // Function set
            self.h_mode = (cmd & 0x01) != 0;
            self.v_mode = (cmd & 0x02) != 0;
            self.pd = (cmd & 0x04) != 0;
        } else if !self.h_mode {
            // Basic instruction set
            if (cmd & 0xFA) == 0x08 {
                // Display control
                self.d_bit = (cmd & 0x04) != 0;
                self.e_bit = (cmd & 0x01) != 0;
            } else if (cmd & 0xF8) == 0x40 {
                // Set Y address
                let y = (cmd & 0x07) as usize;
                if y < PAGES {
                    self.addr_y = y;
                }
            } else if (cmd & 0x80) == 0x80 {
                // Set X address
                let x = (cmd & 0x7F) as usize;
                if x < WIDTH {
                    self.addr_x = x;
                }
            }
        }
    }

    pub fn hex_bitmap(&self) -> String {
        if self.pd || (!self.d_bit && !self.e_bit) {
            return String::new();
        }

        let mut hex = String::with_capacity(PAGES * WIDTH * 2);
        use std::fmt::Write;

        let all_on = !self.d_bit && self.e_bit;
        let inverted = self.d_bit && self.e_bit;

        for page in 0..PAGES {
            for col in 0..WIDTH {
                let mut b = if all_on { 0xFF } else { self.ram[page][col] };
                if inverted {
                    b = !b;
                }
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

        if self.pin_cs.inp_state() {
            self.in_buf = 0;
            self.in_bit = 0;
            return;
        }

        let scl = self.pin_scl.inp_state();
        if scl && !self.last_scl {
            // Rising edge
            self.last_scl = true;
            let bit = if self.pin_si.inp_state() { 1 } else { 0 };
            self.in_buf = (self.in_buf << 1) | bit;
            self.in_bit += 1;

            if self.in_bit == 8 {
                let val = self.in_buf;
                if self.pin_dc.inp_state() {
                    self.write_data(val);
                } else {
                    self.process_command(val);
                }
                self.in_bit = 0;
                self.in_buf = 0;
            }
        } else if !scl {
            self.last_scl = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pcd8544_command_and_data() {
        let mut disp = Pcd8544State::new("LCD1", 50, 4);
        assert_eq!(disp.addr_x, 0);
        assert_eq!(disp.addr_y, 0);

        // Wake up / normal mode: Function set 0x20 (pd=0, v=0, h=0)
        disp.process_command(0x20);
        assert!(!disp.pd);

        // Display control 0x0C (normal mode D=1, E=0)
        disp.process_command(0x0C);
        assert!(disp.d_bit);
        assert!(!disp.e_bit);

        // Write byte 0x55
        disp.write_data(0x55);
        assert_eq!(disp.ram[0][0], 0x55);
        assert_eq!(disp.addr_x, 1);

        let hex = disp.hex_bitmap();
        assert!(!hex.is_empty());
        assert!(hex.starts_with("55"));
    }
}
