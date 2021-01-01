//! C++ `LogicFamily` electrical defaults (Custom / Default).

use super::pin::{DEFAULT_IN_IMP, DEFAULT_OUT_IMP, EDGE_PIN_MULT};

/// Family delay 10 ns in picoseconds (`m_delayBase = 10 * 1000`).
pub const DEFAULT_DELAY_PS: f64 = 10_000.0;
/// Family rise 3 ns (`m_timeLH = 3000`).
pub const DEFAULT_RISE_PS: f64 = 3_000.0;
/// Family fall 4 ns (`m_timeHL = 4000`).
pub const DEFAULT_FALL_PS: f64 = 4_000.0;

#[derive(Clone, Debug)]
pub struct LogicFamily {
    pub supply_v: f64,
    pub inp_high_v: f64,
    pub inp_low_v: f64,
    pub out_high_v: f64,
    pub out_low_v: f64,
    pub inp_imp: f64,
    pub out_imp: f64,
    /// C++ `m_delayBase` (picoseconds as f64).
    pub delay_base: f64,
    /// C++ `m_delayMult` (`pd_n`).
    pub delay_mult: f64,
    /// C++ `m_timeLH` (picoseconds).
    pub time_lh: f64,
    /// C++ `m_timeHL` (picoseconds).
    pub time_hl: f64,
}

impl Default for LogicFamily {
    fn default() -> Self {
        Self::new()
    }
}

impl LogicFamily {
    pub fn new() -> Self {
        Self {
            supply_v: 5.0,
            inp_high_v: 2.5,
            inp_low_v: 2.5,
            out_high_v: 5.0,
            out_low_v: 0.0,
            inp_imp: DEFAULT_IN_IMP,
            out_imp: DEFAULT_OUT_IMP,
            delay_base: DEFAULT_DELAY_PS,
            delay_mult: 1.0,
            time_lh: DEFAULT_RISE_PS,
            time_hl: DEFAULT_FALL_PS,
        }
    }

    /// Pin rise time in picoseconds (`m_timeLH * 1.25`).
    pub fn pin_rise_ps(&self) -> u64 {
        (self.time_lh * EDGE_PIN_MULT).round().max(1.0) as u64
    }

    /// Pin fall time in picoseconds (`m_timeHL * 1.25`).
    pub fn pin_fall_ps(&self) -> u64 {
        (self.time_hl * EDGE_PIN_MULT).round().max(1.0) as u64
    }

    /// Propagation delay in picoseconds.
    pub fn delay_ps(&self) -> u64 {
        (self.delay_base * self.delay_mult).round().max(0.0) as u64
    }

    pub fn set_prop_delay_s(&mut self, pd: f64) {
        let pd = pd.clamp(0.0, 1e6);
        self.delay_base = pd * 1e12;
    }

    pub fn set_rise_s(&mut self, time: f64) {
        let time = time.clamp(1e-12, 1e6);
        self.time_lh = time * 1e12;
    }

    pub fn set_fall_s(&mut self, time: f64) {
        let time = time.clamp(1e-12, 1e6);
        self.time_hl = time * 1e12;
    }

    pub fn apply(&self, pin: &mut super::pin::IoPin) {
        pin.set_thresholds(self.inp_high_v, self.inp_low_v);
        pin.set_input_imp(self.inp_imp);
        pin.set_levels(self.out_high_v, self.out_low_v);
        pin.set_output_imp(self.out_imp);
    }
}
