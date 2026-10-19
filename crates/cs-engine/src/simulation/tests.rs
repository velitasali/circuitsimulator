//! Circuit simulation unit and integration test suite.

use super::*;
use crate::digital::{BcdToDecState, CounterState, DacState, FunctionState, PinMode};
use crate::elements::pins::seven_segment_die_count;
use crate::elements::{Comp, Kind, LedState};
use crate::subcircuit::SubcSearch;

fn approx(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-6, "{a} != {b}");
}

#[test]
fn fixed_volt_resistor_ground() {
    let mut c = Circuit::new();
    c.add_fixed_volt("Fixed Voltage-1", 5.0)
        .add_resistor("Resistor-2", 1000.0)
        .add_ground("Ground-3")
        .connect("Fixed Voltage-1-outnod", "Resistor-2-lPin")
        .connect("Resistor-2-rPin", "Ground-3-Gnd");
    c.solve().unwrap();
    approx(c.pin_voltage("Fixed Voltage-1-outnod").unwrap(), 5.0);
    approx(c.pin_voltage("Ground-3-Gnd").unwrap(), crate::CERO_DOUB);
    approx(c.resistor_current("Resistor-2").unwrap(), 5.0 / 1000.0);
}

#[test]
fn voltage_divider() {
    let mut c = Circuit::new();
    c.add_fixed_volt("V1", 5.0)
        .add_resistor("R1", 1000.0)
        .add_resistor("R2", 1000.0)
        .add_ground("GND")
        .add_node("Node-5")
        .connect("V1-outnod", "R1-lPin")
        .connect("R1-rPin", "Node-5-0")
        .connect("R2-lPin", "Node-5-1")
        .connect("R2-rPin", "GND-Gnd");
    c.solve().unwrap();
    approx(c.pin_voltage("Node-5-0").unwrap(), 2.5);
    approx(c.resistor_current("R1").unwrap(), 2.5 / 1000.0);
}

#[test]
fn battery_resistor_ground() {
    let mut c = Circuit::new();
    c.add_battery("Battery-1", 5.0, 1e-3)
        .add_resistor("R1", 100.0)
        .add_ground("GND")
        .add_node("Node-4")
        .connect("Battery-1-lPin", "R1-lPin")
        .connect("R1-rPin", "Node-4-0")
        .connect("Battery-1-rPin", "Node-4-1")
        .connect("GND-Gnd", "Node-4-2");
    c.solve().unwrap();
    // + terminal is ~5 V above ground; internal 1 mΩ drops a little.
    let vp = c.pin_voltage("Battery-1-lPin").unwrap();
    let vn = c.pin_voltage("Battery-1-rPin").unwrap();
    approx(vn, crate::CERO_DOUB);
    assert!((vp - 5.0).abs() < 1e-3, "plus terminal {vp}");
    approx(c.resistor_current("R1").unwrap(), vp / 100.0);
}

#[test]
fn series_chain_uses_lu() {
    let mut c = Circuit::new();
    c.add_fixed_volt("V1", 5.0)
        .add_resistor("R1", 1000.0)
        .add_resistor("R2", 1000.0)
        .add_resistor("R3", 1000.0)
        .add_ground("GND")
        .add_node("N1")
        .add_node("N2")
        .connect("V1-outnod", "R1-lPin")
        .connect("R1-rPin", "N1-0")
        .connect("R2-lPin", "N1-1")
        .connect("R2-rPin", "N2-0")
        .connect("R3-lPin", "N2-1")
        .connect("R3-rPin", "GND-Gnd");
    c.solve().unwrap();
    approx(c.pin_voltage("N1-0").unwrap(), 10.0 / 3.0);
    approx(c.pin_voltage("N2-0").unwrap(), 5.0 / 3.0);
    approx(c.pin_voltage("GND-Gnd").unwrap(), crate::CERO_DOUB);
}

#[test]
fn floating_battery_holds_voltage() {
    let mut c = Circuit::new();
    c.add_battery("B1", 5.0, 1e-3)
        .add_resistor("R1", 100.0)
        .connect("B1-lPin", "R1-lPin")
        .connect("B1-rPin", "R1-rPin");
    c.solve().unwrap();
    let vp = c.pin_voltage("B1-lPin").unwrap();
    let vn = c.pin_voltage("B1-rPin").unwrap();
    let g_bat = 1.0 / 1e-3;
    let expected = 5.0 * g_bat / (g_bat + 1.0 / 100.0);
    approx(vp - vn, expected);
}

#[test]
fn floating_charged_capacitor_discharges() {
    let mut c = Circuit::new();
    c.dt = 1e-6;
    c.add_resistor("R1", 100.0).add_capacitor("C1", 10e-6);
    for comp in c.components_mut() {
        if let Kind::Capacitor { volt, .. } = &mut comp.kind {
            *volt = 5.0;
        }
    }
    c.connect("R1-lPin", "C1-lPin")
        .connect("R1-rPin", "C1-rPin");
    c.solve().unwrap();
    let diff = c.pin_voltage("C1-lPin").unwrap() - c.pin_voltage("C1-rPin").unwrap();
    let g_c = 10e-6 / c.dt;
    let ratio = g_c / (g_c + 1.0 / 100.0);
    approx(diff, 5.0 * ratio);

    c.run_ps(1_000_000_000).unwrap();
    let v = c.pin_voltage("C1-lPin").unwrap() - c.pin_voltage("C1-rPin").unwrap();
    let be = 5.0 * ratio.powi(1000);
    approx(v, be);
    let analytic = 5.0 * (-1.0_f64).exp();
    assert!(
        (v - analytic).abs() < 0.01,
        "discharge {v} analytic {analytic}"
    );
}

#[test]
fn two_islands_led_knee_does_not_stall() {
    // capacitor.circ1: a floating RC on the left, and on the right a 1 mF
    // capacitor precharged to 5 V feeding a 100 Ω resistor and a 3.5 V LED.
    // The scope's 10 MΩ to ground makes the right island's 3×3 matrix
    // nonsingular but ill-conditioned. At about 33 ms the old determinant
    // solver flipped the LED every iteration and `NLsteps=20` aborted.
    let mut c = Circuit::new();
    c.dt = 1e-6;
    c.max_nl_steps = 20;
    c.add_resistor("R9", 100.0).add_capacitor("C3", 1e-3);
    c.connect("R9-lPin", "C3-lPin")
        .connect("R9-rPin", "C3-rPin");
    c.add_resistor("R1", 100.0)
        .add_resistor("R4", 100.0)
        .add_capacitor("C6", 1e-3)
        .add_led("LED")
        .add_tunnel("T1", "t1", "T1-pin")
        .add_tunnel("T2", "t2", "T2-pin")
        .add_oscope_full(
            "OSC",
            true,
            10.0,
            ["t1".into(), "t2".into(), String::new(), String::new()],
        );
    assert!(c.configure_led("LED", 3.5, 0.03, 0.1, false));
    for comp in c.components_mut() {
        if let Kind::Capacitor { volt, .. } = &mut comp.kind {
            *volt = 5.0;
        }
    }
    // Net A: cap+, both resistor left pins, scope channel t1.
    // Net B: R1 right, LED anode, scope channel t2.
    // Net C: cap−, R4 right, LED cathode.
    c.connect("C6-lPin", "R1-lPin")
        .connect("C6-lPin", "R4-lPin")
        .connect("C6-lPin", "T1-pin")
        .connect("R1-rPin", "LED-lPin")
        .connect("R1-rPin", "T2-pin")
        .connect("C6-rPin", "R4-rPin")
        .connect("C6-rPin", "LED-rPin");

    c.solve().unwrap();
    c.run_ps(50_000_000_000).unwrap();

    let left = c.pin_voltage("C3-lPin").unwrap() - c.pin_voltage("C3-rPin").unwrap();
    let right = c.pin_voltage("C6-lPin").unwrap() - c.pin_voltage("C6-rPin").unwrap();
    let analytic = 5.0 * (-0.5_f64).exp();
    assert!(
        (left - analytic).abs() < 0.01,
        "left RC {left} analytic {analytic}"
    );
    assert!(
        right < 3.5 && right > 1.0,
        "right cap should be past the LED knee, got {right}"
    );
    let led = c.components().iter().find(|comp| comp.id == "LED").unwrap();
    let Kind::Led { state } = &led.kind else {
        panic!("missing LED");
    };
    assert!(
        state.current.abs() < 1e-4,
        "LED should be off, I={}",
        state.current
    );
}

#[test]
fn rc_charges_toward_source() {
    let mut c = Circuit::new();
    c.dt = 1e-6;
    c.add_fixed_volt("V1", 5.0)
        .add_resistor("R1", 1000.0)
        .add_capacitor("C1", 10e-6)
        .add_ground("GND")
        .connect("V1-outnod", "R1-lPin")
        .connect("R1-rPin", "C1-lPin")
        .connect("C1-rPin", "GND-Gnd");
    c.solve().unwrap();
    // Uncharged: first solve sees Ieq=0, G=C/dt=10 S so the cap is a
    // stiff short and the mid node sits near 0.
    let v0 = c.pin_voltage("C1-lPin").unwrap();
    assert!(v0 < 0.5, "start {v0}");
    c.step_n(10_000).unwrap(); // 10 ms = 1 τ
    let v = c.pin_voltage("C1-lPin").unwrap();
    assert!(
        (v - 5.0 * (1.0 - (-1.0f64).exp())).abs() < 0.15,
        "after 1τ got {v}"
    );
    assert!((c.time - 0.01).abs() < 1e-9);
}

#[test]
fn switch_opens_the_path() {
    let mut c = Circuit::new();
    c.add_fixed_volt("V1", 5.0)
        .add_switch("SW1", true)
        .add_resistor("R1", 1000.0)
        .add_ground("GND")
        .connect("V1-outnod", "SW1-pinP0")
        .connect("SW1-switch0pinN", "R1-lPin")
        .connect("R1-rPin", "GND-Gnd");
    c.solve().unwrap();
    approx(c.resistor_current("R1").unwrap(), 5.0 / 1000.0);
    c.set_switch("SW1", false);
    c.solve().unwrap();
    assert!(c.resistor_current("R1").unwrap().abs() < 1e-9);
}

#[test]
fn diode_forward_drop() {
    let mut c = Circuit::new();
    c.add_fixed_volt("V1", 5.0)
        .add_resistor("R1", 1000.0)
        .add_diode("D1")
        .add_ground("GND")
        .connect("V1-outnod", "R1-lPin")
        .connect("R1-rPin", "D1-lPin")
        .connect("D1-rPin", "GND-Gnd");
    c.solve().unwrap();
    let vf = c.pin_voltage("D1-lPin").unwrap();
    assert!(vf > 0.4 && vf < 0.9, "Vf {vf}");
    let i = c.resistor_current("R1").unwrap();
    assert!((i - (5.0 - vf) / 1000.0).abs() < 1e-6);
}

#[test]
fn diode_blocks_reverse() {
    let mut c = Circuit::new();
    c.add_fixed_volt("V1", 5.0)
        .add_resistor("R1", 1000.0)
        .add_diode("D1")
        .add_ground("GND")
        .connect("V1-outnod", "R1-lPin")
        .connect("R1-rPin", "D1-rPin")
        .connect("D1-lPin", "GND-Gnd");
    c.solve().unwrap();
    assert!(c.resistor_current("R1").unwrap().abs() < 1e-6);
}

#[test]
fn led_conducts_above_threshold() {
    let mut c = Circuit::new();
    c.add_fixed_volt("V1", 5.0)
        .add_resistor("R1", 330.0)
        .add_led("LED1")
        .add_ground("GND")
        .connect("V1-outnod", "R1-lPin")
        .connect("R1-rPin", "LED1-lPin")
        .connect("LED1-rPin", "GND-Gnd");
    c.solve().unwrap();
    let vf = c.pin_voltage("LED1-lPin").unwrap();
    assert!((vf - 2.4).abs() < 0.05, "LED Vf {vf}");
    let i = c.resistor_current("R1").unwrap();
    assert!((i - (5.0 - vf) / 330.0).abs() < 1e-5);

    // Current polarities: conventional current flows V1 (+) -> R1 -> LED1 -> GND
    let i_v1 = c.current_out_of_pin("V1-outnod").unwrap();
    let i_r1_in = c.current_out_of_pin("R1-lPin").unwrap();
    let i_r1_out = c.current_out_of_pin("R1-rPin").unwrap();
    let i_led_anode = c.current_out_of_pin("LED1-lPin").unwrap();
    let i_led_cathode = c.current_out_of_pin("LED1-rPin").unwrap();
    let i_gnd = c.current_out_of_pin("GND-Gnd").unwrap();

    assert!(i_v1 > 0.0, "V1 outnod should supply positive outflow");
    assert!(
        i_r1_in < 0.0,
        "R1 lPin should have inflow (negative current out of pin)"
    );
    assert!(
        i_r1_out > 0.0,
        "R1 rPin should have outflow (positive current out of pin)"
    );
    assert!(
        i_led_anode < 0.0,
        "LED1 anode should have inflow (negative current out of pin)"
    );
    assert!(
        i_led_cathode > 0.0,
        "LED1 cathode should have outflow (positive current out of pin)"
    );
    assert!(
        i_gnd < 0.0,
        "GND should sink current (negative current out of pin)"
    );
    assert!(
        (i_gnd.abs() - i_v1).abs() < 1e-4,
        "GND current magnitude {i_gnd} must match V1 outflow {i_v1}"
    );
}

#[test]
fn ground_current_zero_at_idle() {
    let mut c = Circuit::new();
    c.add_ground("GND1");
    c.add_ground("GND2");
    c.connect("GND1-Gnd", "GND2-Gnd");
    c.solve().unwrap();
    let i_gnd = c.current_out_of_pin("GND1-Gnd").unwrap();
    assert!(
        i_gnd.abs() < 1e-9,
        "Idle ground pin current should be ~0 A, got {i_gnd}"
    );
}

#[test]
fn wavegen_audioout_ground_current_conservation() {
    let mut c = Circuit::new();
    // 5V unipolar WaveGen into 8 ohm AudioOut to Ground
    c.add_wave_gen("WG1", "Sine", 1000.0, 2.5, 2.5, 0.5, 0.0, 100, false, false);
    c.add_audio_out("Audio1", 8.0);
    c.add_ground("GND");
    c.connect("WG1-outnod", "Audio1-lPin");
    c.connect("Audio1-rPin", "GND-Gnd");

    c.circ_time = 250_000_000; // quarter period (peak output, 250 us)
    c.solve().unwrap();

    let i_wg = c.current_out_of_pin("WG1-outnod").expect("WG1 current");
    let i_spk_pos = c.current_out_of_pin("Audio1-lPin").expect("Audio1 lPin");
    let i_spk_neg = c.current_out_of_pin("Audio1-rPin").expect("Audio1 rPin");
    let i_gnd = c.current_out_of_pin("GND-Gnd").expect("GND current");

    assert!(i_wg > 0.0, "WaveGen should supply positive current");
    assert!(
        (i_spk_pos + i_wg).abs() < 1e-4,
        "AudioOut (+) pin should have inflow equal to WaveGen outflow: i_spk_pos={i_spk_pos}, i_wg={i_wg}"
    );
    assert!(
        (i_spk_neg - i_wg).abs() < 1e-4,
        "AudioOut (-) pin should have outflow equal to WaveGen outflow: i_spk_neg={i_spk_neg}, i_wg={i_wg}"
    );
    assert!(
        (i_gnd + i_wg).abs() < 1e-4,
        "GND pin should sink current equal to WaveGen outflow: i_gnd={i_gnd}, i_wg={i_wg}"
    );
}

#[test]
fn wavegen_bipolar_floating_current_polarity() {
    let mut c = Circuit::new();
    c.add_wave_gen("WG1", "Sine", 1000.0, 5.0, 0.0, 0.5, 0.0, 100, true, true);
    c.add_resistor("R1", 1000.0);
    c.add_ground("GND");
    c.connect("WG1-outnod", "R1-lPin");
    c.connect("R1-rPin", "WG1-gndnod");
    c.connect("WG1-gndnod", "GND-Gnd");

    c.circ_time = 250_000_000; // peak voltage (250 us)
    c.solve().unwrap();

    let i_out = c.current_out_of_pin("WG1-outnod").expect("outnod current");
    let i_gndnod = c.current_out_of_pin("WG1-gndnod").expect("gndnod current");
    let i_r_in = c.current_out_of_pin("R1-lPin").expect("R1 lPin current");
    let i_r_out = c.current_out_of_pin("R1-rPin").expect("R1 rPin current");

    assert!(
        i_out > 0.0,
        "Floating WaveGen outnod should supply positive current, got {i_out}"
    );
    assert!(
        i_gndnod < 0.0,
        "Floating WaveGen gndnod should sink current (negative current out of pin), got {i_gndnod}"
    );
    assert!(
        (i_out + i_gndnod).abs() < 1e-4,
        "Floating WaveGen pins must balance: outnod={i_out}, gndnod={i_gndnod}"
    );
    assert!(
        (i_out - i_r_out).abs() < 1e-4,
        "R1 outflow must match outnod: R1={i_r_out}, WG={i_out}"
    );
    assert!(
        (i_r_in + i_out).abs() < 1e-4,
        "R1 inflow must match outnod: R1={i_r_in}, WG={i_out}"
    );
}

#[test]
fn switch_current_closed_and_open() {
    let mut c = Circuit::new();
    c.add_fixed_volt("V1", 5.0);
    c.add_switch("SW1", true);
    c.add_resistor("R1", 1000.0);
    c.add_ground("GND");
    c.connect("V1-outnod", "SW1-pinP0");
    c.connect("SW1-switch0pinN", "R1-lPin");
    c.connect("R1-rPin", "GND-Gnd");
    c.solve().unwrap();

    let i_sw_in = c.current_out_of_pin("SW1-pinP0").expect("SW1 pinP0");
    let i_sw_out = c
        .current_out_of_pin("SW1-switch0pinN")
        .expect("SW1 switch0pinN");

    assert!(
        (i_sw_in + 0.005).abs() < 1e-4,
        "SW1 pinP0 should have inflow of 5mA, got {i_sw_in}"
    );
    assert!(
        (i_sw_out - 0.005).abs() < 1e-4,
        "SW1 switch0pinN should have outflow of 5mA, got {i_sw_out}"
    );

    // Open the switch
    c.set_switch("SW1", false);
    c.solve().unwrap();
    let i_sw_in_open = c.current_out_of_pin("SW1-pinP0").expect("SW1 pinP0 open");
    let i_sw_out_open = c
        .current_out_of_pin("SW1-switch0pinN")
        .expect("SW1 switch0pinN open");
    assert_eq!(i_sw_in_open, 0.0);
    assert_eq!(i_sw_out_open, 0.0);
}

#[test]
fn potentiometer_pin_currents_satisfy_kcl() {
    let mut c = Circuit::new();
    c.add_fixed_volt("V1", 10.0);
    c.add_potentiometer("POT1", 10_000.0, 0.5);
    c.add_ground("GND");
    c.connect("V1-outnod", "POT1-PinA");
    c.connect("POT1-PinB", "GND-Gnd");
    c.solve().unwrap();

    let i_a = c.current_out_of_pin("POT1-PinA").expect("PinA");
    let i_b = c.current_out_of_pin("POT1-PinB").expect("PinB");
    let i_m = c.current_out_of_pin("POT1-PinM").expect("PinM");

    assert!(
        (i_a + 0.001).abs() < 1e-5,
        "POT1 PinA should have 1mA inflow, got {i_a}"
    );
    assert!(
        (i_b - 0.001).abs() < 1e-5,
        "POT1 PinB should have 1mA outflow, got {i_b}"
    );
    assert!(
        i_m.abs() < 1e-9,
        "POT1 unconnected wiper should have 0A current, got {i_m}"
    );
    assert!(
        (i_a + i_b + i_m).abs() < 1e-6,
        "KCL must hold on potentiometer pins: sum={}",
        i_a + i_b + i_m
    );
}

#[test]
fn grounded_led_conducts_without_cathode_wire() {
    let mut c = Circuit::new();
    c.add_fixed_volt("V1", 5.0)
        .add_resistor("R1", 330.0)
        .add_led("LED1")
        .connect("V1-outnod", "R1-lPin")
        .connect("R1-rPin", "LED1-lPin");
    assert!(c.configure_led("LED1", 2.4, 0.03, 0.6, true));
    c.solve().unwrap();
    let vf = c.pin_voltage("LED1-lPin").unwrap();
    assert!((vf - 2.4).abs() < 0.05, "grounded LED Vf {vf}");
    let i = c.resistor_current("R1").unwrap();
    assert!((i - (5.0 - vf) / 330.0).abs() < 1e-5);
    assert!(c.pin_voltage("LED1-rPin").is_none());
}

#[test]
fn npn_saturates_with_base_drive() {
    let mut c = Circuit::new();
    c.add_fixed_volt("VCC", 5.0)
        .add_resistor("RC", 1000.0)
        .add_resistor("RB", 10_000.0)
        .add_bjt("Q1", false)
        .add_ground("GND")
        .connect("VCC-outnod", "RC-lPin")
        .connect("RC-rPin", "Q1-collector")
        .connect("VCC-outnod", "RB-lPin")
        .connect("RB-rPin", "Q1-base")
        .connect("Q1-emiter", "GND-Gnd");
    c.solve().unwrap();
    let vc = c.pin_voltage("Q1-collector").unwrap();
    assert!(vc < 0.4, "saturated Vc {vc}");
    let ic = c.resistor_current("RC").unwrap();
    assert!(ic > 4e-3 && ic < 6e-3, "Ic {ic}");
}

#[test]
fn npn_cuts_off_with_base_at_ground() {
    let mut c = Circuit::new();
    c.add_fixed_volt("VCC", 5.0)
        .add_resistor("RC", 1000.0)
        .add_resistor("RB", 10_000.0)
        .add_bjt("Q1", false)
        .add_ground("GND")
        .connect("VCC-outnod", "RC-lPin")
        .connect("RC-rPin", "Q1-collector")
        .connect("RB-lPin", "GND-Gnd")
        .connect("RB-rPin", "Q1-base")
        .connect("Q1-emiter", "GND-Gnd");
    c.solve().unwrap();
    let vc = c.pin_voltage("Q1-collector").unwrap();
    assert!((vc - 5.0).abs() < 0.05, "cutoff Vc {vc}");
    assert!(c.resistor_current("RC").unwrap().abs() < 1e-5);
}

#[test]
fn nmos_conducts_with_gate_high() {
    let mut c = Circuit::new();
    c.add_fixed_volt("VDD", 5.0)
        .add_fixed_volt("VG", 5.0)
        .add_resistor("RD", 1000.0)
        .add_mosfet("M1", false, false)
        .add_ground("GND")
        .connect("VDD-outnod", "RD-lPin")
        .connect("RD-rPin", "M1-Dren")
        .connect("VG-outnod", "M1-Gate")
        .connect("M1-Sour", "GND-Gnd");
    c.solve().unwrap();
    let vd = c.pin_voltage("M1-Dren").unwrap();
    assert!(vd < 0.2, "on Vd {vd}");
    let i = c.resistor_current("RD").unwrap();
    assert!((i - 5.0 / 1000.0).abs() < 5e-4, "Id {i}");
}

#[test]
fn nmos_off_with_gate_low() {
    let mut c = Circuit::new();
    c.add_fixed_volt("VDD", 5.0)
        .add_resistor("RD", 1000.0)
        .add_mosfet("M1", false, false)
        .add_ground("GND")
        .connect("VDD-outnod", "RD-lPin")
        .connect("RD-rPin", "M1-Dren")
        .connect("M1-Gate", "GND-Gnd")
        .connect("M1-Sour", "GND-Gnd");
    c.solve().unwrap();
    let vd = c.pin_voltage("M1-Dren").unwrap();
    assert!((vd - 5.0).abs() < 0.05, "off Vd {vd}");
    assert!(c.resistor_current("RD").unwrap().abs() < 1e-5);
}

#[test]
fn opamp_open_loop_rails_high() {
    let mut c = Circuit::new();
    c.add_fixed_volt("VP", 5.0)
        .add_opamp("OA1")
        .add_resistor("R1", 1000.0)
        .add_ground("GND")
        .connect("VP-outnod", "OA1-inputNinv")
        .connect("OA1-inputInv", "GND-Gnd")
        .connect("OA1-output", "R1-lPin")
        .connect("R1-rPin", "GND-Gnd");
    c.solve().unwrap();
    let vo = c.pin_voltage("OA1-output").unwrap();
    assert!((vo - 5.0).abs() < 0.01, "open-loop high {vo}");
    let i = c.resistor_current("R1").unwrap();
    assert!((i - 5.0 / 1000.0).abs() < 1e-4, "I {i}");
}

#[test]
fn opamp_open_loop_rails_low() {
    let mut c = Circuit::new();
    c.add_fixed_volt("VN", 5.0)
        .add_opamp("OA1")
        .add_resistor("R1", 1000.0)
        .add_ground("GND")
        .connect("VN-outnod", "OA1-inputInv")
        .connect("OA1-inputNinv", "GND-Gnd")
        .connect("OA1-output", "R1-lPin")
        .connect("R1-rPin", "GND-Gnd");
    c.solve().unwrap();
    let vo = c.pin_voltage("OA1-output").unwrap();
    assert!(vo.abs() < 0.01, "open-loop low {vo}");
}

#[test]
fn opamp_voltage_follower() {
    let mut c = Circuit::new();
    c.add_fixed_volt("VIN", 2.5)
        .add_opamp("OA1")
        .add_ground("GND")
        .connect("VIN-outnod", "OA1-inputNinv")
        .connect("OA1-output", "OA1-inputInv")
        .connect("GND-Gnd", "OA1-powerNeg");
    c.solve().unwrap();
    let vo = c.pin_voltage("OA1-output").unwrap();
    assert!((vo - 2.5).abs() < 0.02, "follower {vo}");
}

#[test]
fn opamp_inverting_amplifier() {
    let mut c = Circuit::new();
    c.add_fixed_volt("VIN", 0.2)
        .add_opamp("OA1")
        .add_resistor("RIN", 1000.0)
        .add_resistor("RF", 10_000.0)
        .add_ground("GND");
    assert!(c.configure_opamp("OA1", 1000.0, crate::LOW_IMP, 5.0, -5.0, false));
    c.connect("VIN-outnod", "RIN-lPin")
        .connect("RIN-rPin", "OA1-inputInv")
        .connect("OA1-output", "RF-lPin")
        .connect("RF-rPin", "OA1-inputInv")
        .connect("OA1-inputNinv", "GND-Gnd");
    c.solve().unwrap();
    let vo = c.pin_voltage("OA1-output").unwrap();
    assert!((vo + 2.0).abs() < 0.05, "inverting {vo}");
}

#[test]
fn opamp_power_pins_set_the_rails() {
    let mut c = Circuit::new();
    c.add_fixed_volt("VCC", 3.0)
        .add_fixed_volt("VIN", 2.5)
        .add_opamp("OA1")
        .add_ground("GND");
    assert!(c.configure_opamp("OA1", 1000.0, crate::LOW_IMP, 5.0, 0.0, true));
    c.connect("VIN-outnod", "OA1-inputNinv")
        .connect("OA1-output", "OA1-inputInv")
        .connect("VCC-outnod", "OA1-powerPos")
        .connect("GND-Gnd", "OA1-powerNeg");
    c.solve().unwrap();
    let vo = c.pin_voltage("OA1-output").unwrap();
    // 2.5 V in, rails at 3 V / 0 V — follower stays at 2.5 V.
    assert!((vo - 2.5).abs() < 0.02, "follower with supplies {vo}");
}

#[test]
fn jfet_idss_with_gate_at_source() {
    // Drain at 12 V through 100 Ω, source and gate at 0. Vgs = 0, sat
    // if Vds > |Vp|. Id ≈ Idss (1 + Vds/λ) ≈ 50 mA.
    let mut c = Circuit::new();
    c.add_fixed_volt("VDD", 12.0)
        .add_resistor("RD", 100.0)
        .add_jfet("J1")
        .add_ground("GND")
        .connect("VDD-outnod", "RD-lPin")
        .connect("RD-rPin", "J1-Dren")
        .connect("J1-Sour", "GND-Gnd")
        .connect("J1-Gate", "GND-Gnd");
    c.solve().unwrap();
    let i = c.resistor_current("RD").unwrap();
    assert!(i > 0.048 && i < 0.055, "Idss {i}");
    let vd = c.pin_voltage("J1-Dren").unwrap();
    assert!(vd > 6.0 && vd < 8.0, "Vd {vd}");
}

#[test]
fn jfet_cutoff_with_gate_below_vp() {
    let mut c = Circuit::new();
    c.add_fixed_volt("VDD", 5.0)
        .add_fixed_volt("VG", -5.0)
        .add_resistor("RD", 1000.0)
        .add_jfet("J1")
        .add_ground("GND")
        .connect("VDD-outnod", "RD-lPin")
        .connect("RD-rPin", "J1-Dren")
        .connect("VG-outnod", "J1-Gate")
        .connect("J1-Sour", "GND-Gnd");
    c.solve().unwrap();
    let vd = c.pin_voltage("J1-Dren").unwrap();
    assert!((vd - 5.0).abs() < 0.05, "cutoff Vd {vd}");
    assert!(c.resistor_current("RD").unwrap().abs() < 1e-5);
}

#[test]
fn jfet_swaps_source_when_drain_is_lower() {
    // Physical drain grounded, physical source at 12 V through 100 Ω,
    // gate at ground so Vgs = 0 vs the virtual source (the drain).
    let mut c = Circuit::new();
    c.add_fixed_volt("VS", 12.0)
        .add_resistor("RS", 100.0)
        .add_jfet("J1")
        .add_ground("GND")
        .connect("VS-outnod", "RS-lPin")
        .connect("RS-rPin", "J1-Sour")
        .connect("J1-Dren", "GND-Gnd")
        .connect("J1-Gate", "GND-Gnd");
    c.solve().unwrap();
    let i = c.resistor_current("RS").unwrap();
    assert!(i > 0.048 && i < 0.055, "reverse-channel Idss {i}");
}

#[test]
fn comparator_high_when_plus_above_minus() {
    let mut c = Circuit::new();
    c.add_fixed_volt("VP", 5.0)
        .add_comparator("CMP1")
        .add_resistor("RL", 10_000.0)
        .add_ground("GND")
        .connect("VP-outnod", "CMP1-in0")
        .connect("CMP1-in1", "GND-Gnd")
        .connect("CMP1-out", "RL-lPin")
        .connect("RL-rPin", "GND-Gnd");
    c.solve().unwrap();
    let vo = c.pin_voltage("CMP1-out").unwrap();
    // Thevenin 5 V / 40 Ω into 10 kΩ.
    let expect = 5.0 * 10_000.0 / (10_000.0 + 40.0);
    assert!((vo - expect).abs() < 0.01, "comparator high {vo}");
}

#[test]
fn comparator_low_when_plus_below_minus() {
    let mut c = Circuit::new();
    c.add_fixed_volt("VN", 5.0)
        .add_comparator("CMP1")
        .add_resistor("RL", 10_000.0)
        .add_ground("GND")
        .connect("VN-outnod", "CMP1-in1")
        .connect("CMP1-in0", "GND-Gnd")
        .connect("CMP1-out", "RL-lPin")
        .connect("RL-rPin", "GND-Gnd");
    c.solve().unwrap();
    let vo = c.pin_voltage("CMP1-out").unwrap();
    assert!(vo.abs() < 0.01, "comparator low {vo}");
}

#[test]
fn comparator_inverted() {
    let mut c = Circuit::new();
    c.add_fixed_volt("VP", 5.0)
        .add_comparator("CMP1")
        .add_resistor("RL", 10_000.0)
        .add_ground("GND");
    assert!(c.configure_comparator("CMP1", 5.0, 0.0, 40.0, true, false));
    c.connect("VP-outnod", "CMP1-in0")
        .connect("CMP1-in1", "GND-Gnd")
        .connect("CMP1-out", "RL-lPin")
        .connect("RL-rPin", "GND-Gnd");
    c.solve().unwrap();
    let vo = c.pin_voltage("CMP1-out").unwrap();
    assert!(vo.abs() < 0.01, "inverted high-in {vo}");
}

#[test]
fn comparator_open_collector_pullup() {
    let mut c = Circuit::new();
    c.add_fixed_volt("VP", 5.0)
        .add_fixed_volt("VCC", 5.0)
        .add_comparator("CMP1")
        .add_resistor("RU", 1000.0)
        .add_ground("GND");
    assert!(c.configure_comparator("CMP1", 5.0, 0.0, 40.0, false, true));
    c.connect("VP-outnod", "CMP1-in0")
        .connect("CMP1-in1", "GND-Gnd")
        .connect("VCC-outnod", "RU-lPin")
        .connect("RU-rPin", "CMP1-out");
    c.solve().unwrap();
    let vo = c.pin_voltage("CMP1-out").unwrap();
    assert!((vo - 5.0).abs() < 0.05, "OC high {vo}");
}

#[test]
fn volt_reg_holds_1_2_from_12v() {
    let mut c = Circuit::new();
    c.add_fixed_volt("VIN", 12.0)
        .add_volt_reg("VR1")
        .add_resistor("RL", 100.0)
        .add_ground("GND")
        .connect("VIN-outnod", "VR1-input")
        .connect("VR1-output", "RL-lPin")
        .connect("RL-rPin", "GND-Gnd")
        .connect("VR1-ref", "GND-Gnd");
    c.solve().unwrap();
    let vo = c.pin_voltage("VR1-output").unwrap();
    assert!((vo - 1.2).abs() < 0.02, "regulated {vo}");
    let i = c.resistor_current("RL").unwrap();
    assert!((i - 1.2 / 100.0).abs() < 5e-4, "Iload {i}");
}

#[test]
fn volt_reg_dropout_when_headroom_under_0_7() {
    let mut c = Circuit::new();
    c.add_fixed_volt("VIN", 1.5)
        .add_volt_reg("VR1")
        .add_resistor("RL", 100.0)
        .add_ground("GND")
        .connect("VIN-outnod", "VR1-input")
        .connect("VR1-output", "RL-lPin")
        .connect("RL-rPin", "GND-Gnd")
        .connect("VR1-ref", "GND-Gnd");
    c.solve().unwrap();
    let vo = c.pin_voltage("VR1-output").unwrap();
    // Vin 1.5, target 1.2, delta 0.3 < 0.7 → Vout ≈ Vin − 0.7 = 0.8.
    assert!((vo - 0.8).abs() < 0.05, "dropout {vo}");
}

#[test]
fn volt_reg_output_is_ref_plus_vref() {
    let mut c = Circuit::new();
    c.add_fixed_volt("VIN", 12.0)
        .add_fixed_volt("VREF", 2.0)
        .add_volt_reg("VR1")
        .add_resistor("RL", 100.0)
        .add_ground("GND")
        .connect("VIN-outnod", "VR1-input")
        .connect("VR1-output", "RL-lPin")
        .connect("RL-rPin", "GND-Gnd")
        .connect("VREF-outnod", "VR1-ref");
    c.solve().unwrap();
    let vo = c.pin_voltage("VR1-output").unwrap();
    assert!((vo - 3.2).abs() < 0.02, "ref+1.2 {vo}");
}

#[test]
fn volt_reg_custom_voltage() {
    let mut c = Circuit::new();
    c.add_fixed_volt("VIN", 12.0)
        .add_volt_reg("VR1")
        .add_resistor("RL", 200.0)
        .add_ground("GND");
    assert!(c.configure_volt_reg("VR1", 5.0));
    c.connect("VIN-outnod", "VR1-input")
        .connect("VR1-output", "RL-lPin")
        .connect("RL-rPin", "GND-Gnd")
        .connect("VR1-ref", "GND-Gnd");
    c.solve().unwrap();
    let vo = c.pin_voltage("VR1-output").unwrap();
    assert!((vo - 5.0).abs() < 0.05, "5 V out {vo}");
}

#[test]
fn voltmeter_reads_divider() {
    let mut c = Circuit::new();
    c.add_fixed_volt("V1", 5.0)
        .add_resistor("R1", 1000.0)
        .add_resistor("R2", 1000.0)
        .add_voltmeter("VM1", false)
        .add_ground("GND")
        .connect("V1-outnod", "R1-lPin")
        .connect("R1-rPin", "R2-lPin")
        .connect("R2-rPin", "GND-Gnd")
        .connect("VM1-lPin", "R2-lPin")
        .connect("VM1-rPin", "GND-Gnd");
    c.solve().unwrap();
    let stats = c.instruments.meters.get("VM1").expect("voltmeter stats");
    assert!(
        (stats.value - 2.5).abs() < 0.02,
        "voltmeter {0}",
        stats.value
    );
}

#[test]
fn ammeter_reads_series_current() {
    let mut c = Circuit::new();
    c.add_fixed_volt("V1", 5.0)
        .add_ammeter("AM1", false)
        .add_resistor("R1", 1000.0)
        .add_ground("GND")
        .connect("V1-outnod", "AM1-lPin")
        .connect("AM1-rPin", "R1-lPin")
        .connect("R1-rPin", "GND-Gnd");
    c.solve().unwrap();
    let stats = c.instruments.meters.get("AM1").expect("ammeter stats");
    assert!(
        (stats.value - 0.005).abs() < 2e-4,
        "ammeter {0}",
        stats.value
    );
}

#[test]
fn probe_reads_node_voltage() {
    let mut c = Circuit::new();
    c.add_fixed_volt("V1", 5.0)
        .add_resistor("R1", 1000.0)
        .add_probe("PR1")
        .add_ground("GND")
        .connect("V1-outnod", "R1-lPin")
        .connect("R1-rPin", "GND-Gnd")
        .connect("PR1-inpin", "R1-lPin");
    c.solve().unwrap();
    approx(c.pin_voltage("PR1-inpin").unwrap(), 5.0);
}

#[test]
fn oscope_samples_dc_and_rc() {
    let mut c = Circuit::new();
    c.dt = 1e-6;
    c.add_fixed_volt("V1", 5.0)
        .add_resistor("R1", 1000.0)
        .add_capacitor("C1", 10e-6)
        .add_oscope("OSC1")
        .add_ground("GND")
        .connect("V1-outnod", "R1-lPin")
        .connect("R1-rPin", "C1-lPin")
        .connect("C1-rPin", "GND-Gnd")
        .connect("OSC1-Pin0", "C1-lPin")
        .connect("OSC1-PinG", "GND-Gnd");
    c.solve().unwrap();
    c.step_n(2_000).unwrap();
    let s = c.instruments.scopes.get("OSC1").expect("oscope sampler");
    let last = s.last_values();
    assert!(last[0] > 0.5, "cap voltage sampled {0}", last[0]);
    let buf = s.traces(1e-3, 0.0, -1, 0.0, &crate::instruments::SCOPE_COLORS, None);
    assert_eq!(buf.channels.len(), 4);
    assert!(buf.channels[0].samples.iter().any(|v| *v > 0.4));
}

fn logic_high(v: f64) -> bool {
    v > 4.0
}
fn logic_low(v: f64) -> bool {
    v.abs() < 0.2
}

#[test]
fn and_gate_high_only_when_both_high() {
    let mut c = Circuit::new();
    c.add_fixed_volt("VA", 5.0)
        .add_fixed_volt("VB", 5.0)
        .add_and("AND1", 2)
        .add_resistor("RL", 10_000.0)
        .add_ground("GND")
        .connect("VA-outnod", "AND1-in0")
        .connect("VB-outnod", "AND1-in1")
        .connect("AND1-out", "RL-lPin")
        .connect("RL-rPin", "GND-Gnd");
    c.solve().unwrap();
    assert!(logic_high(c.pin_voltage("AND1-out").unwrap()));

    let mut c = Circuit::new();
    c.add_fixed_volt("VA", 5.0)
        .add_and("AND1", 2)
        .add_resistor("RL", 10_000.0)
        .add_ground("GND")
        .connect("VA-outnod", "AND1-in0")
        .connect("AND1-in1", "GND-Gnd")
        .connect("AND1-out", "RL-lPin")
        .connect("RL-rPin", "GND-Gnd");
    c.solve().unwrap();
    assert!(logic_low(c.pin_voltage("AND1-out").unwrap()));
}

#[test]
fn nand_inverts_and() {
    let mut c = Circuit::new();
    c.add_fixed_volt("VA", 5.0)
        .add_fixed_volt("VB", 5.0)
        .add_nand("G1", 2)
        .add_resistor("RL", 10_000.0)
        .add_ground("GND")
        .connect("VA-outnod", "G1-in0")
        .connect("VB-outnod", "G1-in1")
        .connect("G1-out", "RL-lPin")
        .connect("RL-rPin", "GND-Gnd");
    c.solve().unwrap();
    assert!(logic_low(c.pin_voltage("G1-out").unwrap()));
}

#[test]
fn or_xor_inverter() {
    let mut c = Circuit::new();
    c.add_fixed_volt("VA", 5.0)
        .add_or("OR1", 2)
        .add_xor("XOR1", 2)
        .add_inverter("INV1")
        .add_resistor("R1", 10_000.0)
        .add_resistor("R2", 10_000.0)
        .add_resistor("R3", 10_000.0)
        .add_ground("GND")
        .connect("VA-outnod", "OR1-in0")
        .connect("OR1-in1", "GND-Gnd")
        .connect("OR1-out", "R1-lPin")
        .connect("R1-rPin", "GND-Gnd")
        .connect("VA-outnod", "XOR1-in0")
        .connect("XOR1-in1", "GND-Gnd")
        .connect("XOR1-out", "R2-lPin")
        .connect("R2-rPin", "GND-Gnd")
        .connect("VA-outnod", "INV1-in0")
        .connect("INV1-out", "R3-lPin")
        .connect("R3-rPin", "GND-Gnd");
    c.solve().unwrap();
    assert!(logic_high(c.pin_voltage("OR1-out").unwrap()));
    assert!(logic_high(c.pin_voltage("XOR1-out").unwrap()));
    assert!(logic_low(c.pin_voltage("INV1-out").unwrap()));
}

#[test]
fn gate_delay_holds_output_until_tpd() {
    let mut c = Circuit::new();
    c.add_fixed_volt("VA", 5.0)
        .add_fixed_volt("VB", 5.0)
        .add_and("AND1", 2)
        .add_resistor("RL", 10_000.0)
        .add_ground("GND")
        .connect("VA-outnod", "AND1-in0")
        .connect("VB-outnod", "AND1-in1")
        .connect("AND1-out", "RL-lPin")
        .connect("RL-rPin", "GND-Gnd");
    assert!(c.set_prop_delay("AND1", 100e-9));
    c.run_ps(50_000).unwrap();
    assert!(
        logic_low(c.pin_voltage("AND1-out").unwrap()),
        "still delayed at 50 ns"
    );
    c.run_ps(60_000).unwrap();
    assert!(logic_high(c.pin_voltage("AND1-out").unwrap()));
}

#[test]
fn flipflop_d_takes_d_on_rising_clock() {
    let mut c = Circuit::new();
    c.add_fixed_volt("VCC", 5.0)
        .add_flipflop_d("FF1")
        .add_resistor("RQ", 10_000.0)
        .add_resistor("RQN", 10_000.0)
        .add_ground("GND")
        .connect("VCC-outnod", "FF1-in0") // D
        .connect("VCC-outnod", "FF1-in1") // S (active low, so high = inactive)
        .connect("VCC-outnod", "FF1-in2") // R inactive
        .connect("FF1-out0", "RQ-lPin")
        .connect("RQ-rPin", "GND-Gnd")
        .connect("FF1-out1", "RQN-lPin")
        .connect("RQN-rPin", "GND-Gnd");
    c.solve().unwrap();
    // Clock low: Q stays at q0=false.
    assert!(logic_low(c.pin_voltage("FF1-out0").unwrap()));
    c.connect("VCC-outnod", "FF1-in3"); // rising clock
    c.solve().unwrap();
    assert!(logic_high(c.pin_voltage("FF1-out0").unwrap()));
    assert!(logic_low(c.pin_voltage("FF1-out1").unwrap()));
}

#[test]
fn flipflop_jk_toggles_when_jk_high() {
    let mut c = Circuit::new();
    c.add_fixed_volt("VCC", 5.0)
        .add_flipflop_jk("FF1")
        .add_resistor("RQ", 10_000.0)
        .add_ground("GND")
        .connect("VCC-outnod", "FF1-in0") // J
        .connect("VCC-outnod", "FF1-in1") // K
        .connect("VCC-outnod", "FF1-in2") // S inactive
        .connect("VCC-outnod", "FF1-in3") // R inactive
        .connect("FF1-out0", "RQ-lPin")
        .connect("RQ-rPin", "GND-Gnd");
    c.solve().unwrap();
    let q0 = c.pin_voltage("FF1-out0").unwrap();
    assert!(logic_low(q0));
    c.connect("VCC-outnod", "FF1-in4");
    c.solve().unwrap();
    assert!(logic_high(c.pin_voltage("FF1-out0").unwrap()));
}

#[test]
fn mcu_pin_drives_and_reads() {
    let mut c = Circuit::new();
    c.add_mcu_pin("MCU1")
        .add_resistor("RL", 10_000.0)
        .add_ground("GND")
        .connect("MCU1-pin", "RL-lPin")
        .connect("RL-rPin", "GND-Gnd");
    c.solve().unwrap();
    assert!(c.mcu_pin_set_mode("MCU1", PinMode::Output));
    assert!(c.mcu_pin_drive("MCU1", true));
    c.update().unwrap();
    assert!(logic_high(c.pin_voltage("MCU1-pin").unwrap()));
    assert_eq!(c.mcu_pin_read("MCU1"), Some(true));
    assert!(c.mcu_pin_drive("MCU1", false));
    c.update().unwrap();
    assert!(logic_low(c.pin_voltage("MCU1-pin").unwrap()));
    assert_eq!(c.mcu_pin_read("MCU1"), Some(false));
}

#[test]
fn comparator_delay_then_rise() {
    let mut c = Circuit::new();
    c.add_fixed_volt("VP", 5.0)
        .add_comparator("CMP1")
        .add_resistor("RL", 10_000.0)
        .add_ground("GND")
        .connect("VP-outnod", "CMP1-in0")
        .connect("CMP1-in1", "GND-Gnd")
        .connect("CMP1-out", "RL-lPin")
        .connect("RL-rPin", "GND-Gnd");
    assert!(c.set_prop_delay("CMP1", 80e-9));
    c.run_ps(40_000).unwrap();
    assert!(
        logic_low(c.pin_voltage("CMP1-out").unwrap()),
        "comparator still delayed"
    );
    c.run_ps(50_000).unwrap();
    assert!(logic_high(c.pin_voltage("CMP1-out").unwrap()));
}

#[test]
fn tunnels_union_named_nets() {
    let mut c = Circuit::new();
    c.add_fixed_volt("V1", 5.0)
        .add_resistor("R1", 1000.0)
        .add_ground("GND")
        .add_tunnel("T1", "rail", "T1-pin")
        .add_tunnel("T2", "rail", "T2-pin")
        .connect("V1-outnod", "T1-pin")
        .connect("T2-pin", "R1-lPin")
        .connect("R1-rPin", "GND-Gnd");
    c.solve().unwrap();
    approx(c.pin_voltage("T1-pin").unwrap(), 5.0);
    approx(c.pin_voltage("T2-pin").unwrap(), 5.0);
    approx(c.resistor_current("R1").unwrap(), 5.0 / 1000.0);
}

const VDIV_SRC: &str = r#"<circuit version="2.0.0" >
<item itemtype="Package" CircId="Package-1" label="DIP" width="4" height="4" Pins="Pin; type=; xpos=-8; ypos=8; angle=180; length=8; space=0; id=in; label=in&#xa;Pin; type=; xpos=40; ypos=8; angle=0; length=8; space=0; id=out; label=out&#xa;Pin; type=; xpos=16; ypos=40; angle=270; length=8; space=0; id=gnd; label=gnd" SubcType="None" />
<item itemtype="Resistor" CircId="Resistor-1" Resistance="1 kΩ" />
<item itemtype="Resistor" CircId="Resistor-2" Resistance="1 kΩ" />
<item itemtype="Tunnel" CircId="Tunnel-1" Name="in" />
<item itemtype="Tunnel" CircId="Tunnel-2" Name="out" />
<item itemtype="Tunnel" CircId="Tunnel-3" Name="gnd" />
<item itemtype="Connector" uid="connector-1" startpinid="Tunnel-1-pin" endpinid="Resistor-1-lPin" />
<item itemtype="Connector" uid="connector-2" startpinid="Resistor-1-rPin" endpinid="Resistor-2-lPin" />
<item itemtype="Connector" uid="connector-3" startpinid="Resistor-1-rPin" endpinid="Tunnel-2-pin" />
<item itemtype="Connector" uid="connector-4" startpinid="Resistor-2-rPin" endpinid="Tunnel-3-pin" />
</circuit>
"#;

const VDIV_PARENT: &str = r#"<circuit version="2.0.0" >
<item itemtype="Fixed Voltage" CircId="Fixed Voltage-1" Voltage="5 V" />
<item itemtype="Subcircuit" CircId="vdiv-1" />
<item itemtype="Ground" CircId="Ground-1" />
<item itemtype="Connector" uid="connector-1" startpinid="Fixed Voltage-1-outnod" endpinid="vdiv-1-in" />
<item itemtype="Connector" uid="connector-2" startpinid="vdiv-1-gnd" endpinid="Ground-1-Gnd" />
</circuit>
"#;

#[test]
fn subcircuit_divider_from_memory() {
    let search = crate::subcircuit::SubcSearch::default().with_memory("vdiv", VDIV_SRC);
    let mut c = Circuit::from_sim1_with(VDIV_PARENT, &search).unwrap();
    assert!(
        !c.skipped().iter().any(|s| s.starts_with("Subcircuit")),
        "skipped {:?}",
        c.skipped()
    );
    c.solve().unwrap();
    approx(c.pin_voltage("vdiv-1-in").unwrap(), 5.0);
    approx(c.pin_voltage("vdiv-1-out").unwrap(), 2.5);
    approx(c.pin_voltage("vdiv-1-gnd").unwrap(), crate::CERO_DOUB);
}

#[test]
fn test_unit_and_gate_truth() {
    let mut c = Circuit::new();
    c.add_and("AND1", 2)
        .add_test_unit("TU1")
        .add_resistor("RL", 10_000.0)
        .add_ground("GND");
    assert!(c.configure_test_unit("TU1", "A,B", "Y", 100e-9, &[0, 0, 0, 1]));
    c.connect("TU1-out0", "AND1-in0")
        .connect("TU1-out1", "AND1-in1")
        .connect("AND1-out", "TU1-in0")
        .connect("AND1-out", "RL-lPin")
        .connect("RL-rPin", "GND-Gnd");
    let results = c.run_batch().unwrap();
    assert_eq!(results.len(), 1);
    assert!(results[0].ok, "AND truth {:?}", results);
}

#[test]
fn test_unit_wrong_truth_fails() {
    let mut c = Circuit::new();
    c.add_and("AND1", 2)
        .add_test_unit("TU1")
        .add_resistor("RL", 10_000.0)
        .add_ground("GND");
    assert!(c.configure_test_unit("TU1", "A,B", "Y", 100e-9, &[1, 1, 1, 1]));
    c.connect("TU1-out0", "AND1-in0")
        .connect("TU1-out1", "AND1-in1")
        .connect("AND1-out", "TU1-in0")
        .connect("AND1-out", "RL-lPin")
        .connect("RL-rPin", "GND-Gnd");
    let results = c.run_batch().unwrap();
    assert_eq!(results.len(), 1);
    assert!(!results[0].ok);
}

const PIC14_MCU: &str = r#"
<mcu core="Pic14" data="256" prog="64" progword="2" inst_cycle="4" freq="4000000">
  <regblock start="0" end="0x4F" streg="STATUS">
    <register name="INDF" addr="0x00" reset="0"/>
    <register name="PCL" addr="0x02" reset="0"/>
    <register name="STATUS" addr="0x03" reset="00011000" bits="C,DC,Z,PD,TO,RP0|R0,RP1|R1,IRP"/>
    <register name="FSR" addr="0x04" reset="0"/>
    <register name="PORTA" addr="0x05" reset="0"/>
    <register name="PCLATH" addr="0x0A" reset="0" mask="00011111"/>
  </regblock>
  <regblock start="0x80" end="0x8F">
    <mapped addr="0x80" mapto="0x00"/>
    <mapped addr="0x82" mapto="0x02"/>
    <mapped addr="0x83" mapto="0x03"/>
    <mapped addr="0x84" mapto="0x04"/>
    <register name="OPTION" addr="0x81" reset="11111111"/>
    <register name="TRISA" addr="0x85" reset="11111111"/>
    <mapped addr="0x8A" mapto="0x0A"/>
  </regblock>
  <datablock start="0x0C" end="0x4F"/>
  <port name="PORTA" pins="5" outreg="PORTA" dirreg="!TRISA"/>
</mcu>
"#;

#[test]
fn pic14_drives_porta0_through_resistor() {
    let mut dev = cs_mcu::Device::from_xml("PIC14-1", PIC14_MCU).unwrap();
    // BSF STATUS,RP0; CLRF TRISA; BCF STATUS,RP0; MOVLW 1; MOVWF PORTA; GOTO 5
    dev.load_words(&[0x1683, 0x0185, 0x1283, 0x3001, 0x0085, 0x2805]);
    let mut c = Circuit::new();
    c.add_mcu("PIC14-1", dev)
        .add_resistor("RL", 10_000.0)
        .add_ground("GND")
        .connect("PIC14-1-PORTA0", "RL-lPin")
        .connect("RL-rPin", "GND-Gnd");
    c.solve().unwrap();
    // MCU has not started; pin is still an input.
    assert!(
        c.pin_voltage("PIC14-1-PORTA0").unwrap() < 1.0,
        "idle input should not drive the load"
    );
    // 6 instructions, 1 µs each (GOTO is 2 cycles) → 8 µs is enough.
    c.run_ps(10_000_000).unwrap();
    assert!(
        logic_high(c.pin_voltage("PIC14-1-PORTA0").unwrap()),
        "PORTA0 should be driven high, got {}",
        c.pin_voltage("PIC14-1-PORTA0").unwrap()
    );
    let mcu = c.components.iter().find(|x| x.id == "PIC14-1").unwrap();
    if let Kind::Mcu(m) = &mcu.kind {
        assert_eq!(m.device.w(), 1);
        let pin = m
            .device
            .gpio_pins()
            .find(|p| p.name.ends_with("PORTA0"))
            .unwrap();
        assert!(pin.is_out);
        assert!(pin.out_state);
    } else {
        panic!("expected Kind::Mcu");
    }
    let snap = c.mcu_snap();
    assert_eq!(snap.id, "PIC14-1");
    assert!(snap.has_status);
    assert_eq!(snap.ram[0x05], 1, "PORTA in monitor dump");
    assert_eq!(snap.pc, 5);
    assert!(c.poke_mcu_ram(0x20, 0xAB));
    assert_eq!(c.mcu_snap().ram[0x20], 0xAB);
}

const AVR_MCU: &str = r#"
<mcu core="AVR" data="256" prog="64" progword="2" inst_cycle="1" freq="1000000">
  <datablock start="0" end="0x1F"/>
  <regblock start="0x20" end="0x5F" streg="SREG">
    <register name="PINB" addr="0x36" reset="0"/>
    <register name="DDRB" addr="0x37" reset="0"/>
    <register name="PORTB" addr="0x38" reset="0"/>
    <register name="SPL" addr="0x5D" reset="0"/>
    <register name="SPH" addr="0x5E" reset="0"/>
    <register name="SREG" addr="0x5F" reset="0" bits="C,Z,N,V,S,H,T,I"/>
  </regblock>
  <datablock start="0x60" end="0xFF"/>
  <stack spreg="SPL,SPH" increment="post-dec"/>
  <port name="PORTB" pins="8" outreg="PORTB" dirreg="DDRB" inreg="PINB"/>
</mcu>
"#;

#[test]
fn avr_drives_portb0_through_resistor() {
    let mut dev = cs_mcu::Device::from_xml("AVR-1", AVR_MCU).unwrap();
    // LDI r16,1; OUT DDRB,r16; LDI r16,1; OUT PORTB,r16; RJMP -1
    let ldi_1 = 0xE001u16;
    let out_ddrb = 0xBB07; // OUT 0x17, r16
    let out_portb = 0xBB08; // OUT 0x18, r16
    let rjmp_self = 0xCFFF;
    dev.load_words(&[ldi_1, out_ddrb, ldi_1, out_portb, rjmp_self]);
    let mut c = Circuit::new();
    c.add_mcu("AVR-1", dev)
        .add_resistor("RL", 10_000.0)
        .add_ground("GND")
        .connect("AVR-1-PORTB0", "RL-lPin")
        .connect("RL-rPin", "GND-Gnd");
    c.solve().unwrap();
    assert!(
        c.pin_voltage("AVR-1-PORTB0").unwrap() < 1.0,
        "idle input should not drive the load"
    );
    c.run_ps(10_000_000).unwrap();
    assert!(
        logic_high(c.pin_voltage("AVR-1-PORTB0").unwrap()),
        "PORTB0 should be driven high, got {}",
        c.pin_voltage("AVR-1-PORTB0").unwrap()
    );
    let mcu = c.components.iter().find(|x| x.id == "AVR-1").unwrap();
    if let Kind::Mcu(m) = &mcu.kind {
        let pin = m
            .device
            .gpio_pins()
            .find(|p| p.name.ends_with("PORTB0"))
            .unwrap();
        assert!(pin.is_out);
        assert!(pin.out_state);
    } else {
        panic!("expected Kind::Mcu");
    }
}

const I51_MCU: &str = r#"
<mcu core="8051" data="256" prog="256" progword="1" inst_cycle="12" cpu_cycle="6" freq="12000000">
  <datablock start="0" end="0x7F"/>
  <regblock start="0x80" end="0xFF" streg="PSW">
    <register name="SP" addr="0x81" reset="00000111"/>
    <register name="P1" addr="0x90" reset="11111111"/>
    <register name="PSW" addr="0xD0" reset="0" bits="P,0,OV,RS0,RS1,F0,AC,Cy"/>
    <register name="ACC" addr="0xE0" reset="0"/>
  </regblock>
  <stack spreg="SP" increment="pre-inc"/>
  <port name="PORT1" pins="8" outreg="P1"/>
</mcu>
"#;

#[test]
fn i51_drives_p1_0_through_resistor() {
    let mut dev = cs_mcu::Device::from_xml("I51-1", I51_MCU).unwrap();
    // MOV P1, #0x01; SJMP $
    dev.load_words(&[0x75, 0x90, 0x01, 0x80, 0xFE]);
    let mut c = Circuit::new();
    c.add_mcu("I51-1", dev)
        .add_resistor("RL", 10_000.0)
        .add_ground("GND")
        .connect("I51-1-PORT10", "RL-lPin")
        .connect("RL-rPin", "GND-Gnd");
    c.solve().unwrap();
    c.run_ps(10_000_000).unwrap();
    assert!(
        logic_high(c.pin_voltage("I51-1-PORT10").unwrap()),
        "P1.0 should be driven high, got {}",
        c.pin_voltage("I51-1-PORT10").unwrap()
    );
    let mcu = c.components.iter().find(|x| x.id == "I51-1").unwrap();
    if let Kind::Mcu(m) = &mcu.kind {
        let pin = m
            .device
            .gpio_pins()
            .find(|p| p.name.ends_with("PORT10"))
            .unwrap();
        assert!(pin.is_out);
        assert!(pin.out_state);
    } else {
        panic!("expected Kind::Mcu");
    }
}

#[test]
fn i51_movx_xram_in_circuit() {
    let mut dev = cs_mcu::Device::from_xml("I51-1", I51_MCU).unwrap();
    // MOV A,#0x3C; MOV R0,#0x20; MOVX @R0,A; SJMP $
    // I51_MCU has no DPTR; @R0 is enough.
    dev.load_words(&[0x74, 0x3C, 0x78, 0x20, 0xF2, 0x80, 0xFE]);
    let mut c = Circuit::new();
    c.add_mcu("I51-1", dev)
        .add_resistor("RL", 10_000.0)
        .add_ground("GND")
        .connect("I51-1-PORT10", "RL-lPin")
        .connect("RL-rPin", "GND-Gnd");
    c.solve().unwrap();
    c.run_ps(20_000_000).unwrap();
    let mcu = c.components.iter().find(|x| x.id == "I51-1").unwrap();
    if let Kind::Mcu(m) = &mcu.kind {
        assert_eq!(m.device.xram()[0x20], 0x3C);
        assert_eq!(m.device.w(), 0x3C);
    } else {
        panic!("expected Kind::Mcu");
    }
}

const MCS65_MCU: &str = r#"
<mcu core="6502" data="16" prog="65536" progword="1" inst_cycle="1" freq="1000000">
  <regblock start="0" end="0">
    <register name="P1" addr="0x00" reset="0"/>
  </regblock>
  <port name="PORT1" pins="1" outreg="P1"/>
  <progblock>
    <progval addr="0xFFFC" value="0"/>
    <progval addr="0xFFFD" value="0"/>
  </progblock>
</mcu>
"#;

#[test]
fn mcs65_lda_imm_in_circuit() {
    let mut dev = cs_mcu::Device::from_xml("MCS65-1", MCS65_MCU).unwrap();
    dev.load_words(&[0xA9, 0x55]); // LDA #$55
    let mut c = Circuit::new();
    c.add_mcu("MCS65-1", dev)
        .add_resistor("RL", 10_000.0)
        .add_ground("GND")
        .connect("MCS65-1-PORT10", "RL-lPin")
        .connect("RL-rPin", "GND-Gnd");
    c.solve().unwrap();
    c.run_ps(80_000_000).unwrap();
    let mcu = c.components.iter().find(|x| x.id == "MCS65-1").unwrap();
    if let Kind::Mcu(m) = &mcu.kind {
        assert_eq!(m.device.w(), 0x55);
    } else {
        panic!("expected Kind::Mcu");
    }
}

const Z80_MCU: &str = r#"
<mcu core="Z80" data="16" prog="65536" progword="1" inst_cycle="1" freq="1000000">
  <regblock start="0" end="0">
    <register name="P1" addr="0x00" reset="0"/>
  </regblock>
  <port name="PORT1" pins="1" outreg="P1"/>
</mcu>
"#;

#[test]
fn z80_ld_a_imm_in_circuit() {
    let mut dev = cs_mcu::Device::from_xml("Z80-1", Z80_MCU).unwrap();
    dev.load_words(&[0x3E, 0x55]); // LD A,0x55
    let mut c = Circuit::new();
    c.add_mcu("Z80-1", dev)
        .add_resistor("RL", 10_000.0)
        .add_ground("GND")
        .connect("Z80-1-PORT10", "RL-lPin")
        .connect("RL-rPin", "GND-Gnd");
    c.solve().unwrap();
    c.run_ps(80_000_000).unwrap();
    let mcu = c.components.iter().find(|x| x.id == "Z80-1").unwrap();
    if let Kind::Mcu(m) = &mcu.kind {
        assert_eq!(m.device.w(), 0x55);
    } else {
        panic!("expected Kind::Mcu");
    }
}

const MCS65_BUS_MCU: &str = r#"
<mcu core="6502" data="1" prog="65536" progword="1" inst_cycle="1" freq="1000000">
  <progblock>
    <progval addr="0xFFFC" value="0"/>
    <progval addr="0xFFFD" value="0"/>
  </progblock>
  <ioport name="PORTA" pins="16"/>
  <ioport name="PORTD" pins="8"/>
  <ioport name="CPORT0" pins="RW,P0,P1,P2,SYNC,IRQ,NMI,RDY,SO"/>
</mcu>
"#;

#[test]
fn mcs65_sta_drives_data_bus() {
    let mut dev = cs_mcu::Device::from_xml("MCS65-1", MCS65_BUS_MCU).unwrap();
    // LDA #$55; STA $10; JMP $
    dev.load_words(&[0xA9, 0x55, 0x85, 0x10, 0x4C, 0x04, 0x00]);
    let mut c = Circuit::new();
    c.add_mcu("MCS65-1", dev)
        .add_resistor("RL", 10_000.0)
        .add_ground("GND")
        .connect("MCS65-1-CPORT0RW", "RL-lPin")
        .connect("RL-rPin", "GND-Gnd");
    c.solve().unwrap();
    c.run_ps(80_000_000).unwrap();
    let mcu = c.components.iter().find(|x| x.id == "MCS65-1").unwrap();
    if let Kind::Mcu(m) = &mcu.kind {
        assert_eq!(m.device.w(), 0x55);
        let rw = m.device.gpio_pins().find(|p| p.label == "RW").unwrap();
        assert!(rw.is_out, "RW stays an output");
        assert!(rw.out_state, "idle read after JMP $");
        let a0 = m
            .device
            .gpio_pins()
            .find(|p| p.name.ends_with("PORTA0"))
            .unwrap();
        assert!(a0.is_out, "PORTA is the address bus");
    } else {
        panic!("expected Kind::Mcu");
    }
    let v = c.pin_voltage("MCS65-1-CPORT0RW").unwrap();
    assert!(v > 2.5, "RW high (read) should drive the load, got {v}");
}

const Z80_BUS_MCU: &str = r#"
<mcu core="Z80" data="1" prog="65536" progword="1" inst_cycle="1" freq="1000000">
  <ioport name="PORTA" pins="16"/>
  <ioport name="PORTD" pins="8"/>
  <ioport name="CPORT0" pins="M1,MREQ,IORQ,RD,WR,RFSH,HALT,WAIT,INT,NMI,BUSRQ,BUSAK,RESET"/>
</mcu>
"#;

#[test]
fn z80_mreq_exists_on_netlist() {
    let mut dev = cs_mcu::Device::from_xml("Z80-1", Z80_BUS_MCU).unwrap();
    dev.load_words(&[0x3E, 0x55]); // LD A,0x55
    let mut c = Circuit::new();
    c.add_mcu("Z80-1", dev)
        .add_resistor("RL", 10_000.0)
        .add_ground("GND")
        .connect("Z80-1-CPORT0MREQ", "RL-lPin")
        .connect("RL-rPin", "GND-Gnd");
    c.solve().unwrap();
    c.run_ps(80_000_000).unwrap();
    let mcu = c.components.iter().find(|x| x.id == "Z80-1").unwrap();
    if let Kind::Mcu(m) = &mcu.kind {
        assert_eq!(m.device.w(), 0x55);
        let mreq = m.device.gpio_pins().find(|p| p.label == "MREQ").unwrap();
        assert!(mreq.is_out, "MREQ stays an output");
    } else {
        panic!("expected Kind::Mcu");
    }
    assert!(
        c.pin_voltage("Z80-1-CPORT0MREQ").is_some(),
        "MREQ is on the analog netlist"
    );
}

#[test]
fn esp32_gpio_w1ts_drives_load() {
    use crate::qemu::{ESP32_GPIO_START, QemuComp};
    use cs_qemu::SimAction;
    let mut c = Circuit::new();
    c.add_qemu_esp32("Esp32-1")
        .add_resistor("RL", 10_000.0)
        .add_ground("GND")
        .connect("Esp32-1-G00", "RL-lPin")
        .connect("RL-rPin", "GND-Gnd");
    c.solve().unwrap();
    assert!(
        c.pin_voltage("Esp32-1-G00").unwrap() < 1.0,
        "idle GPIO is an input"
    );
    {
        let q = c.qemu_mut("Esp32-1").unwrap();
        q.post(SimAction::Write, ESP32_GPIO_START + 0x24, 1, 1);
        q.process_pending();
        q.post(SimAction::Write, ESP32_GPIO_START + 0x08, 1, 1);
        q.process_pending();
    }
    // `solve()` would `reset_device_stamps`; `run_ps` restamps without wiping GPIO.
    c.run_ps(1).unwrap();
    let v = c.pin_voltage("Esp32-1-G00").unwrap();
    assert!(v > 2.5, "GPIO0 should be driven high, got {v}");
    let q = c.components.iter().find(|x| x.id == "Esp32-1").unwrap();
    if let Kind::QemuDevice(q) = &q.kind {
        assert_eq!(q.pins[0].mode, PinMode::Output);
        assert!(q.pins[0].get_out_state());
    } else {
        panic!("expected Kind::QemuDevice");
    }
    let _ = QemuComp::esp32("x");
}

#[test]
fn stm32_odr_drives_pa0() {
    use crate::qemu::STM32_GPIOA_START;
    use cs_qemu::SimAction;
    let mut c = Circuit::new();
    c.add_qemu_stm32("STM32-1", 1)
        .add_resistor("RL", 10_000.0)
        .add_ground("GND")
        .connect("STM32-1-PA0", "RL-lPin")
        .connect("RL-rPin", "GND-Gnd");
    c.solve().unwrap();
    {
        let q = c.qemu_mut("STM32-1").unwrap();
        // CRL: pin 0 MODE=11 CNF=00 → 0x3 (50 MHz push-pull output)
        q.post(SimAction::Write, STM32_GPIOA_START, 0x3, 1);
        q.process_pending();
        q.post(SimAction::Write, STM32_GPIOA_START + 0x0C, 1, 1);
        q.process_pending();
    }
    c.run_ps(1).unwrap();
    let v = c.pin_voltage("STM32-1-PA0").unwrap();
    assert!(v > 2.5, "PA0 should be driven high, got {v}");
}

#[test]
fn script_cpu_drives_out_pin() {
    use crate::script::ScriptCpu;
    let mut cpu = ScriptCpu::new("Script-1");
    cpu.pins.push(crate::digital::IoPin::input("Script-1-OUT"));
    cpu.set_script(
        r#"
            void reset() {
                IoPin@ p = component.getPin("OUT");
                p.setPinMode(3);
                p.setOutState(true);
            }
            "#,
    );
    let mut c = Circuit::new();
    c.add_script_cpu("Script-1", cpu)
        .add_resistor("RL", 10_000.0)
        .add_ground("GND")
        .connect("Script-1-OUT", "RL-lPin")
        .connect("RL-rPin", "GND-Gnd");
    c.solve().unwrap();
    let v = c.pin_voltage("Script-1-OUT").unwrap();
    assert!(v > 4.0, "script pin should be driven high, got {v}");
}

#[test]
fn sim1_scripted_mcu_xml_usart_spi_twi() {
    let xml = r#"
<mcu core="scripted" script="cpu.as" data="256" prog="64" inst_cycle="1" freq="16000000">
  <ioport name="P" pins="TX,RX,MOSI,MISO,SCK,SS,SDA,SCL"/>
  <usart name="UART0" number="0">
    <trunit type="tx" pin="TX"/>
    <trunit type="rx" pin="RX"/>
  </usart>
  <spi name="SPI0" pins="MOSI,MISO,SCK,SS"/>
  <twi name="TWI0" pins="SDA,SCL"/>
</mcu>
"#;
    let as_src = r#"
            void reset() {
                UART0.setBaudRate(1000000);
                UART0.setDataBits(8);
                UART0.sendByte(0x01);
                SPI0.setMode(1);
                TWI0.setAddress(0x42);
            }
        "#;
    let search = SubcSearch::default()
        .with_memory_mcu("scriptuart", xml)
        .with_memory_script("cpu.as", as_src);
    let src = r#"<circuit version="2.0.0" >
<item itemtype="MCU" CircId="scriptuart-1" Frequency="16 MHz" />
<item itemtype="Resistor" CircId="Resistor-1" Resistance="10 kΩ" />
<item itemtype="Ground" CircId="Ground-1" />
<item itemtype="Connector" uid="c1" startpinid="scriptuart-1-PTX" endpinid="Resistor-1-lPin" />
<item itemtype="Connector" uid="c2" startpinid="Resistor-1-rPin" endpinid="Ground-1-Gnd" />
</circuit>
"#;
    let mut c = Circuit::from_sim1_with(src, &search).unwrap();
    assert!(
        c.skipped().is_empty(),
        "scripted MCU skipped: {:?}",
        c.skipped()
    );
    match &c.components()[0].kind {
        Kind::ScriptCpu(s) => {
            assert_eq!(s.usarts.len(), 1);
            assert_eq!(s.spis.len(), 1);
            assert_eq!(s.twis.len(), 1);
        }
        other => panic!("expected ScriptCpu, got {other:?}"),
    }
    c.solve().unwrap();
    match &c.components()[0].kind {
        Kind::ScriptCpu(s) => {
            assert_eq!(s.spis[0].module.mode, crate::digital::SpiMode::Master);
            assert_eq!(s.twis[0].module.address, 0x42);
        }
        _ => panic!("expected ScriptCpu after solve"),
    }
    let v = c.pin_voltage("scriptuart-1-PTX").expect("TX pin");
    assert!(v < 1.0, "UART start bit should be low, got {v}");
}

#[test]
fn sim1_mcu_from_memory_pgm() {
    let search = SubcSearch::default().with_memory_mcu("pic14test", PIC14_MCU);
    let src = r#"<circuit version="2.0.0" >
<item itemtype="MCU" CircId="pic14test-1" Frequency="4 MHz" savePGM="true" pgm="5763,389,4739,12289,133,10245," />
<item itemtype="Resistor" CircId="Resistor-1" Resistance="10 kΩ" />
<item itemtype="Ground" CircId="Ground-1" />
<item itemtype="Connector" uid="c1" startpinid="pic14test-1-PORTA0" endpinid="Resistor-1-lPin" />
<item itemtype="Connector" uid="c2" startpinid="Resistor-1-rPin" endpinid="Ground-1-Gnd" />
</circuit>
"#;
    let mut c = Circuit::from_sim1_with(src, &search).unwrap();
    assert!(
        !c.skipped().iter().any(|s| s.starts_with("MCU")),
        "skipped {:?}",
        c.skipped()
    );
    let mcu = c.components.iter().find(|x| x.id == "pic14test-1").unwrap();
    if let Kind::Mcu(m) = &mcu.kind {
        assert!((m.device.freq - 4e6).abs() < 1.0);
        assert_eq!(m.device.flash_word(0), Some(0x1683));
        assert_eq!(m.device.flash_word(4), Some(0x0085));
    } else {
        panic!("expected Kind::Mcu");
    }
    c.solve().unwrap();
    c.run_ps(10_000_000).unwrap();
    assert!(
        logic_high(c.pin_voltage("pic14test-1-PORTA0").unwrap()),
        "PORTA0 should be driven high, got {}",
        c.pin_voltage("pic14test-1-PORTA0").unwrap()
    );
}

#[test]
fn sim1_mcu_missing_is_skipped() {
    let src = r#"<circuit version="2.0.0" >
<item itemtype="MCU" CircId="nope-1" />
</circuit>
"#;
    let c = Circuit::from_sim1(src).unwrap();
    assert!(
        c.skipped().iter().any(|s| s.starts_with("MCU:nope-1")),
        "skipped {:?}",
        c.skipped()
    );
    assert_eq!(c.component_count(), 0);
}

#[test]
fn sim1_mcu_hex_from_circuit_dir() {
    let dir = std::env::temp_dir().join(format!("cs-mcu-sim1-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("pic14test")).unwrap();
    std::fs::write(dir.join("pic14test").join("pic14test.mcu"), PIC14_MCU).unwrap();
    let words = [0x1683u16, 0x0185, 0x1283, 0x3001, 0x0085, 0x2805];
    std::fs::write(dir.join("porta.hex"), cs_mcu::encode_hex(0, &words, 16)).unwrap();
    let sim1 = r#"<circuit version="2.0.0" >
<item itemtype="MCU" CircId="pic14test-1" Frequency="4 MHz" Program="porta.hex" Auto_Load="true" />
<item itemtype="Resistor" CircId="Resistor-1" Resistance="10 kΩ" />
<item itemtype="Ground" CircId="Ground-1" />
<item itemtype="Connector" uid="c1" startpinid="pic14test-1-PORTA0" endpinid="Resistor-1-lPin" />
<item itemtype="Connector" uid="c2" startpinid="Resistor-1-rPin" endpinid="Ground-1-Gnd" />
</circuit>
"#;
    let path = dir.join("porta.sim1");
    std::fs::write(&path, sim1).unwrap();
    let mut c = Circuit::load_sim1_path(&path).unwrap();
    assert!(
        !c.skipped().iter().any(|s| s.starts_with("MCU")),
        "skipped {:?}",
        c.skipped()
    );
    c.solve().unwrap();
    c.run_ps(10_000_000).unwrap();
    assert!(
        logic_high(c.pin_voltage("pic14test-1-PORTA0").unwrap()),
        "PORTA0 should be driven high, got {}",
        c.pin_voltage("pic14test-1-PORTA0").unwrap()
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn sim1_mcu_from_catalog_xml() {
    let dir = std::env::temp_dir().join(format!("cs-circ-mcu-cat-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("AVR")).unwrap();
    std::fs::write(
        dir.join("avr.xml"),
        r#"<itemlib>
            <itemset category="AVR" type="MCU">
                <item name="tiny13" data="AVR/tiny13" />
            </itemset>
            </itemlib>"#,
    )
    .unwrap();
    std::fs::write(dir.join("AVR").join("tiny13.mcu"), PIC14_MCU).unwrap();
    let mut catalog = crate::catalog::Catalog::new();
    catalog.load_dir(&dir);
    let src = r#"<circuit version="2.0.0" >
<item itemtype="MCU" CircId="tiny13-1" Frequency="4 MHz" />
</circuit>
"#;
    let search = SubcSearch::default().with_catalog(catalog);
    let c = Circuit::from_sim1_with(src, &search).unwrap();
    assert!(
        !c.skipped().iter().any(|s| s.starts_with("MCU")),
        "skipped {:?}",
        c.skipped()
    );
    let mcu = c.components.iter().find(|x| x.id == "tiny13-1").unwrap();
    match &mcu.kind {
        Kind::Mcu(m) => assert_eq!(m.device.gpio_pins().count(), 5),
        _ => panic!("expected Kind::Mcu"),
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn function_output_drives_load_in_one_solve() {
    let mut c = Circuit::new();
    c.add_fixed_volt("VA", 5.0)
        .add_comp(Comp {
            id: "F1".into(),
            kind: Kind::Function(FunctionState::new("F1", 1, "A")),
        })
        .add_comp(Comp {
            id: "F2".into(),
            kind: Kind::Function(FunctionState::new("F2", 1, "A")),
        })
        .add_resistor("R1", 10_000.0)
        .add_resistor("R2", 10_000.0)
        .add_ground("GND")
        .connect("VA-outnod", "F1-in0")
        .connect("F1-out0", "F2-in0")
        .connect("F1-out0", "R1-lPin")
        .connect("R2-lPin", "F2-out0")
        .connect("R1-rPin", "GND-Gnd")
        .connect("R2-rPin", "GND-Gnd");
    c.solve().unwrap();
    assert!(logic_high(c.pin_voltage("F1-out0").unwrap()));
    assert!(
        logic_high(c.pin_voltage("F2-out0").unwrap()),
        "second stage must restamp inside the same solve"
    );
}

#[test]
fn dac_full_scale_drives_output_node() {
    let mut c = Circuit::new();
    let dac = DacState::new("DAC", 8, 5.0);
    c.add_fixed_volt("VA", 5.0)
        .add_comp(Comp {
            id: "DAC".into(),
            kind: Kind::Dac(dac),
        })
        .add_resistor("RL", 10_000.0)
        .add_ground("GND")
        .connect("DAC-out0", "RL-lPin")
        .connect("RL-rPin", "GND-Gnd");
    for i in 0..8 {
        c.connect("VA-outnod", format!("DAC-in{i}"));
    }
    c.solve().unwrap();
    let v = c.pin_voltage("DAC-out0").unwrap();
    assert!((v - 5.0).abs() < 1e-3, "dac out {v}");
}

#[test]
fn counter_terminal_count_drives_load() {
    let mut c = Circuit::new();
    c.add_fixed_volt("VA", 5.0)
        .add_comp(Comp {
            id: "CNT".into(),
            kind: Kind::Counter(CounterState::new("CNT", 1, 1)),
        })
        .add_resistor("RL", 10_000.0)
        .add_ground("GND")
        .connect("CNT-out0", "RL-lPin")
        .connect("RL-rPin", "GND-Gnd");
    c.solve().unwrap();
    assert!(logic_low(c.pin_voltage("CNT-out0").unwrap()));
    c.connect("VA-outnod", "CNT-in0");
    c.solve().unwrap();
    assert!(logic_high(c.pin_voltage("CNT-out0").unwrap()));
}

#[test]
fn clock_square_wave_toggles_each_half_period() {
    let mut c = Circuit::new();
    c.add_comp(Comp {
        id: "CLK".into(),
        kind: Kind::Clock {
            voltage: 5.0,
            freq_khz: 1.0,
            state: false,
        },
    })
    .add_resistor("RL", 10_000.0)
    .add_ground("GND")
    .connect("CLK-outnod", "RL-lPin")
    .connect("RL-rPin", "GND-Gnd");
    assert!(c.has_time_sources());
    c.solve().unwrap();
    assert!(logic_low(c.pin_voltage("CLK-outnod").unwrap()));
    // 1 kHz: period 1 ms, first edge at 0.5 ms. The level is derived from time,
    // so the first half stays low.
    c.run_ps(500_000_000).unwrap();
    assert!(logic_high(c.pin_voltage("CLK-outnod").unwrap()));
    c.run_ps(500_000_000).unwrap();
    assert!(logic_low(c.pin_voltage("CLK-outnod").unwrap()));
}

fn relay_kind(
    r_coil: f64,
    i_on: f64,
    i_off: f64,
    norm_close: bool,
    double_throw: bool,
    poles: usize,
) -> Kind {
    Kind::Relay {
        norm_close,
        double_throw,
        poles,
        i_on,
        i_off,
        r_coil,
        inductance: 0.0,
        coil_i: 0.0,
        active: false,
    }
}

#[test]
fn relay_pulls_in_from_coil_current_and_holds() {
    let mut c = Circuit::new();
    c.add_fixed_volt("VC", 1.0)
        .add_fixed_volt("VS", 5.0)
        .add_comp(Comp {
            id: "RY".into(),
            kind: relay_kind(50.0, 0.02, 0.01, false, false, 1),
        })
        .add_resistor("RL", 1_000.0)
        .add_ground("GND")
        .connect("VC-outnod", "RY-lPin")
        .connect("RY-rPin", "GND-Gnd")
        .connect("VS-outnod", "RY-c1")
        .connect("RY-c2", "RL-lPin")
        .connect("RL-rPin", "GND-Gnd");
    // 1 V / 50 Ω = 20 mA, which is i_on. A 100 Ω coil would stay open.
    c.solve().unwrap();
    let closed = c.pin_voltage("RL-lPin").unwrap();
    assert!(closed > 4.5, "contact should close, got {closed}");

    if let Kind::FixedVolt { voltage } = &mut c.comp_mut("VC").unwrap().kind {
        *voltage = 0.75;
    }
    // 0.75 V / 50 Ω = 15 mA, between i_off and i_on.
    c.re_solve().unwrap();
    let held = c.pin_voltage("RL-lPin").unwrap();
    assert!(held > 4.5, "hysteresis should hold the contact, got {held}");

    if let Kind::FixedVolt { voltage } = &mut c.comp_mut("VC").unwrap().kind {
        *voltage = 0.2;
    }
    c.re_solve().unwrap();
    let open = c.pin_voltage("RL-lPin").unwrap();
    assert!(open.abs() < 0.05, "contact should drop out, got {open}");
}

#[test]
fn relay_inductance_delays_pull_in_until_the_coil_current_rises() {
    let mut c = Circuit::new();
    c.add_fixed_volt("VC", 5.0)
        .add_fixed_volt("VS", 5.0)
        .add_comp(Comp {
            id: "RY".into(),
            kind: Kind::Relay {
                norm_close: false,
                double_throw: false,
                poles: 1,
                i_on: 0.02,
                i_off: 0.01,
                r_coil: 100.0,
                inductance: 0.1,
                coil_i: 0.0,
                active: false,
            },
        })
        .add_resistor("RL", 1_000.0)
        .add_ground("GND")
        .connect("VC-outnod", "RY-lPin")
        .connect("RY-rPin", "GND-Gnd")
        .connect("VS-outnod", "RY-c1")
        .connect("RY-c2", "RL-lPin")
        .connect("RL-rPin", "GND-Gnd");
    c.solve().unwrap();
    assert!(
        c.pin_voltage("RL-lPin").unwrap().abs() < 0.05,
        "a 0.1 H coil should not pull in on the first DC solve"
    );
    // tau = L/R = 1 ms. 20 mA is about half a time constant of the 50 mA steady current.
    c.run_ps(2_000_000_000).unwrap();
    let closed = c.pin_voltage("RL-lPin").unwrap();
    assert!(
        closed > 4.5,
        "coil current should pull in by 2 ms, got {closed}"
    );
}

#[test]
fn relay_normally_closed_and_second_pole() {
    let mut c = Circuit::new();
    c.add_fixed_volt("VS", 5.0)
        .add_comp(Comp {
            id: "RY".into(),
            kind: relay_kind(100.0, 0.02, 0.01, true, false, 2),
        })
        .add_resistor("RL0", 1_000.0)
        .add_resistor("RL1", 1_000.0)
        .add_ground("GND")
        .connect("VS-outnod", "RY-c1")
        .connect("VS-outnod", "RY-c1p1")
        .connect("RY-c2", "RL0-lPin")
        .connect("RY-c2p1", "RL1-lPin")
        .connect("RL0-rPin", "GND-Gnd")
        .connect("RL1-rPin", "GND-Gnd");
    c.solve().unwrap();
    assert!(c.pin_voltage("RL0-lPin").unwrap() > 4.5);
    assert!(c.pin_voltage("RL1-lPin").unwrap() > 4.5);

    c.add_fixed_volt("VC", 5.0)
        .connect("VC-outnod", "RY-lPin")
        .connect("RY-rPin", "GND-Gnd");
    c.solve().unwrap();
    assert!(
        c.pin_voltage("RL0-lPin").unwrap().abs() < 0.05,
        "coil current should open a normally-closed contact"
    );
    assert!(c.pin_voltage("RL1-lPin").unwrap().abs() < 0.05);
}

#[test]
fn relay_double_throw_swaps_contacts() {
    let mut c = Circuit::new();
    c.add_fixed_volt("VS", 5.0)
        .add_comp(Comp {
            id: "RY".into(),
            kind: relay_kind(100.0, 0.02, 0.01, false, true, 1),
        })
        .add_resistor("RNO", 1_000.0)
        .add_resistor("RNC", 1_000.0)
        .add_ground("GND")
        .connect("VS-outnod", "RY-c1")
        .connect("RY-c2", "RNO-lPin")
        .connect("RY-c3", "RNC-lPin")
        .connect("RNO-rPin", "GND-Gnd")
        .connect("RNC-rPin", "GND-Gnd");
    c.solve().unwrap();
    assert!(c.pin_voltage("RNO-lPin").unwrap().abs() < 0.05);
    assert!(c.pin_voltage("RNC-lPin").unwrap() > 4.5);

    c.add_fixed_volt("VC", 5.0)
        .connect("VC-outnod", "RY-lPin")
        .connect("RY-rPin", "GND-Gnd");
    c.solve().unwrap();
    assert!(c.pin_voltage("RNO-lPin").unwrap() > 4.5);
    assert!(c.pin_voltage("RNC-lPin").unwrap().abs() < 0.05);
}

#[test]
fn csource_scales_control_voltage_and_current() {
    let mut voltage = Circuit::new();
    voltage
        .add_fixed_volt("VCTRL", 1.5)
        .add_comp(Comp {
            id: "CS".into(),
            kind: Kind::Csource {
                control_pins: true,
                curr_source: false,
                curr_control: false,
                gain: 2.0,
                volt: 9.0,
                current: 1.0,
                ctrl: 0.0,
                sense_ohms: 1.0,
            },
        })
        .add_resistor("RL", 1_000.0)
        .add_ground("GND")
        .connect("VCTRL-outnod", "CS-cpPin")
        .connect("CS-cmPin", "GND-Gnd")
        .connect("CS-s2Pin", "GND-Gnd")
        .connect("CS-s1Pin", "RL-lPin")
        .connect("RL-rPin", "GND-Gnd");
    voltage.solve().unwrap();
    let v = voltage.pin_voltage("CS-s1Pin").unwrap();
    assert!((v - 3.0).abs() < 1e-3, "vcvs out {v}");

    let mut current = Circuit::new();
    current
        .add_fixed_volt("VCTRL", 2.0)
        .add_comp(Comp {
            id: "CS".into(),
            kind: Kind::Csource {
                control_pins: true,
                curr_source: true,
                curr_control: false,
                gain: 0.01,
                volt: 0.0,
                current: 1.0,
                ctrl: 0.0,
                sense_ohms: 1.0,
            },
        })
        .add_resistor("RL", 100.0)
        .add_ground("GND")
        .connect("VCTRL-outnod", "CS-cpPin")
        .connect("CS-cmPin", "GND-Gnd")
        .connect("CS-s2Pin", "GND-Gnd")
        .connect("CS-s1Pin", "RL-lPin")
        .connect("RL-rPin", "GND-Gnd");
    current.solve().unwrap();
    let v = current.pin_voltage("CS-s1Pin").unwrap();
    assert!(
        (v - 2.0).abs() < 1e-3,
        "vccs into 100 Ω should be 2 V, got {v}"
    );

    let mut fixed = Circuit::new();
    fixed
        .add_comp(Comp {
            id: "CS".into(),
            kind: Kind::Csource {
                control_pins: false,
                curr_source: false,
                curr_control: false,
                gain: 10.0,
                volt: 3.3,
                current: 1.0,
                ctrl: 0.0,
                sense_ohms: 1.0,
            },
        })
        .add_resistor("RL", 1_000.0)
        .add_ground("GND")
        .connect("CS-s2Pin", "GND-Gnd")
        .connect("CS-s1Pin", "RL-lPin")
        .connect("RL-rPin", "GND-Gnd");
    fixed.solve().unwrap();
    let v = fixed.pin_voltage("CS-s1Pin").unwrap();
    assert!((v - 3.3).abs() < 1e-3, "fixed source {v}");
}

#[test]
fn csource_current_control_uses_sense_branch() {
    let mut c = Circuit::new();
    c.add_fixed_volt("VCTRL", 5.0)
        .add_resistor("RS", 4.0)
        .add_comp(Comp {
            id: "CS".into(),
            kind: Kind::Csource {
                control_pins: true,
                curr_source: true,
                curr_control: true,
                gain: 0.1,
                volt: 0.0,
                current: 0.0,
                ctrl: 0.0,
                sense_ohms: 1.0,
            },
        })
        .add_resistor("RL", 10.0)
        .add_ground("GND")
        .connect("VCTRL-outnod", "RS-lPin")
        .connect("RS-rPin", "CS-cpPin")
        .connect("CS-cmPin", "GND-Gnd")
        .connect("CS-s2Pin", "GND-Gnd")
        .connect("CS-s1Pin", "RL-lPin")
        .connect("RL-rPin", "GND-Gnd");
    c.solve().unwrap();
    // 5 V across 4 Ω + 1 Ω sense is 1 A. Gain 0.1 A/A into 10 Ω is 1 V.
    let v = c.pin_voltage("CS-s1Pin").unwrap();
    assert!((v - 1.0).abs() < 1e-2, "cccs out {v}");
}

fn rgb_kind() -> Kind {
    Kind::RgbLed {
        common_anode: false,
        leds: [
            LedState::configured(2.4, 0.6, 0.03, false),
            LedState::configured(2.4, 0.6, 0.03, false),
            LedState::configured(2.4, 0.6, 0.03, false),
        ],
    }
}

#[test]
fn led_arrays_stay_dark_until_forward_biased() {
    let mut idle = Circuit::new();
    idle.add_comp(Comp {
        id: "RGB".into(),
        kind: rgb_kind(),
    })
    .add_resistor("R", 10_000.0)
    .add_ground("GND")
    .connect("RGB-cPin", "GND-Gnd")
    .connect("RGB-rPin", "R-lPin")
    .connect("R-rPin", "GND-Gnd");
    idle.solve().unwrap();
    let v = idle.pin_voltage("RGB-rPin").unwrap();
    assert!(v.abs() < 0.05, "unpowered rgb pin injected {v} V");

    let mut forward = Circuit::new();
    forward
        .add_fixed_volt("V", 5.0)
        .add_resistor("R", 220.0)
        .add_comp(Comp {
            id: "RGB".into(),
            kind: rgb_kind(),
        })
        .add_ground("GND")
        .connect("V-outnod", "R-lPin")
        .connect("R-rPin", "RGB-rPin")
        .connect("RGB-cPin", "GND-Gnd");
    forward.solve().unwrap();
    let v = forward.pin_voltage("RGB-rPin").unwrap();
    assert!(
        (2.2..2.8).contains(&v),
        "forward drop should sit near 2.4 V, got {v}"
    );

    let mut bar = Circuit::new();
    bar.add_fixed_volt("V", 5.0)
        .add_resistor("R", 220.0)
        .add_comp(Comp {
            id: "BAR".into(),
            kind: Kind::LedBar {
                segments: 2,
                grounded: true,
                leds: LedState::dies(2, 2.4, 0.6, 0.03, true),
            },
        })
        .add_ground("GND")
        .connect("V-outnod", "R-lPin")
        .connect("R-rPin", "BAR-lPin0")
        .connect("BAR-lPin1", "GND-Gnd");
    bar.solve().unwrap();
    let von = bar.pin_voltage("BAR-lPin0").unwrap();
    let voff = bar.pin_voltage("BAR-lPin1").unwrap();
    assert!((2.2..2.8).contains(&von), "bar segment {von}");
    assert!(voff.abs() < 0.05, "undriven bar segment {voff}");

    let mut seg = Circuit::new();
    seg.add_fixed_volt("V", 5.0)
        .add_resistor("R", 220.0)
        .add_comp(Comp {
            id: "SS".into(),
            kind: Kind::SevenSegment {
                common_anode: false,
                num_displays: 1,
                leds: LedState::dies(seven_segment_die_count(1), 2.4, 0.6, 0.02, false),
            },
        })
        .add_ground("GND")
        .connect("V-outnod", "R-lPin")
        .connect("R-rPin", "SS-a")
        .connect("SS-com", "GND-Gnd")
        .connect("SS-pin_b", "GND-Gnd");
    seg.solve().unwrap();
    let va = seg.pin_voltage("SS-a").unwrap();
    let vb = seg.pin_voltage("SS-pin_b").unwrap();
    assert!((2.2..2.8).contains(&va), "alias segment {va}");
    assert!(vb.abs() < 0.05, "undriven canonical segment {vb}");
}

#[test]
fn stepper_angle_follows_energized_coil() {
    let mut c = Circuit::new();
    c.add_fixed_volt("V", 5.0)
        .add_comp(Comp {
            id: "M".into(),
            kind: Kind::Stepper {
                bipolar: false,
                steps: 32,
                resistance: 100.0,
                angle: 0.0,
            },
        })
        .add_ground("GND")
        .connect("M-PinA1", "GND-Gnd")
        .connect("M-PinA2", "GND-Gnd")
        .connect("M-PinB2", "GND-Gnd")
        .connect("M-PinCo", "GND-Gnd")
        .connect("V-outnod", "M-PinB1");
    c.solve().unwrap();
    let angle = |c: &Circuit| match c.comp("M").unwrap().kind {
        Kind::Stepper { angle, .. } => angle,
        _ => panic!("stepper"),
    };
    assert!(angle(&c).abs() < 1e-6);
    c.run_ps(1_000_000_000).unwrap();
    let mid = angle(&c);
    assert!(
        (1.5..4.0).contains(&mid),
        "1 ms should move part of one step, got {mid}"
    );
    c.run_ps(50_000_000_000).unwrap();
    let settled = angle(&c);
    assert!(
        (10.0..12.5).contains(&settled),
        "B coil should settle one step (11.25°), got {settled}"
    );
}

#[test]
fn lamp_filament_warms_from_cold_resistance() {
    let mut c = Circuit::new();
    c.add_fixed_volt("V", 12.0)
        .add_comp(Comp {
            id: "L".into(),
            kind: Kind::Lamp {
                voltage: 12.0,
                power: 2.4,
                r_cold: 5.0,
                resistance: 5.0,
                heat: 0.0,
                tau: 0.02,
            },
        })
        .add_ground("GND")
        .connect("V-outnod", "L-lPin")
        .connect("L-rPin", "GND-Gnd");
    c.solve().unwrap();
    let resistance = |c: &Circuit| match c.comp("L").unwrap().kind {
        Kind::Lamp { resistance, .. } => resistance,
        _ => panic!("lamp"),
    };
    assert!((resistance(&c) - 5.0).abs() < 1e-6);
    c.dt = 1e-3;
    c.run_ps(200_000_000_000).unwrap();
    let hot = resistance(&c);
    assert!(
        (50.0..65.0).contains(&hot),
        "hot resistance should approach V²/P = 60 Ω, got {hot}"
    );
    c.re_solve().unwrap();
    let i = c.current_out_of_pin("L-lPin").unwrap().abs();
    assert!(i < 0.4, "warm filament current {i}");
}

fn set_volt(c: &mut Circuit, id: &str, volts: f64) {
    if let Kind::FixedVolt { voltage } = &mut c.comp_mut(id).unwrap().kind {
        *voltage = volts;
    }
}

#[test]
fn pcd8544_spi_byte_lands_in_ram() {
    let mut c = Circuit::new();
    c.add_fixed_volt("VR", 5.0)
        .add_fixed_volt("VC", 0.0)
        .add_fixed_volt("VD", 5.0)
        .add_fixed_volt("VS", 0.0)
        .add_fixed_volt("VK", 0.0)
        .add_comp(Comp::pcd8544("LCD", 50, 4))
        .connect("VR-outnod", "LCD-PinRst")
        .connect("VC-outnod", "LCD-PinCs")
        .connect("VD-outnod", "LCD-PinDc")
        .connect("VS-outnod", "LCD-PinSi")
        .connect("VK-outnod", "LCD-PinScl");
    c.solve().unwrap();
    let byte = 0xA5u8;
    for i in (0..8).rev() {
        set_volt(&mut c, "VS", if byte & (1 << i) == 0 { 0.0 } else { 5.0 });
        c.re_solve().unwrap();
        set_volt(&mut c, "VK", 5.0);
        c.re_solve().unwrap();
        set_volt(&mut c, "VK", 0.0);
        c.re_solve().unwrap();
    }
    match &c.comp("LCD").unwrap().kind {
        Kind::Pcd8544(p) => assert_eq!(p.ram[0][0], 0xA5),
        _ => panic!("pcd8544"),
    }
}

#[test]
fn ks0108_parallel_write_lands_in_left_ram() {
    let mut c = Circuit::new();
    c.add_fixed_volt("VR", 5.0)
        .add_fixed_volt("VC1", 5.0)
        .add_fixed_volt("VC2", 0.0)
        .add_fixed_volt("VRW", 0.0)
        .add_fixed_volt("VD", 5.0)
        .add_fixed_volt("VE", 0.0)
        .add_comp(Comp::ks0108("GL", 128, 64))
        .connect("VR-outnod", "GL-PinRst")
        .connect("VC1-outnod", "GL-PinCs1")
        .connect("VC2-outnod", "GL-PinCs2")
        .connect("VRW-outnod", "GL-PinRW")
        .connect("VD-outnod", "GL-PinDC")
        .connect("VE-outnod", "GL-PinEn");
    let byte = 0xA5u8;
    for bit in 0..8 {
        let id = format!("D{bit}");
        c.add_fixed_volt(&id, if byte & (1 << bit) == 0 { 0.0 } else { 5.0 });
        c.connect(format!("{id}-outnod"), format!("GL-dataPin{bit}"));
    }
    c.solve().unwrap();
    set_volt(&mut c, "VE", 5.0);
    c.re_solve().unwrap();
    set_volt(&mut c, "VE", 0.0);
    c.re_solve().unwrap();
    match &c.comp("GL").unwrap().kind {
        Kind::Ks0108(k) => assert_eq!(k.left_ram[0][0], 0xA5),
        _ => panic!("ks0108"),
    }
}

fn i2c_start(c: &mut Circuit) {
    c.solve().unwrap();
    c.re_solve().unwrap();
    set_volt(c, "VSDA", 0.0);
    c.re_solve().unwrap();
}

fn i2c_bit(c: &mut Circuit, one: bool) {
    set_volt(c, "VSCL", 0.0);
    c.re_solve().unwrap();
    set_volt(c, "VSDA", if one { 5.0 } else { 0.0 });
    c.re_solve().unwrap();
    set_volt(c, "VSCL", 5.0);
    c.re_solve().unwrap();
}

fn i2c_byte(c: &mut Circuit, byte: u8, sda_pin: &str) -> f64 {
    for i in (0..8).rev() {
        i2c_bit(c, byte & (1 << i) != 0);
    }
    set_volt(c, "VSCL", 0.0);
    set_volt(c, "VSDA", 5.0);
    c.re_solve().unwrap();
    let ack = c.pin_voltage(sda_pin).unwrap();
    set_volt(c, "VSCL", 5.0);
    c.re_solve().unwrap();
    ack
}

fn i2c_stop(c: &mut Circuit) {
    set_volt(c, "VSCL", 0.0);
    set_volt(c, "VSDA", 0.0);
    c.re_solve().unwrap();
    set_volt(c, "VSCL", 5.0);
    c.re_solve().unwrap();
    set_volt(c, "VSDA", 5.0);
    c.re_solve().unwrap();
}

#[test]
fn aip31068_i2c_writes_a_character_and_acks() {
    let mut c = Circuit::new();
    c.add_fixed_volt("VSCL", 5.0)
        .add_fixed_volt("VSDA", 5.0)
        .add_resistor("RP", 4_700.0)
        .add_comp(Comp::aip31068("LCD", 2, 16))
        .connect("VSCL-outnod", "LCD-PinSCL")
        .connect("VSDA-outnod", "RP-lPin")
        .connect("RP-rPin", "LCD-PinSDA");
    i2c_start(&mut c);
    let ack = i2c_byte(&mut c, 0x7C, "LCD-PinSDA");
    assert!(ack < 0.25, "address ack should pull SDA, got {ack}");
    let _ = i2c_byte(&mut c, 0x40, "LCD-PinSDA");
    let _ = i2c_byte(&mut c, b'A', "LCD-PinSDA");
    i2c_stop(&mut c);
    match &c.comp("LCD").unwrap().kind {
        Kind::Aip31068(a) => assert_eq!(a.hd44780.ddram[0], b'A'),
        _ => panic!("aip31068"),
    }
}

#[test]
fn sh1107_i2c_writes_a_ram_byte() {
    let mut c = Circuit::new();
    c.add_fixed_volt("VSCL", 5.0)
        .add_fixed_volt("VSDA", 5.0)
        .add_resistor("RP", 4_700.0)
        .add_comp(Comp::sh1107("OLED", 128, 128))
        .connect("VSCL-outnod", "OLED-PinSck")
        .connect("VSDA-outnod", "RP-lPin")
        .connect("RP-rPin", "OLED-PinSda");
    i2c_start(&mut c);
    let ack = i2c_byte(&mut c, 0x78, "OLED-PinSda");
    assert!(ack < 0.25, "address ack should pull SDA, got {ack}");
    let _ = i2c_byte(&mut c, 0x40, "OLED-PinSda");
    let _ = i2c_byte(&mut c, 0xA5, "OLED-PinSda");
    i2c_stop(&mut c);
    match &c.comp("OLED").unwrap().kind {
        Kind::Sh1107(s) => assert_eq!(s.ddram[32][0], 0xA5),
        _ => panic!("sh1107"),
    }
}

#[test]
fn i2c_to_parallel_write_drives_the_port_pins() {
    let mut c = Circuit::new();
    c.add_fixed_volt("VSCL", 5.0)
        .add_fixed_volt("VSDA", 5.0)
        .add_resistor("RP", 4_700.0)
        .add_resistor("R0", 10_000.0)
        .add_resistor("R4", 10_000.0)
        .add_ground("GND")
        .add_comp(Comp {
            id: "IO".to_string(),
            kind: Kind::I2CToParallel(crate::digital::I2CToParallelState::new("IO", 0x50)),
        })
        .connect("VSCL-outnod", "IO-in1")
        .connect("VSDA-outnod", "RP-lPin")
        .connect("RP-rPin", "IO-in0")
        .connect("IO-out0", "R0-lPin")
        .connect("R0-rPin", "GND-Gnd")
        .connect("IO-out4", "R4-lPin")
        .connect("R4-rPin", "GND-Gnd");
    i2c_start(&mut c);
    let ack = i2c_byte(&mut c, 0xA0, "IO-in0");
    assert!(ack < 0.25, "address ack should pull SDA, got {ack}");
    let _ = i2c_byte(&mut c, 0x0F, "IO-in0");
    i2c_stop(&mut c);
    let low = c.pin_voltage("IO-out4").unwrap();
    let high = c.pin_voltage("IO-out0").unwrap();
    let latch = match &c.comp("IO").unwrap().kind {
        Kind::I2CToParallel(p) => p.out_latch,
        _ => 0,
    };
    assert_eq!(latch, 0x0F, "written byte should land in the port latch");
    assert!(low < 0.5, "bit 4 should be driven low, got {low}");
    assert!(high > 4.0, "bit 0 should be driven high, got {high}");
}

#[test]
fn max72xx_latches_a_digit_and_ws2812_stores_a_pixel() {
    let mut mx = Circuit::new();
    mx.add_fixed_volt("VD", 0.0)
        .add_fixed_volt("VK", 0.0)
        .add_fixed_volt("VC", 5.0)
        .add_comp(Comp::max72xx("MX", 1, "Yellow"))
        .add_ground("GND")
        .connect("VD-outnod", "MX-din")
        .connect("VK-outnod", "MX-clk")
        .connect("VC-outnod", "MX-cs")
        .connect("MX-gnd", "GND-Gnd");
    mx.solve().unwrap();
    set_volt(&mut mx, "VC", 0.0);
    mx.re_solve().unwrap();
    let word = 0x01A5u16;
    for i in (0..16).rev() {
        set_volt(&mut mx, "VD", if word & (1 << i) == 0 { 0.0 } else { 5.0 });
        mx.re_solve().unwrap();
        set_volt(&mut mx, "VK", 5.0);
        mx.re_solve().unwrap();
        set_volt(&mut mx, "VK", 0.0);
        mx.re_solve().unwrap();
    }
    set_volt(&mut mx, "VC", 5.0);
    mx.re_solve().unwrap();
    match &mx.comp("MX").unwrap().kind {
        Kind::Max72xx { fb, .. } => {
            assert_eq!(fb.digits[0][0], 0xA5);
            assert!(fb.hex().starts_with("A5"));
        }
        _ => panic!("max72xx"),
    }

    let mut px = Circuit::new();
    px.add_fixed_volt("VD", 0.0)
        .add_resistor("RL", 10_000.0)
        .add_comp(Comp::ws2812("PX", 1, 1, 1, 50_000, 400, 850, 850, 400))
        .add_ground("GND")
        .connect("VD-outnod", "PX-din")
        .connect("PX-gnd", "GND-Gnd")
        .connect("PX-dout", "RL-lPin")
        .connect("RL-rPin", "GND-Gnd");
    px.run_ps(1).unwrap();
    let bits = [false; 8].into_iter().chain([true; 8]).chain([false; 8]);
    for bit in bits {
        set_volt(&mut px, "VD", 5.0);
        px.update().unwrap();
        px.circ_time += if bit { 800_000 } else { 400_000 };
        set_volt(&mut px, "VD", 0.0);
        px.update().unwrap();
        px.circ_time += 200_000;
    }
    match &px.comp("PX").unwrap().kind {
        Kind::Ws2812 { fb, .. } => assert_eq!(fb.pixels[0], [255, 0, 0]),
        _ => panic!("ws2812"),
    }
    set_volt(&mut px, "VD", 5.0);
    px.update().unwrap();
    px.circ_time += 800_000;
    set_volt(&mut px, "VD", 0.0);
    px.update().unwrap();
    assert!(
        px.pin_voltage("PX-dout").unwrap() > 4.0,
        "forwarded 1 should drive DOUT"
    );
    px.run_ps(60_000_000).unwrap();
    match &px.comp("PX").unwrap().kind {
        Kind::Ws2812 { fb, .. } => {
            assert_eq!(fb.bit_index, 0);
            assert_eq!(fb.pixels[0], [255, 0, 0]);
        }
        _ => panic!("ws2812"),
    }
}

#[test]
fn sr04_echo_width_follows_slider_and_in_pin() {
    let mut c = Circuit::new();
    c.add_fixed_volt("VCC", 5.0)
        .add_fixed_volt("VT", 0.0)
        .add_fixed_volt("VI", 0.10)
        .add_resistor("RL", 10_000.0)
        .add_comp(Comp::sr04("SR", 0.5, true))
        .add_ground("GND")
        .connect("VCC-outnod", "SR-vccpin")
        .connect("VT-outnod", "SR-trigpin")
        .connect("VI-outnod", "SR-inpin")
        .connect("SR-gndpin", "GND-Gnd")
        .connect("SR-outpin", "RL-lPin")
        .connect("RL-rPin", "GND-Gnd");
    c.run_ps(1).unwrap();
    set_volt(&mut c, "VT", 5.0);
    c.update().unwrap();
    assert!(c.pin_voltage("SR-outpin").unwrap() > 4.5);
    let half = (0.5_f64 / 343.0 * 1e12).round() as u64;
    c.run_ps(half).unwrap();
    assert!(c.pin_voltage("SR-outpin").unwrap() > 4.5, "echo still high");
    c.run_ps(half.saturating_mul(3)).unwrap();
    assert!(c.pin_voltage("SR-outpin").unwrap() < 0.1, "echo should end");

    if let Kind::Sr04 { use_slider, .. } = &mut c.comp_mut("SR").unwrap().kind {
        *use_slider = false;
    }
    set_volt(&mut c, "VT", 0.0);
    c.update().unwrap();
    set_volt(&mut c, "VT", 5.0);
    c.update().unwrap();
    let short = (0.1_f64 / 343.0 * 1e12).round() as u64;
    c.run_ps(short).unwrap();
    assert!(c.pin_voltage("SR-outpin").unwrap() > 4.5);
    c.run_ps(short.saturating_mul(3)).unwrap();
    assert!(c.pin_voltage("SR-outpin").unwrap() < 0.1);
}

#[test]
fn dht22_drives_the_sensirion_frame() {
    let mut c = Circuit::new();
    c.add_fixed_volt("VH", 5.0)
        .add_resistor("RP", 4_700.0)
        .add_comp(Comp::dht22("DH", "DHT22", 22.5, 68.5))
        .connect("VH-outnod", "RP-lPin")
        .connect("RP-rPin", "DH-inPin");
    c.run_ps(1).unwrap();
    set_volt(&mut c, "VH", 0.0);
    c.update().unwrap();
    c.run_ps(1_500_000_000).unwrap();
    set_volt(&mut c, "VH", 5.0);
    c.update().unwrap();
    match &c.comp("DH").unwrap().kind {
        Kind::Dht22 { link, .. } => {
            assert_eq!(link.frame, crate::digital::dht_frame("DHT22", 22.5, 68.5));
        }
        _ => panic!("dht"),
    }
    c.run_ps(40_000_000).unwrap();
    assert!(
        c.pin_voltage("DH-inPin").unwrap() < 0.2,
        "response starts low"
    );
    c.run_ps(20_000_000_000).unwrap();
    match &c.comp("DH").unwrap().kind {
        Kind::Dht22 { pin_driven, .. } => assert!(!pin_driven),
        _ => panic!("dht"),
    }
    assert!(c.pin_voltage("DH-inPin").unwrap() > 4.0);
}

#[test]
fn ds18b20_presence_and_scratchpad_follow_the_one_wire() {
    let mut c = Circuit::new();
    c.add_fixed_volt("VH", 5.0)
        .add_resistor("RP", 4_700.0)
        .add_comp(Comp::ds18b20("OW", "28FF2B450000", 22.0))
        .connect("VH-outnod", "RP-lPin")
        .connect("RP-rPin", "OW-inPin");
    c.run_ps(1).unwrap();

    let hold = |c: &mut Circuit, ps: u64| {
        set_volt(c, "VH", 0.0);
        c.update().unwrap();
        c.run_ps(ps).unwrap();
    };
    let release = |c: &mut Circuit| {
        set_volt(c, "VH", 5.0);
        c.update().unwrap();
    };
    hold(&mut c, 500_000_000);
    release(&mut c);
    c.run_ps(40_000_000).unwrap();
    assert!(c.pin_voltage("OW-inPin").unwrap() < 0.2, "presence pulse");
    c.run_ps(120_000_000).unwrap();
    assert!(c.pin_voltage("OW-inPin").unwrap() > 4.0);

    let write = |c: &mut Circuit, one: bool| {
        hold(c, if one { 5_000_000 } else { 60_000_000 });
        release(c);
        c.run_ps(10_000_000).unwrap();
    };
    for byte in [0xCCu8, 0xBE] {
        for bit in 0..8 {
            write(&mut c, byte & (1 << bit) != 0);
        }
    }
    match &c.comp("OW").unwrap().kind {
        Kind::Ds18b20 { wire, .. } => {
            assert_eq!(wire.scratch[0], 0x60);
            assert_eq!(wire.scratch[1], 0x01);
        }
        _ => panic!("ds18"),
    }
    let mut raw = 0u16;
    for i in 0..16 {
        hold(&mut c, 2_000_000);
        release(&mut c);
        if c.pin_voltage("OW-inPin").unwrap() > 2.0 {
            raw |= 1 << i;
        }
        c.run_ps(60_000_000).unwrap();
    }
    assert_eq!(raw, 0x0160);
}

#[test]
fn ds1621_tout_tracks_the_thresholds() {
    let mut c = Circuit::new();
    c.add_fixed_volt("VDD", 5.0)
        .add_resistor("RL", 10_000.0)
        .add_comp(Comp::ds1621("TS", 22.5))
        .add_ground("GND")
        .connect("VDD-outnod", "TS-PinVdd")
        .connect("TS-PinGnd", "GND-Gnd")
        .connect("TS-outPin0", "RL-lPin")
        .connect("RL-rPin", "GND-Gnd");
    c.solve().unwrap();
    assert!(c.pin_voltage("TS-outPin0").unwrap() < 0.1);
    if let Kind::Ds1621 { temp, .. } = &mut c.comp_mut("TS").unwrap().kind {
        *temp = 31.0;
    }
    c.re_solve().unwrap();
    assert!(c.pin_voltage("TS-outPin0").unwrap() > 4.5);
    if let Kind::Ds1621 { temp, .. } = &mut c.comp_mut("TS").unwrap().kind {
        *temp = 9.0;
    }
    c.re_solve().unwrap();
    assert!(c.pin_voltage("TS-outPin0").unwrap() < 0.1);
}

#[test]
fn ds1307_sqw_toggles_at_its_frequency() {
    let mut c = Circuit::new();
    c.add_resistor("RL", 10_000.0)
        .add_comp(Comp::ds1307("RTC", true))
        .add_ground("GND")
        .connect("RTC-PinSQW", "RL-lPin")
        .connect("RL-rPin", "GND-Gnd");
    if let Kind::Ds1307 { sqw_freq, bus, .. } = &mut c.comp_mut("RTC").unwrap().kind {
        *sqw_freq = 1_000.0;
        assert!(!bus.seeded);
    }
    c.run_ps(1).unwrap();
    assert!(c.pin_voltage("RTC-PinSQW").unwrap() < 0.1);
    match &c.comp("RTC").unwrap().kind {
        Kind::Ds1307 { bus, .. } => assert!(bus.seeded),
        _ => panic!("rtc"),
    }
    c.run_ps(500_000_000).unwrap();
    assert!(c.pin_voltage("RTC-PinSQW").unwrap() > 4.5);
    c.run_ps(500_000_000).unwrap();
    assert!(c.pin_voltage("RTC-PinSQW").unwrap() < 0.1);
}

#[test]
fn esp01_answers_at_with_ok() {
    let mut c = Circuit::new();
    c.add_fixed_volt("VRX", 5.0)
        .add_resistor("RR", 4_700.0)
        .add_resistor("RT", 10_000.0)
        .add_comp(Comp::esp01("ESP", 500_000, false))
        .add_ground("GND")
        .connect("VRX-outnod", "RR-lPin")
        .connect("RR-rPin", "ESP-pin1")
        .connect("ESP-pin0", "RT-lPin")
        .connect("RT-rPin", "GND-Gnd");
    c.run_ps(1).unwrap();
    let idle = c.pin_voltage("ESP-pin0").unwrap();
    assert!((idle - 3.3).abs() < 0.05, "TX idles high, got {idle}");
    let bit = 2_000_000u64;
    for byte in [b'A', b'T', b'\r', b'\n'] {
        set_volt(&mut c, "VRX", 0.0);
        c.update().unwrap();
        for i in 0..8 {
            let one = byte & (1 << i) != 0;
            set_volt(&mut c, "VRX", if one { 5.0 } else { 0.0 });
            c.update().unwrap();
            c.run_ps(if i == 0 { bit + bit / 2 } else { bit }).unwrap();
        }
        set_volt(&mut c, "VRX", 5.0);
        c.update().unwrap();
        c.run_ps(bit).unwrap();
    }
    assert!(c.pin_voltage("ESP-pin0").unwrap() < 0.2, "OK start bit");
    let mut seen = 0u8;
    for i in 0..8 {
        c.run_ps(bit).unwrap();
        if c.pin_voltage("ESP-pin0").unwrap() > 2.0 {
            seen |= 1 << i;
        }
    }
    assert_eq!(seen, b'O');
}

#[test]
fn bcd_to_dec_output_enable_blanks_the_selected_line() {
    fn decoder(active_low: bool) -> Circuit {
        let mut c = Circuit::new();
        c.add_comp(Comp {
            id: "U1".into(),
            kind: Kind::BcdToDec(BcdToDecState::new("U1", false, active_low)),
        })
        .add_resistor("RL", 10_000.0)
        .add_ground("GND")
        .connect("U1-out0", "RL-lPin")
        .connect("RL-rPin", "GND-Gnd");
        c
    }

    let mut enabled = decoder(false);
    enabled.solve().unwrap();
    assert!(
        logic_high(enabled.pin_voltage("U1-out0").unwrap()),
        "unconnected OE stays enabled"
    );
    enabled
        .add_fixed_volt("OE", 5.0)
        .connect("OE-outnod", "U1-in4");
    enabled.solve().unwrap();
    assert!(logic_low(enabled.pin_voltage("U1-out0").unwrap()));

    let mut active_low = decoder(true);
    active_low.solve().unwrap();
    assert!(
        logic_low(active_low.pin_voltage("U1-out0").unwrap()),
        "active-low selected output rests low"
    );
    active_low
        .add_fixed_volt("OE", 5.0)
        .connect("OE-outnod", "U1-in4");
    active_low.solve().unwrap();
    assert!(
        logic_high(active_low.pin_voltage("U1-out0").unwrap()),
        "blanked active-low outputs rest high"
    );
}

#[test]
fn header_and_socket_rows_pass_voltage_and_stay_apart() {
    for (id, kind) in [
        ("H", Kind::Header { pins_count: 2 }),
        ("S", Kind::Socket { pins_count: 2 }),
    ] {
        let mut c = Circuit::new();
        c.add_comp(Comp {
            id: id.into(),
            kind,
        })
        .add_fixed_volt("VA", 5.0)
        .add_fixed_volt("VB", 3.0)
        .add_ground("GND")
        .connect("VA-outnod", format!("{id}-pin0"))
        .connect("VB-outnod", format!("{id}-pin1"))
        .add_resistor("R0", 1_000.0)
        .connect(format!("{id}-pin2"), "R0-lPin")
        .connect("R0-rPin", "GND-Gnd")
        .add_resistor("R1", 1_000.0)
        .connect(format!("{id}-pin3"), "R1-lPin")
        .connect("R1-rPin", "GND-Gnd");
        c.solve().unwrap();
        approx(c.pin_voltage(&format!("{id}-pin2")).unwrap(), 5.0);
        approx(c.pin_voltage(&format!("{id}-pin3")).unwrap(), 3.0);
    }
}

#[test]
fn bus_trunk_is_one_spine_and_lines_stay_apart() {
    let mut c = Circuit::new();
    c.add_comp(Comp {
        id: "B".into(),
        kind: Kind::Bus { width: 2 },
    })
    .add_fixed_volt("VLINE", 5.0)
    .add_fixed_volt("VTRUNK", 2.0)
    .add_ground("GND")
    .connect("VLINE-outnod", "B-ePin1")
    .connect("VTRUNK-outnod", "B-ePin0")
    .add_resistor("RL", 1_000.0)
    .connect("B-ePin2", "RL-lPin")
    .connect("RL-rPin", "GND-Gnd")
    .add_resistor("RT", 1_000.0)
    .connect("B-busPinI", "RT-lPin")
    .connect("RT-rPin", "GND-Gnd");
    c.solve().unwrap();
    approx(c.pin_voltage("B-ePin1").unwrap(), 5.0);
    approx(c.pin_voltage("B-ePin2").unwrap(), 0.0);
    approx(c.pin_voltage("B-ePin0").unwrap(), 2.0);
    approx(c.pin_voltage("B-busPinI").unwrap(), 2.0);
}

#[test]
fn chained_buses_share_overlapping_lines_only() {
    let mut c = Circuit::new();
    c.add_comp(Comp {
        id: "A".into(),
        kind: Kind::Bus { width: 2 },
    })
    .add_comp(Comp {
        id: "B".into(),
        kind: Kind::Bus { width: 2 },
    })
    .add_comp(Comp {
        id: "C".into(),
        kind: Kind::Bus { width: 1 },
    })
    .add_fixed_volt("V0", 5.0)
    .add_fixed_volt("V1", 3.0)
    .add_ground("GND")
    .connect("A-busPinI", "B-ePin0")
    .connect("B-busPinI", "C-ePin0")
    .connect("V0-outnod", "A-ePin1")
    .connect("V1-outnod", "A-ePin2")
    .add_resistor("R1", 1_000.0)
    .connect("C-ePin1", "R1-lPin")
    .connect("R1-rPin", "GND-Gnd")
    .add_resistor("R2", 1_000.0)
    .connect("B-ePin2", "R2-lPin")
    .connect("R2-rPin", "GND-Gnd");
    c.solve().unwrap();
    approx(c.pin_voltage("B-ePin1").unwrap(), 5.0);
    approx(c.pin_voltage("C-ePin1").unwrap(), 5.0);
    approx(c.pin_voltage("A-ePin2").unwrap(), 3.0);
    approx(c.pin_voltage("B-ePin2").unwrap(), 3.0);
    assert!(c.pin_voltage("C-ePin2").is_none());
    approx(c.pin_voltage("A-ePin0").unwrap(), 0.0);
    approx(c.pin_voltage("C-busPinI").unwrap(), 0.0);
}
