//! DC parameter sweep and operating-point curve tracer.
//!
//! Steps independent sources (voltage, current, or parameters) across a range,
//! optionally nesting a secondary family parameter (e.g. stepping base current
//! while sweeping collector voltage), and records node voltages and branch currents.

use serde::{Deserialize, Serialize};

use crate::elements::{Comp, Kind, RESISTOR_MIN_OHMS};
use crate::{Error, Result};

use super::Circuit;

pub const MAX_DC_POINTS: usize = 50_000;

/// Information on a circuit component that can serve as a DC sweep source.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DcSourceInfo {
    pub id: String,
    pub kind: String,
    pub unit: String,
    pub default_val: f64,
}

/// Configuration parameters for a DC parameter sweep.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DcSweepParams {
    /// Id of the primary source stepped along the X axis.
    pub primary_source: String,
    pub primary_start: f64,
    pub primary_stop: f64,
    pub primary_step: f64,

    /// Optional secondary source stepped in the outer loop to generate curve families.
    pub secondary_source: Option<String>,
    pub secondary_start: f64,
    pub secondary_stop: f64,
    pub secondary_step: f64,

    /// Pin identifier to probe for voltage and current.
    pub probe_pin: String,
}

/// Operating-point measurement at a single sweep step.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct DcPoint {
    /// Value of the primary source at this step.
    pub primary_val: f64,
    /// Value of the secondary source for this curve family trace.
    pub secondary_val: f64,
    /// Measured voltage at the probe pin (relative to ground).
    pub v_probe: f64,
    /// Measured current flowing INTO the probe pin (in Amperes).
    pub i_probe: f64,
}

/// A single trace line in a curve family, holding all points for one secondary value.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DcCurve {
    pub secondary_val: f64,
    pub points: Vec<DcPoint>,
}

/// Complete results returned by a DC sweep analysis.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DcSweepResult {
    pub primary_source: String,
    pub secondary_source: Option<String>,
    pub probe_pin: String,
    pub curves: Vec<DcCurve>,
}

impl Circuit {
    /// Discovers all sweepable independent sources in the circuit.
    pub fn dc_sources(&self) -> Vec<DcSourceInfo> {
        self.components.iter().filter_map(source_info).collect()
    }

    /// Sorted list of connected pin IDs that can be probed.
    pub fn dc_probes(&mut self) -> Vec<String> {
        if self.pin_net.is_empty() {
            self.build_nets();
        }
        let mut pins: Vec<String> = self.pin_net.keys().cloned().collect();
        pins.sort();
        pins
    }

    /// Updates the value of an independent source in-place.
    pub fn set_dc_source_val(&mut self, id: &str, val: f64) -> Result<()> {
        for c in &mut self.components {
            if c.id == id {
                match &mut c.kind {
                    Kind::VoltSource { value, running } => {
                        *value = val;
                        *running = true;
                        return Ok(());
                    }
                    Kind::FixedVolt { voltage } | Kind::Rail { voltage } => {
                        *voltage = val;
                        return Ok(());
                    }
                    Kind::Battery { voltage, .. } => {
                        *voltage = val;
                        return Ok(());
                    }
                    Kind::CurrSource { value, running } => {
                        *value = val;
                        *running = true;
                        return Ok(());
                    }
                    Kind::Resistor { resistance } => {
                        *resistance = val.max(RESISTOR_MIN_OHMS);
                        return Ok(());
                    }
                    _ => {
                        return Err(Error::Parse(format!(
                            "Component `{id}` is not a sweepable DC source"
                        )));
                    }
                }
            }
        }
        Err(Error::Parse(format!(
            "DC source `{id}` not found in circuit"
        )))
    }

    /// Executes a DC parameter sweep on a cloned copy of the circuit.
    pub fn dc_sweep(&self, params: &DcSweepParams) -> Result<DcSweepResult> {
        let mut circ = self.clone();
        circ.prepare_dc_circuit()?;
        circ.execute_dc_sweep(params)
    }

    /// Prepares circuit for DC operating point analysis:
    /// - Capacitors become open circuits (zero capacitance).
    /// - Inductors become short circuits (low-value 1 µΩ resistor).
    fn prepare_dc_circuit(&mut self) -> Result<()> {
        for c in &mut self.components {
            match &mut c.kind {
                Kind::Capacitor { capacitance, .. }
                | Kind::ElCapacitor { capacitance, .. }
                | Kind::VarCapacitor { capacitance, .. } => {
                    *capacitance = 0.0;
                }
                Kind::Inductor { .. } | Kind::VarInductor { .. } => {
                    c.kind = Kind::Resistor { resistance: 1e-6 };
                }
                _ => {}
            }
        }
        self.build_nets();
        Ok(())
    }

    fn execute_dc_sweep(&mut self, params: &DcSweepParams) -> Result<DcSweepResult> {
        if self.pin_net.is_empty() {
            self.build_nets();
        }

        if !self.pin_net.contains_key(&params.probe_pin) {
            return Err(Error::Parse(format!(
                "Probe pin `{}` is not connected to any net",
                params.probe_pin
            )));
        }

        if !self
            .components
            .iter()
            .any(|c| c.id == params.primary_source)
        {
            return Err(Error::Parse(format!(
                "Primary source `{}` is not in circuit",
                params.primary_source
            )));
        }

        let has_secondary = if let Some(ref sec_id) = params.secondary_source {
            if !sec_id.is_empty() {
                if !self.components.iter().any(|c| &c.id == sec_id) {
                    return Err(Error::Parse(format!(
                        "Secondary source `{}` is not in circuit",
                        sec_id
                    )));
                }
                true
            } else {
                false
            }
        } else {
            false
        };

        let prim_values = linear_steps(
            params.primary_start,
            params.primary_stop,
            params.primary_step,
        )?;

        let sec_values = if has_secondary {
            linear_steps(
                params.secondary_start,
                params.secondary_stop,
                params.secondary_step,
            )?
        } else {
            vec![0.0]
        };

        let total_points = prim_values.len() * sec_values.len();
        if total_points > MAX_DC_POINTS {
            return Err(Error::Parse(format!(
                "DC sweep has too many points ({total_points} exceeds limit {MAX_DC_POINTS})"
            )));
        }

        let mut curves = Vec::with_capacity(sec_values.len());

        for &sec_val in &sec_values {
            if has_secondary {
                let sec_id = params.secondary_source.as_ref().unwrap();
                self.set_dc_source_val(sec_id, sec_val)?;
            }

            self.reset_device_stamps();
            let mut points = Vec::with_capacity(prim_values.len());

            for &prim_val in &prim_values {
                self.set_dc_source_val(&params.primary_source, prim_val)?;

                let mut solve_res = self.solve_circuit();
                if solve_res.is_err() {
                    // Reset stamps and retry if Newton-Raphson continuation stalled
                    self.reset_device_stamps();
                    solve_res = self.solve_circuit();
                }

                match solve_res {
                    Ok(()) => {
                        let v_probe = self.pin_voltage(&params.probe_pin).unwrap_or(0.0);
                        // Invert out-of-pin current so positive represents current entering the terminal
                        let i_probe = -self.current_out_of_pin(&params.probe_pin).unwrap_or(0.0);
                        points.push(DcPoint {
                            primary_val: prim_val,
                            secondary_val: sec_val,
                            v_probe,
                            i_probe,
                        });
                    }
                    Err(_) if !points.is_empty() => {
                        // Stalled at an extreme limit: hold the last converged point so the curve displays up to breakdown
                        let last = points.last().unwrap();
                        points.push(DcPoint {
                            primary_val: prim_val,
                            secondary_val: sec_val,
                            v_probe: last.v_probe,
                            i_probe: last.i_probe,
                        });
                    }
                    Err(e) => return Err(e),
                }
            }

            curves.push(DcCurve {
                secondary_val: sec_val,
                points,
            });
        }

        Ok(DcSweepResult {
            primary_source: params.primary_source.clone(),
            secondary_source: if has_secondary {
                params.secondary_source.clone()
            } else {
                None
            },
            probe_pin: params.probe_pin.clone(),
            curves,
        })
    }
}

fn source_info(c: &Comp) -> Option<DcSourceInfo> {
    match &c.kind {
        Kind::VoltSource { value, .. } => Some(DcSourceInfo {
            id: c.id.clone(),
            kind: "Voltage Source".into(),
            unit: "V".into(),
            default_val: *value,
        }),
        Kind::FixedVolt { voltage } => Some(DcSourceInfo {
            id: c.id.clone(),
            kind: "Fixed Voltage".into(),
            unit: "V".into(),
            default_val: *voltage,
        }),
        Kind::Rail { voltage } => Some(DcSourceInfo {
            id: c.id.clone(),
            kind: "Power Rail".into(),
            unit: "V".into(),
            default_val: *voltage,
        }),
        Kind::Battery { voltage, .. } => Some(DcSourceInfo {
            id: c.id.clone(),
            kind: "Battery".into(),
            unit: "V".into(),
            default_val: *voltage,
        }),
        Kind::CurrSource { value, .. } => Some(DcSourceInfo {
            id: c.id.clone(),
            kind: "Current Source".into(),
            unit: "A".into(),
            default_val: *value,
        }),
        Kind::Resistor { resistance } => Some(DcSourceInfo {
            id: c.id.clone(),
            kind: "Resistor".into(),
            unit: "Ω".into(),
            default_val: *resistance,
        }),
        _ => None,
    }
}

fn linear_steps(start: f64, stop: f64, step: f64) -> Result<Vec<f64>> {
    if !start.is_finite() || !stop.is_finite() || !step.is_finite() {
        return Err(Error::Parse(
            "Sweep limits and step size must be finite numbers".into(),
        ));
    }
    let step_mag = step.abs();
    if step_mag < 1e-15 {
        return Err(Error::Parse(
            "Sweep step size must be greater than zero".into(),
        ));
    }
    if (stop - start).abs() < 1e-15 {
        return Ok(vec![start]);
    }
    let delta = stop - start;
    let n_steps = (delta.abs() / step_mag).round() as usize;
    if n_steps == 0 {
        return Ok(vec![start, stop]);
    }
    if n_steps > MAX_DC_POINTS {
        return Err(Error::Parse(format!(
            "Sweep has too many points ({n_steps} exceeds limit {MAX_DC_POINTS})"
        )));
    }
    let signed_step = delta / n_steps as f64;
    let mut out = Vec::with_capacity(n_steps + 1);
    for i in 0..=n_steps {
        out.push(start + signed_step * i as f64);
    }
    if let Some(last) = out.last_mut() {
        *last = stop;
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linear_steps_generation() {
        let steps = linear_steps(0.0, 5.0, 1.0).unwrap();
        assert_eq!(steps.len(), 6);
        assert!((steps[0] - 0.0).abs() < 1e-12);
        assert!((steps[5] - 5.0).abs() < 1e-12);

        // Reverse step
        let rev = linear_steps(10.0, 0.0, 2.0).unwrap();
        assert_eq!(rev.len(), 6);
        assert!((rev[0] - 10.0).abs() < 1e-12);
        assert!((rev[5] - 0.0).abs() < 1e-12);

        // Zero step error
        assert!(linear_steps(0.0, 5.0, 0.0).is_err());
    }

    #[test]
    fn dc_sources_and_probes_discovery() {
        let mut c = Circuit::new();
        c.add_fixed_volt("V1", 5.0)
            .add_resistor("R1", 1000.0)
            .add_ground("GND")
            .connect("V1-outnod", "R1-lPin")
            .connect("R1-rPin", "GND-Gnd");

        let sources = c.dc_sources();
        assert!(sources.iter().any(|s| s.id == "V1"));
        assert!(sources.iter().any(|s| s.id == "R1"));

        let probes = c.dc_probes();
        assert!(probes.contains(&"V1-outnod".to_string()));
        assert!(probes.contains(&"R1-lPin".to_string()));
        assert!(probes.contains(&"R1-rPin".to_string()));
    }

    #[test]
    fn diode_iv_curve_exponential_knee() {
        let mut c = Circuit::new();
        c.add_volt_source("V1", 0.0)
            .add_resistor("R1", 100.0)
            .add_diode("D1")
            .add_ground("GND")
            .connect("V1-outPin", "R1-lPin")
            .connect("R1-rPin", "D1-lPin")
            .connect("D1-rPin", "GND-Gnd");

        let params = DcSweepParams {
            primary_source: "V1".into(),
            primary_start: 0.0,
            primary_stop: 2.0,
            primary_step: 0.05,
            secondary_source: None,
            secondary_start: 0.0,
            secondary_stop: 0.0,
            secondary_step: 1.0,
            probe_pin: "D1-lPin".into(),
        };

        let result = c.dc_sweep(&params).unwrap();
        assert_eq!(result.curves.len(), 1);
        let pts = &result.curves[0].points;
        assert_eq!(pts.len(), 41);

        // At low forward bias (< 0.4V), current is negligible (microamps or nanoamps)
        let pt_low = pts
            .iter()
            .find(|p| (p.primary_val - 0.3).abs() < 1e-4)
            .unwrap();
        assert!(
            pt_low.i_probe < 1e-4,
            "Diode current at 0.3V should be near zero: got {}",
            pt_low.i_probe
        );

        // At high input (2.0V), diode has turned on and is clamped around ~0.7V
        let pt_high = pts.last().unwrap();
        assert!(
            (pt_high.v_probe - 0.7).abs() < 0.15,
            "Diode knee voltage should be near 0.7V, got {}",
            pt_high.v_probe
        );
        // Current through 100 ohm resistor with ~1.3V across it should be ~13mA
        assert!(
            (pt_high.i_probe - 0.013).abs() < 0.005,
            "Current should be approx 13mA, got {}",
            pt_high.i_probe
        );
    }

    #[test]
    fn bjt_output_family_curves() {
        let mut c = Circuit::new();
        // NPN BJT common emitter circuit:
        // Base driven by current source IB
        // Collector driven by voltage source VCE through small resistor
        c.add_curr_source("IB", 10e-6)
            .add_volt_source("VCE", 0.0)
            .add_bjt("Q1", false) // false = NPN
            .add_ground("GND")
            .connect("IB-outPin", "Q1-base")
            .connect("VCE-outPin", "Q1-collector")
            .connect("Q1-emiter", "GND-Gnd");

        let params = DcSweepParams {
            primary_source: "VCE".into(),
            primary_start: 0.0,
            primary_stop: 5.0,
            primary_step: 0.2,
            secondary_source: Some("IB".into()),
            secondary_start: 10e-6,
            secondary_stop: 50e-6,
            secondary_step: 10e-6,
            probe_pin: "Q1-collector".into(),
        };

        let result = c.dc_sweep(&params).unwrap();
        assert_eq!(result.curves.len(), 5); // 10uA, 20uA, 30uA, 40uA, 50uA

        for (idx, curve) in result.curves.iter().enumerate() {
            let expected_ib = (idx + 1) as f64 * 10e-6;
            assert!((curve.secondary_val - expected_ib).abs() < 1e-9);

            // At VCE = 0, IC should be 0
            assert!(curve.points[0].i_probe.abs() < 1e-4);

            // In active linear region (VCE = 3V), IC should be approx beta * IB (beta = 100)
            let pt_active = curve
                .points
                .iter()
                .find(|p| (p.primary_val - 3.0).abs() < 1e-4)
                .unwrap();
            let expected_ic = 100.0 * expected_ib;
            let ratio = pt_active.i_probe / expected_ic;
            assert!(
                (ratio - 1.0).abs() < 0.15,
                "IC should be near beta * IB = {expected_ic}: got {} for IB = {expected_ib}",
                pt_active.i_probe
            );
        }
    }

    #[test]
    fn mosfet_transfer_and_output_characteristics() {
        let mut c = Circuit::new();
        // N-channel enhancement MOSFET:
        // Gate driven by VGS, Drain driven by VDS
        c.add_volt_source("VGS", 0.0)
            .add_volt_source("VDS", 5.0)
            .add_mosfet("M1", false, false) // N-channel, enhancement (Vth = 3.0V default)
            .add_ground("GND")
            .connect("VGS-outPin", "M1-Gate")
            .connect("VDS-outPin", "M1-Dren")
            .connect("M1-Sour", "GND-Gnd");

        // Sweep VGS from 0 to 5V to verify quadratic threshold turn-on
        let params = DcSweepParams {
            primary_source: "VGS".into(),
            primary_start: 0.0,
            primary_stop: 5.0,
            primary_step: 0.25,
            secondary_source: None,
            secondary_start: 0.0,
            secondary_stop: 0.0,
            secondary_step: 1.0,
            probe_pin: "M1-Dren".into(),
        };

        let result = c.dc_sweep(&params).unwrap();
        let pts = &result.curves[0].points;

        // Below Vth (3.0V), ID should be 0
        let pt_sub_th = pts
            .iter()
            .find(|p| (p.primary_val - 2.0).abs() < 1e-4)
            .unwrap();
        assert!(
            pt_sub_th.i_probe < 1e-6,
            "MOSFET ID below Vth must be near 0, got {}",
            pt_sub_th.i_probe
        );

        // Above Vth (e.g. at 4.5V and 5.0V), ID conducts substantially
        let pt_on = pts
            .iter()
            .find(|p| (p.primary_val - 4.5).abs() < 1e-4)
            .unwrap();
        assert!(
            pt_on.i_probe > 0.05,
            "MOSFET should conduct above Vth: got {}",
            pt_on.i_probe
        );
    }
}
