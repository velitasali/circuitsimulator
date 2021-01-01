//! C++ `eBJT` (Falstad Ebers-Moll), `eMosfet` (piecewise RDSon), and `eJfet`
//! (square-law + channel-length correction) companions.

use crate::CERO_DOUB;
use crate::elements::PinCache;
use crate::net::ENode;

/// Thermal voltage used by C++ `eBJT::m_vt` / `eDiode::m_vt`.
pub const VT: f64 = 0.025865;

pub const BJT_DEFAULT_GAIN: f64 = 100.0;
pub const BJT_DEFAULT_SAT: f64 = 1e-13;
/// C++ `eBJT::m_rgain`.
pub const BJT_RGAIN: f64 = 0.5;

pub const MOSFET_DEFAULT_RDSON: f64 = 1.0;
pub const MOSFET_DEFAULT_VTH: f64 = 3.0;
pub const MOSFET_ACCURACY: f64 = 5e-6;

#[derive(Clone, Copy, Debug)]
pub struct BjtState {
    pub pnp: bool,
    pub gain: f64,
    pub fgain: f64,
    pub rgain: f64,
    pub vt: f64,
    pub sat_cur: f64,
    pub v_crit: f64,
    pub volt_be: f64,
    pub volt_bc: f64,
    pub ie: f64,
    pub ic: f64,
    pub base_curr: f64,
    pub gee: f64,
    pub gcc: f64,
    pub gce: f64,
    pub gec: f64,
    pub i_base: f64,
    pub i_coll: f64,
    pub i_emit: f64,
    pub step: f64,
}

impl BjtState {
    pub fn npn() -> Self {
        Self::new(false)
    }

    pub fn pnp() -> Self {
        Self::new(true)
    }

    pub fn new(pnp: bool) -> Self {
        let mut s = Self {
            pnp,
            gain: BJT_DEFAULT_GAIN,
            fgain: BJT_DEFAULT_GAIN / (BJT_DEFAULT_GAIN + 1.0),
            rgain: BJT_RGAIN,
            vt: VT,
            sat_cur: BJT_DEFAULT_SAT,
            v_crit: v_crit(VT, BJT_DEFAULT_SAT),
            volt_be: 0.0,
            volt_bc: 0.0,
            ie: 0.0,
            ic: 0.0,
            base_curr: 0.0,
            gee: 0.0,
            gcc: 0.0,
            gce: 0.0,
            gec: 0.0,
            i_base: 0.0,
            i_coll: 0.0,
            i_emit: 0.0,
            step: 0.0,
        };
        s.reset_stamp();
        s
    }

    pub fn set_gain(&mut self, gain: f64) {
        self.gain = gain.max(1e-6);
        self.fgain = self.gain / (self.gain + 1.0);
    }

    pub fn set_threshold(&mut self, v_crit: f64) {
        self.v_crit = v_crit;
        self.sat_cur = self.vt / (exp_clip(v_crit / self.vt) * std::f64::consts::SQRT_2);
    }

    pub fn reset_stamp(&mut self) {
        self.step = 0.0;
        self.volt_be = 0.0;
        self.volt_bc = 0.0;
        self.ie = 0.0;
        self.ic = 0.0;
        self.base_curr = 0.0;
        // C++ `m_changed = true` forces a first evaluation at 0 V so the
        // matrix is not empty on the first Newton iteration.
        self.eval(0.0, 0.0);
    }

    /// Non-linear convergence step.
    pub fn update_nonlinear(&mut self, cache: &PinCache, nodes: &[ENode]) -> bool {
        let vc = cache
            .collector
            .and_then(|i| nodes.get(i))
            .map(|n| n.volt)
            .unwrap_or(0.0);
        let ve = cache
            .emitter
            .and_then(|i| nodes.get(i))
            .map(|n| n.volt)
            .unwrap_or(0.0);
        let vb = cache
            .base
            .and_then(|i| nodes.get(i))
            .map(|n| n.volt)
            .unwrap_or(0.0);
        self.volt_changed(vc, ve, vb)
    }

    /// Returns `true` if this evaluation converged (`|ΔV| < 0.01` on both junctions).
    pub fn volt_changed(&mut self, vc: f64, ve: f64, vb: f64) -> bool {
        let volt_bc = vb - vc;
        let volt_be = vb - ve;
        if (volt_bc - self.volt_bc).abs() < 0.01 && (volt_be - self.volt_be).abs() < 0.01 {
            self.step = 0.0;
            return true;
        }
        self.step += 0.1;
        self.eval(volt_bc, volt_be);
        false
    }

    fn eval(&mut self, mut volt_bc: f64, mut volt_be: f64) {
        let pnp = if self.pnp { -1.0 } else { 1.0 };
        let mut gmin = self.sat_cur * 1e-2 * self.step.exp();
        if gmin > 0.1 {
            gmin = 0.1;
        }

        volt_bc = pnp * limit_step(pnp * volt_bc, pnp * self.volt_bc, self.vt, self.v_crit);
        self.volt_bc = volt_bc;
        volt_be = pnp * limit_step(pnp * volt_be, pnp * self.volt_be, self.vt, self.v_crit);
        self.volt_be = volt_be;

        let pcoef = pnp / self.vt;
        let exp_bc = exp_clip(volt_bc * pcoef);
        let exp_be = exp_clip(volt_be * pcoef);

        self.ie = pnp * self.sat_cur * (-(exp_be - 1.0) / self.fgain + (exp_bc - 1.0));
        self.ic = pnp * self.sat_cur * ((exp_be - 1.0) - (exp_bc - 1.0) / self.rgain);
        self.base_curr = -(self.ie + self.ic);

        let mut gee = -self.sat_cur / self.vt * exp_be / self.fgain;
        let mut gcc = -self.sat_cur / self.vt * exp_bc / self.rgain;
        let gce = -gee * self.fgain;
        let gec = -gcc * self.rgain;
        gcc -= gmin;
        gee -= gmin;
        self.gee = gee;
        self.gcc = gcc;
        self.gce = gce;
        self.gec = gec;

        self.i_base = -self.base_curr - (gec + gcc) * volt_bc - (gee + gce) * volt_be;
        self.i_coll = -self.ic + gce * volt_be + gcc * volt_bc;
        self.i_emit = -self.ie + gee * volt_be + gec * volt_bc;
    }
}

fn v_crit(vt: f64, sat_cur: f64) -> f64 {
    vt * (vt / (std::f64::consts::SQRT_2 * sat_cur)).ln()
}

fn limit_step(mut vnew: f64, vold: f64, vt: f64, v_crit: f64) -> f64 {
    if vnew > v_crit && (vnew - vold).abs() > 2.0 * vt {
        if vold > 0.0 {
            let arg = 1.0 + (vnew - vold) / vt;
            vnew = if arg > 0.0 {
                vold + vt * arg.ln()
            } else {
                v_crit
            };
        } else {
            vnew = vt * (vnew / vt).ln();
        }
    }
    vnew
}

fn exp_clip(x: f64) -> f64 {
    x.clamp(-40.0, 40.0).exp()
}

#[derive(Clone, Copy, Debug)]
pub struct MosfetState {
    pub p_channel: bool,
    pub depletion: bool,
    pub rdson: f64,
    pub threshold: f64,
    pub k_rdson: f64,
    pub gth: f64,
    pub admit: f64,
    pub current: f64,
    pub last_current: f64,
    pub gate_v: f64,
}

impl MosfetState {
    pub fn n_enhancement() -> Self {
        Self::new(false, false)
    }

    pub fn new(p_channel: bool, depletion: bool) -> Self {
        let mut s = Self {
            p_channel,
            depletion,
            rdson: MOSFET_DEFAULT_RDSON,
            threshold: MOSFET_DEFAULT_VTH,
            k_rdson: 0.0,
            gth: 0.0,
            admit: 0.0,
            current: 0.0,
            last_current: 0.0,
            gate_v: 0.0,
        };
        s.update_values();
        s.reset_stamp();
        s
    }

    pub fn set_rdson(&mut self, rdson: f64) {
        self.rdson = rdson.clamp(CERO_DOUB, 1000.0);
        self.update_values();
    }

    pub fn set_threshold(&mut self, th: f64) {
        if th < 0.01 {
            return;
        }
        self.threshold = th;
        self.update_values();
    }

    fn update_values(&mut self) {
        self.k_rdson = self.rdson * (10.0 - self.threshold);
        self.gth = if self.depletion {
            -self.threshold - self.threshold / 4.0
        } else {
            self.threshold - self.threshold / 4.0
        };
    }

    pub fn reset_stamp(&mut self) {
        // C++ `eMosfet::stamp` starts at `1/RDSon` before the first voltChanged.
        self.admit = 1.0 / self.rdson;
        self.current = 0.0;
        self.last_current = 0.0;
        self.gate_v = 0.0;
    }

    /// Non-linear convergence step.
    pub fn update_nonlinear(&mut self, cache: &PinCache, nodes: &[ENode]) -> bool {
        let vd_opt = cache.drain.and_then(|i| nodes.get(i)).map(|n| n.volt);
        let vs_opt = cache.source.and_then(|i| nodes.get(i)).map(|n| n.volt);
        if vd_opt.is_none() || vs_opt.is_none() {
            let was_zero = self.current == 0.0 && self.last_current == 0.0;
            self.admit = CERO_DOUB;
            self.current = 0.0;
            self.last_current = 0.0;
            return was_zero;
        }
        let vd = vd_opt.unwrap();
        let vs = vs_opt.unwrap();
        let vg = cache
            .gate
            .and_then(|i| nodes.get(i))
            .map(|n| n.volt)
            .unwrap_or(vs);
        self.volt_changed(vd, vs, vg)
    }

    /// Returns `true` if the DS current changed by less than `m_accuracy`.
    pub fn volt_changed(&mut self, vd: f64, vs: f64, vg: f64) -> bool {
        let mut vgs = vg - vs;
        let mut vds = vd - vs;
        if self.p_channel {
            vgs = -vgs;
            vds = -vds;
        }

        let mut gate_v = vgs - self.gth;
        if gate_v < 0.0 {
            gate_v = 0.0;
        }

        let mut admit = CERO_DOUB;
        let mut current = 0.0;
        if gate_v > 0.0 {
            admit = 1.0 / self.rdson;
            let max_curr_ds = vds * admit;
            let sat_k = 1.0 + vds / 100.0;
            if vds > gate_v {
                vds = gate_v;
            }
            let mut ds_current = (gate_v * vds - vds * vds / 2.0) * sat_k / self.k_rdson;
            if ds_current > max_curr_ds {
                ds_current = max_curr_ds;
            }
            current = max_curr_ds - ds_current;
        }
        if self.p_channel {
            current = -current;
        }

        // C++ stamps a new G immediately, then treats current as the Newton
        // unknown. We restamp every iteration, so admit must be stable too or
        // the first "current already 0" pass would freeze the initial 1/RDSon.
        let admit_changed = admit != self.admit;
        self.admit = admit;
        self.gate_v = gate_v;
        self.current = current;
        if !admit_changed && (current - self.last_current).abs() < MOSFET_ACCURACY {
            return true;
        }
        self.last_current = current;
        false
    }
}

/// C++ `eJfet` defaults: Idss 50 mA, Vp −3 V, 1/Lambda 1000 V.
pub const JFET_DEFAULT_IDSS: f64 = 0.050;
pub const JFET_DEFAULT_VP: f64 = -3.0;
pub const JFET_DEFAULT_LAMBDA_INV: f64 = 1000.0;
/// C++ `eJfet::initialize` sets `m_accuracy = 1` (ampere).
pub const JFET_ACCURACY: f64 = 1.0;

/// N-channel JFET square-law + LTSpice-style channel-length correction.
///
/// C++ stamps DS as a voltage-controlled conductance `G = Id/Vds`. The
/// `stampCurrent((current - m_lastCurrent) * direction)` after
/// `m_lastCurrent = current` is a no-op, so there is no Norton Ieq.
#[derive(Clone, Copy, Debug)]
pub struct JfetState {
    pub idss: f64,
    pub vp: f64,
    pub lambda_inv: f64,
    pub beta: f64,
    pub accuracy: f64,
    pub admit: f64,
    pub last_current: f64,
    pub gate_v: f64,
}

impl JfetState {
    pub fn new() -> Self {
        let mut s = Self {
            idss: JFET_DEFAULT_IDSS,
            vp: JFET_DEFAULT_VP,
            lambda_inv: JFET_DEFAULT_LAMBDA_INV,
            beta: 0.0,
            accuracy: JFET_ACCURACY,
            admit: CERO_DOUB,
            last_current: CERO_DOUB,
            gate_v: CERO_DOUB,
        };
        s.reset_stamp();
        s
    }

    pub fn set_idss(&mut self, idss: f64) {
        self.idss = idss.abs().min(1000.0);
    }

    pub fn set_vp(&mut self, vp: f64) {
        self.vp = vp;
    }

    pub fn set_lambda_inv(&mut self, lambda_inv: f64) {
        self.lambda_inv = lambda_inv.max(1.0);
    }

    pub fn reset_stamp(&mut self) {
        // C++ `eJfet::initialize` recomputes β from the current Idss / Vp.
        self.beta = self.idss / self.vp.powi(2).max(1e-24);
        self.accuracy = JFET_ACCURACY;
        self.admit = CERO_DOUB;
        self.last_current = CERO_DOUB;
        self.gate_v = CERO_DOUB;
    }

    /// Non-linear convergence step.
    pub fn update_nonlinear(&mut self, cache: &PinCache, nodes: &[ENode]) -> bool {
        let vd_opt = cache.drain.and_then(|i| nodes.get(i)).map(|n| n.volt);
        let vs_opt = cache.source.and_then(|i| nodes.get(i)).map(|n| n.volt);
        if vd_opt.is_none() || vs_opt.is_none() {
            let was_zero = self.last_current == 0.0;
            self.admit = CERO_DOUB;
            self.last_current = 0.0;
            return was_zero;
        }
        let vd = vd_opt.unwrap();
        let vs = vs_opt.unwrap();
        let vg = cache
            .gate
            .and_then(|i| nodes.get(i))
            .map(|n| n.volt)
            .unwrap_or(vs);
        self.volt_changed(vd, vs, vg)
    }

    /// Returns `true` when G is stable. C++ also treats `|ΔI| < 1 A` as
    /// current-converged; we still wait for admit because we restamp every
    /// Newton iteration (same reason as [`MosfetState::volt_changed`]).
    pub fn volt_changed(&mut self, mut vd: f64, mut vs: f64, vg: f64) -> bool {
        if vd < vs {
            // LTSpice-style virtual source swap when Vd < Vs.
            std::mem::swap(&mut vd, &mut vs);
        }
        let vgs = vg - vs;
        let vds = vd - vs;
        let mut gate_v = vgs - self.vp;
        let correction = 1.0 + vds / self.lambda_inv;

        let (current, temp_admit) = if vgs <= self.vp {
            gate_v = CERO_DOUB;
            (CERO_DOUB, CERO_DOUB)
        } else if vds < CERO_DOUB {
            // Analytic G at Vds = 0: dI/dVds = 2 β (Vgs − Vp). C++ divides
            // by Vds and would NaN here.
            (CERO_DOUB, (2.0 * self.beta * gate_v).max(CERO_DOUB))
        } else if vds < gate_v {
            let current = self.beta * (2.0 * gate_v * vds - vds * vds) * correction;
            (current, current / vds)
        } else {
            let current = self.beta * (vgs - self.vp).powi(2) * correction;
            (current, current / vds)
        };

        if (current - self.last_current).abs() < self.accuracy {
            // G = Id/Vds varies continuously, so exact f64 equality never
            // holds. A relative+absolute epsilon is enough to restamp (we
            // stamp before volt_changed, same as MOSFET).
            let scale = 1.0 + temp_admit.abs().max(self.admit.abs());
            let admit_changed = (temp_admit - self.admit).abs() > 1e-9 * scale;
            self.admit = temp_admit;
            self.gate_v = gate_v;
            return !admit_changed;
        }
        self.admit += 0.001 * (temp_admit - self.admit);
        self.gate_v = gate_v;
        self.last_current = current;
        false
    }
}

impl Default for JfetState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bjt_zero_bias_is_small() {
        let mut b = BjtState::npn();
        assert!(b.volt_changed(0.0, 0.0, 0.0));
        assert!(b.ic.abs() < 1e-12);
    }

    #[test]
    fn mosfet_off_at_zero_gate() {
        let mut m = MosfetState::n_enhancement();
        let _ = m.volt_changed(5.0, 0.0, 0.0);
        assert!(m.admit <= CERO_DOUB * 10.0);
        assert!(m.gate_v <= 0.0);
    }

    #[test]
    fn jfet_cutoff_below_pinchoff() {
        let mut j = JfetState::new();
        let _ = j.volt_changed(5.0, 0.0, -5.0);
        assert!(j.admit <= CERO_DOUB * 10.0);
    }

    #[test]
    fn jfet_idss_in_saturation() {
        let mut j = JfetState::new();
        // Vgs = 0, Vds = 5 > |Vp| → sat. Id = Idss (1 + Vds/λ).
        let _ = j.volt_changed(5.0, 0.0, 0.0);
        let id = j.admit * 5.0;
        let expect = JFET_DEFAULT_IDSS * (1.0 + 5.0 / JFET_DEFAULT_LAMBDA_INV);
        assert!((id - expect).abs() < 1e-9, "Id {id} want {expect}");
    }

    #[test]
    fn jfet_virtual_source_swap() {
        let mut j = JfetState::new();
        let _ = j.volt_changed(0.0, 5.0, 0.0);
        let id = j.admit * 5.0;
        let expect = JFET_DEFAULT_IDSS * (1.0 + 5.0 / JFET_DEFAULT_LAMBDA_INV);
        assert!((id - expect).abs() < 1e-9, "swapped Id {id} want {expect}");
    }
}
