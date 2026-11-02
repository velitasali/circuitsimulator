//! Comprehensive integration tests for DC Parameter Sweep and Curve Tracer analysis.

use cs_engine::simulation::{Circuit, DcSweepParams};

#[test]
fn test_diode_forward_knee_and_reverse_saturation() {
    let mut c = Circuit::new();
    c.add_volt_source("V1", 0.0)
        .add_resistor("R1", 220.0)
        .add_diode("D1")
        .add_ground("GND")
        .connect("V1-outPin", "R1-lPin")
        .connect("R1-rPin", "D1-lPin")
        .connect("D1-rPin", "GND-Gnd");

    // Forward sweep from 0V to 3V
    let fwd_params = DcSweepParams {
        primary_source: "V1".into(),
        primary_start: 0.0,
        primary_stop: 3.0,
        primary_step: 0.1,
        secondary_source: None,
        secondary_start: 0.0,
        secondary_stop: 0.0,
        secondary_step: 1.0,
        probe_pin: "D1-lPin".into(),
    };

    let fwd_res = c
        .dc_sweep(&fwd_params)
        .expect("Forward sweep should succeed");
    let pts = &fwd_res.curves[0].points;
    assert_eq!(pts.len(), 31);

    // Below 0.2V, current is negligible
    let pt_low = pts
        .iter()
        .find(|p| (p.primary_val - 0.2).abs() < 1e-4)
        .unwrap();
    assert!(
        pt_low.i_probe < 1e-4,
        "Diode current below knee must be near zero: got {}",
        pt_low.i_probe
    );

    // At 3.0V input, diode forward voltage is clamped in forward conduction (~0.55-0.75V)
    let pt_max = pts.last().unwrap();
    assert!(
        pt_max.v_probe > 0.50 && pt_max.v_probe < 0.85,
        "Diode forward voltage must be near 0.55-0.75V, got {}",
        pt_max.v_probe
    );
    // Current through diode in forward conduction
    assert!(
        pt_max.i_probe > 0.005,
        "Current should be substantial (> 5mA)"
    );

    // Reverse sweep from 0V down to -5V
    let rev_params = DcSweepParams {
        primary_source: "V1".into(),
        primary_start: 0.0,
        primary_stop: -5.0,
        primary_step: 0.5,
        secondary_source: None,
        secondary_start: 0.0,
        secondary_stop: 0.0,
        secondary_step: 1.0,
        probe_pin: "D1-lPin".into(),
    };

    let rev_res = c
        .dc_sweep(&rev_params)
        .expect("Reverse sweep should succeed");
    for p in &rev_res.curves[0].points {
        // Reverse saturation leakage current should remain near zero (< 1 µA)
        assert!(
            p.i_probe.abs() < 1e-6,
            "Reverse leakage current must be < 1uA, got {}",
            p.i_probe
        );
    }
}

#[test]
fn test_zener_diode_reverse_breakdown() {
    let mut c = Circuit::new();
    // Connect Zener cathode to resistor and anode to ground for reverse breakdown
    c.add_volt_source("V1", 0.0)
        .add_resistor("R1", 100.0)
        .add_zener("Z1")
        .add_ground("GND")
        .connect("V1-outPin", "R1-lPin")
        .connect("R1-rPin", "Z1-rPin") // cathode
        .connect("Z1-lPin", "GND-Gnd"); // anode

    // Sweep V1 from 0 to 12V
    let params = DcSweepParams {
        primary_source: "V1".into(),
        primary_start: 0.0,
        primary_stop: 12.0,
        primary_step: 0.5,
        secondary_source: None,
        secondary_start: 0.0,
        secondary_stop: 0.0,
        secondary_step: 1.0,
        probe_pin: "Z1-rPin".into(),
    };

    let res = c.dc_sweep(&params).expect("Zener sweep should succeed");
    let pts = &res.curves[0].points;

    // At low reverse voltage (e.g. 2V), Zener is off, so cathode voltage tracks input
    let pt_off = pts
        .iter()
        .find(|p| (p.primary_val - 2.0).abs() < 1e-4)
        .unwrap();
    assert!(
        (pt_off.v_probe - 2.0).abs() < 0.1,
        "Before breakdown, Vz tracks input"
    );
    assert!(
        pt_off.i_probe < 1e-4,
        "Before breakdown, current is negligible"
    );

    // At 12V input, Zener breaks down and clamps cathode voltage to ~5.6V (default Zener voltage)
    let pt_on = pts.last().unwrap();
    assert!(
        pt_on.v_probe < 7.0 && pt_on.v_probe > 4.5,
        "Zener must clamp reverse voltage near Vz (5-6V): got {}",
        pt_on.v_probe
    );
    assert!(
        pt_on.i_probe > 0.03,
        "Zener conducts significant breakdown current: got {}",
        pt_on.i_probe
    );
}

#[test]
fn test_npn_bjt_common_emitter_family_and_saturation() {
    let mut c = Circuit::new();
    c.add_curr_source("IB", 10e-6)
        .add_volt_source("VCE", 0.0)
        .add_bjt("Q1", false) // NPN
        .add_ground("GND")
        .connect("IB-outPin", "Q1-base")
        .connect("VCE-outPin", "Q1-collector")
        .connect("Q1-emiter", "GND-Gnd");

    let params = DcSweepParams {
        primary_source: "VCE".into(),
        primary_start: 0.0,
        primary_stop: 6.0,
        primary_step: 0.2,
        secondary_source: Some("IB".into()),
        secondary_start: 10e-6,
        secondary_stop: 40e-6,
        secondary_step: 10e-6,
        probe_pin: "Q1-collector".into(),
    };

    let result = c.dc_sweep(&params).expect("BJT sweep should succeed");
    assert_eq!(result.curves.len(), 4);

    for (idx, curve) in result.curves.iter().enumerate() {
        let ib = (idx + 1) as f64 * 10e-6;

        // At VCE = 0, collector current is negative (flowing out of collector due to forward-biased BC junction)
        assert!(curve.points[0].i_probe < 0.0);

        // Active region: IC ≈ beta * IB (beta = 100)
        let pt_act = curve
            .points
            .iter()
            .find(|p| (p.primary_val - 4.0).abs() < 1e-4)
            .unwrap();

        // Active region: IC ≈ beta * IB (beta = 100)
        let expected_ic = 100.0 * ib;
        assert!(
            (pt_act.i_probe - expected_ic).abs() / expected_ic < 0.15,
            "Active IC ({}) should be approx beta * IB ({expected_ic})",
            pt_act.i_probe
        );
    }
}

#[test]
fn test_pnp_bjt_common_emitter_sweep() {
    let mut c = Circuit::new();
    // PNP BJT: Emitter connected to 5V rail, base pulled down by 100k resistor to GND, collector swept with VC
    c.add_fixed_volt("VCC", 5.0)
        .add_resistor("RB", 100_000.0)
        .add_volt_source("VC", 5.0)
        .add_bjt("Q1", true) // PNP
        .add_ground("GND")
        .connect("VCC-outnod", "Q1-emiter")
        .connect("Q1-base", "RB-lPin")
        .connect("RB-rPin", "GND-Gnd")
        .connect("Q1-collector", "VC-outPin");

    let params = DcSweepParams {
        primary_source: "VC".into(),
        primary_start: 5.0,
        primary_stop: 0.0,
        primary_step: 0.25,
        secondary_source: None,
        secondary_start: 0.0,
        secondary_stop: 0.0,
        secondary_step: 1.0,
        probe_pin: "Q1-collector".into(),
    };

    let result = c.dc_sweep(&params).expect("PNP sweep should succeed");
    let pts = &result.curves[0].points;

    // At VC = 5.0V (VEC = 0V), collector current is near zero
    assert!(pts[0].i_probe.abs() < 1e-3);

    // As VC drops (increasing VEC), PNP collector delivers current outward (negative into pin)
    let pt_low = pts.last().unwrap();
    assert!(
        pt_low.i_probe < -0.001,
        "PNP collector should supply current outward: got {}",
        pt_low.i_probe
    );
}

#[test]
fn test_mosfet_output_family_and_saturation() {
    let mut c = Circuit::new();
    c.add_volt_source("VGS", 3.5)
        .add_volt_source("VDS", 0.0)
        .add_mosfet("M1", false, false) // N-channel enhancement (Vth = 3.0V default)
        .add_ground("GND")
        .connect("VGS-outPin", "M1-Gate")
        .connect("VDS-outPin", "M1-Dren")
        .connect("M1-Sour", "GND-Gnd");

    // Output family: sweep VDS from 0 to 8V for VGS = 3.5V, 4.0V, 4.5V, 5.0V
    let params = DcSweepParams {
        primary_source: "VDS".into(),
        primary_start: 0.0,
        primary_stop: 8.0,
        primary_step: 0.25,
        secondary_source: Some("VGS".into()),
        secondary_start: 3.5,
        secondary_stop: 5.0,
        secondary_step: 0.5,
        probe_pin: "M1-Dren".into(),
    };

    let result = c
        .dc_sweep(&params)
        .expect("MOSFET family sweep should succeed");
    assert_eq!(result.curves.len(), 4);

    // Higher VGS must produce higher saturation ID
    let mut prev_sat_id = 0.0;
    for curve in &result.curves {
        let sat_pt = curve
            .points
            .iter()
            .find(|p| (p.primary_val - 6.0).abs() < 1e-4)
            .unwrap();
        assert!(
            sat_pt.i_probe > prev_sat_id,
            "Saturation ID must monotonically increase with VGS: got {} vs prev {prev_sat_id}",
            sat_pt.i_probe
        );
        prev_sat_id = sat_pt.i_probe;

        // At VDS = 0, ID must be zero
        assert!(curve.points[0].i_probe.abs() < 1e-5);
    }
}

#[test]
fn test_dc_sweep_capacitors_open_and_inductors_short() {
    let mut c = Circuit::new();
    // Circuit with:
    // - Series resistor R1 (100 ohm)
    // - Parallel capacitor C1 (10 uF) to GND (should be open in DC)
    // - Series inductor L1 (10 mH) (should be short in DC)
    // - Load resistor R2 (100 ohm) to GND
    //
    // Equivalent DC circuit is a voltage divider with R1 = 100, R2 = 100.
    // At V1 = 10V, V(R2) = 5V.
    c.add_volt_source("V1", 10.0)
        .add_resistor("R1", 100.0)
        .add_capacitor("C1", 10e-6)
        .add_inductor("L1", 10e-3)
        .add_resistor("R2", 100.0)
        .add_ground("GND")
        .connect("V1-outPin", "R1-lPin")
        .connect("R1-rPin", "C1-lPin")
        .connect("C1-lPin", "L1-lPin")
        .connect("L1-rPin", "R2-lPin")
        .connect("C1-rPin", "GND-Gnd")
        .connect("R2-rPin", "GND-Gnd");

    let params = DcSweepParams {
        primary_source: "V1".into(),
        primary_start: 0.0,
        primary_stop: 10.0,
        primary_step: 2.0,
        secondary_source: None,
        secondary_start: 0.0,
        secondary_stop: 0.0,
        secondary_step: 1.0,
        probe_pin: "R2-lPin".into(),
    };

    let result = c
        .dc_sweep(&params)
        .expect("DC sweep with reactive elements should succeed");
    let pts = &result.curves[0].points;
    let pt10 = pts.last().unwrap();

    assert!(
        (pt10.v_probe - 5.0).abs() < 0.01,
        "Voltage divider across DC reactive circuit must give 5.0V: got {}",
        pt10.v_probe
    );
    assert!(
        (pt10.i_probe - 0.05).abs() < 0.001,
        "Current into R2 must be 50mA: got {}",
        pt10.i_probe
    );
}

#[test]
fn test_dc_sweep_validation_errors() {
    let mut c = Circuit::new();
    c.add_volt_source("V1", 5.0)
        .add_resistor("R1", 100.0)
        .add_ground("GND")
        .connect("V1-outPin", "R1-lPin")
        .connect("R1-rPin", "GND-Gnd");

    // Nonexistent source
    let bad_source = DcSweepParams {
        primary_source: "V_UNKNOWN".into(),
        primary_start: 0.0,
        primary_stop: 5.0,
        primary_step: 1.0,
        secondary_source: None,
        secondary_start: 0.0,
        secondary_stop: 0.0,
        secondary_step: 1.0,
        probe_pin: "R1-lPin".into(),
    };
    assert!(c.dc_sweep(&bad_source).is_err());

    // Disconnected probe
    let bad_probe = DcSweepParams {
        primary_source: "V1".into(),
        primary_start: 0.0,
        primary_stop: 5.0,
        primary_step: 1.0,
        secondary_source: None,
        secondary_start: 0.0,
        secondary_stop: 0.0,
        secondary_step: 1.0,
        probe_pin: "UNCONNECTED_PIN".into(),
    };
    assert!(c.dc_sweep(&bad_probe).is_err());

    // Zero step size
    let zero_step = DcSweepParams {
        primary_source: "V1".into(),
        primary_start: 0.0,
        primary_stop: 5.0,
        primary_step: 0.0,
        secondary_source: None,
        secondary_start: 0.0,
        secondary_stop: 0.0,
        secondary_step: 1.0,
        probe_pin: "R1-lPin".into(),
    };
    assert!(c.dc_sweep(&zero_step).is_err());
}
