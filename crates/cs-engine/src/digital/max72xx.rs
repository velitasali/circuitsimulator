//! MAX7219 / MAX7221 cascade: 16-bit SPI frames into digit registers.
//!
//! `CS` is active-low. A rising `CLK` while `CS` is low shifts `DIN` in, MSB
//! first. The bit that falls out of the last module is `DOUT`. A rising `CS`
//! latches one 16-bit word per module. The oldest word lands in the module
//! farthest from `DIN`. Address nibbles 1..=8 are the digit bytes.

#[derive(Clone, Debug)]
pub struct Max72Fb {
    pub digits: Vec<[u8; 8]>,
    /// Oldest bit at index 0 (next bit to leave as `DOUT`).
    chain: Vec<u8>,
    last_clk: bool,
    last_cs: bool,
    pub dout_high: bool,
}

impl Max72Fb {
    pub fn new(modules: usize) -> Self {
        let n = modules.max(1);
        Self {
            digits: vec![[0; 8]; n],
            chain: vec![0; n * 16],
            last_clk: false,
            // Idle `CS` is high, so the first high sample is not a latch edge.
            last_cs: true,
            dout_high: false,
        }
    }

    pub fn hex(&self) -> String {
        let mut hex = String::with_capacity(self.digits.len() * 16);
        for module in &self.digits {
            for byte in module {
                hex.push_str(&format!("{byte:02X}"));
            }
        }
        hex
    }

    /// Returns whether `DOUT` changed.
    pub fn step(&mut self, modules: usize, din: bool, clk: bool, cs_high: bool) -> bool {
        let modules = modules.max(1);
        if self.digits.len() != modules || self.chain.len() != modules * 16 {
            self.digits.resize(modules, [0; 8]);
            self.chain.resize(modules * 16, 0);
        }
        let mut dout_changed = false;
        let rising_clk = clk && !self.last_clk;
        let rising_cs = cs_high && !self.last_cs;
        if rising_clk && !cs_high {
            let dout = self.chain.first().copied().unwrap_or(0) != 0;
            if !self.chain.is_empty() {
                self.chain.remove(0);
            }
            self.chain.push(u8::from(din));
            if self.dout_high != dout {
                self.dout_high = dout;
                dout_changed = true;
            }
        }
        if rising_cs {
            self.latch(modules);
        }
        self.last_clk = clk;
        self.last_cs = cs_high;
        dout_changed
    }

    fn latch(&mut self, modules: usize) {
        for group in 0..modules {
            let mut word = 0u16;
            for bit in 0..16 {
                word = (word << 1) | u16::from(self.chain[group * 16 + bit] & 1);
            }
            let module = modules - 1 - group;
            let addr = ((word >> 8) & 0x0F) as usize;
            let data = (word & 0xFF) as u8;
            if (1..=8).contains(&addr) {
                if let Some(digits) = self.digits.get_mut(module) {
                    digits[addr - 1] = data;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn latch_digit_register_on_one_module() {
        let mut fb = Max72Fb::new(1);
        let word = 0x01A5u16;
        fb.step(1, false, false, false);
        for i in (0..16).rev() {
            let din = (word >> i) & 1 == 1;
            fb.step(1, din, false, false);
            fb.step(1, din, true, false);
        }
        fb.step(1, false, false, true);
        assert_eq!(fb.digits[0][0], 0xA5);
        assert!(fb.hex().starts_with("A5"));
    }
}
