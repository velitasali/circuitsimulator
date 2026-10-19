//! WS2812 single-wire GRB pixels.
//!
//! A falling edge classifies the high pulse against the midpoint of `T0H` and
//! `T1H`. Bits are MSB first, green then red then blue. After `count * 24`
//! bits, further bits are copied to `DOUT`. A low longer than the reset time
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
    pub fn on_level(
        &mut self,
        high: bool,
        t: u64,
        count: usize,
        t0h_ns: u32,
        t1h_ns: u32,
        rst_ns: u32,
    ) -> (bool, Option<u64>) {
        if self.pixels.len() != count {
            self.pixels.resize(count, [0; 3]);
        }
        let mut changed = false;
        let mut arm = None;
        if high && !self.last_high {
            let rst_ps = u64::from(rst_ns.max(1)).saturating_mul(1_000);
            if self.fall_ps > 0 && t.saturating_sub(self.fall_ps) >= rst_ps {
                self.restart();
            }
            self.rise_ps = t;
            self.last_high = true;
        } else if !high && self.last_high {
            let width_ns = t.saturating_sub(self.rise_ps) / 1_000;
            let mid = (u64::from(t0h_ns) + u64::from(t1h_ns)) / 2;
            self.feed(width_ns >= mid, count, &mut changed);
            self.last_high = false;
            self.fall_ps = t;
            let rst_ps = u64::from(rst_ns.max(1)).saturating_mul(1_000);
            arm = Some(t.saturating_add(rst_ps));
        }
        (changed, arm)
    }

    pub fn on_reset(&mut self, t: u64, rst_ns: u32) {
        let rst_ps = u64::from(rst_ns.max(1)).saturating_mul(1_000);
        if !self.last_high && self.fall_ps > 0 && t >= self.fall_ps.saturating_add(rst_ps) {
            self.restart();
        }
    }

    fn restart(&mut self) {
        self.bit_index = 0;
        self.acc = 0;
        self.acc_bits = 0;
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
            fb.on_level(true, t, 1, 400, 850, 50_000);
            t += if bit { 800_000 } else { 400_000 };
            fb.on_level(false, t, 1, 400, 850, 50_000);
            t += 200_000;
        }
        assert_eq!(fb.pixels[0], [255, 0, 0]);
        fb.on_level(true, t, 1, 400, 850, 50_000);
        t += 800_000;
        let (changed, _) = fb.on_level(false, t, 1, 400, 850, 50_000);
        assert!(changed && fb.dout_high);
        assert_eq!(fb.pixels[0], [255, 0, 0]);
        fb.on_reset(t + 50_000_000, 50_000);
        assert_eq!(fb.bit_index, 0);
        assert_eq!(fb.pixels[0], [255, 0, 0]);
    }
}
