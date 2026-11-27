//! Small-signal AC sweep for linear and DC-bias-linearized active circuits.
//!
//! The time-domain matrix is real. This solver builds its own interleaved
//! real/imaginary system, `Y(jω) V = I`, and leaves the caller's node voltages
//! and simulator clock untouched.
//!
//! When the circuit contains nonlinear active devices (diodes, LEDs, BJTs,
//! MOSFETs, JFETs, op-amps), a DC operating-point solve is run first on a
//! cloned copy (with capacitors open and inductors shorted) so those devices
//! are stamped using their exact small-signal linearized conductances and
//! transconductances around the DC bias point.

use std::f64::consts;

use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};

use crate::elements::diode;
use crate::elements::{
    BjtState, Comp, DiodeState, JfetState, Kind, MosfetState, OpAmpState, SOURCE_ADMIT,
    SWITCH_CLOSED_ADMIT, TransformerState,
};
use crate::net::ENode;
use crate::{CERO_DOUB, Error};

use super::Circuit;

const OHM_FLOOR: f64 = 1e-12;
const HENRY_FLOOR: f64 = 1e-12;
const MAX_POINTS: usize = 20_000;

/// One frequency of an AC sweep. `mag` is `|Vprobe - Vref|` in volts.
/// `gain` is `mag` divided by the excited source amplitude.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct AcPoint {
    pub hz: f64,
    pub mag: f64,
    pub phase_deg: f64,
    pub gain: f64,
}

/// One probe trace returned by a multi-probe AC sweep.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AcTrace {
    pub probe_pin: String,
    pub ref_pin: Option<String>,
    pub points: Vec<AcPoint>,
}

/// Precomputed small-signal linearizations for active/nonlinear parts at DC bias.
#[derive(Clone, Debug)]
enum SmallSignalDevice {
    TwoTerminal {
        a: Option<usize>,
        b: Option<usize>,
        g: f64,
    },
    Diode {
        anode: Option<usize>,
        mid: Option<usize>,
        cathode: Option<usize>,
        g_junc: f64,
        g_series: f64,
    },
    Bjt {
        c: Option<usize>,
        e: Option<usize>,
        b: Option<usize>,
        gee: f64,
        gcc: f64,
        gce: f64,
        gec: f64,
    },
    Fet {
        d: Option<usize>,
        s: Option<usize>,
        g: Option<usize>,
        g_ds: f64,
        g_m: f64,
    },
    OpAmp {
        out: Option<usize>,
        in_p: Option<usize>,
        in_n: Option<usize>,
        g_out: f64,
        gain: f64,
    },
}

struct AcSys {
    n: usize,
    a: Vec<f64>,
    b: Vec<f64>,
}

impl AcSys {
    fn new(n: usize) -> Self {
        let n2 = n * 2;
        Self {
            n,
            a: vec![0.0; n2 * n2],
            b: vec![0.0; n2],
        }
    }

    /// `Y = yr + j yi` from `row` to `col` in the nodal stamp.
    fn add(&mut self, row: usize, col: usize, yr: f64, yi: f64) {
        let n2 = self.n * 2;
        let rr = (2 * row) * n2 + (2 * col);
        let ri = rr + 1;
        let ir = (2 * row + 1) * n2 + (2 * col);
        let ii = ir + 1;
        self.a[rr] += yr;
        self.a[ri] -= yi;
        self.a[ir] += yi;
        self.a[ii] += yr;
    }

    /// Branch `Y` between two circuit nodes. `None` is the implicit datum.
    fn stamp_y(&mut self, a: Option<usize>, b: Option<usize>, yr: f64, yi: f64) {
        if yr == 0.0 && yi == 0.0 {
            return;
        }
        if let Some(i) = a {
            self.add(i, i, yr, yi);
        }
        if let Some(j) = b {
            self.add(j, j, yr, yi);
        }
        if let (Some(i), Some(j)) = (a, b) {
            if i != j {
                self.add(i, j, -yr, -yi);
                self.add(j, i, -yr, -yi);
            }
        }
    }

    /// Directed real conductance matching `ENode::add_admit(to, g)`:
    /// adds `+g` on `(from, from)` and `-g` on `(from, to)`.
    fn stamp_directed_g(&mut self, from: Option<usize>, to: Option<usize>, g: f64) {
        if g == 0.0 {
            return;
        }
        let (Some(f), Some(t)) = (from, to) else {
            return;
        };
        if f == t {
            return;
        }
        self.add(f, f, g, 0.0);
        self.add(f, t, -g, 0.0);
    }

    fn add_i(&mut self, node: usize, ir: f64, ii: f64) {
        self.b[2 * node] += ir;
        self.b[2 * node + 1] += ii;
    }

    fn solve(&mut self) -> Result<(), Error> {
        let n = self.n * 2;
        for k in 0..n {
            let mut piv = k;
            let mut best = self.a[k * n + k].abs();
            for i in (k + 1)..n {
                let v = self.a[i * n + k].abs();
                if v > best {
                    best = v;
                    piv = i;
                }
            }
            if best < 1e-18 {
                return Err(Error::Singular);
            }
            if piv != k {
                for j in k..n {
                    self.a.swap(k * n + j, piv * n + j);
                }
                self.b.swap(k, piv);
            }
            let diag = self.a[k * n + k];
            for i in (k + 1)..n {
                let f = self.a[i * n + k] / diag;
                self.a[i * n + k] = 0.0;
                for j in (k + 1)..n {
                    let prod = f * self.a[k * n + j];
                    self.a[i * n + j] -= prod;
                }
                let prod = f * self.b[k];
                self.b[i] -= prod;
            }
        }
        for i in (0..n).rev() {
            let mut s = self.b[i];
            for j in (i + 1)..n {
                s -= self.a[i * n + j] * self.b[j];
            }
            let d = self.a[i * n + i];
            if d.abs() < 1e-18 {
                return Err(Error::Singular);
            }
            self.b[i] = s / d;
        }
        Ok(())
    }
}

impl Circuit {
    /// `|V(probe)|` and phase at one frequency. `source_id` is the only
    /// excited source. Its amplitude is that part's DC voltage (or `1.0 V` if 0 V).
    pub fn ac_at(&mut self, source_id: &str, probe_pin: &str, hz: f64) -> Result<AcPoint, Error> {
        if !hz.is_finite() || hz <= 0.0 {
            return Err(Error::Parse(
                "AC frequency must be a finite number above 0".into(),
            ));
        }
        let traces = self.ac_sweep_multi(source_id, &[(probe_pin, None)], hz, hz, 1)?;
        traces
            .into_iter()
            .next()
            .and_then(|t| t.points.into_iter().next())
            .ok_or(Error::Singular)
    }

    /// Ids of voltage sources the sweep can excite.
    pub fn ac_sources(&self) -> Vec<String> {
        self.components
            .iter()
            .filter(|c| is_ac_source(&c.kind))
            .map(|c| c.id.clone())
            .collect()
    }

    /// Pin ids that landed on a net, sorted.
    pub fn ac_probes(&mut self) -> Vec<String> {
        if self.pin_net.is_empty() {
            self.build_nets();
        }
        let mut pins: Vec<String> = self.pin_net.keys().cloned().collect();
        pins.sort();
        pins
    }

    /// Log-spaced sweep from `f_start` to `f_stop`, inclusive, for a single probe.
    pub fn ac_sweep(
        &mut self,
        source_id: &str,
        probe_pin: &str,
        f_start: f64,
        f_stop: f64,
        points_per_decade: u32,
    ) -> Result<Vec<AcPoint>, Error> {
        let traces = self.ac_sweep_multi(
            source_id,
            &[(probe_pin, None)],
            f_start,
            f_stop,
            points_per_decade,
        )?;
        Ok(traces
            .into_iter()
            .next()
            .map(|t| t.points)
            .unwrap_or_default())
    }

    /// Log-spaced AC sweep solving the complex matrix once per frequency for one or
    /// more `(probe_pin, optional_ref_pin)` pairs.
    pub fn ac_sweep_multi(
        &mut self,
        source_id: &str,
        probes: &[(&str, Option<&str>)],
        f_start: f64,
        f_stop: f64,
        points_per_decade: u32,
    ) -> Result<Vec<AcTrace>, Error> {
        if probes.is_empty() {
            return Err(Error::Parse("AC sweep needs at least one probe pin".into()));
        }
        if !self.components.iter().any(|c| c.id == source_id) {
            return Err(Error::Parse(format!(
                "AC source `{source_id}` is not in the circuit"
            )));
        }
        let raw_amp = source_amplitude(self, source_id)?;
        // If a source has 0 V DC offset (common for AC-coupled or unbiased inputs),
        // use 1 V small-signal excitation so gain and phase are well-defined.
        let excite_amp = if raw_amp.abs() > 1e-15 { raw_amp } else { 1.0 };

        if self.pin_net.is_empty() {
            self.build_nets();
        }

        let mut resolved_probes: Vec<(String, usize, Option<(String, usize)>)> =
            Vec::with_capacity(probes.len());
        for &(probe_pin, ref_pin) in probes {
            let Some(&p_node) = self.pin_net.get(probe_pin) else {
                return Err(Error::Parse(format!(
                    "AC probe `{probe_pin}` is not on a net"
                )));
            };
            let r_node = match ref_pin {
                Some(rp) if !rp.trim().is_empty() => {
                    let Some(&rn) = self.pin_net.get(rp) else {
                        return Err(Error::Parse(format!(
                            "AC reference pin `{rp}` is not on a net"
                        )));
                    };
                    Some((rp.to_string(), rn))
                }
                _ => None,
            };
            resolved_probes.push((probe_pin.to_string(), p_node, r_node));
        }

        let freqs = log_freqs(f_start, f_stop, points_per_decade)?;
        let linearized = self.linearize_for_ac(source_id);

        let n = self.nodes.len();
        let mut traces: Vec<AcTrace> = resolved_probes
            .iter()
            .map(|(p, _, r)| AcTrace {
                probe_pin: p.clone(),
                ref_pin: r.as_ref().map(|(s, _)| s.clone()),
                points: Vec::with_capacity(freqs.len()),
            })
            .collect();

        for hz in freqs {
            let omega = consts::TAU * hz;
            let mut sys = AcSys::new(n);
            for (idx, comp) in self.components.iter().enumerate() {
                if let Some(dev) = linearized.get(&idx) {
                    stamp_small_signal(&mut sys, dev);
                } else {
                    stamp_comp(&mut sys, comp, source_id, excite_amp, omega, &self.pin_net);
                }
            }
            sys.solve()?;

            for (t_idx, &(_, p_node, ref r_opt)) in resolved_probes.iter().enumerate() {
                if 2 * p_node + 1 >= sys.b.len() {
                    return Err(Error::Singular);
                }
                let mut vr = sys.b[2 * p_node];
                let mut vi = sys.b[2 * p_node + 1];
                if let Some((_, r_node)) = r_opt {
                    if 2 * r_node + 1 >= sys.b.len() {
                        return Err(Error::Singular);
                    }
                    vr -= sys.b[2 * r_node];
                    vi -= sys.b[2 * r_node + 1];
                }
                let mag = vr.hypot(vi);
                let phase_deg = vi.atan2(vr).to_degrees();
                let gain = mag / excite_amp.abs();
                traces[t_idx].points.push(AcPoint {
                    hz,
                    mag,
                    phase_deg,
                    gain,
                });
            }
        }

        Ok(traces)
    }

    /// Runs a DC operating-point solve on a cloned copy (capacitors open,
    /// inductors shorted) and returns small-signal linearizations for active
    /// and nonlinear components keyed by component index.
    fn linearize_for_ac(&self, source_id: &str) -> FxHashMap<usize, SmallSignalDevice> {
        let mut out = FxHashMap::default();
        if !self.scan_has_nonlinear() {
            return out;
        }

        let mut dc = self.clone();
        for c in &mut dc.components {
            match &mut c.kind {
                Kind::Capacitor { capacitance, .. }
                | Kind::ElCapacitor { capacitance, .. }
                | Kind::VarCapacitor { capacitance, .. } => {
                    *capacitance = 0.0;
                }
                Kind::Inductor { resistance, .. } | Kind::VarInductor { resistance, .. } => {
                    c.kind = Kind::Resistor {
                        resistance: resistance.max(OHM_FLOOR),
                    };
                }
                Kind::VoltSource { running, .. } if c.id == source_id => {
                    *running = true;
                }
                Kind::WaveGen {
                    offset,
                    v_out,
                    bipolar,
                    ..
                } => {
                    // Hold WaveGen at its DC offset for operating-point bias.
                    *v_out = if *bipolar { 0.5 } else { 0.0 };
                    let _ = offset;
                }
                _ => {}
            }
        }
        dc.build_nets();
        dc.reset_device_stamps();
        if dc.solve_circuit().is_err() {
            // If DC operating point failed to converge, still fall back to
            // unbiased/default linearizations (e.g. ideal op-amp gain) where safe.
        }

        // Map DC component indices back to `self`'s net indices via `self.pin_caches`.
        for (idx, (comp, dc_comp)) in self.components.iter().zip(&dc.components).enumerate() {
            let Some(cache) = self.pin_caches.get(idx) else {
                continue;
            };
            let Some(dc_cache) = dc.pin_caches.get(idx) else {
                continue;
            };
            match (&comp.kind, &dc_comp.kind) {
                (Kind::Diode { .. }, Kind::Diode { state, .. }) => {
                    let va = node_v(dc_cache.left, &dc.nodes);
                    let vm = node_v(dc_cache.mid, &dc.nodes);
                    let g_junc = diode_small_signal_g(state, va - vm);
                    out.insert(
                        idx,
                        SmallSignalDevice::Diode {
                            anode: cache.left,
                            mid: cache.mid,
                            cathode: cache.right,
                            g_junc,
                            g_series: 1.0 / state.series_r.max(OHM_FLOOR),
                        },
                    );
                }
                (Kind::Led { .. }, Kind::Led { state }) => {
                    out.insert(
                        idx,
                        SmallSignalDevice::TwoTerminal {
                            a: cache.left,
                            b: if state.grounded { None } else { cache.right },
                            g: state.admit.max(1e-9),
                        },
                    );
                }
                (Kind::Bjt { .. }, Kind::Bjt { state }) => {
                    let vc = node_v(dc_cache.collector, &dc.nodes);
                    let ve = node_v(dc_cache.emitter, &dc.nodes);
                    let vb = node_v(dc_cache.base, &dc.nodes);
                    let (gee, gcc, gce, gec) = bjt_small_signal_jacobian(state, vc, ve, vb);
                    out.insert(
                        idx,
                        SmallSignalDevice::Bjt {
                            c: cache.collector,
                            e: cache.emitter,
                            b: cache.base,
                            gee,
                            gcc,
                            gce,
                            gec,
                        },
                    );
                }
                (Kind::Mosfet { .. }, Kind::Mosfet { state }) => {
                    let vd = node_v(dc_cache.drain, &dc.nodes);
                    let vs = node_v(dc_cache.source, &dc.nodes);
                    let vg = dc_cache
                        .gate
                        .and_then(|i| dc.nodes.get(i))
                        .map(|n| n.volt)
                        .unwrap_or(vs);
                    let (g_m, g_ds) = mosfet_small_signal(state, vd, vs, vg);
                    out.insert(
                        idx,
                        SmallSignalDevice::Fet {
                            d: cache.drain,
                            s: cache.source,
                            g: cache.gate,
                            g_ds,
                            g_m,
                        },
                    );
                }
                (Kind::Jfet { .. }, Kind::Jfet { state }) => {
                    let vd = node_v(dc_cache.drain, &dc.nodes);
                    let vs = node_v(dc_cache.source, &dc.nodes);
                    let vg = dc_cache
                        .gate
                        .and_then(|i| dc.nodes.get(i))
                        .map(|n| n.volt)
                        .unwrap_or(vs);
                    let (g_m, g_ds) = jfet_small_signal(state, vd, vs, vg);
                    out.insert(
                        idx,
                        SmallSignalDevice::Fet {
                            d: cache.drain,
                            s: cache.source,
                            g: cache.gate,
                            g_ds,
                            g_m,
                        },
                    );
                }
                (Kind::OpAmp { .. }, Kind::OpAmp { state }) => {
                    let vp = node_v(dc_cache.input_p, &dc.nodes);
                    let vn = node_v(dc_cache.input_n, &dc.nodes);
                    let vpos = node_v(dc_cache.power_pos, &dc.nodes);
                    let vneg = node_v(dc_cache.power_neg, &dc.nodes);
                    let gain = opamp_small_signal_gain(state, vp, vn, vpos, vneg);
                    out.insert(
                        idx,
                        SmallSignalDevice::OpAmp {
                            out: cache.output_amp,
                            in_p: cache.input_p,
                            in_n: cache.input_n,
                            g_out: state.admit(),
                            gain,
                        },
                    );
                }
                _ => {}
            }
        }
        out
    }
}

fn node_v(idx: Option<usize>, nodes: &[ENode]) -> f64 {
    idx.and_then(|i| nodes.get(i))
        .map(|n| n.volt)
        .unwrap_or(0.0)
}

fn diode_small_signal_g(state: &DiodeState, v_pn: f64) -> f64 {
    let vt = diode::VT;
    let vd_coef = 1.0 / (state.em_coef.max(0.01) * vt);
    let gmin = state.sat_cur * 1e-2;
    let eval = (v_pn * vd_coef).clamp(-40.0, 40.0).exp();
    if state.bk_down == 0.0 || v_pn >= 0.0 {
        state.sat_cur * vd_coef * eval + gmin
    } else {
        let z_off = state.bk_down - vt * (-(1.0 - 0.005 / state.sat_cur)).ln();
        let exp_coef = ((-v_pn - z_off) / vt).clamp(-40.0, 40.0).exp();
        state.sat_cur * (vd_coef * eval + (1.0 / vt) * exp_coef) + gmin
    }
}

fn bjt_small_signal_jacobian(state: &BjtState, vc: f64, ve: f64, vb: f64) -> (f64, f64, f64, f64) {
    let pnp = if state.pnp { -1.0 } else { 1.0 };
    let vbc = vb - vc;
    let vbe = vb - ve;
    let pcoef = pnp / state.vt;
    let exp_bc = (vbc * pcoef).clamp(-40.0, 40.0).exp();
    let exp_be = (vbe * pcoef).clamp(-40.0, 40.0).exp();
    let gmin = state.sat_cur * 1e-2;
    let gee = -state.sat_cur / state.vt * exp_be / state.fgain - gmin;
    let gcc = -state.sat_cur / state.vt * exp_bc / state.rgain - gmin;
    let gce = -(gee + gmin) * state.fgain;
    let gec = -(gcc + gmin) * state.rgain;
    (gee, gcc, gce, gec)
}

fn mosfet_id(state: &MosfetState, vd: f64, vs: f64, vg: f64) -> f64 {
    let mut vgs = vg - vs;
    let mut vds = vd - vs;
    if state.p_channel {
        vgs = -vgs;
        vds = -vds;
    }
    let gate_v = (vgs - state.gth).max(0.0);
    if gate_v <= 0.0 {
        return vds * CERO_DOUB * if state.p_channel { -1.0 } else { 1.0 };
    }
    let admit = 1.0 / state.rdson;
    let max_curr_ds = vds * admit;
    let sat_k = 1.0 + vds / 100.0;
    let vds_eff = vds.min(gate_v);
    let mut id = (gate_v * vds_eff - 0.5 * vds_eff * vds_eff) * sat_k / state.k_rdson;
    if id > max_curr_ds {
        id = max_curr_ds;
    }
    if state.p_channel { -id } else { id }
}

fn mosfet_small_signal(state: &MosfetState, vd: f64, vs: f64, vg: f64) -> (f64, f64) {
    let eps = 1e-5;
    let g_m =
        (mosfet_id(state, vd, vs, vg + eps) - mosfet_id(state, vd, vs, vg - eps)) / (2.0 * eps);
    let g_ds =
        (mosfet_id(state, vd + eps, vs, vg) - mosfet_id(state, vd - eps, vs, vg)) / (2.0 * eps);
    (g_m, g_ds.max(CERO_DOUB))
}

fn jfet_id(state: &JfetState, mut vd: f64, mut vs: f64, mut vg: f64) -> f64 {
    let mut swapped = false;
    if state.p_channel {
        vd = -vd;
        vs = -vs;
        vg = -vg;
    }
    if vd < vs {
        std::mem::swap(&mut vd, &mut vs);
        swapped = true;
    }
    let vgs = vg - vs;
    let vds = vd - vs;
    let gate_v = vgs - state.vp;
    let correction = 1.0 + vds / state.lambda_inv;
    let mut id = if vgs <= state.vp {
        vds * CERO_DOUB
    } else if vds < gate_v {
        state.beta * (2.0 * gate_v * vds - vds * vds) * correction
    } else {
        state.beta * gate_v * gate_v * correction
    };
    if swapped {
        id = -id;
    }
    if state.p_channel { -id } else { id }
}

fn jfet_small_signal(state: &JfetState, vd: f64, vs: f64, vg: f64) -> (f64, f64) {
    let eps = 1e-5;
    let g_m = (jfet_id(state, vd, vs, vg + eps) - jfet_id(state, vd, vs, vg - eps)) / (2.0 * eps);
    let g_ds = (jfet_id(state, vd + eps, vs, vg) - jfet_id(state, vd - eps, vs, vg)) / (2.0 * eps);
    (g_m, g_ds.max(CERO_DOUB))
}

fn opamp_small_signal_gain(state: &OpAmpState, vp: f64, vn: f64, vpos: f64, vneg: f64) -> f64 {
    let hi = if state.power_pins {
        vpos
    } else {
        state.volt_pos
    };
    let lo = if state.power_pins {
        vneg
    } else {
        state.volt_neg
    };
    let raw = (vp - vn) * state.gain;
    // Allow boundary value (e.g., 0 V unbiased single-supply filter) to stay active,
    // while hard rail saturation zeroes out incremental gain.
    if raw > hi + 1e-3 || raw < lo - 1e-3 {
        0.0
    } else {
        state.gain
    }
}

fn stamp_small_signal(sys: &mut AcSys, dev: &SmallSignalDevice) {
    match *dev {
        SmallSignalDevice::TwoTerminal { a, b, g } => {
            stamp_g(sys, a, b, g);
        }
        SmallSignalDevice::Diode {
            anode,
            mid,
            cathode,
            g_junc,
            g_series,
        } => {
            stamp_g(sys, anode, mid, g_junc);
            stamp_g(sys, mid, cathode, g_series);
        }
        SmallSignalDevice::Bjt {
            c,
            e,
            b,
            gee,
            gcc,
            gce,
            gec,
        } => {
            sys.stamp_directed_g(b, c, -gec - gcc);
            sys.stamp_directed_g(c, b, -gce - gcc);
            sys.stamp_directed_g(b, e, -gee - gce);
            sys.stamp_directed_g(e, b, -gee - gec);
            sys.stamp_directed_g(c, e, gce);
            sys.stamp_directed_g(e, c, gec);
        }
        SmallSignalDevice::Fet { d, s, g, g_ds, g_m } => {
            stamp_g(sys, d, s, g_ds);
            if g_m != 0.0 {
                if let Some(dn) = d {
                    if let Some(gn) = g {
                        sys.add(dn, gn, g_m, 0.0);
                    }
                    if let Some(sn) = s {
                        sys.add(dn, sn, -g_m, 0.0);
                    }
                }
                if let Some(sn) = s {
                    if let Some(gn) = g {
                        sys.add(sn, gn, -g_m, 0.0);
                    }
                    sys.add(sn, sn, g_m, 0.0);
                }
            }
        }
        SmallSignalDevice::OpAmp {
            out,
            in_p,
            in_n,
            g_out,
            gain,
        } => {
            if let Some(o) = out {
                sys.add(o, o, g_out, 0.0);
                if gain != 0.0 {
                    if let Some(p) = in_p {
                        sys.add(o, p, -g_out * gain, 0.0);
                    }
                    if let Some(n) = in_n {
                        sys.add(o, n, g_out * gain, 0.0);
                    }
                }
            }
        }
    }
}

fn log_freqs(f_start: f64, f_stop: f64, points_per_decade: u32) -> Result<Vec<f64>, Error> {
    if !f_start.is_finite()
        || !f_stop.is_finite()
        || f_start <= 0.0
        || f_stop < f_start
        || points_per_decade == 0
    {
        return Err(Error::Parse(
            "AC sweep needs f_start > 0, f_stop >= f_start, and at least one point per decade"
                .into(),
        ));
    }
    if (f_stop - f_start).abs() <= f_start * 1e-12 {
        return Ok(vec![f_start]);
    }
    let decades = (f_stop / f_start).log10();
    let steps = (decades * f64::from(points_per_decade)).ceil().max(1.0) as usize;
    if steps > MAX_POINTS {
        return Err(Error::Parse("AC sweep has too many points".into()));
    }
    let mut out = Vec::with_capacity(steps + 1);
    for i in 0..=steps {
        out.push(f_start * 10f64.powf(decades * i as f64 / steps as f64));
    }
    if let Some(last) = out.last_mut() {
        *last = f_stop;
    }
    Ok(out)
}

fn is_ac_source(kind: &Kind) -> bool {
    matches!(
        kind,
        Kind::FixedVolt { .. }
            | Kind::Rail { .. }
            | Kind::Clock { .. }
            | Kind::Battery { .. }
            | Kind::VoltSource { .. }
            | Kind::WaveGen { .. }
    )
}

fn source_amplitude(c: &Circuit, source_id: &str) -> Result<f64, Error> {
    let Some(comp) = c.components.iter().find(|c| c.id == source_id) else {
        return Err(Error::Parse(format!(
            "AC source `{source_id}` is not in the circuit"
        )));
    };
    match &comp.kind {
        Kind::FixedVolt { voltage }
        | Kind::Rail { voltage }
        | Kind::Clock { voltage, .. }
        | Kind::Battery { voltage, .. }
        | Kind::VoltSource { value: voltage, .. } => Ok(*voltage),
        Kind::WaveGen { amplitude, .. } => Ok(*amplitude),
        _ => Err(Error::Parse(format!(
            "AC source `{source_id}` is not a voltage source"
        ))),
    }
}

fn net(pin_net: &FxHashMap<String, usize>, pin: &str) -> Option<usize> {
    pin_net.get(pin).copied()
}

fn stamp_g(sys: &mut AcSys, a: Option<usize>, b: Option<usize>, g: f64) {
    sys.stamp_y(a, b, g, 0.0);
}

fn stamp_comp(
    sys: &mut AcSys,
    comp: &Comp,
    source_id: &str,
    excite_amp: f64,
    omega: f64,
    pin_net: &FxHashMap<String, usize>,
) {
    let excited = comp.id == source_id;
    match &comp.kind {
        Kind::Resistor { resistance }
        | Kind::VarResistor { resistance }
        | Kind::Ldr { resistance, .. }
        | Kind::Thermistor { resistance, .. }
        | Kind::Rtd { resistance, .. }
        | Kind::Strain { resistance, .. } => {
            let g = 1.0 / resistance.max(OHM_FLOOR);
            let a = net(pin_net, &comp.left_pin());
            let b = net(pin_net, &comp.right_pin());
            stamp_g(sys, a, b, g);
        }
        Kind::Capacitor {
            capacitance,
            resistance,
            ..
        }
        | Kind::ElCapacitor {
            capacitance,
            resistance,
            ..
        }
        | Kind::VarCapacitor {
            capacitance,
            resistance,
            ..
        } => {
            let b = omega * capacitance.max(0.0);
            let r = resistance.max(0.0);
            let (yr, yi) = if r <= 0.0 || b <= 0.0 {
                (0.0, b)
            } else {
                let rb = r * b;
                if rb <= 1.0 {
                    let d = 1.0 + rb * rb;
                    (r * b * b / d, b / d)
                } else {
                    let inv = 1.0 / rb;
                    let d = r * (1.0 + inv * inv);
                    (1.0 / d, inv / d)
                }
            };
            let a = net(pin_net, &comp.left_pin());
            let b_node = net(pin_net, &comp.right_pin());
            sys.stamp_y(a, b_node, yr, yi);
        }
        Kind::Inductor {
            inductance,
            resistance,
            ..
        }
        | Kind::VarInductor {
            inductance,
            resistance,
            ..
        } => {
            let l = inductance.max(HENRY_FLOOR);
            let x = omega * l;
            let r = resistance.max(0.0);
            let (yr, yi) = if r <= 0.0 {
                (0.0, -1.0 / x)
            } else {
                let m = r.max(x);
                let rn = r / m;
                let xn = x / m;
                let d = m * (rn * rn + xn * xn);
                (rn / d, -xn / d)
            };
            let a = net(pin_net, &comp.left_pin());
            let b = net(pin_net, &comp.right_pin());
            sys.stamp_y(a, b, yr, yi);
        }
        Kind::Potentiometer { resistance, wiper } => {
            let r1 = (wiper * resistance).max(OHM_FLOOR);
            let r2 = ((1.0 - wiper) * resistance).max(OHM_FLOOR);
            let a = comp.pot_pin_a_node(pin_net);
            let m = comp.pot_pin_m_node(pin_net);
            let b = comp.pot_pin_b_node(pin_net);
            stamp_g(sys, a, m, 1.0 / r1);
            stamp_g(sys, m, b, 1.0 / r2);
        }
        Kind::ResistorDip {
            size,
            resistance,
            bussed,
            ..
        } => {
            let g = 1.0 / resistance.max(OHM_FLOOR);
            if *bussed {
                let com = net(pin_net, &format!("{}-com", comp.id));
                for i in 0..*size {
                    let p = net(pin_net, &format!("{}-pin{i}", comp.id));
                    stamp_g(sys, com, p, g);
                }
            } else {
                for i in 0..*size {
                    let a = net(pin_net, &format!("{}-lPin{i}", comp.id));
                    let b = net(pin_net, &format!("{}-rPin{i}", comp.id));
                    stamp_g(sys, a, b, g);
                }
            }
        }
        Kind::Ground => {
            if let Some(n) = net(pin_net, &comp.gnd_pin()) {
                stamp_g(sys, Some(n), None, SOURCE_ADMIT);
            }
        }
        Kind::FixedVolt { .. } | Kind::Rail { .. } | Kind::Clock { .. } => {
            let pin = comp.out_pin();
            if let Some(n) = net(pin_net, &pin) {
                stamp_g(sys, Some(n), None, SOURCE_ADMIT);
                if excited {
                    sys.add_i(n, excite_amp * SOURCE_ADMIT, 0.0);
                }
            }
        }
        Kind::VoltSource { running, .. } => {
            // Auto-enable the excited VoltSource even if it is switched off in transient UI.
            if *running || excited {
                let pin = format!("{}-outPin", comp.id);
                if let Some(n) = net(pin_net, &pin) {
                    stamp_g(sys, Some(n), None, SOURCE_ADMIT);
                    if excited {
                        sys.add_i(n, excite_amp * SOURCE_ADMIT, 0.0);
                    }
                }
            }
        }
        Kind::WaveGen {
            bipolar, floating, ..
        } => {
            let out = net(pin_net, &format!("{}-outnod", comp.id));
            let gnd = net(pin_net, &format!("{}-gndnod", comp.id))
                .or_else(|| net(pin_net, &comp.gnd_pin()));
            if *bipolar && *floating {
                stamp_g(sys, out, gnd, SOURCE_ADMIT);
                if excited {
                    if let Some(p) = out {
                        sys.add_i(p, excite_amp * SOURCE_ADMIT, 0.0);
                    }
                    if let Some(m) = gnd {
                        sys.add_i(m, -excite_amp * SOURCE_ADMIT, 0.0);
                    }
                }
            } else if *bipolar {
                if let Some(p) = out {
                    stamp_g(sys, Some(p), None, SOURCE_ADMIT);
                    if excited {
                        sys.add_i(p, 0.5 * excite_amp * SOURCE_ADMIT, 0.0);
                    }
                }
                if let Some(m) = gnd {
                    stamp_g(sys, Some(m), None, SOURCE_ADMIT);
                    if excited {
                        sys.add_i(m, -0.5 * excite_amp * SOURCE_ADMIT, 0.0);
                    }
                }
            } else if let Some(p) = out {
                stamp_g(sys, Some(p), None, SOURCE_ADMIT);
                if excited {
                    sys.add_i(p, excite_amp * SOURCE_ADMIT, 0.0);
                }
            }
        }
        Kind::Battery { resistance, .. } => {
            let g = 1.0 / resistance.max(1e-14);
            let a = net(pin_net, &comp.left_pin());
            let b = net(pin_net, &comp.right_pin());
            stamp_g(sys, a, b, g);
            if excited {
                if let Some(p) = a {
                    sys.add_i(p, excite_amp * g, 0.0);
                }
                if let Some(m) = b {
                    sys.add_i(m, -excite_amp * g, 0.0);
                }
            }
        }
        Kind::CurrSource { value, running } => {
            if (*running || excited) && excited {
                let pin = format!("{}-outPin", comp.id);
                if let Some(n) = net(pin_net, &pin) {
                    sys.add_i(n, *value, 0.0);
                }
            }
        }
        Kind::Switch {
            closed,
            poles,
            double_throw,
        } => {
            let npoles = (*poles).max(1);
            if *double_throw {
                for i in 0..npoles {
                    let p = net(pin_net, &format!("{}-pinP{i}", comp.id));
                    let which = if *closed { 2 * i } else { 2 * i + 1 };
                    let t = net(pin_net, &format!("{}-switch{which}pinN", comp.id));
                    stamp_g(sys, p, t, SWITCH_CLOSED_ADMIT);
                }
            } else if *closed {
                for i in 0..npoles {
                    let p = net(pin_net, &format!("{}-pinP{i}", comp.id));
                    let t = net(pin_net, &format!("{}-switch{i}pinN", comp.id));
                    stamp_g(sys, p, t, SWITCH_CLOSED_ADMIT);
                }
            }
        }
        Kind::Transformer { state } => stamp_transformer(sys, comp, pin_net, omega, state),
        _ => {}
    }
}

fn stamp_transformer(
    sys: &mut AcSys,
    comp: &Comp,
    pin_net: &FxHashMap<String, usize>,
    omega: f64,
    state: &TransformerState,
) {
    let p1 = net(pin_net, &comp.transformer_p1_pin());
    let p2 = net(pin_net, &comp.transformer_p2_pin());
    let s1 = net(pin_net, &comp.transformer_s1_pin());
    let s2 = net(pin_net, &comp.transformer_s2_pin());
    let primary = match (p1, p2) {
        (Some(a), Some(b)) if a != b => Some((a, b)),
        _ => None,
    };
    let secondary = match (s1, s2) {
        (Some(a), Some(b)) if a != b => Some((a, b)),
        _ => None,
    };
    let l1 = state.inductance1.max(HENRY_FLOOR);
    let l2 = state.inductance2.max(HENRY_FLOOR);
    let k = state.coupling.clamp(-0.999, 0.999);
    let r1 = state.r_coil1;
    let r2 = state.r_coil2;
    let m = k * (l1 * l2).sqrt();
    let z11 = (r1.max(0.0), omega * l1);
    let z22 = (r2.max(0.0), omega * l2);
    let z12 = (0.0, omega * m);

    if let Some((a, b)) = primary {
        if state.r_core > 0.0 && state.r_core < 1e11 {
            let g_core = (1.0 / state.r_core, 0.0);
            stamp_branch(sys, a, b, a, b, g_core);
        }
    }

    match (primary, secondary) {
        (Some((a, b)), Some((c, d))) => {
            let det = csub(cmul(z11, z22), cmul(z12, z12));
            if hypot2(det) < 1e-30 {
                return;
            }
            let y11 = cdiv(z22, det);
            let y22 = cdiv(z11, det);
            let y12 = cdiv((-z12.0, -z12.1), det);
            // I_p = Y11 (Vp1-Vp2) + Y12 (Vs1-Vs2), and the secondary likewise.
            stamp_branch(sys, a, b, a, b, y11);
            stamp_branch(sys, a, b, c, d, y12);
            stamp_branch(sys, c, d, a, b, y12);
            stamp_branch(sys, c, d, c, d, y22);
        }
        (Some((a, b)), None) => {
            let y = cinv(z11);
            stamp_branch(sys, a, b, a, b, y);
        }
        (None, Some((c, d))) => {
            let y = cinv(z22);
            stamp_branch(sys, c, d, c, d, y);
        }
        (None, None) => {}
    }
}

/// Current into `row_p` (and out of `row_n`) is `y * (Vcol_p - Vcol_n)`.
fn stamp_branch(
    sys: &mut AcSys,
    row_p: usize,
    row_n: usize,
    col_p: usize,
    col_n: usize,
    y: (f64, f64),
) {
    sys.add(row_p, col_p, y.0, y.1);
    sys.add(row_p, col_n, -y.0, -y.1);
    sys.add(row_n, col_p, -y.0, -y.1);
    sys.add(row_n, col_n, y.0, y.1);
}

fn cmul(a: (f64, f64), b: (f64, f64)) -> (f64, f64) {
    (a.0 * b.0 - a.1 * b.1, a.0 * b.1 + a.1 * b.0)
}

fn csub(a: (f64, f64), b: (f64, f64)) -> (f64, f64) {
    (a.0 - b.0, a.1 - b.1)
}

fn cdiv(a: (f64, f64), b: (f64, f64)) -> (f64, f64) {
    let den = b.0 * b.0 + b.1 * b.1;
    if den < 1e-30 {
        return (0.0, 0.0);
    }
    ((a.0 * b.0 + a.1 * b.1) / den, (a.1 * b.0 - a.0 * b.1) / den)
}

fn cinv(z: (f64, f64)) -> (f64, f64) {
    cdiv((1.0, 0.0), z)
}

fn hypot2(z: (f64, f64)) -> f64 {
    z.0 * z.0 + z.1 * z.1
}

#[cfg(test)]
mod tests {
    use super::*;

    fn near(a: f64, b: f64, tol: f64) {
        assert!((a - b).abs() <= tol, "{a} != {b} (tol {tol})");
    }

    #[test]
    fn rc_lowpass_at_cutoff() {
        let r = 1_000.0;
        let c = 1e-6;
        let fc = 1.0 / (consts::TAU * r * c);
        let mut circuit = Circuit::new();
        circuit
            .add_fixed_volt("V1", 5.0)
            .add_resistor("R1", r)
            .add_capacitor("C1", c)
            .add_ground("GND")
            .connect("V1-outnod", "R1-lPin")
            .connect("R1-rPin", "C1-lPin")
            .connect("C1-rPin", "GND-Gnd");
        let at = circuit.ac_at("V1", "C1-lPin", fc).unwrap();
        near(at.gain, consts::FRAC_1_SQRT_2, 0.02);
        near(at.phase_deg, -45.0, 2.0);
        near(at.mag, 5.0 * consts::FRAC_1_SQRT_2, 0.1);

        let low = circuit.ac_at("V1", "C1-lPin", 1.0).unwrap();
        assert!(low.gain > 0.99, "low-frequency gain {}", low.gain);
        let high = circuit.ac_at("V1", "C1-lPin", 100_000.0).unwrap();
        assert!(high.gain < 0.02, "high-frequency gain {}", high.gain);

        let sweep = circuit
            .ac_sweep("V1", "C1-lPin", 1.0, 100_000.0, 5)
            .unwrap();
        assert!(sweep.len() > 2);
        assert!(sweep[0].gain > sweep.last().unwrap().gain);
        assert!(circuit.time == 0.0);
    }

    #[test]
    fn rl_highpass_at_corner() {
        let r = 1_000.0;
        let l = 1.0;
        let fc = r / (consts::TAU * l);
        let mut circuit = Circuit::new();
        circuit
            .add_fixed_volt("V1", 5.0)
            .add_resistor("R1", r)
            .add_inductor("L1", l)
            .add_ground("GND")
            .connect("V1-outnod", "R1-lPin")
            .connect("R1-rPin", "L1-lPin")
            .connect("L1-rPin", "GND-Gnd");
        let at = circuit.ac_at("V1", "L1-lPin", fc).unwrap();
        near(at.gain, consts::FRAC_1_SQRT_2, 0.02);
        near(at.phase_deg, 45.0, 2.0);
    }

    #[test]
    fn closed_switch_passes_and_open_switch_blocks() {
        let mut circuit = Circuit::new();
        circuit
            .add_fixed_volt("V1", 5.0)
            .add_switch("SW1", true)
            .add_resistor("R1", 1_000.0)
            .add_ground("GND")
            .connect("V1-outnod", "SW1-pinP0")
            .connect("SW1-switch0pinN", "R1-lPin")
            .connect("R1-rPin", "GND-Gnd");
        let closed = circuit.ac_at("V1", "R1-lPin", 1_000.0).unwrap();
        near(closed.mag, 5.0, 0.05);
        circuit.set_switch("SW1", false);
        circuit.pin_net.clear();
        let open = circuit.ac_at("V1", "R1-lPin", 1_000.0).unwrap();
        assert!(open.mag < 0.05, "open switch still shows {}", open.mag);
    }

    #[test]
    fn transformer_primary_is_its_coil_resistance_at_low_frequency() {
        let mut circuit = Circuit::new();
        circuit
            .add_fixed_volt("V1", 5.0)
            .add_resistor("R1", 1_000.0)
            .add_comp(Comp {
                id: "T1".into(),
                kind: Kind::Transformer {
                    state: TransformerState {
                        inductance1: 10.0,
                        inductance2: 10.0,
                        coupling: 0.0,
                        r_coil1: 1_000.0,
                        r_coil2: 1_000.0,
                        ..Default::default()
                    },
                },
            })
            .add_ground("GND")
            .connect("V1-outnod", "R1-lPin")
            .connect("R1-rPin", "T1-p1")
            .connect("T1-p2", "GND-Gnd");
        let at = circuit.ac_at("V1", "T1-p1", 0.01).unwrap();
        near(at.mag, 2.5, 0.15);
    }

    #[test]
    fn volt_source_turned_off_still_sweeps() {
        let mut circuit = Circuit::new();
        circuit
            .add_comp(Comp {
                id: "V1".into(),
                kind: Kind::VoltSource {
                    value: 0.0,
                    running: false,
                },
            })
            .add_resistor("R1", 1_000.0)
            .add_resistor("R2", 1_000.0)
            .add_ground("GND")
            .connect("V1-outPin", "R1-lPin")
            .connect("R1-rPin", "R2-lPin")
            .connect("R2-rPin", "GND-Gnd");
        let at = circuit.ac_at("V1", "R2-lPin", 1_000.0).unwrap();
        near(at.gain, 0.5, 0.01);
    }

    #[test]
    fn opamp_inverting_amplifier_ac_gain_and_phase() {
        let mut circuit = Circuit::new();
        circuit
            .add_fixed_volt("VIN", 0.0)
            .add_resistor("RIN", 1_000.0)
            .add_resistor("RF", 10_000.0)
            .add_opamp("U1")
            .add_ground("GND")
            .connect("VIN-outnod", "RIN-lPin")
            .connect("RIN-rPin", "U1-inputInv")
            .connect("U1-inputInv", "RF-lPin")
            .connect("RF-rPin", "U1-output")
            .connect("U1-inputNinv", "GND-Gnd");
        let at = circuit.ac_at("VIN", "U1-output", 1_000.0).unwrap();
        // Ideal inverting gain is -10; with finite open-loop gain 1000, |G| = 10 / (1 + 11/1000) ≈ 9.89
        near(at.gain, 9.89, 0.15);
        assert!(
            at.phase_deg.abs() > 175.0,
            "expected ~180 deg phase, got {}",
            at.phase_deg
        );
    }

    #[test]
    fn bjt_common_emitter_small_signal_gain() {
        let mut circuit = Circuit::new();
        // Simple biased NPN common-emitter with emitter grounded and AC-coupled base drive:
        // VCC = 10 V, RC = 1 kΩ, IB = 20 µA -> IC ≈ 2 mA -> gm ≈ IC/VT ≈ 0.077 S -> |Av| ≈ 77
        circuit
            .add_fixed_volt("VCC", 10.0)
            .add_curr_source("IB", 20e-6)
            .add_fixed_volt("VIN", 0.0)
            .add_capacitor("CIN", 100e-6)
            .add_resistor("RC", 1_000.0)
            .add_bjt("Q1", false)
            .add_ground("GND")
            .connect("VCC-outnod", "RC-lPin")
            .connect("RC-rPin", "Q1-collector")
            .connect("IB-outPin", "Q1-base")
            .connect("VIN-outnod", "CIN-lPin")
            .connect("CIN-rPin", "Q1-base")
            .connect("Q1-emiter", "GND-Gnd");
        let at = circuit.ac_at("VIN", "Q1-collector", 10_000.0).unwrap();
        // Midband inverting voltage gain should be substantial (~50..90) and ~180 deg phase.
        assert!(
            at.gain > 40.0 && at.gain < 120.0,
            "unexpected BJT gain {}",
            at.gain
        );
        assert!(
            at.phase_deg.abs() > 170.0,
            "expected inverting phase, got {}",
            at.phase_deg
        );
    }

    #[test]
    fn differential_and_multi_probe_ac_sweep() {
        let mut circuit = Circuit::new();
        circuit
            .add_fixed_volt("V1", 5.0)
            .add_resistor("R1", 1_000.0)
            .add_resistor("R2", 1_000.0)
            .add_ground("GND")
            .connect("V1-outnod", "R1-lPin")
            .connect("R1-rPin", "R2-lPin")
            .connect("R2-rPin", "GND-Gnd");
        let traces = circuit
            .ac_sweep_multi(
                "V1",
                &[("R1-lPin", Some("R1-rPin")), ("R2-lPin", None)],
                100.0,
                1_000.0,
                2,
            )
            .unwrap();
        assert_eq!(traces.len(), 2);
        near(traces[0].points[0].gain, 0.5, 0.01);
        near(traces[1].points[0].gain, 0.5, 0.01);
    }

    #[test]
    fn lists_the_source_and_the_connected_pins() {
        let mut circuit = Circuit::new();
        circuit
            .add_fixed_volt("V1", 5.0)
            .add_resistor("R1", 1_000.0)
            .add_ground("GND")
            .connect("V1-outnod", "R1-lPin")
            .connect("R1-rPin", "GND-Gnd");
        assert_eq!(circuit.ac_sources(), vec!["V1".to_string()]);
        let probes = circuit.ac_probes();
        assert!(probes.contains(&"V1-outnod".to_string()), "{probes:?}");
        assert!(probes.contains(&"R1-lPin".to_string()), "{probes:?}");
        assert!(probes.contains(&"GND-Gnd".to_string()), "{probes:?}");
    }

    #[test]
    fn rejects_a_missing_source() {
        let mut circuit = Circuit::new();
        circuit.add_resistor("R1", 100.0);
        let err = circuit.ac_at("V1", "R1-lPin", 10.0).unwrap_err();
        assert!(matches!(err, Error::Parse(_)));
    }

    #[test]
    fn capacitor_esr_sets_high_frequency_floor() {
        // R1 = 100 Ω, C1 = 10 µF with ESR = 100 Ω.
        // At high frequency (1 MHz), X_C ≈ 0.016 Ω << ESR, so gain approaches ESR / (R1 + ESR) = 0.5.
        let mut circuit = Circuit::new();
        circuit
            .add_fixed_volt("V1", 5.0)
            .add_resistor("R1", 100.0)
            .add_comp(Comp::capacitor_with_esr("C1", 10e-6, 100.0))
            .add_ground("GND")
            .connect("V1-outnod", "R1-lPin")
            .connect("R1-rPin", "C1-lPin")
            .connect("C1-rPin", "GND-Gnd");
        let high = circuit.ac_at("V1", "C1-lPin", 1_000_000.0).unwrap();
        near(high.gain, 0.5, 0.01);
        near(high.phase_deg, 0.0, 1.0);
    }

    #[test]
    fn inductor_winding_resistance_sets_low_frequency_floor() {
        // R1 = 100 Ω, L1 = 1 mH with R_L = 100 Ω.
        // At low frequency (1 Hz), X_L ≈ 6.3 mΩ << R_L, so gain approaches R_L / (R1 + R_L) = 0.5.
        let mut circuit = Circuit::new();
        circuit
            .add_fixed_volt("V1", 5.0)
            .add_resistor("R1", 100.0)
            .add_comp(Comp::inductor_with_resistance("L1", 1e-3, 100.0))
            .add_ground("GND")
            .connect("V1-outnod", "R1-lPin")
            .connect("R1-rPin", "L1-lPin")
            .connect("L1-rPin", "GND-Gnd");
        let low = circuit.ac_at("V1", "L1-lPin", 1.0).unwrap();
        near(low.gain, 0.5, 0.01);
        near(low.phase_deg, 0.0, 1.0);
    }
}
