//! Falstad / C++ `eDiode` Shockley companion, plus the piecewise LED model.

use crate::elements::PinCache;
use crate::net::ENode;

/// Thermal voltage used by C++ `eDiode::m_vt`.
pub const VT: f64 = 0.025865;

/// C++ `m_diodes["Diode Default"]`: satCurr_nA, emCoef, brkDown, resist.
pub const DIODE_DEFAULT_SAT_NA: f64 = 171.4352819281;
pub const DIODE_DEFAULT_EM: f64 = 2.0;
pub const DIODE_DEFAULT_RS: f64 = 0.05;

/// C++ `m_zeners["Zener Default"]`.
pub const ZENER_DEFAULT_BV: f64 = 5.6;

/// C++ `eLed` defaults.
pub const LED_DEFAULT_VTH: f64 = 2.4;
pub const LED_DEFAULT_OHMS: f64 = 0.6;
pub const LED_DEFAULT_IMAX: f64 = 0.03;

#[derive(Clone, Copy, Debug)]
pub struct DiodeState {
    pub sat_cur: f64,
    pub em_coef: f64,
    pub series_r: f64,
    pub bk_down: f64,
    pub threshold: f64,
    pub max_current: f64,
    pub volt_pn: f64,
    pub admit: f64,
    pub current: f64,
    pub step: f64,
}

impl DiodeState {
    pub fn diode_default() -> Self {
        Self::from_model(
            DIODE_DEFAULT_SAT_NA * 1e-9,
            DIODE_DEFAULT_EM,
            0.0,
            DIODE_DEFAULT_RS,
        )
    }

    pub fn zener_default() -> Self {
        Self::from_model(
            DIODE_DEFAULT_SAT_NA * 1e-9,
            DIODE_DEFAULT_EM,
            ZENER_DEFAULT_BV,
            DIODE_DEFAULT_RS,
        )
    }

    pub fn from_model(sat_cur: f64, em_coef: f64, bk_down: f64, series_r: f64) -> Self {
        let mut s = Self {
            sat_cur: sat_cur.max(1e-18),
            em_coef: em_coef.max(0.01),
            series_r: series_r.max(1e-12),
            bk_down: bk_down.max(0.0),
            threshold: if bk_down > 0.0 { 4.7 } else { 0.7 },
            max_current: 1.0,
            volt_pn: 0.0,
            admit: 0.0,
            current: 0.0,
            step: 0.0,
        };
        s.reset_stamp();
        s
    }

    fn v_scale(self) -> f64 {
        self.em_coef * VT
    }

    fn vd_coef(self) -> f64 {
        1.0 / self.v_scale()
    }

    fn b_admit(self) -> f64 {
        self.sat_cur * 1e-2
    }

    fn v_crit(self) -> f64 {
        let vs = self.v_scale();
        vs * (vs / (std::f64::consts::SQRT_2 * self.sat_cur)).ln()
    }

    pub fn reset_stamp(&mut self) {
        self.admit = self.b_admit();
        self.volt_pn = 0.0;
        self.current = 0.0;
        self.step = 0.0;
    }

    /// Norton current into the anode (`-stCurr` in C++).
    pub fn i_src(self) -> f64 {
        self.admit * self.volt_pn - self.current
    }

    /// Non-linear convergence step.
    pub fn update_nonlinear(&mut self, cache: &PinCache, nodes: &[ENode]) -> bool {
        let v_anode = cache.left.and_then(|i| nodes.get(i)).map(|n| n.volt);
        let v_cathode = cache.right.and_then(|i| nodes.get(i)).map(|n| n.volt);
        let v_mid = cache.mid.and_then(|i| nodes.get(i)).map(|n| n.volt);

        // If either external terminal is disconnected from the circuit, no current can flow.
        if v_anode.is_none() || v_cathode.is_none() {
            let was_zero = self.current == 0.0 && self.volt_pn == 0.0;
            self.reset_stamp();
            return was_zero;
        }

        let va = v_anode.unwrap_or(0.0);
        let vm = v_mid.unwrap_or(0.0);
        self.volt_changed(va, vm)
    }

    /// Returns `true` if this evaluation converged (`|ΔV| < 0.01`).
    pub fn volt_changed(&mut self, va: f64, vk: f64) -> bool {
        let mut volt_pn = va - vk;
        if (volt_pn - self.volt_pn).abs() < 0.01 {
            self.step = 0.0;
            return true;
        }
        self.step += 0.01;
        let mut gmin = self.b_admit() * self.step.exp();
        if gmin > 0.1 {
            gmin = 0.1;
        }

        let vs = self.v_scale();
        let vc = self.v_crit();
        if volt_pn > vc && (volt_pn - self.volt_pn).abs() > vs * 2.0 {
            volt_pn = limit_step(volt_pn, self.volt_pn, vs, vc);
        } else if self.bk_down != 0.0 && volt_pn < 0.0 {
            let mut vp = -volt_pn - self.z_offset();
            let vold = -self.volt_pn - self.z_offset();
            let vz_crit = VT * (VT / (std::f64::consts::SQRT_2 * self.sat_cur)).ln();
            if vp > vz_crit && (vp - vold).abs() > VT * 2.0 {
                vp = limit_step(vp, vold, VT, vz_crit);
            }
            volt_pn = -(vp + self.z_offset());
        }
        self.volt_pn = volt_pn;

        let eval = exp_clip(volt_pn * self.vd_coef());
        if self.bk_down == 0.0 || volt_pn >= 0.0 {
            self.admit = self.sat_cur * self.vd_coef() * eval + gmin;
            self.current = self.sat_cur * (eval - 1.0);
        } else {
            let exp_coef = exp_clip((-volt_pn - self.z_offset()) * (1.0 / VT));
            self.admit = self.sat_cur * (self.vd_coef() * eval + (1.0 / VT) * exp_coef) + gmin;
            self.current = self.sat_cur * (eval - 1.0 - exp_coef);
        }
        false
    }

    fn z_offset(self) -> f64 {
        self.bk_down - VT * (-(1.0 - 0.005 / self.sat_cur)).ln()
    }
}

fn limit_step(vnew: f64, vold: f64, scale: f64, vc: f64) -> f64 {
    if vold > 0.0 {
        let arg = 1.0 + (vnew - vold) / scale;
        if arg > 0.0 {
            vold + scale * arg.ln()
        } else {
            vc
        }
    } else {
        scale * (vnew / scale).ln()
    }
}

fn exp_clip(x: f64) -> f64 {
    x.clamp(-40.0, 40.0).exp()
}

#[derive(Clone, Copy, Debug)]
pub struct LedState {
    pub threshold: f64,
    pub impedance: f64,
    pub max_current: f64,
    pub admit: f64,
    pub th_current: f64,
    pub last_th_current: f64,
    pub current: f64,
    /// C++ `LedBase::m_grounded`: cathode is a dummy 0 V eNode, not a matrix node.
    pub grounded: bool,
}

impl LedState {
    pub fn default_led() -> Self {
        Self {
            threshold: LED_DEFAULT_VTH,
            impedance: LED_DEFAULT_OHMS,
            max_current: LED_DEFAULT_IMAX,
            admit: 1e-9,
            th_current: 0.0,
            last_th_current: 0.0,
            current: 0.0,
            grounded: false,
        }
    }

    pub fn reset_stamp(&mut self) {
        self.admit = 1e-9;
        self.th_current = 0.0;
        self.last_th_current = 0.0;
        self.current = 0.0;
    }

    /// Non-linear convergence step.
    pub fn update_nonlinear(&mut self, cache: &PinCache, nodes: &[ENode]) -> bool {
        let va_opt = cache.left.and_then(|i| nodes.get(i)).map(|n| n.volt);
        let vc_opt = if self.grounded {
            Some(0.0)
        } else {
            cache.right.and_then(|i| nodes.get(i)).map(|n| n.volt)
        };

        match (va_opt, vc_opt) {
            (Some(va), Some(vc)) => self.volt_changed(va, vc),
            _ => {
                // If either terminal of an ungrounded LED is disconnected, circuit is open: 0 current.
                let was_zero = self.current == 0.0 && self.th_current == 0.0;
                self.reset_stamp();
                was_zero
            }
        }
    }

    pub fn volt_changed(&mut self, va: f64, vk: f64) -> bool {
        let volt_pn = va - vk;
        let mut current = 0.0;
        let mut admit = 1e-9;
        let mut th_current = 0.0;
        let delta = volt_pn - self.threshold;
        if delta > -1e-12 {
            admit = 1.0 / self.impedance;
            th_current = self.threshold * admit;
            if delta > 0.0 {
                current = delta * admit;
            }
        }
        self.admit = admit;
        self.current = current;
        self.th_current = th_current;
        if th_current == self.last_th_current {
            true
        } else {
            self.last_th_current = th_current;
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_diode_converges_at_zero() {
        let mut d = DiodeState::diode_default();
        assert!(d.volt_changed(0.0, 0.0));
    }
}
