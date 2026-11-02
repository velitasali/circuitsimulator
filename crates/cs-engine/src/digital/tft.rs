//! Color TFT LCD controller state machine (ILI9341, ST7735, ST7789, GC9A01A, PCF8833).
//!
//! Implements standard MIPI-DCS graphics controller commands over SPI:
//! SWRESET (0x01), SLPOUT (0x11), DISPON (0x29), DISPOFF (0x28),
//! CASET (0x2A), RASET/PASET (0x2B), RAMWR (0x2C), MADCTL (0x36), COLMOD (0x3A).

/// State machine for MIPI-DCS color TFT displays (ILI9341, ST7735, ST7789, GC9A01A, etc.).
#[derive(Clone, Debug, PartialEq)]
pub struct TftState {
    pub width: u32,
    pub height: u32,
    pub bgr: bool,
    pub fb: Vec<u16>,
    pub dirty: bool,
    pub dirty_seq: u64,

    // Bus signals
    pub last_sck: bool,

    // Protocol state
    pub display_on: bool,
    pub sleep: bool,
    cur_cmd: u8,
    cmd_params: Vec<u8>,

    // Window bounds and cursor
    col_start: u16,
    col_end: u16,
    row_start: u16,
    row_end: u16,
    curr_x: u16,
    curr_y: u16,
    pixel_high_byte: Option<u8>,

    // SPI receiver
    in_byte: u8,
    in_bit_count: u8,
}

impl TftState {
    pub fn new(width: u32, height: u32, bgr: bool) -> Self {
        let w = width.max(1);
        let h = height.max(1);
        let len = (w as usize) * (h as usize);
        Self {
            width: w,
            height: h,
            bgr,
            fb: vec![0u16; len],
            dirty: false,
            dirty_seq: 1,
            last_sck: false,
            display_on: true,
            sleep: false,
            cur_cmd: 0,
            cmd_params: Vec::with_capacity(4),
            col_start: 0,
            col_end: (w - 1) as u16,
            row_start: 0,
            row_end: (h - 1) as u16,
            curr_x: 0,
            curr_y: 0,
            pixel_high_byte: None,
            in_byte: 0,
            in_bit_count: 0,
        }
    }

    pub fn reset(&mut self) {
        self.display_on = true;
        self.sleep = false;
        self.dirty = true;
        self.dirty_seq = self.dirty_seq.wrapping_add(1);
        self.cur_cmd = 0;
        self.cmd_params.clear();
        self.col_start = 0;
        self.col_end = (self.width.saturating_sub(1)) as u16;
        self.row_start = 0;
        self.row_end = (self.height.saturating_sub(1)) as u16;
        self.curr_x = 0;
        self.curr_y = 0;
        self.pixel_high_byte = None;
        self.in_byte = 0;
        self.in_bit_count = 0;
    }

    /// Process pin inputs for 8-bit SPI TFT (PinCS, PinDC, PinCK, PinDI, PinRS).
    ///
    /// Returns whether the framebuffer or display state changed.
    pub fn step(
        &mut self,
        cs_high: bool,
        dc_high: bool,
        sck_high: bool,
        di_high: bool,
        rs_high: bool,
    ) -> bool {
        if !rs_high {
            // Hardware reset asserted (active low)
            self.reset();
            return false;
        }

        if cs_high {
            self.in_bit_count = 0;
            self.last_sck = sck_high;
            return false;
        }

        let rising_sck = sck_high && !self.last_sck;
        self.last_sck = sck_high;

        if rising_sck {
            self.in_byte = (self.in_byte << 1) | u8::from(di_high);
            self.in_bit_count += 1;
            if self.in_bit_count >= 8 {
                self.in_bit_count = 0;
                let byte = self.in_byte;
                if !dc_high {
                    self.process_command(byte);
                } else {
                    self.process_data(byte);
                }
            }
        }

        let changed = self.dirty;
        self.dirty = false;
        changed
    }

    fn process_command(&mut self, cmd: u8) {
        self.cur_cmd = cmd;
        self.cmd_params.clear();
        self.pixel_high_byte = None;

        match cmd {
            0x01 => self.reset(),
            0x10 => {
                if !self.sleep {
                    self.sleep = true;
                    self.dirty = true;
                    self.dirty_seq = self.dirty_seq.wrapping_add(1);
                }
            }
            0x11 => {
                if self.sleep {
                    self.sleep = false;
                    self.dirty = true;
                    self.dirty_seq = self.dirty_seq.wrapping_add(1);
                }
            }
            0x28 => {
                if self.display_on {
                    self.display_on = false;
                    self.dirty = true;
                    self.dirty_seq = self.dirty_seq.wrapping_add(1);
                }
            }
            0x29 => {
                if !self.display_on {
                    self.display_on = true;
                    self.dirty = true;
                    self.dirty_seq = self.dirty_seq.wrapping_add(1);
                }
            }
            0x2C => {
                self.curr_x = self.col_start;
                self.curr_y = self.row_start;
            }
            _ => {}
        }
    }

    fn process_data(&mut self, byte: u8) {
        match self.cur_cmd {
            0x2A => {
                // CASET: Column Address Set
                self.cmd_params.push(byte);
                if self.cmd_params.len() == 4 {
                    self.col_start = u16::from_be_bytes([self.cmd_params[0], self.cmd_params[1]]);
                    self.col_end = u16::from_be_bytes([self.cmd_params[2], self.cmd_params[3]]);
                    self.curr_x = self.col_start;
                } else if self.cmd_params.len() == 2 {
                    // 8-bit addressing support
                    self.col_start = u16::from(self.cmd_params[0]);
                    self.col_end = u16::from(self.cmd_params[1]);
                    self.curr_x = self.col_start;
                }
            }
            0x2B => {
                // RASET/PASET: Row/Page Address Set
                self.cmd_params.push(byte);
                if self.cmd_params.len() == 4 {
                    self.row_start = u16::from_be_bytes([self.cmd_params[0], self.cmd_params[1]]);
                    self.row_end = u16::from_be_bytes([self.cmd_params[2], self.cmd_params[3]]);
                    self.curr_y = self.row_start;
                } else if self.cmd_params.len() == 2 {
                    // 8-bit addressing support
                    self.row_start = u16::from(self.cmd_params[0]);
                    self.row_end = u16::from(self.cmd_params[1]);
                    self.curr_y = self.row_start;
                }
            }
            0x2C => {
                // RAMWR: stream 16-bit RGB565 pixels
                match self.pixel_high_byte {
                    None => {
                        self.pixel_high_byte = Some(byte);
                    }
                    Some(high) => {
                        self.pixel_high_byte = None;
                        let color = (u16::from(high) << 8) | u16::from(byte);
                        self.write_pixel(color);
                    }
                }
            }
            _ => {}
        }
    }

    fn write_pixel(&mut self, color: u16) {
        if (self.curr_x as u32) < self.width && (self.curr_y as u32) < self.height {
            let idx = (self.curr_y as usize) * (self.width as usize) + (self.curr_x as usize);
            if idx < self.fb.len() && self.fb[idx] != color {
                self.fb[idx] = color;
                self.dirty = true;
                self.dirty_seq = self.dirty_seq.wrapping_add(1);
            }
        }

        if self.curr_x < self.col_end {
            self.curr_x += 1;
        } else {
            self.curr_x = self.col_start;
            if self.curr_y < self.row_end {
                self.curr_y += 1;
            } else {
                self.curr_y = self.row_start;
            }
        }
    }
}

/// State machine for PCF8833 132x132 color LCD over 9-bit SPI.
#[derive(Clone, Debug, PartialEq)]
pub struct Pcf8833State {
    pub tft: TftState,
    in_word: u16,
    in_bit_count: u8,
    last_clk: bool,
}

impl Pcf8833State {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            tft: TftState::new(width, height, false),
            in_word: 0,
            in_bit_count: 0,
            last_clk: false,
        }
    }

    pub fn reset(&mut self) {
        self.tft.reset();
        self.in_word = 0;
        self.in_bit_count = 0;
    }

    /// Process pin inputs for 9-bit SPI PCF8833 (PinCS, PinCLK, PinDATA, PinRESET).
    pub fn step(&mut self, cs_high: bool, clk_high: bool, data_high: bool, rst_high: bool) -> bool {
        if !rst_high {
            self.reset();
            return false;
        }

        if cs_high {
            self.in_bit_count = 0;
            self.last_clk = clk_high;
            return false;
        }

        let rising_clk = clk_high && !self.last_clk;
        self.last_clk = clk_high;

        if rising_clk {
            self.in_word = (self.in_word << 1) | u16::from(data_high);
            self.in_bit_count += 1;
            if self.in_bit_count >= 9 {
                self.in_bit_count = 0;
                // Bit 8 (MSB of the 9-bit transfer) is D/C (0 = Command, 1 = Data)
                let dc_high = (self.in_word & 0x100) != 0;
                let byte = (self.in_word & 0xFF) as u8;
                if !dc_high {
                    self.tft.process_command(byte);
                } else {
                    self.tft.process_data(byte);
                }
            }
        }

        let changed = self.tft.dirty;
        self.tft.dirty = false;
        changed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tft_state_caset_raset_ramwr() {
        let mut tft = TftState::new(240, 320, false);

        let send = |tft: &mut TftState, dc: bool, b: u8| {
            for i in 0..8 {
                let di = (b & (0x80 >> i)) != 0;
                // SCK rising edge
                tft.step(false, dc, true, di, true);
                // SCK falling edge
                tft.step(false, dc, false, di, true);
            }
        };

        // CASET (0x2A): columns 10..11
        send(&mut tft, false, 0x2A);
        send(&mut tft, true, 0x00);
        send(&mut tft, true, 10);
        send(&mut tft, true, 0x00);
        send(&mut tft, true, 11);

        // RASET (0x2B): rows 20..20
        send(&mut tft, false, 0x2B);
        send(&mut tft, true, 0x00);
        send(&mut tft, true, 20);
        send(&mut tft, true, 0x00);
        send(&mut tft, true, 20);

        // RAMWR (0x2C): two pixels (RGB565 Red = 0xF800, Green = 0x07E0)
        send(&mut tft, false, 0x2C);
        send(&mut tft, true, 0xF8);
        send(&mut tft, true, 0x00);
        send(&mut tft, true, 0x07);
        send(&mut tft, true, 0xE0);

        // Check pixels in framebuffer
        let idx1 = 20 * 240 + 10;
        let idx2 = 20 * 240 + 11;
        assert_eq!(tft.fb[idx1], 0xF800, "Pixel 1 should be Red");
        assert_eq!(tft.fb[idx2], 0x07E0, "Pixel 2 should be Green");
    }

    #[test]
    fn test_pcf8833_state_9bit_spi() {
        let mut pcf = Pcf8833State::new(132, 132);

        let send_9bit = |pcf: &mut Pcf8833State, dc: bool, byte: u8| {
            // First bit is D/C
            pcf.step(false, true, dc, true);
            pcf.step(false, false, dc, true);
            // Next 8 bits are data
            for i in 0..8 {
                let di = (byte & (0x80 >> i)) != 0;
                pcf.step(false, true, di, true);
                pcf.step(false, false, di, true);
            }
        };

        // CASET (0x2A): col 5..5 (2-byte format)
        send_9bit(&mut pcf, false, 0x2A);
        send_9bit(&mut pcf, true, 5);
        send_9bit(&mut pcf, true, 5);

        // RASET (0x2B): row 8..8
        send_9bit(&mut pcf, false, 0x2B);
        send_9bit(&mut pcf, true, 8);
        send_9bit(&mut pcf, true, 8);

        // RAMWR (0x2C): 1 pixel Blue (0x001F)
        send_9bit(&mut pcf, false, 0x2C);
        send_9bit(&mut pcf, true, 0x00);
        send_9bit(&mut pcf, true, 0x1F);

        let idx = 8 * 132 + 5;
        assert_eq!(pcf.tft.fb[idx], 0x001F, "Pixel should be Blue");
    }
}
