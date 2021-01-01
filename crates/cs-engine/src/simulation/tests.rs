//! Circuit simulation unit and integration test suite.

use super::*;
use crate::Error;
use crate::digital::PinMode;
use crate::elements::Kind;
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
fn floating_battery_is_singular() {
    let mut c = Circuit::new();
    c.add_battery("B1", 5.0, 1e-3)
        .add_resistor("R1", 100.0)
        .connect("B1-lPin", "R1-lPin")
        .connect("B1-rPin", "R1-rPin");
    assert!(matches!(c.solve(), Err(Error::Singular)));
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
