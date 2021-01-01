//! SSD1306 OLED graphical display controller (C++ `Ssd1306` + `OledController`).

use super::PinMode;
use super::pin::IoPin;
use super::twi::{TwiModule, TwiPins, TwiState};

pub const HORI_ADDR_MODE: u8 = 0;
pub const VERT_ADDR_MODE: u8 = 1;
pub const PAGE_ADDR_MODE: u8 = 2;

#[derive(Clone, Debug)]
pub struct Ssd1306State {
    pub id: String,
    pub width: usize,
    pub height: usize,
    pub rows: usize,
    pub color: String,
    pub rotate: bool,
    pub control_code: u8,
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
    // Scrolling
    pub scroll: bool,
    pub scroll_v: bool,
    pub scroll_dir: u8,
    pub scroll_single: bool,
    pub scroll_start_y: usize,
    pub scroll_end_y: usize,
    pub scroll_step: u16,
    pub scroll_count: u16,
    pub v_scroll_offset: u8,
    pub scroll_top: u8,
    pub scroll_rows: u8,
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

impl Ssd1306State {
    pub fn new(
        id: impl Into<String>,
        width: usize,
        height: usize,
        control_code: u8,
        color: &str,
        rotate: bool,
        freq_khz: f64,
    ) -> Self {
        let id_str = id.into();
        let w = width.clamp(32, 128);
        let h = ((height.clamp(16, 64) / 8) * 8).max(16);
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
            color: color.to_string(),
            rotate,
            control_code,
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
            scroll: false,
            scroll_v: false,
            scroll_dir: 0,
            scroll_single: false,
            scroll_start_y: 0,
            scroll_end_y: 7,
            scroll_step: 5,
            scroll_count: 0,
            v_scroll_offset: 0,
            scroll_top: 0,
            scroll_rows: 0,
            disp_offset: 0,
            ram_offset: 0,
            disp_on: false,
            disp_full: false,
            disp_inv: false,
            scan_inv: false,
            remap: false,
            mr: 63,
            start: true,
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

    pub fn reset(&mut self) {
        self.mr = 63;
        self.addr_x = 0;
        self.addr_y = 0;
        self.start_x = 0;
        self.end_x = self.width.saturating_sub(1);
        self.start_y = 0;
        self.end_y = self.rows.saturating_sub(1);

        self.scroll = false;
        self.scroll_v = false;
        self.scroll_dir = 0;
        self.scroll_single = false;
        self.scroll_start_y = 0;
        self.scroll_end_y = 7;
        self.scroll_step = 5;
        self.scroll_count = 0;
        self.v_scroll_offset = 0;

        self.disp_offset = 0;
        self.ram_offset = 0;
        self.read_bytes = 0;

        self.disp_on = false;
        self.disp_full = false;
        self.disp_inv = false;
        self.scan_inv = false;
        self.remap = false;

        self.addr_mode = PAGE_ADDR_MODE;
        self.start = true;
        self.data = false;
        self.co = false;
    }

    pub fn clear_ddram(&mut self) {
        for col in &mut self.ddram {
            for cell in col {
                *cell = 0;
            }
        }
    }

    pub fn initialize(&mut self) {
        self.clear_ddram();
        self.reset();
        self.twi.set_slave(self.control_code);
    }

    pub fn read_byte(&mut self, val: u8) {
        if self.start {
            if (val & 0b00111111) != 0 {
                // Reserved bits [5:0] are non-zero: treat as data
                self.start = false;
                self.write_data(val);
            } else {
                self.start = false;
                self.co = (val & 0b10000000) != 0;
                self.data = (val & 0b01000000) != 0;
            }
        } else if self.data {
            self.write_data(val);
        } else if self.read_bytes > 0 {
            self.parameter(val);
        } else {
            self.process_command(val);
        }

        if self.read_bytes == 0 {
            self.start = self.co;
        }
    }

    pub fn process_command(&mut self, val: u8) {
        self.last_command = val;
        self.read_index = 0;
        self.read_bytes = 0;

        if val < 0x20 {
            if self.addr_mode != PAGE_ADDR_MODE {
                return;
            }
            if val < 0x10 {
                self.addr_x = (self.addr_x & 0xF0) | (val as usize & 0x0F);
            } else {
                self.addr_x = (self.addr_x & 0x0F) | ((val as usize & 0x0F) << 4);
            }
            if self.addr_x >= self.width {
                self.addr_x %= self.width.max(1);
            }
        } else if (0x40..=0x7F).contains(&val) {
            self.ram_offset = val & self.line_mask;
        } else if (0xB0..=0xB7).contains(&val) {
            if self.addr_mode == PAGE_ADDR_MODE {
                self.addr_y = (val & self.row_mask) as usize;
            }
        } else {
            match val {
                0x20 => self.read_bytes = 1,        // Memory Addressing Mode
                0x21 => self.read_bytes = 2,        // Column Address (Start-End)
                0x22 => self.read_bytes = 2,        // Page Address (Start-End)
                0x23 => self.read_bytes = 1,        // Fade Out / Blinking Mode
                0x26 | 0x27 => self.read_bytes = 6, // Continuous Horizontal Scroll Setup
                0x29 | 0x2A => self.read_bytes = 5, // Continuous Vertical and Horizontal Scroll Setup
                0x2C | 0x2D => self.read_bytes = 6, // One Column Horizontal Scroll Setup
                0x2E => self.scroll = false,        // Deactivate scroll
                0x2F => {
                    self.scroll = true; // Activate scroll
                    self.scroll_count = 0;
                }
                0x81 => self.read_bytes = 1,    // Contrast Control
                0x8D => self.read_bytes = 1,    // Charge Pump
                0xA0 => self.remap = false,     // Segment Re-map OFF
                0xA1 => self.remap = true,      // Segment Re-map ON
                0xA3 => self.read_bytes = 2,    // Vertical Scroll Area
                0xA4 => self.disp_full = false, // Entire Display Off
                0xA5 => self.disp_full = true,  // Entire Display ON
                0xA6 => self.disp_inv = false,  // Inverse Display OFF
                0xA7 => self.disp_inv = true,   // Inverse Display ON
                0xA8 => self.read_bytes = 1,    // Multiplex Ratio
                0xAE => self.disp_on = false,   // Display OFF
                0xAF => self.disp_on = true,    // Display ON
                0xC0 => self.scan_inv = false,  // COM Output Scan Inverted OFF
                0xC8 => self.scan_inv = true,   // COM Output Scan Inverted ON
                0xD3 => self.read_bytes = 1,    // Display Offset
                0xD5 => self.read_bytes = 1,    // Display Clock Divide Ratio
                0xD6 => self.read_bytes = 1,    // Zoom in Mode
                0xD9 => self.read_bytes = 1,    // Precharge
                0xDA => self.read_bytes = 1,    // COM Pins Hardware Configuration
                0xDB => self.read_bytes = 1,    // VCOM DETECT
                _ => {}
            }
        }
    }

    pub fn parameter(&mut self, val: u8) {
        self.read_index += 1;
        if self.read_index > self.read_bytes {
            return;
        }
        if self.read_index == self.read_bytes {
            self.read_bytes = 0;
        }

        match self.last_command {
            0x20 => {
                self.addr_mode = val & 3;
            }
            0x21 => {
                if self.addr_mode != PAGE_ADDR_MODE {
                    if self.read_index == 1 {
                        self.addr_x = (val & 0x7F) as usize;
                        self.start_x = self.addr_x;
                    } else {
                        self.end_x = (val & 0x7F) as usize;
                    }
                }
            }
            0x22 => {
                if self.addr_mode != PAGE_ADDR_MODE {
                    if self.read_index == 1 {
                        self.addr_y = (val & self.row_mask) as usize;
                        self.start_y = self.addr_y;
                    } else {
                        self.end_y = (val & self.row_mask) as usize;
                    }
                }
            }
            0x26 | 0x27 | 0x29 | 0x2A => {
                self.config_scroll(self.last_command, val);
            }
            0x2C | 0x2D => {
                self.scroll_single = true;
                self.config_scroll(self.last_command - 6, val);
            }
            0xA3 => match self.read_index {
                1 => self.scroll_top = val & self.line_mask,
                2 => self.scroll_rows = val & 0x7F,
                _ => {}
            },
            0xA8 => {
                let mux_ratio = val & self.line_mask;
                if mux_ratio > 14 {
                    self.mr = mux_ratio;
                }
            }
            0xD3 => {
                self.disp_offset = val & self.line_mask;
            }
            _ => {}
        }
    }

    fn config_scroll(&mut self, cmd: u8, val: u8) {
        self.scroll_v = cmd > 0x27;
        self.scroll_dir = cmd & 0b11;

        match self.read_index {
            2 => self.scroll_start_y = (val & self.row_mask) as usize,
            3 => {
                self.scroll_step = match val & self.row_mask {
                    0 => 5,
                    1 => 64,
                    2 => 128,
                    3 => 256,
                    4 => 3,
                    5 => 4,
                    6 => 25,
                    _ => 2,
                };
            }
            4 => self.scroll_end_y = (val & self.row_mask) as usize,
            5 => self.v_scroll_offset = val & self.line_mask,
            _ => {}
        }
    }

    pub fn write_data(&mut self, val: u8) {
        if self.addr_x < self.width && self.addr_y < self.rows {
            self.ddram[self.addr_x][self.addr_y] = val;
        }

        if (self.addr_mode & VERT_ADDR_MODE) != 0 {
            self.addr_y += 1;
            if self.addr_y > self.end_y {
                self.addr_y = self.start_y;
                if self.addr_mode != VERT_ADDR_MODE {
                    return;
                }
                self.addr_x += 1;
                if self.addr_x > self.end_x {
                    self.addr_x = self.start_x;
                }
            }
        } else {
            self.addr_x += 1;
            if self.addr_x > self.end_x {
                self.addr_x = self.start_x;
                if self.addr_mode != HORI_ADDR_MODE {
                    return;
                }
                self.addr_y += 1;
                if self.addr_y > self.end_y {
                    self.addr_y = self.start_y;
                }
            }
        }
    }

    pub fn i2c_stop(&mut self) {
        self.start = true;
        self.data = false;
        self.co = false;
    }

    pub fn start_write(&mut self) {
        if !self.data || self.co {
            self.start = true;
        }
    }

    pub fn on_twi_state(&mut self, state: TwiState) {
        if state == TwiState::NoState {
            self.i2c_stop();
        }
    }

    /// Step simulation on pin change directly without pin vector allocation.
    pub fn volt_changed_direct(&mut self) {
        let tick = self
            .twi
            .volt_changed_pins(&mut self.pin_scl, &mut self.pin_sda);
        if tick.stopped {
            self.i2c_stop();
        }
        if tick.need_start_write {
            self.start_write();
        }
        if let Some(b) = tick.byte_received {
            self.read_byte(b);
        }
        if let Some(st) = tick.state {
            self.on_twi_state(st);
        }
    }

    /// Step simulation on pin change.
    pub fn volt_changed(&mut self, pins: &mut [IoPin], scl_idx: usize, sda_idx: usize) {
        let map = TwiPins {
            scl: Some(scl_idx),
            sda: Some(sda_idx),
        };
        let tick = self.twi.volt_changed(pins, map);
        if tick.stopped {
            self.i2c_stop();
        }
        if tick.need_start_write {
            self.start_write();
        }
        if let Some(b) = tick.byte_received {
            self.read_byte(b);
        }
        if let Some(st) = tick.state {
            self.on_twi_state(st);
        }
    }

    /// Render 1-bit monochrome pixels buffer `width x height`.
    pub fn render_pixels(&self) -> Vec<bool> {
        let mut pixels = vec![false; self.width * self.height];
        if !self.disp_on {
            return pixels;
        }
        if self.disp_full {
            pixels.fill(true);
            return pixels;
        }

        for col in 0..self.width {
            for row in 0..self.rows {
                let mut ram_y = row * 8;
                if self.ram_offset != 0 {
                    ram_y += self.ram_offset as usize;
                    if ram_y >= self.height {
                        ram_y -= self.height;
                    }
                }
                if ram_y > self.mr as usize {
                    continue;
                }

                let row_byte = ram_y / 8;
                let mut byte0 = self
                    .ddram
                    .get(col)
                    .and_then(|c| c.get(row_byte))
                    .copied()
                    .unwrap_or(0);
                if self.disp_inv {
                    byte0 = !byte0;
                }

                let start_bit = ram_y % 8;
                let mut byte1 = 0u8;
                if start_bit != 0 {
                    let next_row = if row_byte + 1 < self.rows {
                        row_byte + 1
                    } else {
                        0
                    };
                    byte1 = self
                        .ddram
                        .get(col)
                        .and_then(|c| c.get(next_row))
                        .copied()
                        .unwrap_or(0);
                    if self.disp_inv {
                        byte1 = !byte1;
                    }
                }

                let mut dy = row * 8;
                if self.disp_offset != 0 {
                    dy += self.disp_offset as usize;
                    if dy >= self.height {
                        dy -= self.height;
                    }
                }

                for bit in start_bit..(start_bit + 8) {
                    let pixel = if bit < 8 {
                        (byte0 & (1 << bit)) != 0
                    } else {
                        (byte1 & (1 << (bit - 8))) != 0
                    };

                    if pixel {
                        let mut screen_y = if self.scan_inv {
                            self.height - 1 - dy
                        } else {
                            dy
                        };
                        let mut screen_x = if self.remap {
                            self.width - 1 - col
                        } else {
                            col
                        };
                        if self.rotate {
                            screen_y = self.height - 1 - screen_y;
                            screen_x = self.width - 1 - screen_x;
                        }
                        if screen_x < self.width && screen_y < self.height {
                            pixels[screen_y * self.width + screen_x] = true;
                        }
                    }
                    dy += 1;
                    if dy >= self.height {
                        dy -= self.height;
                    }
                }
            }
        }
        pixels
    }

    /// Ticking update step for hardware scrolling.
    pub fn update_step(&mut self) {
        if !self.scroll_single && !self.scroll {
            return;
        }

        if self.scroll_single {
            self.scroll_single = false;
        } else {
            self.scroll_count += 1;
            if self.scroll_count < self.scroll_step {
                return;
            }
            self.scroll_count = 0;
        }

        let max_x = self.width.saturating_sub(1);
        let scroll_right = if self.scroll_v {
            self.scroll_dir == 1
        } else {
            self.scroll_dir == 2
        };

        let end_y = self.scroll_end_y.min(self.rows.saturating_sub(1));
        let start_y = self.scroll_start_y.min(end_y);

        for row in start_y..=end_y {
            let dy = row;
            if scroll_right {
                let end = self.ddram[max_x][dy];
                for col in (1..=max_x).rev() {
                    self.ddram[col][dy] = self.ddram[col - 1][dy];
                }
                self.ddram[0][dy] = end;
            } else {
                let start = self.ddram[0][dy];
                for col in 0..max_x {
                    self.ddram[col][dy] = self.ddram[col + 1][dy];
                }
                self.ddram[max_x][dy] = start;
            }
        }

        if !self.scroll_v {
            return;
        }

        for col in 0..self.width {
            let mut ram_col: u64 = 0;
            for row in (start_y..=end_y).rev() {
                ram_col <<= 8;
                ram_col |= self.ddram[col][row] as u64;
            }
            if ram_col == 0 {
                continue;
            }

            let n_bits = (end_y - start_y + 1) * 8;
            let offset = (self.v_scroll_offset as usize) % n_bits;
            if offset > 0 {
                let mask = (1u64 << offset) - 1;
                let upper = (ram_col & mask) << (n_bits - offset);
                let lower = ram_col >> offset;
                ram_col = upper | lower;
            }

            for row in start_y..=end_y {
                self.ddram[col][row] = (ram_col & 0xFF) as u8;
                ram_col >>= 8;
            }
        }
    }

    /// Encode pixel bitmap as hex string for QML/UI readings.
    /// Format: page-by-page (8 vertical pixels per byte, 2 hex chars per byte).
    pub fn hex_bitmap(&self) -> String {
        if !self.disp_on {
            return String::new();
        }
        let pixels = self.render_pixels();
        let num_pages = self.height / 8;
        let mut hex = String::with_capacity(num_pages * self.width * 2);
        for page in 0..num_pages {
            for col in 0..self.width {
                let mut b = 0u8;
                for bit in 0..8 {
                    let py = page * 8 + bit;
                    if py < self.height && col < self.width {
                        if pixels[py * self.width + col] {
                            b |= 1 << bit;
                        }
                    }
                }
                use std::fmt::Write;
                let _ = write!(hex, "{b:02X}");
            }
        }
        hex
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ssd1306_initialization_and_reset() {
        let mut ssd = Ssd1306State::new("SSD1", 128, 64, 0x3C, "White", false, 100.0);
        assert_eq!(ssd.width, 128);
        assert_eq!(ssd.height, 64);
        assert_eq!(ssd.rows, 8);
        assert_eq!(ssd.control_code, 0x3C);
        assert_eq!(ssd.color, "White");
        assert!(!ssd.rotate);
        assert_eq!(ssd.addr_mode, PAGE_ADDR_MODE);
        assert!(!ssd.disp_on);
        assert!(!ssd.disp_full);
        assert!(!ssd.disp_inv);
        assert!(!ssd.scan_inv);
        assert!(!ssd.remap);
        assert_eq!(ssd.mr, 63);

        // Verify DDRAM is 128 x 8 zeros
        assert_eq!(ssd.ddram.len(), 128);
        for col in &ssd.ddram {
            assert_eq!(col.len(), 8);
            assert!(col.iter().all(|&b| b == 0));
        }

        // When display is off, render_pixels returns all false and hex_bitmap is empty
        let pixels = ssd.render_pixels();
        assert_eq!(pixels.len(), 128 * 64);
        assert!(pixels.iter().all(|&p| !p));
        assert_eq!(ssd.hex_bitmap(), "");

        // Mutate and reset
        ssd.disp_on = true;
        ssd.disp_inv = true;
        ssd.reset();
        assert!(!ssd.disp_on);
        assert!(!ssd.disp_inv);
    }

    #[test]
    fn test_ssd1306_display_on_off_and_all_on() {
        let mut ssd = Ssd1306State::new("SSD1", 128, 64, 0x3C, "White", false, 100.0);

        // Control byte 0x00 (Co=0, D/C=0 -> command stream)
        ssd.read_byte(0x00);
        // Command 0xAF (Display ON)
        ssd.read_byte(0xAF);
        assert!(ssd.disp_on);

        // With all DDRAM zero, hex_bitmap should be 1024 zero bytes (2048 '0's)
        let hex = ssd.hex_bitmap();
        assert_eq!(hex.len(), 128 * 8 * 2);
        assert_eq!(hex, "00".repeat(128 * 8));

        // Command 0xA5 (Entire Display ON)
        ssd.read_byte(0xA5);
        assert!(ssd.disp_full);
        let pixels = ssd.render_pixels();
        assert!(pixels.iter().all(|&p| p));
        let hex_full = ssd.hex_bitmap();
        assert_eq!(hex_full, "FF".repeat(128 * 8));

        // Command 0xA4 (Resume to RAM content)
        ssd.read_byte(0xA4);
        assert!(!ssd.disp_full);

        // Command 0xAE (Display OFF)
        ssd.read_byte(0xAE);
        assert!(!ssd.disp_on);
        assert_eq!(ssd.hex_bitmap(), "");
    }

    #[test]
    fn test_ssd1306_page_addressing_mode_and_pixel_rendering() {
        let mut ssd = Ssd1306State::new("SSD1", 128, 64, 0x3C, "White", false, 100.0);

        // Send Command stream: Set Page Mode (0x20, 0x02), Page 2 (0xB2), Col 5 (0x05, 0x10 -> (1<<4)|5 = 21, let's do 0x05, 0x10)
        ssd.read_byte(0x00); // Control: command stream
        ssd.read_byte(0x20); // Addressing mode command
        ssd.read_byte(0x02); // Page addressing mode
        assert_eq!(ssd.addr_mode, PAGE_ADDR_MODE);

        ssd.read_byte(0xB2); // Page 2 (y: 16..23)
        assert_eq!(ssd.addr_y, 2);

        ssd.read_byte(0x05); // Lower col nibble = 5
        ssd.read_byte(0x10); // Higher col nibble = 0 -> col = 5
        assert_eq!(ssd.addr_x, 5);

        ssd.read_byte(0xAF); // Display ON

        // Stop I2C, then send data stream
        ssd.i2c_stop();
        ssd.read_byte(0x40); // Control: data stream (Co=0, D/C=1)
        ssd.read_byte(0x01); // Bit 0 set (y = 2*8 + 0 = 16)
        ssd.read_byte(0x80); // Bit 7 set (y = 2*8 + 7 = 23)

        assert_eq!(ssd.ddram[5][2], 0x01);
        assert_eq!(ssd.ddram[6][2], 0x80);
        assert_eq!(ssd.addr_x, 7);

        let pixels = ssd.render_pixels();
        // Check pixel at (col=5, row=2, bit=0) -> x=5, y=16
        assert!(pixels[16 * 128 + 5]);
        // Check pixel at (col=6, row=2, bit=7) -> x=6, y=23
        assert!(pixels[23 * 128 + 6]);
        // Check neighboring pixels are false
        assert!(!pixels[15 * 128 + 5]);
        assert!(!pixels[17 * 128 + 5]);
        assert!(!pixels[16 * 128 + 4]);
        assert!(!pixels[16 * 128 + 6]);
    }

    #[test]
    fn test_ssd1306_horizontal_addressing_mode() {
        let mut ssd = Ssd1306State::new("SSD1", 128, 64, 0x3C, "White", false, 100.0);

        // Command stream
        ssd.read_byte(0x00);
        ssd.read_byte(0x20); // Set addr mode
        ssd.read_byte(0x00); // Horizontal mode
        assert_eq!(ssd.addr_mode, HORI_ADDR_MODE);

        ssd.read_byte(0x21); // Set column address
        ssd.read_byte(10); // Start col = 10
        ssd.read_byte(12); // End col = 12 (3 cols total)

        ssd.read_byte(0x22); // Set page address
        ssd.read_byte(1); // Start page = 1
        ssd.read_byte(2); // End page = 2 (2 pages total)

        // Switch to Data Stream
        ssd.i2c_stop();
        ssd.read_byte(0x40);

        // Write 6 bytes
        let data = [0x11, 0x22, 0x33, 0x44, 0x55, 0x66];
        for &b in &data {
            ssd.read_byte(b);
        }

        // Verify col 10..12 in page 1
        assert_eq!(ssd.ddram[10][1], 0x11);
        assert_eq!(ssd.ddram[11][1], 0x22);
        assert_eq!(ssd.ddram[12][1], 0x33);

        // Verify col 10..12 in page 2
        assert_eq!(ssd.ddram[10][2], 0x44);
        assert_eq!(ssd.ddram[11][2], 0x55);
        assert_eq!(ssd.ddram[12][2], 0x66);

        // Write 7th byte -> wraps around to (col=10, page=1)
        ssd.read_byte(0x99);
        assert_eq!(ssd.ddram[10][1], 0x99);
    }

    #[test]
    fn test_ssd1306_vertical_addressing_mode() {
        let mut ssd = Ssd1306State::new("SSD1", 128, 64, 0x3C, "White", false, 100.0);

        // Command stream
        ssd.read_byte(0x00);
        ssd.read_byte(0x20); // Addr mode
        ssd.read_byte(0x01); // Vertical mode
        assert_eq!(ssd.addr_mode, VERT_ADDR_MODE);

        ssd.read_byte(0x21);
        ssd.read_byte(20);
        ssd.read_byte(21); // 2 columns: 20, 21

        ssd.read_byte(0x22);
        ssd.read_byte(0);
        ssd.read_byte(2); // 3 pages: 0, 1, 2

        // Data stream
        ssd.i2c_stop();
        ssd.read_byte(0x40);

        let data = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06];
        for &b in &data {
            ssd.read_byte(b);
        }

        // Vertical mode advances page first, then column
        assert_eq!(ssd.ddram[20][0], 0x01);
        assert_eq!(ssd.ddram[20][1], 0x02);
        assert_eq!(ssd.ddram[20][2], 0x03);
        assert_eq!(ssd.ddram[21][0], 0x04);
        assert_eq!(ssd.ddram[21][1], 0x05);
        assert_eq!(ssd.ddram[21][2], 0x06);
    }

    #[test]
    fn test_ssd1306_rotation_inversion_and_offsets() {
        let mut ssd = Ssd1306State::new("SSD1", 128, 64, 0x3C, "White", false, 100.0);
        ssd.disp_on = true;

        // Write a single pixel at col 0, row 0, bit 0 (x=0, y=0)
        ssd.ddram[0][0] = 0x01;

        let pixels_normal = ssd.render_pixels();
        assert!(pixels_normal[0 * 128 + 0]);

        // Test Rotation (180 deg) -> maps (0,0) to (127, 63)
        ssd.rotate = true;
        let pixels_rotated = ssd.render_pixels();
        assert!(!pixels_rotated[0 * 128 + 0]);
        assert!(pixels_rotated[63 * 128 + 127]);

        ssd.rotate = false;

        // Test Display Inversion (0xA7)
        ssd.read_byte(0x00);
        ssd.read_byte(0xA7);
        assert!(ssd.disp_inv);
        let pixels_inv = ssd.render_pixels();
        // The lit pixel should now be false, all others true
        assert!(!pixels_inv[0 * 128 + 0]);
        assert!(pixels_inv[0 * 128 + 1]);

        // Test Segment Remap (0xA1) -> flips x
        ssd.read_byte(0xA6); // Normal display
        ssd.read_byte(0xA1); // Remap ON
        assert!(ssd.remap);
        let pixels_remap = ssd.render_pixels();
        assert!(pixels_remap[0 * 128 + 127]);

        // Test COM Scan Direction Invert (0xC8) -> flips y
        ssd.read_byte(0xA0); // Remap OFF
        ssd.read_byte(0xC8); // COM scan inverted
        assert!(ssd.scan_inv);
        let pixels_scan_inv = ssd.render_pixels();
        assert!(pixels_scan_inv[63 * 128 + 0]);
    }

    #[test]
    fn test_ssd1306_horizontal_and_vertical_scrolling() {
        let mut ssd = Ssd1306State::new("SSD1", 128, 64, 0x3C, "White", false, 100.0);

        // Put pattern in col 0, row 0
        ssd.ddram[0][0] = 0x55;

        // Configure Continuous Horizontal Scroll Right (0x26)
        ssd.read_byte(0x00);
        ssd.read_byte(0x26); // Setup right scroll
        ssd.read_byte(0x00); // Dummy
        ssd.read_byte(0x00); // Start page 0
        ssd.read_byte(0x07); // Scroll step = 2 (case 7 -> 2)
        ssd.read_byte(0x00); // End page 0
        ssd.read_byte(0x00); // Dummy
        ssd.read_byte(0xFF); // Dummy

        assert_eq!(ssd.scroll_start_y, 0);
        assert_eq!(ssd.scroll_end_y, 0);
        assert_eq!(ssd.scroll_step, 2);

        // Activate scroll (0x2F)
        ssd.read_byte(0x2F);
        assert!(ssd.scroll);

        // First step (count 0 -> 1 < 2)
        ssd.update_step();
        assert_eq!(ssd.ddram[0][0], 0x55);

        // Second step (count 1 -> 2 >= 2 -> shifts right!)
        ssd.update_step();
        assert_eq!(ssd.ddram[0][0], 0);
        assert_eq!(ssd.ddram[1][0], 0x55);

        // Deactivate scroll (0x2E)
        ssd.read_byte(0x2E);
        assert!(!ssd.scroll);

        // Update step after deactivation does not shift
        ssd.update_step();
        ssd.update_step();
        assert_eq!(ssd.ddram[1][0], 0x55);
    }
}
