//! C++ `OpAmp` companion: high-gain differential amp with a Thevenin output
//! to implicit ground (IoPin `source` mode).

use crate::LOW_IMP;
use crate::elements::PinCache;
use crate::net::ENode;

/// C++ constructor `m_gain = 1000`.
pub const OPAMP_DEFAULT_GAIN: f64 = 1000.0;
/// C++ `m_outImp = low_imp`.
pub const OPAMP_DEFAULT_OUT_IMP: f64 = LOW_IMP;
/// C++ `m_voltPosDef = 5`.
pub const OPAMP_DEFAULT_VOLT_POS: f64 = 5.0;
/// C++ `m_voltNegDef = 0`.
pub const OPAMP_DEFAULT_VOLT_NEG: f64 = 0.0;
/// C++ `m_accuracy = 5e-6`.
pub const OPAMP_ACCURACY: f64 = 5e-6;

#[derive(Clone, Copy, Debug)]
pub struct OpAmpState {
    pub gain: f64,
    pub out_imp: f64,
    pub volt_pos: f64,
    pub volt_neg: f64,
    pub power_pins: bool,
    pub switch_pins: bool,
    pub accuracy: f64,
    pub k: f64,
    pub last_out: f64,
    pub last_in: f64,
    pub step: u8,
}

impl Default for OpAmpState {
    fn default() -> Self {
        Self::new()
    }
}

impl OpAmpState {
    pub fn new() -> Self {
        let mut s = Self {
            gain: OPAMP_DEFAULT_GAIN,
            out_imp: OPAMP_DEFAULT_OUT_IMP,
            volt_pos: OPAMP_DEFAULT_VOLT_POS,
            volt_neg: OPAMP_DEFAULT_VOLT_NEG,
            power_pins: false,
            switch_pins: false,
            accuracy: OPAMP_ACCURACY,
            k: 1e-6 / OPAMP_DEFAULT_GAIN,
            last_out: 0.0,
            last_in: 0.0,
            step: 0,
        };
        s.reset_stamp();
        s
    }

    pub fn set_gain(&mut self, gain: f64) {
        self.gain = gain.max(1e-12);
        self.k = 1e-6 / self.gain;
    }

    pub fn set_out_imp(&mut self, imp: f64) {
        self.out_imp = imp.max(LOW_IMP);
    }

    pub fn admit(&self) -> f64 {
        1.0 / self.out_imp
    }

    pub fn reset_stamp(&mut self) {
        self.last_out = 0.0;
        self.last_in = 0.0;
        self.step = 0;
        self.k = 1e-6 / self.gain.max(1e-12);
        self.accuracy = OPAMP_ACCURACY;
    }

    /// Non-linear convergence step.
    pub fn update_nonlinear(&mut self, cache: &PinCache, nodes: &[ENode]) -> bool {
        let vp = cache
            .input_p
            .and_then(|i| nodes.get(i))
            .map(|n| n.volt)
            .unwrap_or(0.0);
        let vn = cache
            .input_n
            .and_then(|i| nodes.get(i))
            .map(|n| n.volt)
            .unwrap_or(0.0);
        let vpos = cache
            .power_pos
            .and_then(|i| nodes.get(i))
            .map(|n| n.volt)
            .unwrap_or(0.0);
        let vneg = cache
            .power_neg
            .and_then(|i| nodes.get(i))
            .map(|n| n.volt)
            .unwrap_or(0.0);
        self.volt_changed(vp, vn, vpos, vneg)
    }

    /// C++ `OpAmp::voltChanged`. `vpos`/`vneg` are supply pin voltages; ignored
    /// unless `power_pins` is set. Returns `true` when this step converged.
    pub fn volt_changed(&mut self, vp: f64, vn: f64, vpos: f64, vneg: f64) -> bool {
        let volt_pos = if self.power_pins { vpos } else { self.volt_pos };
        let volt_neg = if self.power_pins { vneg } else { self.volt_neg };
        let vd = vp - vn;

        let mut out = vd * self.gain;
        if out > volt_pos {
            out = volt_pos;
        } else if out < volt_neg {
            out = volt_neg;
        }

        if self.step == 0
            && (self.last_in - vd).abs() < self.accuracy
            && (out - self.last_out).abs() < self.accuracy
        {
            return true;
        }

        if self.step == 0 {
            let d_out = if vd > 0.0 { 1e-6 } else { -1e-6 };
            out = self.last_out + d_out;
            self.step = 1;
        } else {
            if self.last_in != vd {
                let d_in = (self.last_in - vd).abs();
                out = (self.last_out * d_in + vd * 1e-6) / (d_in + self.k);
            }
            self.step = 0;
        }
        if out >= volt_pos {
            out = volt_pos;
        } else if out <= volt_neg {
            out = volt_neg;
        }

        self.last_in = vd;
        self.last_out = out;
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rails_when_open_loop() {
        let mut s = OpAmpState::new();
        s.reset_stamp();
        let mut n = 0;
        while n < 20 && !s.volt_changed(5.0, 0.0, 0.0, 0.0) {
            n += 1;
        }
        assert!(n < 20, "did not converge");
        assert!((s.last_out - 5.0).abs() < 1e-4, "last_out {}", s.last_out);
    }
}
