//! Power semiconductor physics models for Silicon Controlled Rectifier (SCR),
//! TRIAC (bidirectional thyristor), and DIAC (bidirectional trigger diode).
//!
//! Provides realistic multi-quadrant trigger sensitivity (Quadrants I through IV for TRIACs),
//! dynamic holding ($I_H$) and latching ($I_L$) current dynamics, critical rate-of-rise ($dv/dt$)
//! voltage triggering, and circuit-commutated turn-off recovery delays ($t_q$).

use crate::SWITCH_CLOSED_ADMIT;
use crate::elements::PinCache;
use crate::net::ENode;

/// Minimum voltage / current clamps to prevent numerical singularity.
const MIN_R: f64 = 1e-6;
const MIN_HOLD: f64 = 1e-6;

// =========================================================================
// 1. SILICON CONTROLLED RECTIFIER (SCR / THYRISTOR)
// =========================================================================

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScrState {
    /// Gate trigger voltage threshold $V_{GT}$ (V).
    pub v_gate_th: f64,
    /// Holding current $I_H$ (A): minimum principal current to maintain conduction.
    pub i_hold: f64,
    /// Gate trigger current $I_{GT}$ (A): minimum gate current required to trigger conduction.
    pub i_trig: f64,
    /// Latching current $I_L$ (A): current that anode must reach before gate drive is removed.
    pub i_latch: f64,
    /// Gate-to-cathode resistance $R_G$ (Ω).
    pub r_gate: f64,
    /// Critical rate-of-rise of off-state voltage $(dv/dt)_{crit}$ (V/s).
    pub crit_dv_dt: f64,
    /// Circuit-commutated turn-off time $t_q$ (s).
    pub t_q: f64,
    /// Forward on-state threshold voltage $V_T$ (V).
    pub v_on: f64,
    /// On-state dynamic resistance $R_{on}$ (Ω).
    pub r_on: f64,

    // Dynamic runtime state
    pub conducting: bool,
    pub latched: bool,
    pub v_ak_prev: f64,
    pub t_off: f64,
    pub i_anode: f64,
    pub i_gate: f64,
}

impl Default for ScrState {
    fn default() -> Self {
        Self {
            v_gate_th: 0.7,
            i_hold: 0.0082,
            i_trig: 0.01,
            i_latch: 0.015,
            r_gate: 100.0,
            crit_dv_dt: 50.0e6, // 50 V/µs
            t_q: 30.0e-6,       // 30 µs
            v_on: 0.7,
            r_on: 1.0 / SWITCH_CLOSED_ADMIT,

            conducting: false,
            latched: false,
            v_ak_prev: 0.0,
            t_off: -1.0,
            i_anode: 0.0,
            i_gate: 0.0,
        }
    }
}

impl ScrState {
    pub fn new(v_gate_th: f64, i_hold: f64, i_trig: f64, r_gate: f64) -> Self {
        let mut s = Self::default();
        s.v_gate_th = v_gate_th;
        s.i_hold = i_hold;
        s.i_trig = i_trig;
        s.i_latch = (i_hold * 1.8).max(i_hold);
        s.r_gate = r_gate;
        s
    }

    pub fn reset(&mut self) {
        self.conducting = false;
        self.latched = false;
        self.v_ak_prev = 0.0;
        self.t_off = -1.0;
        self.i_anode = 0.0;
        self.i_gate = 0.0;
    }

    pub fn copy_runtime_from(&mut self, prev: &Self) {
        self.conducting = prev.conducting;
        self.latched = prev.latched;
        self.v_ak_prev = prev.v_ak_prev;
        self.t_off = prev.t_off;
        self.i_anode = prev.i_anode;
        self.i_gate = prev.i_gate;
    }

    pub fn update_step(&mut self, cache: &PinCache, nodes: &[ENode]) {
        let va = cache
            .left
            .and_then(|i| nodes.get(i))
            .map(|n| n.volt)
            .unwrap_or(0.0);
        let vk = cache
            .right
            .and_then(|i| nodes.get(i))
            .map(|n| n.volt)
            .unwrap_or(0.0);
        self.v_ak_prev = va - vk;
    }

    pub fn update_nonlinear(
        &mut self,
        cache: &PinCache,
        nodes: &[ENode],
        dt: f64,
        sim_time: f64,
    ) -> bool {
        let va_opt = cache.left.and_then(|i| nodes.get(i)).map(|n| n.volt);
        let vk_opt = cache.right.and_then(|i| nodes.get(i)).map(|n| n.volt);
        if va_opt.is_none() || vk_opt.is_none() {
            let was_off = !self.conducting;
            self.conducting = false;
            self.latched = false;
            return was_off;
        }
        let va = va_opt.unwrap();
        let vk = vk_opt.unwrap();
        let vg = cache
            .gate
            .and_then(|i| nodes.get(i))
            .map(|n| n.volt)
            .unwrap_or(0.0);

        let v_ak = va - vk;
        let v_gk = vg - vk;
        let i_g = (v_gk / self.r_gate.max(MIN_R)).max(0.0);
        self.i_gate = i_g;

        let prev_conducting = self.conducting;
        let prev_latched = self.latched;
        let mut state = self.conducting;

        if state {
            let i_anode = ((v_ak - self.v_on) / self.r_on.max(MIN_R)).max(0.0);
            self.i_anode = i_anode;

            if i_anode >= self.i_latch {
                self.latched = true;
            }

            let gate_active = v_gk >= self.v_gate_th && i_g >= self.i_trig;

            if i_anode < self.i_hold.max(MIN_HOLD) || v_ak <= 0.0 {
                // Drop below holding current or reverse-biased anode -> turn off
                state = false;
                self.latched = false;
                self.t_off = sim_time;
            } else if !self.latched && !gate_active {
                // Gate trigger dropped before load current reached latching current -> turn off
                state = false;
                self.latched = false;
                self.t_off = sim_time;
            }
        } else {
            self.i_anode = v_ak * 1e-9;
            if v_ak > self.v_on {
                // 1. Gate trigger condition
                let gate_trig = v_gk >= self.v_gate_th && i_g >= self.i_trig;

                // 2. Critical dv/dt off-state voltage rate-of-rise triggering
                let dv_dt_trig = if dt > 0.0 {
                    let rate = (v_ak - self.v_ak_prev) / dt;
                    rate >= self.crit_dv_dt && v_ak >= 1.0
                } else {
                    false
                };

                // 3. Commutation turn-off recovery delay (tq failure)
                let tq_trig = if dt > 0.0 && self.t_off >= 0.0 && sim_time > self.t_off {
                    (sim_time - self.t_off) < self.t_q
                } else {
                    false
                };

                if gate_trig || dv_dt_trig || tq_trig {
                    state = true;
                    self.latched = false;
                }
            }
        }

        self.conducting = state;
        self.conducting == prev_conducting && self.latched == prev_latched
    }
}

// =========================================================================
// 2. BIDIRECTIONAL TRIODE THYRISTOR (TRIAC)
// =========================================================================

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TriacState {
    /// Gate trigger voltage threshold $V_{GT}$ (V).
    pub v_gate_th: f64,
    /// Holding current $I_H$ (A).
    pub i_hold: f64,
    /// Base Gate trigger current $I_{GT}$ in Quadrant I (A).
    pub i_trig: f64,
    /// Latching current $I_L$ (A).
    pub i_latch: f64,
    /// Gate-to-MT1 resistance $R_G$ (Ω).
    pub r_gate: f64,
    /// Critical rate-of-rise of off-state voltage $(dv/dt)_{crit}$ (V/s).
    pub crit_dv_dt: f64,
    /// Commutation recovery time $t_q$ (s).
    pub t_q: f64,
    /// On-state threshold voltage (V) (0.0 for piecewise closed-switch model).
    pub v_on: f64,
    /// On-state dynamic resistance $R_{on}$ (Ω).
    pub r_on: f64,

    // Quadrant trigger multipliers
    pub q1_mult: f64, // MT2+, G+
    pub q2_mult: f64, // MT2+, G-
    pub q3_mult: f64, // MT2-, G-
    pub q4_mult: f64, // MT2-, G+

    // Dynamic runtime state
    pub conducting: bool,
    pub latched: bool,
    pub conduction_dir: f64, // +1.0 for MT2 -> MT1, -1.0 for MT1 -> MT2, 0.0 when off
    pub v_mt_prev: f64,
    pub t_off: f64,
    pub i_main: f64,
    pub i_gate: f64,
}

impl Default for TriacState {
    fn default() -> Self {
        Self {
            v_gate_th: 0.7,
            i_hold: 0.0082,
            i_trig: 0.01,
            i_latch: 0.015,
            r_gate: 100.0,
            crit_dv_dt: 50.0e6, // 50 V/µs
            t_q: 30.0e-6,       // 30 µs
            v_on: 0.0,
            r_on: 1.0 / SWITCH_CLOSED_ADMIT,

            q1_mult: 1.0,
            q2_mult: 1.4,
            q3_mult: 1.0,
            q4_mult: 2.0,

            conducting: false,
            latched: false,
            conduction_dir: 0.0,
            v_mt_prev: 0.0,
            t_off: -1.0,
            i_main: 0.0,
            i_gate: 0.0,
        }
    }
}

impl TriacState {
    pub fn new(v_gate_th: f64, i_hold: f64, i_trig: f64, r_gate: f64) -> Self {
        let mut s = Self::default();
        s.v_gate_th = v_gate_th;
        s.i_hold = i_hold;
        s.i_trig = i_trig;
        s.i_latch = (i_hold * 1.8).max(i_hold);
        s.r_gate = r_gate;
        s
    }

    pub fn reset(&mut self) {
        self.conducting = false;
        self.latched = false;
        self.conduction_dir = 0.0;
        self.v_mt_prev = 0.0;
        self.t_off = -1.0;
        self.i_main = 0.0;
        self.i_gate = 0.0;
    }

    pub fn copy_runtime_from(&mut self, prev: &Self) {
        self.conducting = prev.conducting;
        self.latched = prev.latched;
        self.conduction_dir = prev.conduction_dir;
        self.v_mt_prev = prev.v_mt_prev;
        self.t_off = prev.t_off;
        self.i_main = prev.i_main;
        self.i_gate = prev.i_gate;
    }

    pub fn update_step(&mut self, cache: &PinCache, nodes: &[ENode]) {
        let v1 = cache
            .left
            .and_then(|i| nodes.get(i))
            .map(|n| n.volt)
            .unwrap_or(0.0);
        let v2 = cache
            .right
            .and_then(|i| nodes.get(i))
            .map(|n| n.volt)
            .unwrap_or(0.0);
        self.v_mt_prev = v2 - v1;
    }

    pub fn update_nonlinear(
        &mut self,
        cache: &PinCache,
        nodes: &[ENode],
        dt: f64,
        sim_time: f64,
    ) -> bool {
        let v1_opt = cache.left.and_then(|i| nodes.get(i)).map(|n| n.volt);
        let v2_opt = cache.right.and_then(|i| nodes.get(i)).map(|n| n.volt);
        if v1_opt.is_none() || v2_opt.is_none() {
            let was_off = !self.conducting;
            self.conducting = false;
            self.latched = false;
            self.conduction_dir = 0.0;
            return was_off;
        }
        let v1 = v1_opt.unwrap();
        let v2 = v2_opt.unwrap();
        let vg = cache
            .gate
            .and_then(|i| nodes.get(i))
            .map(|n| n.volt)
            .unwrap_or(0.0);

        let v_mt = v2 - v1; // MT2 relative to MT1
        let v_g = vg - v1; // Gate relative to MT1
        let i_g = v_g / self.r_gate.max(MIN_R);
        self.i_gate = i_g;

        let prev_conducting = self.conducting;
        let prev_latched = self.latched;
        let mut state = self.conducting;

        if state {
            let i_main = v_mt.abs() * SWITCH_CLOSED_ADMIT;
            self.i_main = if v_mt >= 0.0 { i_main } else { -i_main };

            if i_main >= self.i_latch {
                self.latched = true;
            }

            // Multi-quadrant gate active check
            let gate_active = (v_g.abs() >= self.v_gate_th && i_g.abs() >= self.i_trig)
                || self.is_gate_triggered(v_mt, v_g, i_g);

            if i_main < self.i_hold.max(MIN_HOLD) {
                state = false;
                self.latched = false;
                self.conduction_dir = 0.0;
                self.t_off = sim_time;
            } else if !self.latched && !gate_active {
                state = false;
                self.latched = false;
                self.conduction_dir = 0.0;
                self.t_off = sim_time;
            }
        } else {
            self.i_main = v_mt * 1e-9;
            if v_mt.abs() > 0.2 {
                // 1. Multi-quadrant gate trigger
                let gate_trig = self.is_gate_triggered(v_mt, v_g, i_g);

                // 2. Critical dv/dt triggering
                let dv_dt_trig = if dt > 0.0 {
                    let rate = (v_mt - self.v_mt_prev).abs() / dt;
                    rate >= self.crit_dv_dt && v_mt.abs() >= 1.0
                } else {
                    false
                };

                // 3. Commutation turn-off recovery delay (tq failure)
                let tq_trig = if dt > 0.0 && self.t_off >= 0.0 && sim_time > self.t_off {
                    (sim_time - self.t_off) < self.t_q
                } else {
                    false
                };

                if gate_trig || dv_dt_trig || tq_trig {
                    state = true;
                    self.latched = false;
                    self.conduction_dir = if v_mt >= 0.0 { 1.0 } else { -1.0 };
                }
            }
        }

        self.conducting = state;
        self.conducting == prev_conducting && self.latched == prev_latched
    }

    /// Evaluates realistic multi-quadrant trigger sensitivity:
    /// - Quadrant I ($I+$): $V_{MT2-1} > 0, V_{G-1} > 0$
    /// - Quadrant II ($I-$): $V_{MT2-1} > 0, V_{G-1} < 0$
    /// - Quadrant III ($III-$): $V_{MT2-1} < 0, V_{G-1} < 0$
    /// - Quadrant IV ($III+$): $V_{MT2-1} < 0, V_{G-1} > 0$
    #[inline]
    fn is_gate_triggered(&self, v_mt: f64, v_g: f64, i_g: f64) -> bool {
        if v_mt >= 0.0 {
            if v_g > 0.0 {
                // Quadrant I (I+)
                v_g >= self.v_gate_th && i_g >= (self.i_trig * self.q1_mult)
            } else {
                // Quadrant II (I-)
                -v_g >= self.v_gate_th && -i_g >= (self.i_trig * self.q2_mult)
            }
        } else if v_g < 0.0 {
            // Quadrant III (III-)
            -v_g >= self.v_gate_th && -i_g >= (self.i_trig * self.q3_mult)
        } else {
            // Quadrant IV (III+) - lowest sensitivity
            v_g >= (self.v_gate_th * 1.2) && i_g >= (self.i_trig * self.q4_mult)
        }
    }
}

// =========================================================================
// 3. BIDIRECTIONAL TRIGGER DIODE (DIAC)
// =========================================================================

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DiacState {
    /// Breakover voltage $V_{BO}$ (V).
    pub v_breakover: f64,
    /// On-state dynamic resistance $R_{on}$ (Ω).
    pub res_on: f64,
    /// Off-state blocking resistance $R_{off}$ (Ω).
    pub res_off: f64,
    /// Holding current $I_H$ (A).
    pub hold_curr: f64,
    /// Breakback voltage foldback $\Delta V_{BB}$ (V).
    pub breakback_v: f64,
    /// Critical rate-of-rise $dv/dt$ triggering rate (V/s).
    pub crit_dv_dt: f64,

    // Dynamic runtime state
    pub conducting: bool,
    pub conduction_dir: f64,
    pub v_prev: f64,
    pub i_main: f64,
}

impl Default for DiacState {
    fn default() -> Self {
        Self {
            v_breakover: 30.0,
            res_on: 500.0,
            res_off: 1.0e8,
            hold_curr: 0.01,
            breakback_v: 5.0,
            crit_dv_dt: 20.0e6, // 20 V/µs

            conducting: false,
            conduction_dir: 0.0,
            v_prev: 0.0,
            i_main: 0.0,
        }
    }
}

impl DiacState {
    pub fn new(v_breakover: f64, res_on: f64, res_off: f64, hold_curr: f64) -> Self {
        let mut s = Self::default();
        s.v_breakover = v_breakover;
        s.res_on = res_on;
        s.res_off = res_off;
        s.hold_curr = hold_curr;
        s
    }

    pub fn reset(&mut self) {
        self.conducting = false;
        self.conduction_dir = 0.0;
        self.v_prev = 0.0;
        self.i_main = 0.0;
    }

    pub fn copy_runtime_from(&mut self, prev: &Self) {
        self.conducting = prev.conducting;
        self.conduction_dir = prev.conduction_dir;
        self.v_prev = prev.v_prev;
        self.i_main = prev.i_main;
    }

    pub fn update_step(&mut self, cache: &PinCache, nodes: &[ENode]) {
        let v1 = cache
            .left
            .and_then(|i| nodes.get(i))
            .map(|n| n.volt)
            .unwrap_or(0.0);
        let v2 = cache
            .right
            .and_then(|i| nodes.get(i))
            .map(|n| n.volt)
            .unwrap_or(0.0);
        self.v_prev = v1 - v2;
    }

    pub fn update_nonlinear(
        &mut self,
        cache: &PinCache,
        nodes: &[ENode],
        dt: f64,
        _sim_time: f64,
    ) -> bool {
        let v1_opt = cache.left.and_then(|i| nodes.get(i)).map(|n| n.volt);
        let v2_opt = cache.right.and_then(|i| nodes.get(i)).map(|n| n.volt);
        if v1_opt.is_none() || v2_opt.is_none() {
            let was_off = !self.conducting;
            self.conducting = false;
            self.conduction_dir = 0.0;
            return was_off;
        }
        let v1 = v1_opt.unwrap();
        let v2 = v2_opt.unwrap();
        let v_diff = v1 - v2;

        let prev_conducting = self.conducting;
        let mut state = self.conducting;

        if state {
            let i_main = v_diff / self.res_on.max(MIN_R);
            self.i_main = i_main;

            if i_main.abs() < self.hold_curr.max(MIN_HOLD) {
                state = false;
                self.conduction_dir = 0.0;
            }
        } else {
            self.i_main = v_diff / self.res_off.max(1.0);

            // 1. Breakover voltage check
            let v_trig = v_diff.abs() >= self.v_breakover;

            // 2. Critical dv/dt triggering check
            let dv_dt_trig = if dt > 0.0 {
                let rate = (v_diff - self.v_prev).abs() / dt;
                rate >= self.crit_dv_dt && v_diff.abs() >= (self.v_breakover * 0.5)
            } else {
                false
            };

            if v_trig || dv_dt_trig {
                state = true;
                self.conduction_dir = if v_diff >= 0.0 { 1.0 } else { -1.0 };
            }
        }

        self.conducting = state;
        self.conducting == prev_conducting
    }
}
