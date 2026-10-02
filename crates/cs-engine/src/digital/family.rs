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

/// One `LogicSubc` field. A bit is set only after that property is edited or loaded.
pub const LOGIC_IN_HIGH: u16 = 1 << 0;
pub const LOGIC_IN_LOW: u16 = 1 << 1;
pub const LOGIC_IN_IMP: u16 = 1 << 2;
pub const LOGIC_OUT_HIGH: u16 = 1 << 3;
pub const LOGIC_OUT_LOW: u16 = 1 << 4;
pub const LOGIC_OUT_IMP: u16 = 1 << 5;
pub const LOGIC_DELAY: u16 = 1 << 6;
pub const LOGIC_RISE: u16 = 1 << 7;
pub const LOGIC_FALL: u16 = 1 << 8;

/// SimulIDE `LogicSubc` electrical and timing values.
///
/// `mask` records which fields have been set. Unset fields keep the defaults
/// for the property panel and are not pushed onto child parts.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LogicOverride {
    pub mask: u16,
    pub in_high_v: f64,
    pub in_low_v: f64,
    pub in_imp: f64,
    pub out_high_v: f64,
    pub out_low_v: f64,
    pub out_imp: f64,
    /// Seconds. `Tpd_ps` getter is `m_propDelay * 1e-12`.
    pub delay_s: f64,
    pub rise_s: f64,
    pub fall_s: f64,
}

impl Default for LogicOverride {
    fn default() -> Self {
        Self {
            mask: 0,
            in_high_v: 2.5,
            in_low_v: 2.5,
            in_imp: 1e9,
            out_high_v: 5.0,
            out_low_v: 0.0,
            out_imp: DEFAULT_OUT_IMP,
            delay_s: DEFAULT_DELAY_PS * 1e-12,
            rise_s: DEFAULT_RISE_PS * 1e-12,
            fall_s: DEFAULT_FALL_PS * 1e-12,
        }
    }
}

impl LogicOverride {
    pub fn apply(self, family: &mut LogicFamily) {
        if self.mask & LOGIC_IN_HIGH != 0 {
            family.inp_high_v = self.in_high_v;
        }
        if self.mask & LOGIC_IN_LOW != 0 {
            family.inp_low_v = self.in_low_v;
        }
        if self.mask & LOGIC_IN_IMP != 0 {
            family.inp_imp = self.in_imp;
        }
        if self.mask & LOGIC_OUT_HIGH != 0 {
            family.out_high_v = self.out_high_v;
        }
        if self.mask & LOGIC_OUT_LOW != 0 {
            family.out_low_v = self.out_low_v;
        }
        if self.mask & LOGIC_OUT_IMP != 0 {
            family.out_imp = self.out_imp;
        }
        if self.mask & LOGIC_DELAY != 0 {
            family.set_prop_delay_s(self.delay_s);
        }
        if self.mask & LOGIC_RISE != 0 {
            family.set_rise_s(self.rise_s);
        }
        if self.mask & LOGIC_FALL != 0 {
            family.set_fall_s(self.fall_s);
        }
    }
}
