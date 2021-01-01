//! Hitachi HD44780 5x8 dot-matrix liquid crystal display controller.
//!
//! Handles 4-bit and 8-bit parallel interfacing with RS, RW, En, and D0-D7.
//! Maintains DDRAM (80 bytes) and CGRAM (64 bytes), cursor state, shift, and
//! generates display lines for canvas and UI rendering.

use super::pin::IoPin;

#[derive(Clone, Debug)]
pub struct Hd44780State {
    pub id: String,
    pub rows: usize,
    pub cols: usize,
    pub ddram: [u8; 80],
    pub cgram: [u8; 64],
    pub dd_addr: u8,
    pub cg_addr: u8,
    pub write_ddram: bool,
    pub shift_pos: usize,
    pub direction: i32,
    pub shift_disp: bool,
    pub disp_on: bool,
    pub cursor_on: bool,
    pub cursor_blink: bool,
    pub data_length: u8,
    pub line_length: usize,
    pub nibble: u8,
    pub input: u8,
    pub last_clock: bool,
    pub blinking: bool,
    pub last_circ_time: u64,
    pub pin_rs: IoPin,
    pub pin_rw: IoPin,
    pub pin_en: IoPin,
    pub data_pins: Vec<IoPin>,
}

impl Hd44780State {
    pub fn new(id: &str, rows: usize, cols: usize) -> Self {
        let rows = rows.clamp(1, 4);
        let cols = cols.clamp(8, 20);

        let init_pin = |name: &str| {
            let mut p = IoPin::input(format!("{id}-{name}"));
            p.set_levels(5.0, 0.0);
            p.set_thresholds(2.5, 2.5);
            p.set_pullup(1e5);
            p
        };

        let pin_rs = init_pin("PinRS");
        let pin_rw = init_pin("PinRW");
        let pin_en = init_pin("PinEn");
        let data_pins = (0..8).map(|i| init_pin(&format!("dataPin{i}"))).collect();

        let mut st = Self {
            id: id.to_string(),
            rows,
            cols,
            ddram: [b' '; 80],
            cgram: [0; 64],
            dd_addr: 0,
            cg_addr: 0,
            write_ddram: true,
            shift_pos: 0,
            direction: 1,
            shift_disp: false,
            disp_on: false,
            cursor_on: false,
            cursor_blink: false,
            data_length: 8,
            line_length: 80,
            nibble: 0,
            input: 0,
            last_clock: false,
            blinking: false,
            last_circ_time: 0,
            pin_rs,
            pin_rw,
            pin_en,
            data_pins,
        };
        st.clear_lcd();
        st
    }

    pub fn pin_ids(&self) -> Vec<String> {
        let mut ids = vec![
            self.pin_rs.id.clone(),
            self.pin_rw.id.clone(),
            self.pin_en.id.clone(),
        ];
        for p in &self.data_pins {
            ids.push(p.id.clone());
        }
        ids
    }

    pub fn pins(&self) -> Vec<&IoPin> {
        let mut p = vec![&self.pin_rs, &self.pin_rw, &self.pin_en];
        for d in &self.data_pins {
            p.push(d);
        }
        p
    }

    pub fn pins_mut(&mut self) -> Vec<&mut IoPin> {
        let mut p = vec![&mut self.pin_rs, &mut self.pin_rw, &mut self.pin_en];
        for d in &mut self.data_pins {
            p.push(d);
        }
        p
    }

    pub fn clear_lcd(&mut self) {
        self.clear_ddram();
        self.cursor_home();
    }

    pub fn cursor_home(&mut self) {
        self.dd_addr = 0;
        self.shift_pos = 0;
    }

    pub fn clear_ddram(&mut self) {
        for b in &mut self.ddram {
            *b = b' ';
        }
    }

    pub fn set_dd_addr(&mut self, mut addr: usize) {
        if self.line_length == 40 && addr > 63 {
            addr = addr.saturating_sub(24);
        }
        self.dd_addr = (addr & 0x7F) as u8;
        self.write_ddram = true;
    }

    pub fn set_cg_addr(&mut self, addr: usize) {
        self.cg_addr = (addr & 0x3F) as u8;
        self.write_ddram = false;
    }

    pub fn function_set(&mut self, data: u8) {
        self.data_length = if data & 0x10 != 0 { 8 } else { 4 };
        if data & 0x08 != 0 {
            if self.rows == 1 {
                self.rows = 2;
            }
            self.line_length = 40;
        } else {
            self.line_length = 80;
        }
    }

    pub fn cd_shift(&mut self, data: u8) {
        let dir: i32 = if data & 0x04 == 0 { -1 } else { 1 };
        if data & 0x08 != 0 {
            // Shift display
            let mut sp = self.shift_pos as i32 + dir;
            let end = self.line_length as i32 - 1;
            if sp > end {
                sp = 0;
            } else if sp < 0 {
                sp = end;
            }
            self.shift_pos = sp as usize;
        } else {
            // Move cursor
            let mut addr = self.dd_addr as i32 + dir;
            if addr > 79 {
                addr = 0;
            } else if addr < 0 {
                addr = 79;
            }
            self.dd_addr = addr as u8;
        }
    }

    pub fn disp_control(&mut self, data: u8) {
        self.disp_on = (data & 0x04) != 0;
        self.cursor_on = (data & 0x02) != 0;
        self.cursor_blink = (data & 0x01) != 0;
    }

    pub fn entry_mode(&mut self, data: u8) {
        self.direction = if data & 0x02 != 0 { 1 } else { -1 };
        self.shift_disp = (data & 0x01) != 0;
    }

    pub fn process_command(&mut self, cmd: u8) {
        if cmd == 0 {
            return;
        }
        if cmd & 0x80 != 0 {
            self.set_dd_addr((cmd & 0x7F) as usize);
        } else if cmd & 0x40 != 0 {
            self.set_cg_addr((cmd & 0x3F) as usize);
        } else if cmd & 0x20 != 0 {
            self.function_set(cmd);
        } else if cmd & 0x10 != 0 {
            self.cd_shift(cmd);
        } else if cmd & 0x08 != 0 {
            self.disp_control(cmd);
        } else if cmd & 0x04 != 0 {
            self.entry_mode(cmd);
        } else if cmd & 0x02 != 0 {
            self.cursor_home();
        } else if cmd & 0x01 != 0 {
            self.clear_lcd();
        }
    }

    pub fn write_data(&mut self, data: u8) {
        if self.write_ddram {
            let addr = self.dd_addr as usize;
            if addr < 80 {
                self.ddram[addr] = data;
            }
            let mut next = self.dd_addr as i32 + self.direction;
            if next > 79 {
                next = 0;
            } else if next < 0 {
                next = 79;
            }
            self.dd_addr = next as u8;

            if self.shift_disp {
                let mut sp = self.shift_pos as i32 + self.direction;
                let end = self.line_length as i32 - 1;
                if sp > end {
                    sp = 0;
                } else if sp < 0 {
                    sp = end;
                }
                self.shift_pos = sp as usize;
            }
        } else {
            let addr = self.cg_addr as usize;
            if addr < 64 {
                self.cgram[addr] = data;
            }
            self.cg_addr = (self.cg_addr + 1) % 64;
        }
    }

    /// Step simulation on clock / picosecond event.
    pub fn step(&mut self, circ_time: u64) {
        // Blink timer (period ~409.6 ms = 409_600_000_000 ps)
        if circ_time.saturating_sub(self.last_circ_time) >= 409_600_000_000 {
            self.last_circ_time = circ_time;
            if self.cursor_blink {
                self.blinking = !self.blinking;
            } else {
                self.blinking = false;
            }
        }

        let clk_high = self.pin_en.inp_state();
        if clk_high {
            self.last_clock = true;
            return;
        }
        if !self.last_clock {
            return; // Not a falling edge
        }
        self.last_clock = false;

        // Falling edge: Sample inputs
        if self.data_length == 8 {
            let mut val = 0u8;
            for (i, p) in self.data_pins.iter().enumerate() {
                if p.inp_state() {
                    val |= 1 << i;
                }
            }
            self.input = val;
        } else {
            // 4-bit mode using D4..D7
            if self.nibble == 0 {
                let mut val = 0u8;
                for i in 4..8 {
                    if self.data_pins[i].inp_state() {
                        val |= 1 << i;
                    }
                }
                self.input = val;
                self.nibble = 1;
                return;
            } else {
                for i in 4..8 {
                    if self.data_pins[i].inp_state() {
                        self.input |= 1 << (i - 4);
                    }
                }
                self.nibble = 0;
            }
        }

        if !self.pin_rw.inp_state() {
            // Write cycle
            let inp = self.input;
            if self.pin_rs.inp_state() {
                self.write_data(inp);
            } else {
                self.process_command(inp);
            }
        }
    }

    /// Returns the 8-row 5-dot bit pattern for a character code.
    /// If `char_code < 8`, resolves from custom CGRAM.
    /// Otherwise, resolves from `HD44780_ROM`.
    pub fn char_pattern(&self, char_code: u8) -> [u8; 8] {
        super::hd44780_font::char_pattern(&self.cgram, char_code)
    }

    /// Computes the active 5x8 dot patterns for all visible characters,
    /// taking into account DDRAM content, display shifting, custom CGRAM glyphs,
    /// cursor underline (`cursor_on`), and cursor blinking (`cursor_blink` / `blinking`).
    ///
    /// Returns a vector of rows, each containing a vector of `[u8; 8]` patterns per column.
    pub fn visible_char_patterns(&self) -> Vec<Vec<[u8; 8]>> {
        let mut rows = Vec::with_capacity(self.rows);
        for row in 0..self.rows {
            let mut cols = Vec::with_capacity(self.cols);
            for col in 0..self.cols {
                if !self.disp_on {
                    cols.push([0u8; 8]);
                    continue;
                }

                let mem_pos = if row < 2 {
                    row * 40 + col
                } else {
                    (row - 2) * 40 + 20 + col
                };

                let (line_start, line_end) = if self.line_length == 40 {
                    if mem_pos < 40 { (0, 39) } else { (40, 79) }
                } else {
                    (0, 79)
                };

                let mut pos = mem_pos as i32 + self.shift_pos as i32;
                if pos > line_end {
                    pos -= self.line_length as i32;
                }
                if pos < line_start {
                    pos += self.line_length as i32;
                }

                let ch = if (0..80).contains(&pos) {
                    self.ddram[pos as usize]
                } else {
                    b' '
                };

                let mut pattern = self.char_pattern(ch);

                // Cursor and blinking at current DDRAM address
                if pos == self.dd_addr as i32 {
                    if self.cursor_blink && self.blinking {
                        pattern = [0x1F; 8];
                    } else if self.cursor_on {
                        pattern[7] |= 0x1F;
                    }
                }

                cols.push(pattern);
            }
            rows.push(cols);
        }
        rows
    }

    /// Encodes the visible character patterns into a hex string for QML/UI.
    /// Format: for each character from top-left to bottom-right, 8 bytes
    /// formatted as 16 uppercase hex characters (2 hex chars per row byte).
    pub fn hex_bitmap(&self) -> String {
        if !self.disp_on {
            return String::new();
        }
        let patterns = self.visible_char_patterns();
        let mut hex = String::with_capacity(self.rows * self.cols * 16);
        use std::fmt::Write;
        for row in &patterns {
            for pat in row {
                for byte in pat {
                    let _ = write!(hex, "{:02X}", byte & 0x1F);
                }
            }
        }
        hex
    }

    /// Returns the visible characters for each row (formatted strings).
    pub fn lines(&self) -> Vec<String> {
        let mut lines = Vec::with_capacity(self.rows);
        for row in 0..self.rows {
            let mut line = String::with_capacity(self.cols);
            for col in 0..self.cols {
                let mem_pos = if row < 2 {
                    row * 40 + col
                } else {
                    (row - 2) * 40 + 20 + col
                };

                let (line_start, line_end) = if self.line_length == 40 {
                    if mem_pos < 40 { (0, 39) } else { (40, 79) }
                } else {
                    (0, 79)
                };

                let mut pos = mem_pos as i32 + self.shift_pos as i32;
                if pos > line_end {
                    pos -= self.line_length as i32;
                }
                if pos < line_start {
                    pos += self.line_length as i32;
                }

                let ch = if (0..80).contains(&pos) {
                    self.ddram[pos as usize]
                } else {
                    b' '
                };

                if (32..=126).contains(&ch) {
                    line.push(ch as char);
                } else if ch == 0 {
                    line.push(' ');
                } else {
                    line.push(ch as char);
                }
            }
            lines.push(line);
        }
        lines
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_char_pattern_and_hex_bitmap() {
        let mut hd = Hd44780State::new("LCD1", 2, 16);
        hd.disp_on = true;
        // Write 'A' at pos 0
        hd.ddram[0] = b'A';
        let patterns = hd.visible_char_patterns();
        assert_eq!(patterns.len(), 2);
        assert_eq!(patterns[0].len(), 16);
        // 'A' pattern: [0x0E, 0x11, 0x11, 0x11, 0x1F, 0x11, 0x11, 0x00]
        assert_eq!(
            patterns[0][0],
            [0x0E, 0x11, 0x11, 0x11, 0x1F, 0x11, 0x11, 0x00]
        );
        // Space pattern for second char
        assert_eq!(patterns[0][1], [0x00; 8]);

        let hex = hd.hex_bitmap();
        assert_eq!(hex.len(), 2 * 16 * 16);
        assert!(hex.starts_with("0E1111111F111100"));
    }

    #[test]
    fn test_custom_cgram_and_cursor() {
        let mut hd = Hd44780State::new("LCD1", 2, 16);
        hd.disp_on = true;
        // CGRAM char 0
        hd.cgram[0..8].copy_from_slice(&[0x01, 0x03, 0x07, 0x0F, 0x1F, 0x0F, 0x07, 0x03]);
        hd.ddram[0] = 0; // custom char 0
        hd.dd_addr = 0;
        hd.cursor_on = true;

        let patterns = hd.visible_char_patterns();
        // Row 7 had 0x03, with cursor underline (0x1F) it should become 0x1F
        assert_eq!(patterns[0][0][7], 0x1F);
    }
}
