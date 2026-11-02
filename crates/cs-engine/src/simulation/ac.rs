//! Small-signal AC sweep for linear R, L, and C.
//!
//! The time-domain matrix is real. This solver builds its own interleaved
//! real/imaginary system, `Y(jω) V = I`, and leaves node voltages and the
//! simulator clock alone. Nonlinear and digital parts are left out of the
//! stamp. A DC source that is not the excited input is an AC short.

use crate::Error;
use crate::elements::{Kind, SOURCE_ADMIT, SWITCH_CLOSED_ADMIT};

use super::Circuit;

const OHM_FLOOR: f64 = 1e-12;
const HENRY_FLOOR: f64 = 1e-12;
const MAX_POINTS: usize = 20_000;

/// One frequency of an AC sweep. `mag` is `|Vprobe|` in volts.
/// `gain` is `mag` divided by the excited source amplitude.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AcPoint {
    pub hz: f64,
    pub mag: f64,
    pub phase_deg: f64,
    pub gain: f64,
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
    /// excited source. Its amplitude is that part's DC voltage.
    pub fn ac_at(&mut self, source_id: &str, probe_pin: &str, hz: f64) -> Result<AcPoint, Error> {
        if !hz.is_finite() || hz <= 0.0 {
            return Err(Error::Parse(
                "AC frequency must be a finite number above 0".into(),
            ));
        }
        self.ac_point(source_id, probe_pin, hz)
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

    /// Log-spaced sweep from `f_start` to `f_stop`, inclusive.
    pub fn ac_sweep(
        &mut self,
        source_id: &str,
        probe_pin: &str,
        f_start: f64,
        f_stop: f64,
        points_per_decade: u32,
    ) -> Result<Vec<AcPoint>, Error> {
        let freqs = log_freqs(f_start, f_stop, points_per_decade)?;
        let mut out = Vec::with_capacity(freqs.len());
        for hz in freqs {
            out.push(self.ac_point(source_id, probe_pin, hz)?);
        }
        Ok(out)
    }

    fn ac_point(&mut self, source_id: &str, probe_pin: &str, hz: f64) -> Result<AcPoint, Error> {
        if !self.components.iter().any(|c| c.id == source_id) {
            return Err(Error::Parse(format!(
                "AC source `{source_id}` is not in the circuit"
            )));
        }
        let amplitude = source_amplitude(self, source_id)?;
        if self.pin_net.is_empty() {
            self.build_nets();
        }
        let Some(&probe) = self.pin_net.get(probe_pin) else {
            return Err(Error::Parse(format!(
                "AC probe `{probe_pin}` is not on a net"
            )));
        };
        let omega = std::f64::consts::TAU * hz;
        let n = self.nodes.len();
        let mut sys = AcSys::new(n);
        for comp in &self.components {
            stamp_comp(&mut sys, comp, source_id, omega, &self.pin_net);
        }
        sys.solve()?;
        if 2 * probe + 1 >= sys.b.len() {
            return Err(Error::Singular);
        }
        let vr = sys.b[2 * probe];
        let vi = sys.b[2 * probe + 1];
        let mag = vr.hypot(vi);
        let phase_deg = vi.atan2(vr).to_degrees();
        let gain = if amplitude.abs() > 1e-15 {
            mag / amplitude.abs()
        } else {
            mag
        };
        Ok(AcPoint {
            hz,
            mag,
            phase_deg,
            gain,
        })
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
        _ => Err(Error::Parse(format!(
            "AC source `{source_id}` is not a voltage source"
        ))),
    }
}

fn net(pin_net: &rustc_hash::FxHashMap<String, usize>, pin: &str) -> Option<usize> {
    pin_net.get(pin).copied()
}

fn stamp_g(sys: &mut AcSys, a: Option<usize>, b: Option<usize>, g: f64) {
    sys.stamp_y(a, b, g, 0.0);
}

fn stamp_comp(
    sys: &mut AcSys,
    comp: &crate::elements::Comp,
    source_id: &str,
    omega: f64,
    pin_net: &rustc_hash::FxHashMap<String, usize>,
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
        Kind::Capacitor { capacitance, .. }
        | Kind::ElCapacitor { capacitance, .. }
        | Kind::VarCapacitor { capacitance, .. } => {
            let yi = omega * *capacitance;
            let a = net(pin_net, &comp.left_pin());
            let b = net(pin_net, &comp.right_pin());
            sys.stamp_y(a, b, 0.0, yi);
        }
        Kind::Inductor { inductance, .. } | Kind::VarInductor { inductance, .. } => {
            let l = inductance.max(HENRY_FLOOR);
            let yi = -1.0 / (omega * l);
            let a = net(pin_net, &comp.left_pin());
            let b = net(pin_net, &comp.right_pin());
            sys.stamp_y(a, b, 0.0, yi);
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
        Kind::FixedVolt { voltage } | Kind::Rail { voltage } | Kind::Clock { voltage, .. } => {
            let pin = comp.out_pin();
            if let Some(n) = net(pin_net, &pin) {
                stamp_g(sys, Some(n), None, SOURCE_ADMIT);
                if excited {
                    sys.add_i(n, *voltage * SOURCE_ADMIT, 0.0);
                }
            }
        }
        Kind::VoltSource { value, running } => {
            if *running {
                let pin = format!("{}-outPin", comp.id);
                if let Some(n) = net(pin_net, &pin) {
                    stamp_g(sys, Some(n), None, SOURCE_ADMIT);
                    if excited {
                        sys.add_i(n, *value * SOURCE_ADMIT, 0.0);
                    }
                }
            }
        }
        Kind::Battery {
            voltage,
            resistance,
        } => {
            let g = 1.0 / resistance.max(1e-14);
            let a = net(pin_net, &comp.left_pin());
            let b = net(pin_net, &comp.right_pin());
            stamp_g(sys, a, b, g);
            if excited {
                if let Some(p) = a {
                    sys.add_i(p, *voltage * g, 0.0);
                }
                if let Some(m) = b {
                    sys.add_i(m, -*voltage * g, 0.0);
                }
            }
        }
        Kind::CurrSource { value, running } => {
            if *running && excited {
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
        Kind::Transformer {
            inductance1,
            inductance2,
            coupling,
            r_coil1,
            r_coil2,
            ..
        } => stamp_transformer(
            sys,
            comp,
            pin_net,
            omega,
            *inductance1,
            *inductance2,
            *coupling,
            *r_coil1,
            *r_coil2,
        ),
        _ => {}
    }
}

fn stamp_transformer(
    sys: &mut AcSys,
    comp: &crate::elements::Comp,
    pin_net: &rustc_hash::FxHashMap<String, usize>,
    omega: f64,
    l1: f64,
    l2: f64,
    k: f64,
    r1: f64,
    r2: f64,
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
    let l1 = l1.max(HENRY_FLOOR);
    let l2 = l2.max(HENRY_FLOOR);
    let m = k.clamp(-0.999, 0.999) * (l1 * l2).sqrt();
    let z11 = (r1.max(0.0), omega * l1);
    let z22 = (r2.max(0.0), omega * l2);
    let z12 = (0.0, omega * m);
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
        let fc = 1.0 / (std::f64::consts::TAU * r * c);
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
        near(at.gain, std::f64::consts::FRAC_1_SQRT_2, 0.02);
        near(at.phase_deg, -45.0, 2.0);
        near(at.mag, 5.0 * std::f64::consts::FRAC_1_SQRT_2, 0.1);

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
        let fc = r / (std::f64::consts::TAU * l);
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
        near(at.gain, std::f64::consts::FRAC_1_SQRT_2, 0.02);
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
            .add_comp(crate::elements::Comp {
                id: "T1".into(),
                kind: Kind::Transformer {
                    inductance1: 10.0,
                    inductance2: 10.0,
                    coupling: 0.0,
                    r_coil1: 1_000.0,
                    r_coil2: 1_000.0,
                    i1: 0.0,
                    i2: 0.0,
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
}
