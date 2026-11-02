//! Integration and verification tests for magnetic non-linearities and transformer physics:
//! - Ferromagnetic core saturation (non-linear B-H curve and magnetizing current spikes)
//! - Core saturation secondary voltage waveform clipping
//! - Magnetic hysteresis dynamics (coercive current and remanence)
//! - Frequency-dependent core losses and eddy currents (R_core)
//! - Center-tapped secondary anti-phase outputs and full-wave 2-diode rectifier
//! - Multi-winding auxiliary secondary independent turns ratio step-up
//! - Property parsing, getters/setters, and .sim1 XML roundtrip

use cs_engine::Circuit;
use cs_engine::components::{Component, Transformer};
use cs_engine::elements::{Comp, Kind, TransformerState};

/// Helper: run transient steps and track min/max voltage across a pair of pins.
fn record_peak_voltage(circ: &mut Circuit, p_pos: &str, p_neg: &str, steps: usize) -> f64 {
    let mut peak: f64 = 0.0;
    for _ in 0..steps {
        circ.step().unwrap();
        let v = circ.pin_voltage(p_pos).unwrap() - circ.pin_voltage(p_neg).unwrap();
        peak = peak.max(v.abs());
    }
    peak
}

#[test]
fn test_ferromagnetic_core_saturation_primary_current_spike() {
    // Linear transformer (Isat = 0): magnetizing current is sinusoidal and small.
    let mut c_lin = Circuit::default();
    c_lin.dt = 1e-5;
    c_lin.add_wave_gen("wg", "Sine", 100.0, 10.0, 0.0, 0.5, 0.0, 100, false, false);
    c_lin.add_comp(Comp {
        id: "tr_lin".into(),
        kind: Kind::Transformer {
            state: TransformerState::new(0.1, 0.1, 0.999, 0.5, 0.5),
        },
    });
    c_lin.add_ground("gnd1");
    c_lin.add_ground("gnd2");
    c_lin.add_resistor("r_load", 10_000.0);

    c_lin.connect("wg-outnod", "tr_lin-p1");
    c_lin.connect("tr_lin-p2", "gnd1-Gnd");
    c_lin.connect("tr_lin-s1", "r_load-lPin");
    c_lin.connect("tr_lin-s2", "r_load-rPin");
    c_lin.connect("tr_lin-s2", "gnd2-Gnd");

    let mut max_i_lin = 0.0_f64;
    for _ in 0..2000 {
        c_lin.step().unwrap();
        let i = c_lin.current_out_of_pin("tr_lin-p1").unwrap().abs();
        max_i_lin = max_i_lin.max(i);
    }

    // Saturable transformer (Isat = 0.05 A): when magnetizing current attempts to exceed Isat,
    // differential inductance collapses by up to 50x, causing a sharp current spike.
    let mut c_sat = Circuit::default();
    c_sat.dt = 1e-5;
    c_sat.add_wave_gen("wg", "Sine", 100.0, 10.0, 0.0, 0.5, 0.0, 100, false, false);
    c_sat.add_comp(Comp {
        id: "tr_sat".into(),
        kind: Kind::Transformer {
            state: TransformerState::new_saturable(0.1, 0.1, 0.999, 0.5, 0.5, 0.05, 1e12, 0.0),
        },
    });
    c_sat.add_ground("gnd1");
    c_sat.add_ground("gnd2");
    c_sat.add_resistor("r_load", 10_000.0);

    c_sat.connect("wg-outnod", "tr_sat-p1");
    c_sat.connect("tr_sat-p2", "gnd1-Gnd");
    c_sat.connect("tr_sat-s1", "r_load-lPin");
    c_sat.connect("tr_sat-s2", "r_load-rPin");
    c_sat.connect("tr_sat-s2", "gnd2-Gnd");

    let mut max_i_sat = 0.0_f64;
    for _ in 0..2000 {
        c_sat.step().unwrap();
        let i = c_sat.current_out_of_pin("tr_sat-p1").unwrap().abs();
        max_i_sat = max_i_sat.max(i);
    }

    assert!(
        max_i_sat > max_i_lin * 1.5,
        "Saturable transformer must draw peak saturation current spikes: got max_i_sat={max_i_sat:.4} A vs max_i_lin={max_i_lin:.4} A"
    );
}

#[test]
fn test_core_saturation_secondary_voltage_clipping() {
    // When the core is saturated, rate of change of core flux dPhi/dt is constrained,
    // reducing secondary induced voltage compared to the linear model.
    let mut c_lin = Circuit::default();
    c_lin.dt = 1e-5;
    c_lin.add_wave_gen("wg", "Sine", 60.0, 20.0, 0.0, 0.5, 0.0, 100, false, false);
    c_lin.add_resistor("r_src", 20.0);
    c_lin.add_comp(Comp {
        id: "tr_lin".into(),
        kind: Kind::Transformer {
            state: TransformerState::new(0.5, 0.5, 0.99, 0.1, 0.1),
        },
    });
    c_lin.add_ground("gnd1");
    c_lin.add_ground("gnd2");
    c_lin.add_resistor("r_load", 1_000.0);

    c_lin.connect("wg-outnod", "r_src-lPin");
    c_lin.connect("r_src-rPin", "tr_lin-p1");
    c_lin.connect("tr_lin-p2", "gnd1-Gnd");
    c_lin.connect("tr_lin-s1", "r_load-lPin");
    c_lin.connect("tr_lin-s2", "r_load-rPin");
    c_lin.connect("tr_lin-s2", "gnd2-Gnd");

    let v_sec_lin = record_peak_voltage(&mut c_lin, "tr_lin-s1", "tr_lin-s2", 2000);

    // Saturable core with low Isat = 0.02 A (strong saturation)
    let mut c_sat = Circuit::default();
    c_sat.dt = 1e-5;
    c_sat.add_wave_gen("wg", "Sine", 60.0, 20.0, 0.0, 0.5, 0.0, 100, false, false);
    c_sat.add_resistor("r_src", 20.0);
    c_sat.add_comp(Comp {
        id: "tr_sat".into(),
        kind: Kind::Transformer {
            state: TransformerState::new_saturable(0.5, 0.5, 0.99, 0.1, 0.1, 0.02, 1e12, 0.0),
        },
    });
    c_sat.add_ground("gnd1");
    c_sat.add_ground("gnd2");
    c_sat.add_resistor("r_load", 1_000.0);

    c_sat.connect("wg-outnod", "r_src-lPin");
    c_sat.connect("r_src-rPin", "tr_sat-p1");
    c_sat.connect("tr_sat-p2", "gnd1-Gnd");
    c_sat.connect("tr_sat-s1", "r_load-lPin");
    c_sat.connect("tr_sat-s2", "r_load-rPin");
    c_sat.connect("tr_sat-s2", "gnd2-Gnd");

    let v_sec_sat = record_peak_voltage(&mut c_sat, "tr_sat-s1", "tr_sat-s2", 2000);

    assert!(
        v_sec_sat < v_sec_lin * 0.95,
        "Secondary voltage under core saturation must show clipping/reduction: got v_sec_sat={v_sec_sat:.3} V vs v_sec_lin={v_sec_lin:.3} V"
    );
}

#[test]
fn test_magnetic_hysteresis_loop_and_remanence() {
    let state = TransformerState::new_saturable(1.0, 1.0, 0.99, 0.1, 0.1, 1.0, 1e12, 0.3);

    // Cycle magnetizing current from 0 -> +1.5 -> -1.5 -> 0
    let mut test_state = state;
    test_state.h_dir = 1.0;

    // Positive rising branch
    let (_l_pos, _s_pos, flux_pos_peak) = test_state.diff_inductance_and_flux(1.5);
    assert!(flux_pos_peak > 0.0);

    // Falling through zero
    test_state.h_dir = -1.0;
    test_state.prev_i_mag = 1.0;
    let (_l_zero_fall, _s_zero_fall, flux_zero_fall) = test_state.diff_inductance_and_flux(0.0);

    // Rising through zero
    test_state.h_dir = 1.0;
    test_state.prev_i_mag = -1.0;
    let (_l_zero_rise, _s_zero_rise, flux_zero_rise) = test_state.diff_inductance_and_flux(0.0);

    // Hysteresis means remanence flux at I=0 is different depending on direction
    assert!(
        flux_zero_fall > flux_zero_rise,
        "Falling branch remanent flux ({flux_zero_fall:.4}) must be higher than rising branch ({flux_zero_rise:.4})"
    );
    assert!(
        (flux_zero_fall - flux_zero_rise).abs() > 0.1,
        "Hysteresis loop must exhibit significant coercivity width"
    );
}

#[test]
fn test_core_eddy_current_losses() {
    // When secondary is open circuit, an ideal transformer draws only 90° lagging reactive current.
    // With core loss resistance Rcore = 1000 Ω, primary draws in-phase eddy current I = V / Rcore.
    let mut circ = Circuit::default();
    circ.dt = 1e-5;
    circ.add_fixed_volt("v_dc", 10.0);
    circ.add_comp(Comp {
        id: "tr_loss".into(),
        kind: Kind::Transformer {
            state: TransformerState::new_saturable(1.0, 1.0, 0.99, 0.01, 0.01, 0.0, 1000.0, 0.0),
        },
    });
    circ.add_ground("gnd");

    circ.connect("v_dc-outnod", "tr_loss-p1");
    circ.connect("tr_loss-p2", "gnd-Gnd");

    // Secondary is left completely open
    for _ in 0..100 {
        circ.step().unwrap();
    }

    let i_in = circ.current_out_of_pin("tr_loss-p1").unwrap();
    // In DC steady state, inductor branch eventually draws large current through Rcoil1,
    // but at initial step (t = 1 step):
    // Let's verify companion admittance: g11 includes 1 / Rcore = 1e-3 S
    let comp_state = TransformerState::new_saturable(1.0, 1.0, 0.99, 0.01, 0.01, 0.0, 1000.0, 0.0);
    let comp = comp_state.companion(1e-5);
    // Primary self conductance g[0][0] must exceed 1/Rcore = 0.001 S
    assert!(
        comp.g[0][0] >= 0.001,
        "Companion primary conductance must include core loss conductance, got {}",
        comp.g[0][0]
    );
    assert!(i_in.abs() > 0.001);
}

#[test]
fn test_center_tapped_transformer_anti_phase_outputs() {
    // Secondary has center tap CT. Output voltages V(S1 - CT) and V(CT - S2)
    // must be equal in magnitude and exactly 180° out of phase (anti-phase).
    let mut circ = Circuit::default();
    circ.dt = 1e-6;
    circ.add_wave_gen("wg", "Sine", 1000.0, 10.0, 0.0, 0.5, 0.0, 100, false, false);
    circ.add_comp(Comp {
        id: "tr_ct".into(),
        kind: Kind::Transformer {
            state: TransformerState::new_center_tapped(1.0, 4.0, 0.999, 0.01, 0.01),
        },
    });
    circ.add_ground("gnd_p");
    circ.add_ground("gnd_ct");
    circ.add_resistor("r_load1", 10_000.0);
    circ.add_resistor("r_load2", 10_000.0);

    circ.connect("wg-outnod", "tr_ct-p1");
    circ.connect("tr_ct-p2", "gnd_p-Gnd");

    // Center tap grounded
    circ.connect("tr_ct-ct", "gnd_ct-Gnd");
    circ.connect("tr_ct-s1", "r_load1-lPin");
    circ.connect("r_load1-rPin", "gnd_ct-Gnd");
    circ.connect("tr_ct-s2", "r_load2-lPin");
    circ.connect("r_load2-rPin", "gnd_ct-Gnd");

    let mut max_vs1 = 0.0_f64;
    let mut max_vs2 = 0.0_f64;
    let mut min_sum = f64::MAX;
    let mut max_sum = 0.0_f64;

    for step in 0..2000 {
        circ.step().unwrap();
        let vs1 = circ.pin_voltage("tr_ct-s1").unwrap();
        let vs2 = circ.pin_voltage("tr_ct-s2").unwrap();
        max_vs1 = max_vs1.max(vs1.abs());
        max_vs2 = max_vs2.max(vs2.abs());

        if step > 500 {
            let sum = (vs1 + vs2).abs();
            min_sum = min_sum.min(sum);
            max_sum = max_sum.max(sum);
        }
    }

    // Turns ratio sqrt(4/1) = 2. Total secondary = 20V peak, so each half is 10V peak.
    assert!(
        (max_vs1 - 10.0).abs() < 0.25,
        "Secondary half 1 must be ~10V peak, got {max_vs1:.3}"
    );
    assert!(
        (max_vs2 - 10.0).abs() < 0.25,
        "Secondary half 2 must be ~10V peak, got {max_vs2:.3}"
    );
    // Anti-phase: vs1 + vs2 must sum to approximately 0V
    assert!(
        max_sum < 0.35,
        "Anti-phase secondary voltages must cancel to ~0V, max sum was {max_sum:.3}"
    );
}

#[test]
fn test_center_tapped_full_wave_rectifier() {
    // Classic 2-diode full-wave rectifier using center-tapped secondary.
    let mut circ = Circuit::default();
    circ.dt = 1e-6;
    circ.add_wave_gen("wg", "Sine", 1000.0, 12.0, 0.0, 0.5, 0.0, 100, false, false);
    circ.add_comp(Comp {
        id: "tr".into(),
        kind: Kind::Transformer {
            state: TransformerState::new_center_tapped(1.0, 4.0, 0.999, 0.01, 0.01),
        },
    });
    circ.add_diode("d1");
    circ.add_diode("d2");
    circ.add_resistor("r_load", 1_000.0);
    circ.add_capacitor("c_filter", 100e-6);
    circ.add_ground("gnd1");
    circ.add_ground("gnd2");

    // Primary
    circ.connect("wg-outnod", "tr-p1");
    circ.connect("tr-p2", "gnd1-Gnd");

    // Center tap grounded as reference return
    circ.connect("tr-ct", "gnd2-Gnd");

    // Diode 1 from S1, Diode 2 from S2 (reversed phase)
    circ.connect("tr-s1", "d1-lPin");
    circ.connect("tr-s2", "d2-lPin");

    // Common cathode DC bus
    circ.connect("d1-rPin", "r_load-lPin");
    circ.connect("d2-rPin", "r_load-lPin");
    circ.connect("c_filter-lPin", "r_load-lPin");

    // Load ground return to center tap
    circ.connect("r_load-rPin", "gnd2-Gnd");
    circ.connect("c_filter-rPin", "gnd2-Gnd");

    for _ in 0..5000 {
        circ.step().unwrap();
    }

    let v_dc = circ.pin_voltage("r_load-lPin").unwrap();
    // Peak voltage each half is ~12V, minus diode drop (~0.7V) -> ~11.3V peak, filtered DC ~9.6V
    assert!(
        v_dc > 9.0 && v_dc < 12.5,
        "Center-tapped full-wave rectifier must produce filtered DC ~9.6-11.3V, got {v_dc:.3} V"
    );
}

#[test]
fn test_auxiliary_secondary_coupling_and_turns_ratio() {
    // 3-winding transformer:
    // Primary: L1 = 1 H
    // Secondary: L2 = 4 H (2:1 step-up)
    // Auxiliary Secondary: L3 = 9 H (3:1 step-up)
    let mut circ = Circuit::default();
    circ.dt = 1e-6;
    circ.add_wave_gen("wg", "Sine", 1000.0, 2.0, 0.0, 0.5, 0.0, 100, false, false);
    circ.add_comp(Comp {
        id: "tr_3w".into(),
        kind: Kind::Transformer {
            state: TransformerState::new_multi_winding(1.0, 4.0, 9.0, 0.999, 0.01, 0.01, 0.01),
        },
    });
    circ.add_ground("gnd_p");
    circ.add_ground("gnd_s");
    circ.add_ground("gnd_a");
    circ.add_resistor("r_sec", 10_000.0);
    circ.add_resistor("r_aux", 10_000.0);

    circ.connect("wg-outnod", "tr_3w-p1");
    circ.connect("tr_3w-p2", "gnd_p-Gnd");

    circ.connect("tr_3w-s1", "r_sec-lPin");
    circ.connect("tr_3w-s2", "r_sec-rPin");
    circ.connect("tr_3w-s2", "gnd_s-Gnd");

    circ.connect("tr_3w-a1", "r_aux-lPin");
    circ.connect("tr_3w-a2", "r_aux-rPin");
    circ.connect("tr_3w-a2", "gnd_a-Gnd");

    let mut max_vp = 0.0_f64;
    let mut max_vs = 0.0_f64;
    let mut max_va = 0.0_f64;

    for _ in 0..2000 {
        circ.step().unwrap();
        let vp = circ.pin_voltage("tr_3w-p1").unwrap();
        let vs = circ.pin_voltage("tr_3w-s1").unwrap();
        let va = circ.pin_voltage("tr_3w-a1").unwrap();
        max_vp = max_vp.max(vp);
        max_vs = max_vs.max(vs);
        max_va = max_va.max(va);
    }

    assert!(
        (max_vp - 2.0).abs() < 0.05,
        "Primary peak should be ~2.0V, got {max_vp:.3}"
    );
    assert!(
        (max_vs - 4.0).abs() < 0.15,
        "Secondary 1:2 turns ratio peak should be ~4.0V, got {max_vs:.3}"
    );
    assert!(
        (max_va - 6.0).abs() < 0.25,
        "Auxiliary 1:3 turns ratio peak should be ~6.0V, got {max_va:.3}"
    );
}

#[test]
fn test_transformer_props_and_sim1_roundtrip() {
    let mut tr = Transformer::default();
    assert_eq!(tr.type_id(), "Transformer");
    assert_eq!(tr.r_core, 1e12);
    assert_eq!(tr.i_sat, 0.0);
    assert_eq!(tr.hysteresis, 0.0);
    assert!(!tr.center_tap);
    assert!(!tr.aux_secondary);

    // Verify properties list
    let prop_names: Vec<&str> = Transformer::props().iter().map(|p| p.id).collect();
    assert!(prop_names.contains(&"Inductance1"));
    assert!(prop_names.contains(&"Inductance2"));
    assert!(prop_names.contains(&"Coupling"));
    assert!(prop_names.contains(&"Rcoil1"));
    assert!(prop_names.contains(&"Rcoil2"));
    assert!(prop_names.contains(&"Rcore"));
    assert!(prop_names.contains(&"Isat"));
    assert!(prop_names.contains(&"Hysteresis"));
    assert!(prop_names.contains(&"CenterTap"));
    assert!(prop_names.contains(&"AuxSecondary"));
    assert!(prop_names.contains(&"Inductance3"));
    assert!(prop_names.contains(&"Rcoil3"));

    // Verify pin count adjustments
    assert_eq!(tr.pin_geoms().len(), 4);
    tr.center_tap = true;
    assert_eq!(tr.pin_geoms().len(), 5);
    tr.aux_secondary = true;
    assert_eq!(tr.pin_geoms().len(), 7);

    // XML parsing roundtrip
    let xml = r#"<circuit version="1.0.0" width="800" height="600">
<item itemtype="Transformer" CircId="TR-1" Inductance1="2.5 H" Inductance2="10 H" Coupling="0.98" Rcoil1="0.5 Ω" Rcoil2="1.2 Ω" Rcore="50 kΩ" Isat="1.5 A" Hysteresis="200 mA" CenterTap="true" AuxSecondary="true" Inductance3="5 H" Rcoil3="0.8 Ω" Pos="100,100" />
</circuit>"#;
    let circ = Circuit::from_sim1(xml).unwrap();
    let comp = circ.components().iter().find(|c| c.id == "TR-1").unwrap();
    if let Kind::Transformer { state } = &comp.kind {
        assert_eq!(state.inductance1, 2.5);
        assert_eq!(state.inductance2, 10.0);
        assert_eq!(state.coupling, 0.98);
        assert_eq!(state.r_coil1, 0.5);
        assert_eq!(state.r_coil2, 1.2);
        assert_eq!(state.r_core, 50_000.0);
        assert_eq!(state.i_sat, 1.5);
        assert_eq!(state.hysteresis, 0.2);
        assert!(state.center_tap);
        assert!(state.aux_secondary);
        assert_eq!(state.inductance3, 5.0);
        assert_eq!(state.r_coil3, 0.8);
    } else {
        panic!("Expected Kind::Transformer");
    }
}
