//! C++ `VoltReg`: eResistor G = 1e6 between I/O plus a dropout current source.

use crate::elements::PinCache;
use crate::net::ENode;

/// C++ constructor `setPropStr("Voltage", "1.2")`.
pub const VOLTREG_DEFAULT_VREF: f64 = 1.2;
/// C++ constructor `m_admit = 1e6`.
pub const VOLTREG_ADMIT: f64 = 1e6;
/// C++ dropout clamp `0.7`.
pub const VOLTREG_DROPOUT: f64 = 0.7;
/// C++ `qFabs(m_lastCurrent - current) < 1e-3`.
pub const VOLTREG_CURRENT_EPS: f64 = 1e-3;

#[derive(Clone, Copy, Debug)]
pub struct VoltRegState {
    pub v_ref: f64,
    pub admit: f64,
    pub last_current: f64,
}

impl Default for VoltRegState {
    fn default() -> Self {
        Self::new()
    }
}

impl VoltRegState {
    pub fn new() -> Self {
        Self {
            v_ref: VOLTREG_DEFAULT_VREF,
            admit: VOLTREG_ADMIT,
            last_current: 0.0,
        }
    }

    pub fn set_out_volt(&mut self, v: f64) {
        self.v_ref = v;
    }

    pub fn reset_stamp(&mut self) {
        self.last_current = 0.0;
    }

    /// Non-linear convergence step.
    pub fn update_nonlinear(&mut self, cache: &PinCache, nodes: &[ENode]) -> bool {
        let vin_opt = cache.voltreg_in.and_then(|i| nodes.get(i)).map(|n| n.volt);
        let vout_opt = cache.voltreg_out.and_then(|i| nodes.get(i)).map(|n| n.volt);
        if vin_opt.is_none() || vout_opt.is_none() {
            let was_zero = self.last_current == 0.0;
            self.last_current = 0.0;
            return was_zero;
        }
        let vin = vin_opt.unwrap();
        let vout = vout_opt.unwrap();
        let vref = cache
            .voltreg_ref
            .and_then(|i| nodes.get(i))
            .map(|n| n.volt)
            .unwrap_or(0.0);
        self.volt_changed(vin, vout, vref)
    }

    /// C++ `VoltReg::voltChanged`. `vin`/`vout`/`vref` are pin voltages.
    /// Returns `true` when `|ΔI| < 1e-3`.
    pub fn volt_changed(&mut self, vin: f64, _vout: f64, vref: f64) -> bool {
        let mut in_volt = vin;
        if in_volt < 1e-6 {
            in_volt = 0.0;
        }
        let out_target = vref + self.v_ref;
        let mut delta = in_volt - out_target;
        if delta < VOLTREG_DROPOUT {
            if in_volt < VOLTREG_DROPOUT {
                delta = in_volt;
            } else {
                delta = VOLTREG_DROPOUT;
            }
        }
        let current = delta * self.admit;
        if (self.last_current - current).abs() < VOLTREG_CURRENT_EPS {
            return true;
        }
        self.last_current = current;
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extra_current_is_vin_minus_target_times_g() {
        let mut s = VoltRegState::new();
        s.reset_stamp();
        assert!(!s.volt_changed(12.0, 12.0, 0.0));
        let expect = (12.0 - VOLTREG_DEFAULT_VREF) * VOLTREG_ADMIT;
        assert!((s.last_current - expect).abs() < 1e-6);
    }

    #[test]
    fn dropout_clamps_delta_to_0_7() {
        let mut s = VoltRegState::new();
        s.reset_stamp();
        // Vin = 1.5, target = 1.2 → delta 0.3 < 0.7 → clamp to 0.7.
        assert!(!s.volt_changed(1.5, 1.5, 0.0));
        assert!((s.last_current - 0.7 * VOLTREG_ADMIT).abs() < 1e-6);
    }
}
