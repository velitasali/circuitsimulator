//! Comprehensive test suite for Power Semiconductor Physics (SCR, TRIAC, DIAC).
//!
//! Validates:
//! - Multi-quadrant TRIAC trigger sensitivity (Quadrants I through IV).
//! - Dynamic holding ($I_H$) and latching ($I_L$) current dynamics.
//! - Critical rate-of-rise voltage triggering ($dv/dt$).
//! - Commutation turn-off recovery delays ($t_q$).
//! - DIAC symmetric breakover, dynamic switching, and $dv/dt$ sensitivity.

use cs_engine::Circuit;

fn approx(actual: f64, expected: f64, tol: f64, context: &str) {
    let diff = (actual - expected).abs();
    assert!(
        diff <= tol,
        "{context}: actual={actual:.4}, expected={expected:.4}, diff={diff:.4} > tol={tol}"
    );
}

// =========================================================================
// 1. TRIAC MULTI-QUADRANT TRIGGER SENSITIVITY
// =========================================================================

#[test]
fn test_triac_multi_quadrant_triggering() {
    // Quadrants relative to MT1 (reference at GND):
    // Q1 (I+):   MT2 > 0, Gate > 0 (sensitive, mult = 1.0)
    // Q2 (I-):   MT2 > 0, Gate < 0 (intermediate, mult = 1.4)
    // Q3 (III-): MT2 < 0, Gate < 0 (sensitive, mult = 1.0)
    // Q4 (III+): MT2 < 0, Gate > 0 (least sensitive, mult = 2.0)

    // Helper to build a circuit with given V_MT2 and V_Gate relative to MT1 (at GND)
    let run_triac = |v_mt2: f64, v_gate: f64| -> f64 {
        let mut c = Circuit::new();
        c.add_fixed_volt("VSUP", v_mt2)
            .add_fixed_volt("VGATE", v_gate)
            .add_triac("TR1")
            .add_resistor("RLOAD", 50.0) // 50 ohm load
            .add_ground("GND");

        // MT1 to GND, MT2 to RLOAD to VSUP, Gate to VGATE
        c.connect("VSUP-outnod", "RLOAD-lPin")
            .connect("RLOAD-rPin", "TR1-rPin") // MT2
            .connect("TR1-lPin", "GND-Gnd") // MT1
            .connect("VGATE-outnod", "TR1-gPin");

        c.solve().unwrap();
        // Return MT2 voltage: if blocking, near v_mt2; if conducting, near 0.0V
        c.pin_voltage("TR1-rPin").unwrap()
    };

    // 1. Quadrant I (I+): V_MT2 = +12V, V_Gate = +1.5V -> I_g = 15mA > 10mA -> TRIGGERS
    let v_q1 = run_triac(12.0, 1.5);
    assert!(v_q1.abs() < 0.2, "Q1 should trigger: Vmt2={v_q1:.3}");

    // 1b. Quadrant I with weak gate: V_Gate = +0.5V < 0.7V -> BLOCKS
    let v_q1_weak = run_triac(12.0, 0.5);
    approx(v_q1_weak, 12.0, 0.2, "Q1 weak gate should block");

    // 2. Quadrant II (I-): V_MT2 = +12V, V_Gate = -2.0V -> I_g = -20mA > 14mA (1.4x) -> TRIGGERS
    let v_q2 = run_triac(12.0, -2.0);
    assert!(v_q2.abs() < 0.2, "Q2 should trigger: Vmt2={v_q2:.3}");

    // 2b. Quadrant II with sub-threshold current: V_Gate = -1.1V -> I_g = -11mA < 14mA -> BLOCKS
    let v_q2_sub = run_triac(12.0, -1.1);
    approx(v_q2_sub, 12.0, 0.2, "Q2 insufficient current should block");

    // 3. Quadrant III (III-): V_MT2 = -12V, V_Gate = -1.5V -> I_g = -15mA > 10mA -> TRIGGERS
    let v_q3 = run_triac(-12.0, -1.5);
    assert!(v_q3.abs() < 0.2, "Q3 should trigger: Vmt2={v_q3:.3}");

    // 3b. Quadrant III with sub-threshold voltage: V_Gate = -0.4V -> BLOCKS
    let v_q3_weak = run_triac(-12.0, -0.4);
    approx(v_q3_weak, -12.0, 0.2, "Q3 weak gate should block");

    // 4. Quadrant IV (III+): V_MT2 = -12V, V_Gate = +2.5V -> I_g = +25mA > 20mA (2.0x) -> TRIGGERS
    let v_q4 = run_triac(-12.0, 2.5);
    assert!(
        v_q4.abs() < 0.2,
        "Q4 should trigger with strong drive: Vmt2={v_q4:.3}"
    );

    // 4b. Quadrant IV with moderate drive: V_Gate = +1.5V -> I_g = +15mA < 20mA -> BLOCKS
    let v_q4_mid = run_triac(-12.0, 1.5);
    approx(
        v_q4_mid,
        -12.0,
        0.2,
        "Q4 moderate drive (15mA) should block due to lower sensitivity",
    );
}

// =========================================================================
// 2. SCR DYNAMIC LATCHING (I_L) AND HOLDING (I_H) CURRENT DYNAMICS
// =========================================================================

#[test]
fn test_scr_latching_and_holding_dynamics() {
    // Standard SCR: I_hold = 8.2 mA, I_latch = 15 mA (1.8x)
    // If gate pulse triggers SCR:
    // - Case A: Low load resistance (e.g. 100 Ω, I_load ≈ 113 mA > 15 mA)
    //   Anode current exceeds I_latch -> SCR latches ON even when gate is removed.
    // - Case B: High load resistance (e.g. 1500 Ω, I_load ≈ 7.5 mA < 8.2 mA)
    //   Anode current cannot maintain holding current -> immediately turns off.
    // - Case C: Intermediate load (e.g. 1000 Ω, I_load ≈ 11.3 mA, between I_hold and I_latch)
    //   Conducts while gate is driven, but unlatches when gate is removed because I_load < I_latch!

    let v_sup = 12.0;

    // Case A: R_LOAD = 100 Ω (I_load ≈ 113 mA)
    let mut c_latched = Circuit::new();
    c_latched
        .add_fixed_volt("VSUP", v_sup)
        .add_fixed_volt("VGATE", 2.0)
        .add_scr("SCR1")
        .add_resistor("RLOAD", 100.0)
        .add_ground("GND");

    c_latched
        .connect("VSUP-outnod", "RLOAD-lPin")
        .connect("RLOAD-rPin", "SCR1-lPin") // Anode
        .connect("SCR1-rPin", "GND-Gnd") // Cathode
        .connect("VGATE-outnod", "SCR1-gPin");

    c_latched.solve().unwrap();
    let va1 = c_latched.pin_voltage("SCR1-lPin").unwrap();
    approx(va1, 0.7, 0.15, "SCR on with gate active and R=100");

    // Now remove gate pulse (VGATE = 0V) by in-place resolve:
    // SCR should STAY on because I_A ≈ 113 mA > I_latch (15 mA)!
    for c in c_latched.components_mut() {
        if c.id == "VGATE" {
            if let cs_engine::elements::Kind::FixedVolt { ref mut voltage } = c.kind {
                *voltage = 0.0;
            }
        }
    }
    c_latched.re_solve().unwrap();
    let va_latched = c_latched.pin_voltage("SCR1-lPin").unwrap();
    approx(
        va_latched,
        0.7,
        0.15,
        "SCR remains latched when gate drops and I_A > I_L",
    );

    // Now increase load resistance to 2000 Ω (I_A = (12 - 0.7)/2000 ≈ 5.6 mA < I_hold 8.2 mA)
    // SCR must extinguish and anode voltage must rise to VSUP (12V)!
    for c in c_latched.components_mut() {
        if c.id == "RLOAD" {
            if let cs_engine::elements::Kind::Resistor { ref mut resistance } = c.kind {
                *resistance = 2000.0;
            }
        }
    }
    c_latched.re_solve().unwrap();
    let va_extinguished = c_latched.pin_voltage("SCR1-lPin").unwrap();
    approx(
        va_extinguished,
        12.0,
        0.2,
        "SCR extinguishes when I_A drops below I_hold",
    );
}

#[test]
fn test_scr_unlatched_when_current_below_latching() {
    // Intermediate load (e.g. 1000 Ω):
    // I_A = (12 - 0.7) / 1000 = 11.3 mA.
    // Notice: 8.2 mA (I_H) < 11.3 mA < 15.0 mA (I_L)!
    // With gate active (2.0V): SCR conducts.
    // When gate drops to 0.0V: Because I_A never reached I_L, SCR MUST turn OFF!

    let mut c = Circuit::new();
    c.add_fixed_volt("VSUP", 12.0)
        .add_fixed_volt("VGATE", 2.0)
        .add_scr("SCR1")
        .add_resistor("RLOAD", 1000.0)
        .add_ground("GND");

    c.connect("VSUP-outnod", "RLOAD-lPin")
        .connect("RLOAD-rPin", "SCR1-lPin")
        .connect("SCR1-rPin", "GND-Gnd")
        .connect("VGATE-outnod", "SCR1-gPin");

    c.solve().unwrap();
    let va_on = c.pin_voltage("SCR1-lPin").unwrap();
    approx(va_on, 0.7, 0.15, "SCR conducts while gate is present");

    // Remove gate pulse:
    for comp in c.components_mut() {
        if comp.id == "VGATE" {
            if let cs_engine::elements::Kind::FixedVolt { ref mut voltage } = comp.kind {
                *voltage = 0.0;
            }
        }
    }
    c.re_solve().unwrap();
    let va_off = c.pin_voltage("SCR1-lPin").unwrap();
    approx(
        va_off,
        12.0,
        0.2,
        "SCR unlatches and turns off because I_A < I_L",
    );
}

// =========================================================================
// 3. CRITICAL RATE-OF-RISE VOLTAGE TRIGGERING (dV/dt)
// =========================================================================

#[test]
fn test_scr_critical_dv_dt_triggering() {
    // Critical dv/dt is set to 50 V/µs = 50e6 V/s.
    // If anode voltage experiences a fast step edge (dV/dt > 50e6 V/s),
    // displacement current triggers the SCR without any gate signal!

    let mut c = Circuit::new();
    c.dt = 1e-7; // 100 ns step
    c.add_fixed_volt("VSUP", 0.0) // starts at 0V
        .add_fixed_volt("VGATE", 0.0) // gate is tied to 0V
        .add_scr("SCR1")
        .add_resistor("RLOAD", 50.0)
        .add_ground("GND");

    c.connect("VSUP-outnod", "RLOAD-lPin")
        .connect("RLOAD-rPin", "SCR1-lPin")
        .connect("SCR1-rPin", "GND-Gnd")
        .connect("VGATE-outnod", "SCR1-gPin");

    c.solve().unwrap();
    approx(
        c.pin_voltage("SCR1-lPin").unwrap(),
        0.0,
        0.05,
        "Initial state at 0V",
    );

    // Advance 1 step
    c.step().unwrap();

    // Now apply a fast voltage jump to 20V in 100 ns -> dV/dt = 20V / 1e-7s = 200 MV/s > 50 MV/s!
    for comp in c.components_mut() {
        if comp.id == "VSUP" {
            if let cs_engine::elements::Kind::FixedVolt { ref mut voltage } = comp.kind {
                *voltage = 20.0;
            }
        }
    }

    c.step().unwrap();
    let va = c.pin_voltage("SCR1-lPin").unwrap();
    approx(
        va,
        0.7,
        0.15,
        "SCR triggers via dV/dt without gate drive: Va ~ 0.7V",
    );
}

// =========================================================================
// 4. COMMUTATION TURN-OFF DELAY (t_q)
// =========================================================================

#[test]
fn test_scr_commutation_turn_off_time() {
    // Commutation recovery time t_q = 30 µs.
    // If current drops below holding current (turns off), but forward voltage
    // is re-applied before 30 µs have elapsed, residual minority carriers
    // re-trigger the device!
    // If forward voltage is re-applied after > 30 µs, it successfully blocks.

    let dt = 5e-6; // 5 µs per step
    let mut c = Circuit::new();
    c.dt = dt;
    c.add_fixed_volt("VSUP", 12.0)
        .add_fixed_volt("VGATE", 2.0)
        .add_scr("SCR1")
        .add_resistor("RLOAD", 100.0)
        .add_ground("GND");

    c.connect("VSUP-outnod", "RLOAD-lPin")
        .connect("RLOAD-rPin", "SCR1-lPin")
        .connect("SCR1-rPin", "GND-Gnd")
        .connect("VGATE-outnod", "SCR1-gPin");

    c.solve().unwrap();
    // Step forward a few steps while conducting
    for _ in 0..5 {
        c.step().unwrap();
    }
    approx(
        c.pin_voltage("SCR1-lPin").unwrap(),
        0.7,
        0.15,
        "SCR conducting",
    );

    // Remove gate drive
    for comp in c.components_mut() {
        if comp.id == "VGATE" {
            if let cs_engine::elements::Kind::FixedVolt { ref mut voltage } = comp.kind {
                *voltage = 0.0;
            }
        }
    }
    c.step().unwrap();

    // Now extinguish SCR by setting VSUP = 0V for 10 µs (< t_q = 30 µs)
    for comp in c.components_mut() {
        if comp.id == "VSUP" {
            if let cs_engine::elements::Kind::FixedVolt { ref mut voltage } = comp.kind {
                *voltage = 0.0;
            }
        }
    }
    c.step().unwrap(); // 5 µs
    c.step().unwrap(); // 10 µs extinguished (total elapsed < 30 µs)

    // Re-apply forward voltage of 12V before t_q (elapsed = 10 µs < 30 µs)
    for comp in c.components_mut() {
        if comp.id == "VSUP" {
            if let cs_engine::elements::Kind::FixedVolt { ref mut voltage } = comp.kind {
                *voltage = 12.0;
            }
        }
    }
    c.step().unwrap();
    let va_retriggered = c.pin_voltage("SCR1-lPin").unwrap();
    approx(
        va_retriggered,
        0.7,
        0.15,
        "Re-triggers if forward voltage reapplied before t_q",
    );

    // Now extinguish and wait for 8 steps = 40 µs (> t_q = 30 µs)
    for comp in c.components_mut() {
        if comp.id == "VSUP" {
            if let cs_engine::elements::Kind::FixedVolt { ref mut voltage } = comp.kind {
                *voltage = 0.0;
            }
        }
    }
    for _ in 0..8 {
        c.step().unwrap();
    }

    // Now slowly restore forward voltage (rate below critical dv/dt)
    for comp in c.components_mut() {
        if comp.id == "VSUP" {
            if let cs_engine::elements::Kind::FixedVolt { ref mut voltage } = comp.kind {
                *voltage = 12.0;
            }
        }
    }
    // Update previous voltage so dv/dt is not triggered on this step
    for comp in c.components_mut() {
        if let cs_engine::elements::Kind::Scr { ref mut state } = comp.kind {
            state.v_ak_prev = 12.0;
        }
    }
    c.step().unwrap();
    let va_blocked = c.pin_voltage("SCR1-lPin").unwrap();
    approx(
        va_blocked,
        12.0,
        0.2,
        "Blocks forward voltage if reapplied after t_q has elapsed",
    );
}

// =========================================================================
// 5. DIAC CRITICAL dV/dt AND SYMMETRIC BREAKOVER
// =========================================================================

#[test]
fn test_diac_dv_dt_triggering() {
    // DIAC breakover voltage is 30V, critical dv/dt is 20 V/µs = 20e6 V/s.
    // If voltage across DIAC is only 18V (< 30V), but rises at 180 MV/s > 20 MV/s,
    // the DIAC triggers via dv/dt!

    let mut c = Circuit::new();
    c.dt = 1e-7; // 100 ns
    c.add_fixed_volt("VIN", 0.0)
        .add_diac("D1")
        .add_resistor("RLOAD", 1000.0)
        .add_ground("GND");

    c.connect("VIN-outnod", "D1-lPin")
        .connect("D1-rPin", "RLOAD-lPin")
        .connect("RLOAD-rPin", "GND-Gnd");

    c.solve().unwrap();
    c.step().unwrap();

    // Fast step from 0V to 18V in 100 ns (180 MV/s > 20 MV/s)
    for comp in c.components_mut() {
        if comp.id == "VIN" {
            if let cs_engine::elements::Kind::FixedVolt { ref mut voltage } = comp.kind {
                *voltage = 18.0;
            }
        }
    }
    c.step().unwrap();

    let vo = c.pin_voltage("RLOAD-lPin").unwrap();
    assert!(
        vo > 10.0,
        "DIAC triggers on fast dV/dt edge: Vo={vo:.2} > 10V"
    );
}

#[test]
fn test_diac_symmetric_breakover_both_polarities() {
    // Forward polarity (positive): +35V > +30V -> breaks over and conducts
    let mut c_pos = Circuit::new();
    c_pos
        .add_fixed_volt("VIN", 35.0)
        .add_diac("D1")
        .add_resistor("RLOAD", 500.0)
        .add_ground("GND");

    c_pos
        .connect("VIN-outnod", "D1-lPin")
        .connect("D1-rPin", "RLOAD-lPin")
        .connect("RLOAD-rPin", "GND-Gnd");

    c_pos.solve().unwrap();
    let vo_pos = c_pos.pin_voltage("RLOAD-lPin").unwrap();
    approx(
        vo_pos,
        17.5,
        0.5,
        "DIAC conducts positive half cycle (Vout = 35 * 500/1000)",
    );

    // Reverse polarity (negative): -35V < -30V -> breaks over and conducts negatively
    let mut c_neg = Circuit::new();
    c_neg
        .add_fixed_volt("VIN", -35.0)
        .add_diac("D1")
        .add_resistor("RLOAD", 500.0)
        .add_ground("GND");

    c_neg
        .connect("VIN-outnod", "D1-lPin")
        .connect("D1-rPin", "RLOAD-lPin")
        .connect("RLOAD-rPin", "GND-Gnd");

    c_neg.solve().unwrap();
    let vo_neg = c_neg.pin_voltage("RLOAD-lPin").unwrap();
    approx(
        vo_neg,
        -17.5,
        0.5,
        "DIAC conducts negative half cycle (Vout = -35 * 500/1000)",
    );
}

#[test]
fn test_triac_latching_and_holding_dynamics() {
    // TRIAC: I_hold = 8.2 mA, I_latch = 15 mA
    // 1. Trigger in Q1 with load drawing ~120 mA > 15 mA (I_latch)
    let mut c = Circuit::new();
    c.add_fixed_volt("VSUP", 12.0)
        .add_fixed_volt("VGATE", 2.0)
        .add_triac("TR1")
        .add_resistor("RLOAD", 100.0)
        .add_ground("GND");

    c.connect("VSUP-outnod", "RLOAD-lPin")
        .connect("RLOAD-rPin", "TR1-rPin") // MT2
        .connect("TR1-lPin", "GND-Gnd") // MT1
        .connect("VGATE-outnod", "TR1-gPin");

    c.solve().unwrap();
    let v_mt2_on = c.pin_voltage("TR1-rPin").unwrap();
    assert!(v_mt2_on.abs() < 0.2, "TRIAC conducting while gate active");

    // Remove gate drive -> stays latched
    for comp in c.components_mut() {
        if comp.id == "VGATE" {
            if let cs_engine::elements::Kind::FixedVolt { ref mut voltage } = comp.kind {
                *voltage = 0.0;
            }
        }
    }
    c.re_solve().unwrap();
    let v_mt2_latched = c.pin_voltage("TR1-rPin").unwrap();
    assert!(
        v_mt2_latched.abs() < 0.2,
        "TRIAC remains latched when gate removed"
    );

    // Increase load resistance to 2500 Ω -> I_main ≈ 4.8 mA < 8.2 mA (I_hold)
    // TRIAC must extinguish
    for comp in c.components_mut() {
        if comp.id == "RLOAD" {
            if let cs_engine::elements::Kind::Resistor { ref mut resistance } = comp.kind {
                *resistance = 2500.0;
            }
        }
    }
    c.re_solve().unwrap();
    let v_mt2_extinguished = c.pin_voltage("TR1-rPin").unwrap();
    approx(
        v_mt2_extinguished,
        12.0,
        0.2,
        "TRIAC extinguishes below holding current",
    );
}

#[test]
fn test_triac_critical_dv_dt_triggering() {
    // Critical dv/dt = 50 MV/s.
    // Fast step of 20V across MT2-MT1 in 100 ns -> rate = 200 MV/s > 50 MV/s triggers conduction.
    let mut c = Circuit::new();
    c.dt = 1e-7;
    c.add_fixed_volt("VSUP", 0.0)
        .add_fixed_volt("VGATE", 0.0)
        .add_triac("TR1")
        .add_resistor("RLOAD", 50.0)
        .add_ground("GND");

    c.connect("VSUP-outnod", "RLOAD-lPin")
        .connect("RLOAD-rPin", "TR1-rPin") // MT2
        .connect("TR1-lPin", "GND-Gnd") // MT1
        .connect("VGATE-outnod", "TR1-gPin");

    c.solve().unwrap();
    c.step().unwrap();

    // Fast jump to 20V
    for comp in c.components_mut() {
        if comp.id == "VSUP" {
            if let cs_engine::elements::Kind::FixedVolt { ref mut voltage } = comp.kind {
                *voltage = 20.0;
            }
        }
    }
    c.step().unwrap();
    let v_mt2 = c.pin_voltage("TR1-rPin").unwrap();
    assert!(
        v_mt2.abs() < 0.2,
        "TRIAC triggers on critical dV/dt edge: Vmt2={v_mt2:.3}"
    );
}

#[test]
fn test_triac_commutation_turn_off_time() {
    let dt = 5e-6; // 5 µs per step
    let mut c = Circuit::new();
    c.dt = dt;
    c.add_fixed_volt("VSUP", 12.0)
        .add_fixed_volt("VGATE", 2.0)
        .add_triac("TR1")
        .add_resistor("RLOAD", 100.0)
        .add_ground("GND");

    c.connect("VSUP-outnod", "RLOAD-lPin")
        .connect("RLOAD-rPin", "TR1-rPin")
        .connect("TR1-lPin", "GND-Gnd")
        .connect("VGATE-outnod", "TR1-gPin");

    c.solve().unwrap();
    for _ in 0..5 {
        c.step().unwrap();
    }
    assert!(
        c.pin_voltage("TR1-rPin").unwrap().abs() < 0.2,
        "TRIAC conducting"
    );

    // Remove gate drive
    for comp in c.components_mut() {
        if comp.id == "VGATE" {
            if let cs_engine::elements::Kind::FixedVolt { ref mut voltage } = comp.kind {
                *voltage = 0.0;
            }
        }
    }
    c.step().unwrap();

    // Extinguish by zeroing supply for 10 µs (< t_q = 30 µs)
    for comp in c.components_mut() {
        if comp.id == "VSUP" {
            if let cs_engine::elements::Kind::FixedVolt { ref mut voltage } = comp.kind {
                *voltage = 0.0;
            }
        }
    }
    c.step().unwrap();
    c.step().unwrap();

    // Re-apply voltage before t_q (10 µs < 30 µs) -> re-triggers!
    for comp in c.components_mut() {
        if comp.id == "VSUP" {
            if let cs_engine::elements::Kind::FixedVolt { ref mut voltage } = comp.kind {
                *voltage = 12.0;
            }
        }
    }
    c.step().unwrap();
    assert!(
        c.pin_voltage("TR1-rPin").unwrap().abs() < 0.2,
        "TRIAC re-triggers before t_q has elapsed"
    );

    // Extinguish and wait for 40 µs (> t_q = 30 µs)
    for comp in c.components_mut() {
        if comp.id == "VSUP" {
            if let cs_engine::elements::Kind::FixedVolt { ref mut voltage } = comp.kind {
                *voltage = 0.0;
            }
        }
    }
    for _ in 0..8 {
        c.step().unwrap();
    }

    // Slowly restore voltage
    for comp in c.components_mut() {
        if comp.id == "VSUP" {
            if let cs_engine::elements::Kind::FixedVolt { ref mut voltage } = comp.kind {
                *voltage = 12.0;
            }
        }
    }
    for comp in c.components_mut() {
        if let cs_engine::elements::Kind::Triac { ref mut state } = comp.kind {
            state.v_mt_prev = 12.0;
        }
    }
    c.step().unwrap();
    let v_blocked = c.pin_voltage("TR1-rPin").unwrap();
    approx(v_blocked, 12.0, 0.2, "TRIAC blocks after t_q has elapsed");
}
