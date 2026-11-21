//! Integration and physics verification tests for electromechanical components:
//! - Stepper motor: rotor inertia, viscous damping, holding torque, and step-slip under load.
//! - Servo motor: dynamic power consumption (idle, run, stall currents), mechanical torque stall, and UVLO.
//! - Relay: contact bounce intervals, switching arcing, and contact dielectric breakdown.
//! - Component properties, getters/setters, units, and validation.

use cs_engine::Circuit;
use cs_engine::components::{Component, Relay, Servo, Stepper};
use cs_engine::elements::{Comp, Kind, RelayState, ServoState, StepperState};

#[test]
fn test_stepper_inertial_settling_and_damping() {
    let mut c = Circuit::new();
    // 32-step motor (11.25 deg/step), rated coil resistance 100 ohms.
    c.add_fixed_volt("V", 5.0)
        .add_comp(Comp {
            id: "M".into(),
            kind: Kind::Stepper {
                bipolar: false,
                steps: 32,
                resistance: 100.0,
                angle: 0.0,
                state: StepperState::new(0.4, 6e-6, 0.004, 0.0),
            },
        })
        .add_ground("GND")
        .connect("M-PinA1", "GND-Gnd")
        .connect("M-PinA2", "GND-Gnd")
        .connect("M-PinB2", "GND-Gnd")
        .connect("M-PinCo", "GND-Gnd")
        .connect("V-outnod", "M-PinB1");

    c.solve().unwrap();

    let get_stepper = |circ: &Circuit| match circ.comp("M").unwrap().kind {
        Kind::Stepper { angle, state, .. } => (angle, state),
        _ => panic!("expected Stepper"),
    };

    let (angle0, state0) = get_stepper(&c);
    assert!(angle0.abs() < 1e-6, "rotor starts at 0 deg");
    assert!(!state0.is_stalled);
    assert_eq!(state0.step_slip, 0);

    // After 1 ms: rotor has inertia J, so acceleration is gradual
    c.run_ps(1_000_000_000).unwrap();
    let (mid_angle, _) = get_stepper(&c);
    assert!(
        mid_angle > 1.0 && mid_angle < 5.0,
        "rotor should partially advance at 1 ms under inertia: got {mid_angle}"
    );

    // After 50 ms: viscous friction B damps out ringing and settles to target pole step (11.25°)
    c.run_ps(50_000_000_000).unwrap();
    let (settled_angle, settled_state) = get_stepper(&c);
    assert!(
        (10.0..12.5).contains(&settled_angle),
        "rotor should settle cleanly near 11.25°: got {settled_angle}"
    );
    assert!(!settled_state.is_stalled);
    assert_eq!(settled_state.step_slip, 0);
}

#[test]
fn test_stepper_step_slip_under_excessive_load() {
    let mut c = Circuit::new();
    // Holding torque Th = 0.4 N·m, but load torque TL = 0.8 N·m (opposing rotation)
    let overloaded_state = StepperState::new(0.4, 6e-6, 0.004, 0.8);
    c.add_fixed_volt("V", 5.0)
        .add_comp(Comp {
            id: "M".into(),
            kind: Kind::Stepper {
                bipolar: false,
                steps: 32,
                resistance: 100.0,
                angle: 0.0,
                state: overloaded_state,
            },
        })
        .add_ground("GND")
        .connect("M-PinA1", "GND-Gnd")
        .connect("M-PinA2", "GND-Gnd")
        .connect("M-PinB2", "GND-Gnd")
        .connect("M-PinCo", "GND-Gnd")
        .connect("V-outnod", "M-PinB1");

    c.solve().unwrap();

    let get_stepper = |circ: &Circuit| match circ.comp("M").unwrap().kind {
        Kind::Stepper { angle, state, .. } => (angle, state),
        _ => panic!("expected Stepper"),
    };

    // Advance time: electromagnetic torque cannot overcome 0.8 N·m load torque
    c.run_ps(50_000_000_000).unwrap();
    let (angle, state) = get_stepper(&c);

    // Rotor remains stuck at or very close to rest
    assert!(
        angle < 1.0,
        "rotor under excessive load must not advance: got {angle}"
    );
    assert!(state.is_stalled, "stepper must report stalled condition");
}

#[test]
fn test_servo_idle_vs_running_current_consumption() {
    let mut c = Circuit::new();
    // Servo with idle = 15 mA, run = 250 mA, stall = 1.0 A
    let servo_state = ServoState::new(0.015, 0.25, 1.0, 0.0, 3.5);
    c.add_fixed_volt("VCC", 5.0)
        .add_ground("GND")
        .add_fixed_volt("SIG", 0.0)
        .add_comp(Comp {
            id: "SRV".into(),
            kind: Kind::Servo {
                speed: 0.2, // 0.2 s / 60 deg -> 0.6 s for 180 deg
                min_pulse: 1000.0,
                max_pulse: 2000.0,
                pos: 90.0,
                target_pos: 90.0,
                pulse_start_ps: 0,
                sig_high: false,
                state: servo_state,
            },
        })
        .connect("VCC-outnod", "SRV-PinV+")
        .connect("GND-Gnd", "SRV-PinGnd")
        .connect("SIG-outnod", "SRV-PinSig");

    c.solve().unwrap();

    // In steady-state holding position (pos == target_pos == 90°), current draw is idle (~15 mA)
    let i_vcc_idle = c.current_out_of_pin("VCC-outnod").unwrap().abs();
    assert!(
        (i_vcc_idle - 0.015).abs() < 2e-3,
        "quiescent servo current draw should match idle (15 mA), got {i_vcc_idle} A"
    );

    // Command rotation to 0 degrees by sending a 1000 µs pulse on SIG
    c.update_component_kind("SIG", &Kind::FixedVolt { voltage: 5.0 });
    c.re_solve().unwrap();
    c.run_ps(1_000_000_000).unwrap(); // 1 ms pulse width = 1000 µs
    c.update_component_kind("SIG", &Kind::FixedVolt { voltage: 0.0 });
    c.re_solve().unwrap();

    // Advance 50 ms while the servo is in active motion towards 0°
    c.run_ps(50_000_000_000).unwrap();
    c.re_solve().unwrap();

    // Check that target_pos is updated to 0° and horn is moving
    let (pos_moving, target) = match c.comp("SRV").unwrap().kind {
        Kind::Servo {
            pos, target_pos, ..
        } => (pos, target_pos),
        _ => panic!("expected Servo"),
    };
    assert!((target - 0.0).abs() < 1e-3, "target should be 0 deg");
    assert!(
        pos_moving < 85.0 && pos_moving > 10.0,
        "horn is in transit from 90° towards 0°: got {pos_moving}"
    );

    // During active transit, current drawn from VCC must equal running current (250 mA)
    let i_vcc_run = c.current_out_of_pin("VCC-outnod").unwrap().abs();
    assert!(
        (i_vcc_run - 0.25).abs() < 0.05,
        "active moving servo current draw should match run_current (250 mA), got {i_vcc_run} A"
    );

    // Allow horn enough time to complete travel to 0° (0.6 s = 600 ms = 600_000_000_000 ps)
    c.run_ps(600_000_000_000).unwrap();
    let (pos_settled, _) = match c.comp("SRV").unwrap().kind {
        Kind::Servo {
            pos, target_pos, ..
        } => (pos, target_pos),
        _ => panic!("expected Servo"),
    };
    assert!(
        (pos_settled - 0.0).abs() < 1.0,
        "horn must have reached 0 deg target: got {pos_settled}"
    );

    // Once settled at the target position, current draw drops back to idle current
    c.re_solve().unwrap();
    let i_vcc_settled = c.current_out_of_pin("VCC-outnod").unwrap().abs();
    assert!(
        (i_vcc_settled - 0.015).abs() < 2e-3,
        "settled servo current draw must return to idle (15 mA), got {i_vcc_settled} A"
    );
}

#[test]
fn test_servo_stall_under_heavy_load() {
    let mut c = Circuit::new();
    // Stall torque = 3.5 N·m; load torque = 4.0 N·m (overloaded)
    let servo_state = ServoState::new(0.015, 0.25, 1.0, 4.0, 3.5);
    c.add_fixed_volt("VCC", 5.0)
        .add_ground("GND")
        .add_fixed_volt("SIG", 0.0)
        .add_comp(Comp {
            id: "SRV".into(),
            kind: Kind::Servo {
                speed: 0.2,
                min_pulse: 1000.0,
                max_pulse: 2000.0,
                pos: 90.0,
                target_pos: 90.0,
                pulse_start_ps: 0,
                sig_high: false,
                state: servo_state,
            },
        })
        .connect("VCC-outnod", "SRV-PinV+")
        .connect("GND-Gnd", "SRV-PinGnd")
        .connect("SIG-outnod", "SRV-PinSig");

    c.solve().unwrap();

    // Command move to 0 deg
    c.update_component_kind("SIG", &Kind::FixedVolt { voltage: 5.0 });
    c.re_solve().unwrap();
    c.run_ps(1_000_000_000).unwrap();
    c.update_component_kind("SIG", &Kind::FixedVolt { voltage: 0.0 });
    c.re_solve().unwrap();

    // Check stalled state and stall current
    let state = match c.comp("SRV").unwrap().kind {
        Kind::Servo { state, pos, .. } => {
            assert!(
                (pos - 90.0).abs() < 1e-4,
                "stalled horn must not rotate away from 90°"
            );
            state
        }
        _ => panic!("expected Servo"),
    };
    assert!(state.is_stalled, "servo must be flagged as stalled");

    let i_stall = c.current_out_of_pin("VCC-outnod").unwrap().abs();
    assert!(
        (i_stall - 1.0).abs() < 0.1,
        "stalled servo should draw stall current (1.0 A), got {i_stall} A"
    );
}

#[test]
fn test_servo_undervoltage_lockout() {
    let mut c = Circuit::new();
    // Supply voltage = 2.0 V (below UVLO threshold 2.5 V)
    let servo_state = ServoState::new(0.015, 0.25, 1.0, 0.0, 3.5);
    c.add_fixed_volt("VCC", 2.0)
        .add_ground("GND")
        .add_fixed_volt("SIG", 0.0)
        .add_comp(Comp {
            id: "SRV".into(),
            kind: Kind::Servo {
                speed: 0.2,
                min_pulse: 1000.0,
                max_pulse: 2000.0,
                pos: 90.0,
                target_pos: 90.0,
                pulse_start_ps: 0,
                sig_high: false,
                state: servo_state,
            },
        })
        .connect("VCC-outnod", "SRV-PinV+")
        .connect("GND-Gnd", "SRV-PinGnd")
        .connect("SIG-outnod", "SRV-PinSig");

    c.solve().unwrap();

    // Under-voltage disables internal H-bridge and control electronics
    let i_vcc = c.current_out_of_pin("VCC-outnod").unwrap().abs();
    assert!(
        i_vcc < 1e-6,
        "servo below 2.5 V UVLO must shut down quiescent draw, got {i_vcc} A"
    );
}

#[test]
fn test_relay_contact_bounce_transient_intervals() {
    let mut c = Circuit::new();
    c.dt = 1e-4; // 100 µs step
    // Relay with 2 ms bounce time, contact resistance 0.05 ohm
    let relay_state = RelayState::new(0.002, 500.0, 0.05);
    c.add_fixed_volt("V_COIL", 0.0)
        .add_fixed_volt("V_LOAD", 12.0)
        .add_ground("GND")
        .add_resistor("R_LOAD", 10.0)
        .add_comp(Comp {
            id: "K".into(),
            kind: Kind::Relay {
                norm_close: false,
                double_throw: false,
                poles: 1,
                i_on: 0.02,
                i_off: 0.01,
                r_coil: 100.0,
                inductance: 0.0,
                coil_i: 0.0,
                active: false,
                state: relay_state,
            },
        })
        .connect("V_COIL-outnod", "K-lPin")
        .connect("GND-Gnd", "K-rPin")
        .connect("V_LOAD-outnod", "K-c1")
        .connect("K-c2", "R_LOAD-lPin")
        .connect("R_LOAD-rPin", "GND-Gnd");

    c.solve().unwrap();

    // Initial state: coil de-energized, contacts open -> load voltage is 0
    let v_load_init = c.pin_voltage("R_LOAD-lPin").unwrap();
    assert!(
        v_load_init.abs() < 1e-3,
        "initial contact open, load at 0 V"
    );

    // Energize coil with 12V (Coil current = 120 mA > I_on = 20 mA)
    c.update_component_kind("V_COIL", &Kind::FixedVolt { voltage: 12.0 });
    c.re_solve().unwrap();

    // During the 2 ms bounce period, monitor contact conduction
    let mut seen_closed_during_bounce = false;
    let mut seen_open_during_bounce = false;
    for _ in 0..20 {
        // 20 steps of 100 µs = 2.0 ms
        c.run_ps(100_000_000).unwrap();
        let v_load = c.pin_voltage("R_LOAD-lPin").unwrap();
        if v_load > 10.0 {
            seen_closed_during_bounce = true;
        } else if v_load < 2.0 {
            seen_open_during_bounce = true;
        }
    }

    assert!(
        seen_closed_during_bounce && seen_open_during_bounce,
        "contact bounce must exhibit both closing and opening bounce intervals before settling"
    );

    // After bounce interval has passed (> 2.5 ms), contact must be solidly settled closed
    for _ in 0..10 {
        c.run_ps(100_000_000).unwrap();
    }
    let v_load_settled = c.pin_voltage("R_LOAD-lPin").unwrap();
    assert!(
        v_load_settled > 11.9,
        "settled contact must deliver full voltage to load, got {v_load_settled} V"
    );
}

#[test]
fn test_relay_dielectric_voltage_breakdown() {
    let mut c = Circuit::new();
    c.dt = 1e-4;
    // Relay with 500 V breakdown limit
    let relay_state = RelayState::new(0.002, 500.0, 0.05);
    c.add_fixed_volt("V_HV", 100.0) // Initially 100 V (below 500 V breakdown)
        .add_ground("GND")
        .add_resistor("R_LIMIT", 1000.0)
        .add_comp(Comp {
            id: "K".into(),
            kind: Kind::Relay {
                norm_close: false,
                double_throw: false,
                poles: 1,
                i_on: 0.02,
                i_off: 0.01,
                r_coil: 100.0,
                inductance: 0.0,
                coil_i: 0.0,
                active: false,
                state: relay_state,
            },
        })
        .connect("GND-Gnd", "K-lPin")
        .connect("GND-Gnd", "K-rPin")
        .connect("V_HV-outnod", "K-c1")
        .connect("K-c2", "R_LIMIT-lPin")
        .connect("R_LIMIT-rPin", "GND-Gnd");

    c.solve().unwrap();

    // At 100 V (< 500 V), contact is open and completely insulates
    let i_leak = c.current_out_of_pin("V_HV-outnod").unwrap().abs();
    assert!(
        i_leak < 1e-6,
        "100 V across open contact must not break down, got {i_leak} A"
    );

    // Increase voltage to 600 V (> 500 V breakdown)
    c.update_component_kind("V_HV", &Kind::FixedVolt { voltage: 600.0 });
    c.re_solve().unwrap();

    // Dielectric breakdown causes spark arc conduction across the open contacts
    let i_breakdown = c.current_out_of_pin("V_HV-outnod").unwrap().abs();
    assert!(
        i_breakdown > 0.1,
        "dielectric breakdown at 600 V must ignite arc conduction, got {i_breakdown} A"
    );
    let state = match c.comp("K").unwrap().kind {
        Kind::Relay { state, .. } => state,
        _ => panic!("expected Relay"),
    };
    assert!(
        state.arcing,
        "relay state must indicate dielectric breakdown arcing"
    );
}

#[test]
fn test_relay_inductive_switching_arcing() {
    let mut state = RelayState::new(0.002, 500.0, 0.05);
    // Simulate contact opening when breaking 24 V at 0.5 A
    // Before opening: energized, conducting
    state.contact_closed = true;
    state.in_bounce = false;

    // Contact opens (de-energized) with inductive flyback V = 30 V (> arc_voltage = 15 V) and I = 0.5 A (> arc_min_current = 0.1 A)
    let changed = state.update_contact_state(false, 30.0, 0.5, 0.010);
    assert!(changed);
    assert!(
        state.arcing,
        "inductive interruption should trigger switching arc"
    );
    assert!(
        state.contact_conductance() >= 0.1,
        "arcing contact conducts across gap"
    );

    // After arc duration expires (> 3 ms) and current decreases below minimum
    state.update_contact_state(false, 5.0, 0.01, 0.020);
    assert!(
        !state.arcing,
        "arc should extinguish when current and voltage drop"
    );
    assert_eq!(
        state.contact_conductance(),
        1e-12,
        "extinguished contact restores open isolation"
    );
}

#[test]
fn test_electromechanical_properties_and_accessors() {
    // 1. Stepper properties
    let mut stepper = Stepper::default();
    assert_eq!(stepper.get_prop_text("HoldingTorque").unwrap(), "400 mN·m");
    assert_eq!(stepper.get_prop_text("Inertia").unwrap(), "6 µkg·m²");
    assert_eq!(stepper.get_prop_text("Friction").unwrap(), "4 mN·m·s/rad");
    assert_eq!(stepper.get_prop_text("LoadTorque").unwrap(), "0 N·m");

    assert!(stepper.set_prop_text("HoldingTorque", "0.85 N·m").is_ok());
    assert!((stepper.holding_torque - 0.85).abs() < 1e-4);
    assert!(stepper.set_prop_text("Inertia", "12 µkg·m²").is_ok());
    assert!((stepper.inertia - 1.2e-5).abs() < 1e-8);
    assert!(stepper.set_prop_text("Friction", "12 mN·m·s/rad").is_ok());
    assert!((stepper.friction - 0.012).abs() < 1e-5);
    assert!(stepper.set_prop_text("LoadTorque", "0.3 N·m").is_ok());
    assert!((stepper.load_torque - 0.3).abs() < 1e-4);

    // 2. Servo properties
    let mut servo = Servo::default();
    assert_eq!(servo.get_prop_text("IdleCurrent").unwrap(), "15 mA");
    assert_eq!(servo.get_prop_text("RunCurrent").unwrap(), "250 mA");
    assert_eq!(servo.get_prop_text("StallCurrent").unwrap(), "1 A");
    assert_eq!(servo.get_prop_text("LoadTorque").unwrap(), "0 kg·cm");
    assert_eq!(servo.get_prop_text("StallTorque").unwrap(), "3.5 kg·cm");

    assert!(servo.set_prop_text("IdleCurrent", "25 mA").is_ok());
    assert!((servo.idle_current - 0.025).abs() < 1e-4);
    assert!(servo.set_prop_text("RunCurrent", "400 mA").is_ok());
    assert!((servo.run_current - 0.4).abs() < 1e-4);
    assert!(servo.set_prop_text("StallCurrent", "1.8 A").is_ok());
    assert!((servo.stall_current - 1.8).abs() < 1e-4);
    assert!(servo.set_prop_text("LoadTorque", "1.2 kg·cm").is_ok());
    assert!((servo.load_torque - 1.2).abs() < 1e-4);
    assert!(servo.set_prop_text("StallTorque", "4.5 kg·cm").is_ok());
    assert!((servo.stall_torque - 4.5).abs() < 1e-4);

    // 3. Relay properties
    let mut relay = Relay::default();
    assert_eq!(relay.get_prop_text("BounceTime").unwrap(), "2 ms");
    assert_eq!(relay.get_prop_text("BreakdownV").unwrap(), "500 V");
    assert_eq!(relay.get_prop_text("ContactR").unwrap(), "50 mΩ");

    assert!(relay.set_prop_text("BounceTime", "5 ms").is_ok());
    assert!((relay.bounce_time - 0.005).abs() < 1e-5);
    assert!(relay.set_prop_text("BreakdownV", "750 V").is_ok());
    assert!((relay.breakdown_v - 750.0).abs() < 1e-3);
    assert!(relay.set_prop_text("ContactR", "20 mΩ").is_ok());
    assert!((relay.contact_r - 0.02).abs() < 1e-5);
}
