//! WS2812 single-wire GRB pixels.
//!
//! A falling edge classifies the high pulse against the midpoint of `T0H` and
//! `T1H`. The low pulse is validated on the next rising edge against `T0L` or
//! `T1L` (accepting low durations at least half of the nominal low time), or
//! latched automatically when the line remains low for at least `RstTime`.
//! Bits are MSB first, green then red then blue. After `count * 24` bits,
//! further bits are copied to `DOUT`. A low longer than the reset time
//! restarts the bit index and leaves the latched pixels in place.

#[derive(Clone, Debug)]
pub struct Ws2812Fb {
    pub pixels: Vec<[u8; 3]>,
    pub bit_index: u32,
    acc: u32,
    acc_bits: u8,
    last_high: bool,
    rise_ps: u64,
    pub fall_ps: u64,
    pending_bit: Option<bool>,
    pub dout_high: bool,
}

impl Ws2812Fb {
    pub fn new(count: usize) -> Self {
        Self {
            pixels: vec![[0; 3]; count],
            bit_index: 0,
            acc: 0,
            acc_bits: 0,
            last_high: false,
            rise_ps: 0,
            fall_ps: 0,
            pending_bit: None,
            dout_high: false,
        }
    }

    pub fn hex(&self) -> String {
        let mut hex = String::with_capacity(self.pixels.len() * 6);
        for px in &self.pixels {
            hex.push_str(&format!("{:02X}{:02X}{:02X}", px[0], px[1], px[2]));
        }
        hex
    }

    /// Sample `DIN`. `arm_at` is the absolute time of the reset check.
    #[allow(clippy::too_many_arguments)]
    pub fn on_level(
        &mut self,
        high: bool,
        t: u64,
        count: usize,
        t0h_ns: u32,
        t0l_ns: u32,
        t1h_ns: u32,
        t1l_ns: u32,
        rst_ns: u32,
    ) -> (bool, Option<u64>) {
        if self.pixels.len() != count {
            self.pixels.resize(count, [0; 3]);
        }
        let mut changed = false;
        let mut arm = None;
        if high && !self.last_high {
            let rst_ps = u64::from(rst_ns.max(1)).saturating_mul(1_000);
            if self.fall_ps > 0 {
                let low_ps = t.saturating_sub(self.fall_ps);
                if low_ps >= rst_ps {
                    if let Some(bit) = self.pending_bit.take() {
                        self.feed(bit, count, &mut changed);
                    }
                    self.restart();
                } else if let Some(bit) = self.pending_bit.take() {
                    let low_ns = low_ps / 1_000;
                    let expected_low_ns = if bit {
                        u64::from(t1l_ns)
                    } else {
                        u64::from(t0l_ns)
                    };
                    let min_low_ns = expected_low_ns / 2;
                    if low_ns >= min_low_ns {
                        self.feed(bit, count, &mut changed);
                    }
                }
            }
            self.rise_ps = t;
            self.last_high = true;
        } else if !high && self.last_high {
            let width_ns = t.saturating_sub(self.rise_ps) / 1_000;
            let mid = (u64::from(t0h_ns) + u64::from(t1h_ns)) / 2;
            self.pending_bit = Some(width_ns >= mid);
            self.last_high = false;
            self.fall_ps = t;
            let rst_ps = u64::from(rst_ns.max(1)).saturating_mul(1_000);
            arm = Some(t.saturating_add(rst_ps));
        }
        (changed, arm)
    }

    pub fn on_reset(&mut self, t: u64, count: usize, rst_ns: u32) -> bool {
        if self.pixels.len() != count {
            self.pixels.resize(count, [0; 3]);
        }
        let mut changed = false;
        let rst_ps = u64::from(rst_ns.max(1)).saturating_mul(1_000);
        if !self.last_high && self.fall_ps > 0 && t >= self.fall_ps.saturating_add(rst_ps) {
            if let Some(bit) = self.pending_bit.take() {
                self.feed(bit, count, &mut changed);
            }
            self.restart();
        }
        changed
    }

    fn restart(&mut self) {
        self.bit_index = 0;
        self.acc = 0;
        self.acc_bits = 0;
        self.pending_bit = None;
    }

    fn feed(&mut self, bit: bool, count: usize, changed: &mut bool) {
        let owned = count.saturating_mul(24);
        if (self.bit_index as usize) < owned {
            self.acc = (self.acc << 1) | u32::from(bit);
            self.acc_bits = self.acc_bits.saturating_add(1);
            if self.acc_bits == 24 {
                let g = ((self.acc >> 16) & 0xFF) as u8;
                let r = ((self.acc >> 8) & 0xFF) as u8;
                let b = (self.acc & 0xFF) as u8;
                let pix = self.bit_index as usize / 24;
                if let Some(px) = self.pixels.get_mut(pix) {
                    *px = [r, g, b];
                }
                self.acc = 0;
                self.acc_bits = 0;
            }
        } else {
            if self.dout_high != bit {
                self.dout_high = bit;
                *changed = true;
            }
        }
        self.bit_index = self.bit_index.saturating_add(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grb_high_time_stores_red_and_reset_keeps_it() {
        let mut fb = Ws2812Fb::new(1);
        let mut t = 1_000u64;
        let bits = [false; 8].into_iter().chain([true; 8]).chain([false; 8]);
        for bit in bits {
            fb.on_level(true, t, 1, 400, 850, 850, 400, 50_000);
            t += if bit { 800_000 } else { 400_000 };
            fb.on_level(false, t, 1, 400, 850, 850, 400, 50_000);
            t += if bit { 400_000 } else { 850_000 };
        }
        // Rising edge of forwarded bit 25 completes bit 24's low period and latches pixel 0.
        fb.on_level(true, t, 1, 400, 850, 850, 400, 50_000);
        assert_eq!(fb.pixels[0], [255, 0, 0]);
        t += 800_000;
        fb.on_level(false, t, 1, 400, 850, 850, 400, 50_000);
        // Reset latches the final pending forwarded bit and resets bit_index.
        let changed = fb.on_reset(t + 50_000_000, 1, 50_000);
        assert!(changed && fb.dout_high);
        assert_eq!(fb.bit_index, 0);
        assert_eq!(fb.pixels[0], [255, 0, 0]);
    }

    #[test]
    fn final_bit_latches_on_reset_without_extra_pulse() {
        let mut fb = Ws2812Fb::new(1);
        let mut t = 1_000u64;
        // GRB = 0x00, 0x00, 0x01 (blue = 1, last bit is '1')
        let mut bits = [false; 24];
        bits[23] = true;
        for bit in bits {
            fb.on_level(true, t, 1, 400, 850, 850, 400, 50_000);
            t += if bit { 800_000 } else { 400_000 };
            fb.on_level(false, t, 1, 400, 850, 850, 400, 50_000);
            t += if bit { 400_000 } else { 850_000 };
        }
        // Before reset, bit 24 is still pending.
        assert_eq!(fb.pixels[0], [0, 0, 0]);
        fb.on_reset(t + 50_000_000, 1, 50_000);
        assert_eq!(fb.pixels[0], [0, 0, 1]);
        assert_eq!(fb.bit_index, 0);
    }

    #[test]
    fn short_low_pulse_rejected_against_t0l_and_t1l() {
        let mut fb = Ws2812Fb::new(1);
        let mut t = 1_000u64;
        // T0L = 850 ns (min 425 ns), T1L = 400 ns (min 200 ns).
        // Send 200 ns low for every bit: 0-bits fail T0L check so pixel never completes.
        let bits = [false; 8].into_iter().chain([true; 8]).chain([false; 8]);
        for bit in bits {
            fb.on_level(true, t, 1, 400, 850, 850, 400, 50_000);
            t += if bit { 800_000 } else { 400_000 };
            fb.on_level(false, t, 1, 400, 850, 850, 400, 50_000);
            t += 200_000;
        }
        fb.on_reset(t + 50_000_000, 1, 50_000);
        assert_eq!(fb.pixels[0], [0, 0, 0]);

        // Now send all 1-bits (white) with 100 ns low against T1L = 400 ns (min 200 ns): rejected.
        t += 60_000_000;
        for _ in 0..24 {
            fb.on_level(true, t, 1, 400, 850, 850, 400, 50_000);
            t += 800_000;
            fb.on_level(false, t, 1, 400, 850, 850, 400, 50_000);
            t += 100_000;
        }
        fb.on_reset(t + 50_000_000, 1, 50_000);
        assert_eq!(fb.pixels[0], [0, 0, 0]);
    }
}
