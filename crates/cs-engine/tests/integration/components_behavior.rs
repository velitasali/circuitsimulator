//! Comprehensive component behavior, physics, and C++ parity tests.
//!
//! Tests active components (Op-Amps, BJTs, Diodes, Zeners, LEDs, MOSFETs,
//! JFETs, Comparators, Voltage Regulators, SCRs, TRIACs, DIACs, Analog MUX)
//! and passives in classical analog topologies.

use cs_engine::elements::Kind;
use cs_engine::{CERO_DOUB, Circuit, LOW_IMP};

fn approx(actual: f64, expected: f64, tol: f64, context: &str) {
    let diff = (actual - expected).abs();
    assert!(
        diff <= tol,
        "{context}: actual={actual:.4}, expected={expected:.4}, diff={diff:.4} > tol={tol}"
    );
}

// =========================================================================
// 1. OP-AMP TOPOLOGY TESTS
// =========================================================================

#[test]
fn test_opamp_voltage_follower() {
    // Unity gain buffer: Vout = Vin
    for &vin in &[0.5, 1.2, 2.5, 3.8, 4.5] {
        let mut c = Circuit::new();
        c.add_fixed_volt("VIN", vin)
            .add_opamp("OA1")
            .add_resistor("RLOAD", 10_000.0)
            .add_ground("GND");
        assert!(c.configure_opamp("OA1", 1000.0, LOW_IMP, 5.0, 0.0, false));

        // Connect Vin to non-inverting input, output to inverting input (feedback)
        c.connect("VIN-outnod", "OA1-inputNinv")
            .connect("OA1-output", "OA1-inputInv")
            .connect("OA1-output", "RLOAD-lPin")
            .connect("RLOAD-rPin", "GND-Gnd");

        c.solve().unwrap();
        let vo = c.pin_voltage("OA1-output").unwrap();
        approx(vo, vin, 0.05, &format!("Voltage follower Vin={vin}"));
    }
}

#[test]
fn test_opamp_non_inverting_amplifier() {
    // Vout = Vin * (1 + Rf / R1)
    // Vin = 1.0V, R1 = 2k, Rf = 6k -> Gain = 1 + 6k/2k = 4.0 -> Vout = 4.0V
    let mut c = Circuit::new();
    c.add_fixed_volt("VIN", 1.0)
        .add_opamp("OA1")
        .add_resistor("R1", 2000.0)
        .add_resistor("RF", 6000.0)
        .add_resistor("RLOAD", 10_000.0)
        .add_ground("GND");
    assert!(c.configure_opamp("OA1", 1000.0, LOW_IMP, 10.0, 0.0, false));

    c.connect("VIN-outnod", "OA1-inputNinv")
        .connect("OA1-inputInv", "R1-lPin")
        .connect("R1-rPin", "GND-Gnd")
        .connect("OA1-inputInv", "RF-lPin")
        .connect("RF-rPin", "OA1-output")
        .connect("OA1-output", "RLOAD-lPin")
        .connect("RLOAD-rPin", "GND-Gnd");

    c.solve().unwrap();
    let vo = c.pin_voltage("OA1-output").unwrap();
    approx(vo, 4.0, 0.05, "Non-inverting amplifier Gain=4");
}

#[test]
fn test_opamp_inverting_amplifier() {
    // Vout = -Vin * (Rf / Rin)
    // Vin = 0.5V, Rin = 1k, Rf = 4k, Dual supply +10V/-10V -> Vout = -2.0V
    let mut c = Circuit::new();
    c.add_fixed_volt("VIN", 0.5)
        .add_opamp("OA1")
        .add_resistor("RIN", 1000.0)
        .add_resistor("RF", 4000.0)
        .add_ground("GND");
    assert!(c.configure_opamp("OA1", 1000.0, LOW_IMP, 10.0, -10.0, false));

    c.connect("VIN-outnod", "RIN-lPin")
        .connect("RIN-rPin", "OA1-inputInv")
        .connect("OA1-inputInv", "RF-lPin")
        .connect("RF-rPin", "OA1-output")
        .connect("OA1-inputNinv", "GND-Gnd");

    c.solve().unwrap();
    let vo = c.pin_voltage("OA1-output").unwrap();
    approx(vo, -2.0, 0.05, "Inverting amplifier Gain=-4");
}

#[test]
fn test_opamp_summing_amplifier() {
    // Inverting summing amp: Vout = -Rf * (V1/R1 + V2/R2)
    // V1 = 0.4V, V2 = 0.6V, R1 = 10k, R2 = 10k, Rf = 20k -> Vout = -20k * (0.4/10k + 0.6/10k) = -2.0V
    let mut c = Circuit::new();
    c.add_fixed_volt("V1", 0.4)
        .add_fixed_volt("V2", 0.6)
        .add_opamp("OA1")
        .add_resistor("R1", 10_000.0)
        .add_resistor("R2", 10_000.0)
        .add_resistor("RF", 20_000.0)
        .add_ground("GND");
    assert!(c.configure_opamp("OA1", 1000.0, LOW_IMP, 10.0, -10.0, false));

    c.connect("V1-outnod", "R1-lPin")
        .connect("V2-outnod", "R2-lPin")
        .connect("R1-rPin", "OA1-inputInv")
        .connect("R2-rPin", "OA1-inputInv")
        .connect("OA1-inputInv", "RF-lPin")
        .connect("RF-rPin", "OA1-output")
        .connect("OA1-inputNinv", "GND-Gnd");

    c.solve().unwrap();
    let vo = c.pin_voltage("OA1-output").unwrap();
    approx(vo, -2.0, 0.05, "Summing amplifier Vout=-2.0V");
}

#[test]
fn test_opamp_differential_amplifier() {
    // Difference amp: Vout = (R2/R1) * (V2 - V1) when bridge resistors match
    // V1 = 1.0V (inv side), V2 = 3.0V (non-inv side), R1=R3=10k, R2=R4=10k -> Gain=1 -> Vout = 2.0V
    let mut c = Circuit::new();
    c.add_fixed_volt("V1", 1.0)
        .add_fixed_volt("V2", 3.0)
        .add_opamp("OA1")
        .add_resistor("R1", 10_000.0)
        .add_resistor("R2", 10_000.0)
        .add_resistor("R3", 10_000.0)
        .add_resistor("R4", 10_000.0)
        .add_ground("GND");
    assert!(c.configure_opamp("OA1", 1000.0, LOW_IMP, 10.0, -10.0, false));

    // V1 to R1 to inverting pin, R2 feedback to output
    c.connect("V1-outnod", "R1-lPin")
        .connect("R1-rPin", "OA1-inputInv")
        .connect("OA1-inputInv", "R2-lPin")
        .connect("R2-rPin", "OA1-output");

    // V2 to R3 to non-inverting pin, R4 to GND
    c.connect("V2-outnod", "R3-lPin")
        .connect("R3-rPin", "OA1-inputNinv")
        .connect("OA1-inputNinv", "R4-lPin")
        .connect("R4-rPin", "GND-Gnd");

    c.solve().unwrap();
    let vo = c.pin_voltage("OA1-output").unwrap();
    approx(vo, 2.0, 0.05, "Differential amplifier Vout=2.0V");
}

#[test]
fn test_opamp_dual_power_rail_clamping() {
    // With power_pins enabled, positive output clips at powerPos and negative clips at powerNeg
    let mut c = Circuit::new();
    c.add_fixed_volt("VPOS", 12.0)
        .add_fixed_volt("VNEG", -12.0)
        .add_fixed_volt("VIN", 5.0) // Driven high, gain=100 -> would be 500V without rails
        .add_opamp("OA1")
        .add_resistor("RLOAD", 10_000.0)
        .add_ground("GND");

    assert!(c.configure_opamp("OA1", 1000.0, LOW_IMP, 12.0, -12.0, true));

    c.connect("VPOS-outnod", "OA1-powerPos")
        .connect("VNEG-outnod", "OA1-powerNeg")
        .connect("VIN-outnod", "OA1-inputNinv")
        .connect("GND-Gnd", "OA1-inputInv")
        .connect("OA1-output", "RLOAD-lPin")
        .connect("RLOAD-rPin", "GND-Gnd");

    c.solve().unwrap();
    let vo = c.pin_voltage("OA1-output").unwrap();
    approx(vo, 12.0, 0.05, "OpAmp clipped at positive supply rail 12V");
}

// =========================================================================
// 2. BJT TRANSISTOR TESTS (NPN & PNP)
// =========================================================================

#[test]
fn test_npn_cutoff_and_saturation() {
    // NPN Switch Circuit: VCC = 5V, RC = 1k, RB = 10k
    // 1. Cutoff: VB = 0V -> IC = 0 -> VC = 5V
    let mut c_off = Circuit::new();
    c_off
        .add_fixed_volt("VCC", 5.0)
        .add_fixed_volt("VIN", 0.0)
        .add_bjt("Q1", false)
        .add_resistor("RC", 1000.0)
        .add_resistor("RB", 10_000.0)
        .add_ground("GND");

    c_off
        .connect("VCC-outnod", "RC-lPin")
        .connect("RC-rPin", "Q1-collector")
        .connect("VIN-outnod", "RB-lPin")
        .connect("RB-rPin", "Q1-base")
        .connect("Q1-emiter", "GND-Gnd");

    c_off.solve().unwrap();
    let vc_off = c_off.pin_voltage("Q1-collector").unwrap();
    approx(vc_off, 5.0, 0.05, "NPN cutoff collector voltage = VCC");

    // 2. Saturation: VB = 5V -> strong base drive -> VC < 0.25V
    let mut c_on = Circuit::new();
    c_on.add_fixed_volt("VCC", 5.0)
        .add_fixed_volt("VIN", 5.0)
        .add_bjt("Q1", false)
        .add_resistor("RC", 1000.0)
        .add_resistor("RB", 10_000.0)
        .add_ground("GND");

    c_on.connect("VCC-outnod", "RC-lPin")
        .connect("RC-rPin", "Q1-collector")
        .connect("VIN-outnod", "RB-lPin")
        .connect("RB-rPin", "Q1-base")
        .connect("Q1-emiter", "GND-Gnd");

    c_on.solve().unwrap();
    let vc_on = c_on.pin_voltage("Q1-collector").unwrap();
    let vb_on = c_on.pin_voltage("Q1-base").unwrap();
    assert!(vc_on < 0.25, "NPN saturated Vce={vc_on:.3} < 0.25V");
    approx(vb_on, 0.7, 0.15, "NPN forward biased Vbe ~ 0.7V");
}

#[test]
fn test_npn_emitter_follower() {
    // Emitter follower (Common Collector): VE = VB - VBE ≈ Vin - 0.7V
    // Vin = 3.3V, RE = 1k, VCC = 10V -> VE ≈ 2.6V
    let mut c = Circuit::new();
    c.add_fixed_volt("VCC", 10.0)
        .add_fixed_volt("VIN", 3.3)
        .add_bjt("Q1", false)
        .add_resistor("RE", 1000.0)
        .add_ground("GND");

    c.connect("VCC-outnod", "Q1-collector")
        .connect("VIN-outnod", "Q1-base")
        .connect("Q1-emiter", "RE-lPin")
        .connect("RE-rPin", "GND-Gnd");

    c.solve().unwrap();
    let ve = c.pin_voltage("Q1-emiter").unwrap();
    approx(
        ve,
        3.3 - 0.65,
        0.15,
        "NPN Emitter Follower Ve ≈ Vin - 0.65V",
    );
}

#[test]
fn test_pnp_high_side_switch() {
    // PNP High-Side Switch: Emitter at VCC = 5V, Collector to load resistor RL = 1k to GND.
    // 1. Off state: Base at 5V (VBE = 0) -> Load voltage ≈ 0V
    let mut c_off = Circuit::new();
    c_off
        .add_fixed_volt("VCC", 5.0)
        .add_fixed_volt("VCTRL", 5.0)
        .add_bjt("Q1", true) // PNP
        .add_resistor("RB", 10_000.0)
        .add_resistor("RL", 1000.0)
        .add_ground("GND");

    c_off
        .connect("VCC-outnod", "Q1-emiter")
        .connect("VCTRL-outnod", "RB-lPin")
        .connect("RB-rPin", "Q1-base")
        .connect("Q1-collector", "RL-lPin")
        .connect("RL-rPin", "GND-Gnd");

    c_off.solve().unwrap();
    let v_load_off = c_off.pin_voltage("RL-lPin").unwrap();
    assert!(v_load_off < 0.1, "PNP off: Vload={v_load_off:.3} < 0.1V");

    // 2. On state: Base pulled low to GND through RB -> PNP conducts -> Load voltage ≈ 5V - VEC,sat ≈ 4.8V
    let mut c_on = Circuit::new();
    c_on.add_fixed_volt("VCC", 5.0)
        .add_fixed_volt("VCTRL", 0.0)
        .add_bjt("Q1", true) // PNP
        .add_resistor("RB", 4700.0)
        .add_resistor("RL", 1000.0)
        .add_ground("GND");

    c_on.connect("VCC-outnod", "Q1-emiter")
        .connect("VCTRL-outnod", "RB-lPin")
        .connect("RB-rPin", "Q1-base")
        .connect("Q1-collector", "RL-lPin")
        .connect("RL-rPin", "GND-Gnd");

    c_on.solve().unwrap();
    let v_load_on = c_on.pin_voltage("RL-lPin").unwrap();
    assert!(v_load_on > 4.7, "PNP on: Vload={v_load_on:.3} > 4.7V");
}

// =========================================================================
// 3. DIODES, ZENERS, AND LEDS
// =========================================================================

#[test]
fn test_diode_forward_and_reverse_bias() {
    // 1. Forward bias: Vin = 5V through 1k resistor -> Diode forward drop ~0.53V for default model
    let mut c_fwd = Circuit::new();
    c_fwd
        .add_fixed_volt("VIN", 5.0)
        .add_resistor("R1", 1000.0)
        .add_diode("D1")
        .add_ground("GND");

    c_fwd
        .connect("VIN-outnod", "R1-lPin")
        .connect("R1-rPin", "D1-lPin")
        .connect("D1-rPin", "GND-Gnd");

    c_fwd.solve().unwrap();
    let vf = c_fwd.pin_voltage("D1-lPin").unwrap();
    approx(
        vf,
        0.53,
        0.05,
        "Diode default model forward drop Vf ~ 0.53V",
    );

    // 2. Reverse bias: Diode blocks 10V with minimal leakage
    let mut c_rev = Circuit::new();
    c_rev
        .add_fixed_volt("VIN", 10.0)
        .add_resistor("R1", 10_000.0)
        .add_diode("D1")
        .add_ground("GND");

    c_rev
        .connect("VIN-outnod", "D1-rPin") // Cathode to +10V
        .connect("D1-lPin", "R1-lPin") // Anode through resistor to GND
        .connect("R1-rPin", "GND-Gnd");

    c_rev.solve().unwrap();
    let vr = c_rev.pin_voltage("D1-lPin").unwrap();
    assert!(
        vr < 0.01,
        "Diode reverse leakage voltage Vr={vr:.6} < 0.01V"
    );
}

#[test]
fn test_zener_voltage_regulator() {
    // 5.6V Zener Shunt Regulator with varying input voltages
    for &vin in &[8.0, 10.0, 12.0, 15.0] {
        let mut c = Circuit::new();
        c.add_fixed_volt("VIN", vin)
            .add_resistor("RS", 470.0)
            .add_zener("Z1")
            .add_resistor("RLOAD", 2200.0)
            .add_ground("GND");

        // Cathode connected to positive rail via RS, Anode to GND
        c.connect("VIN-outnod", "RS-lPin")
            .connect("RS-rPin", "Z1-rPin") // Zener cathode
            .connect("Z1-lPin", "GND-Gnd") // Zener anode
            .connect("Z1-rPin", "RLOAD-lPin")
            .connect("RLOAD-rPin", "GND-Gnd");

        c.solve().unwrap();
        let vz = c.pin_voltage("Z1-rPin").unwrap();
        approx(
            vz,
            5.6,
            0.35,
            &format!("Zener 5.6V regulation with Vin={vin}V"),
        );
    }
}

#[test]
fn test_diode_bridge_rectifier() {
    // 4-diode full wave bridge rectifier:
    // AC differential inputs (+10V / 0V) produce positive DC output
    let mut c = Circuit::new();
    c.add_fixed_volt("VAC_P", 10.0)
        .add_fixed_volt("VAC_N", 0.0)
        .add_diode("D1")
        .add_diode("D2")
        .add_diode("D3")
        .add_diode("D4")
        .add_resistor("RLOAD", 1000.0)
        .add_ground("GND");

    // D1: VAC_P to DC+
    c.connect("VAC_P-outnod", "D1-lPin")
        .connect("D1-rPin", "RLOAD-lPin");
    // D2: DC- to VAC_P
    c.connect("RLOAD-rPin", "D2-lPin")
        .connect("D2-rPin", "VAC_P-outnod");
    // D3: VAC_N to DC+
    c.connect("VAC_N-outnod", "D3-lPin")
        .connect("D3-rPin", "RLOAD-lPin");
    // D4: DC- to VAC_N
    c.connect("RLOAD-rPin", "D4-lPin")
        .connect("D4-rPin", "VAC_N-outnod");

    c.connect("RLOAD-rPin", "GND-Gnd");

    c.solve().unwrap();
    let v_dc = c.pin_voltage("RLOAD-lPin").unwrap();
    approx(v_dc, 9.43, 0.15, "Bridge Rectifier DC output");
}

// =========================================================================
// 4. MOSFET & JFET TESTS
// =========================================================================

#[test]
fn test_nmos_switch_and_saturation() {
    // NMOS: VDD = 10V, RD = 1k, Source to GND
    // 1. Cutoff: VGS = 0V -> Vds = 10V
    let mut c_off = Circuit::new();
    c_off
        .add_fixed_volt("VDD", 10.0)
        .add_fixed_volt("VGS", 0.0)
        .add_mosfet_with("M1", false, false, 1.0, 3.0) // N-channel, threshold=3V
        .add_resistor("RD", 1000.0)
        .add_ground("GND");

    c_off
        .connect("VDD-outnod", "RD-lPin")
        .connect("RD-rPin", "M1-Dren")
        .connect("VGS-outnod", "M1-Gate")
        .connect("M1-Sour", "GND-Gnd");

    c_off.solve().unwrap();
    let vd_off = c_off.pin_voltage("M1-Dren").unwrap();
    approx(vd_off, 10.0, 0.05, "NMOS Cutoff Vds = VDD");

    // 2. On/Conducting: VGS = 5V (> Vth=3V) -> conducts, pulling drain down
    let mut c_on = Circuit::new();
    c_on.add_fixed_volt("VDD", 10.0)
        .add_fixed_volt("VGS", 5.0)
        .add_mosfet_with("M1", false, false, 1.0, 3.0)
        .add_resistor("RD", 1000.0)
        .add_ground("GND");

    c_on.connect("VDD-outnod", "RD-lPin")
        .connect("RD-rPin", "M1-Dren")
        .connect("VGS-outnod", "M1-Gate")
        .connect("M1-Sour", "GND-Gnd");

    c_on.solve().unwrap();
    let vd_on = c_on.pin_voltage("M1-Dren").unwrap();
    assert!(vd_on < 1.0, "NMOS on: Vds={vd_on:.3} < 1.0V");
}

#[test]
fn test_jfet_pinchoff_and_saturation() {
    // JFET N-channel: Idss = 20mA, Vp = -4V, VDD = 10V, RD = 100Ω
    // 1. VGS = 0V: JFET is in saturation conducting large current -> Drain drops significantly
    let mut c_on = Circuit::new();
    c_on.add_fixed_volt("VDD", 10.0)
        .add_fixed_volt("VGS", 0.0)
        .add_jfet("J1")
        .add_resistor("RD", 100.0)
        .add_ground("GND");

    c_on.connect("VDD-outnod", "RD-lPin")
        .connect("RD-rPin", "J1-Dren")
        .connect("VGS-outnod", "J1-Gate")
        .connect("J1-Sour", "GND-Gnd");

    c_on.solve().unwrap();
    let vd_on = c_on.pin_voltage("J1-Dren").unwrap();
    assert!(vd_on < 9.0, "JFET conducts at VGS=0: Vd={vd_on:.3} < 9.0V");

    // 2. VGS = -5V (< Vp = -4V): JFET channel pinched off -> Drain stays at VDD
    let mut c_off = Circuit::new();
    c_off
        .add_fixed_volt("VDD", 10.0)
        .add_fixed_volt("VGS", -5.0)
        .add_jfet("J1")
        .add_resistor("RD", 100.0)
        .add_ground("GND");

    c_off
        .connect("VDD-outnod", "RD-lPin")
        .connect("RD-rPin", "J1-Dren")
        .connect("VGS-outnod", "J1-Gate")
        .connect("J1-Sour", "GND-Gnd");

    c_off.solve().unwrap();
    let vd_off = c_off.pin_voltage("J1-Dren").unwrap();
    approx(vd_off, 10.0, 0.05, "JFET Pinched off at VGS <= Vp");
}

// =========================================================================
// 5. COMPARATOR AND VOLTAGE REGULATOR
// =========================================================================

#[test]
fn test_comparator_switching() {
    // Comparator with Reference = 2.5V
    // 1. Vin = 3.0V (> Vref) -> Output High = 5.0V
    let mut c_hi = Circuit::new();
    c_hi.add_fixed_volt("VREF", 2.5)
        .add_fixed_volt("VIN", 3.0)
        .add_comparator("CMP1")
        .add_resistor("RLOAD", 10_000.0)
        .add_ground("GND");

    c_hi.connect("VIN-outnod", "CMP1-in0") // Non-inverting (+)
        .connect("VREF-outnod", "CMP1-in1") // Inverting (-)
        .connect("CMP1-out", "RLOAD-lPin")
        .connect("RLOAD-rPin", "GND-Gnd");

    c_hi.run_ps(100_000).unwrap();
    let vo_hi = c_hi.pin_voltage("CMP1-out").unwrap();
    approx(vo_hi, 5.0, 0.1, "Comparator output High = 5V");

    // 2. Vin = 2.0V (< Vref) -> Output Low = 0.0V
    let mut c_lo = Circuit::new();
    c_lo.add_fixed_volt("VREF", 2.5)
        .add_fixed_volt("VIN", 2.0)
        .add_comparator("CMP1")
        .add_resistor("RLOAD", 10_000.0)
        .add_ground("GND");

    c_lo.connect("VIN-outnod", "CMP1-in0")
        .connect("VREF-outnod", "CMP1-in1")
        .connect("CMP1-out", "RLOAD-lPin")
        .connect("RLOAD-rPin", "GND-Gnd");

    c_lo.run_ps(100_000).unwrap();
    let vo_lo = c_lo.pin_voltage("CMP1-out").unwrap();
    approx(vo_lo, CERO_DOUB, 0.1, "Comparator output Low = 0V");
}

#[test]
fn test_voltage_regulator_tracking() {
    // VoltReg: Vout = Vref + V_set (default V_set = 1.2V)
    // When Ref pin is at 3.8V, Vout should regulate to 3.8 + 1.2 = 5.0V
    let mut c = Circuit::new();
    c.add_fixed_volt("VIN", 10.0)
        .add_fixed_volt("VREF", 3.8)
        .add_volt_reg("VR1")
        .add_resistor("RLOAD", 100.0)
        .add_ground("GND");

    c.connect("VIN-outnod", "VR1-input")
        .connect("VREF-outnod", "VR1-ref")
        .connect("VR1-output", "RLOAD-lPin")
        .connect("RLOAD-rPin", "GND-Gnd");

    c.solve().unwrap();
    let vo = c.pin_voltage("VR1-output").unwrap();
    approx(vo, 5.0, 0.05, "Voltage regulator output 5.0V");
}

// =========================================================================
// 6. SCR, TRIAC, DIAC, AND ANALOG MUX
// =========================================================================

#[test]
fn test_scr_triggering_and_conduction() {
    // SCR: Gate trigger turns it on
    // 1. Without gate trigger: SCR remains OFF
    let mut c_off = Circuit::new();
    c_off
        .add_fixed_volt("VSUP", 12.0)
        .add_fixed_volt("VGATE", 0.0)
        .add_scr("SCR1")
        .add_resistor("RLOAD", 100.0)
        .add_ground("GND");

    c_off
        .connect("VSUP-outnod", "RLOAD-lPin")
        .connect("RLOAD-rPin", "SCR1-lPin") // Anode
        .connect("SCR1-rPin", "GND-Gnd") // Cathode
        .connect("VGATE-outnod", "SCR1-gPin");

    c_off.solve().unwrap();
    let va_off = c_off.pin_voltage("SCR1-lPin").unwrap();
    approx(va_off, 12.0, 0.1, "SCR blocking: Anode stays at VSUP");

    // 2. With gate trigger (VGATE = 2.0V): SCR turns ON, anode drops to ~0.7V
    let mut c_on = Circuit::new();
    c_on.add_fixed_volt("VSUP", 12.0)
        .add_fixed_volt("VGATE", 2.0)
        .add_scr("SCR1")
        .add_resistor("RLOAD", 100.0)
        .add_ground("GND");

    c_on.connect("VSUP-outnod", "RLOAD-lPin")
        .connect("RLOAD-rPin", "SCR1-lPin")
        .connect("SCR1-rPin", "GND-Gnd")
        .connect("VGATE-outnod", "SCR1-gPin");

    c_on.solve().unwrap();
    let va_on = c_on.pin_voltage("SCR1-lPin").unwrap();
    approx(
        va_on,
        0.7,
        0.15,
        "SCR triggered: Anode voltage drops to ~0.7V",
    );
}

#[test]
fn test_triac_conduction() {
    // TRIAC: Gate trigger conducts current bidirectionally
    let mut c = Circuit::new();
    c.add_fixed_volt("VSUP", 12.0)
        .add_fixed_volt("VGATE", 2.5)
        .add_triac("TR1")
        .add_resistor("RLOAD", 100.0)
        .add_ground("GND");

    c.connect("VSUP-outnod", "RLOAD-lPin")
        .connect("RLOAD-rPin", "TR1-lPin") // MT1
        .connect("TR1-rPin", "GND-Gnd") // MT2
        .connect("VGATE-outnod", "TR1-gPin");

    c.solve().unwrap();
    let v_mt1 = c.pin_voltage("TR1-lPin").unwrap();
    assert!(v_mt1 < 0.2, "TRIAC conducting: Vmt1={v_mt1:.3} < 0.2V");
}

#[test]
fn test_analog_mux_routing() {
    // 4-Channel Analog MUX: Routes selected channel to common Z pin
    // Channel 0 = 1.0V, Channel 1 = 2.0V, Channel 2 = 3.0V, Channel 3 = 4.0V
    // Address = 2 (A0=0V, A1=5V) -> Common pin should read 3.0V
    let mut c = Circuit::new();
    c.add_fixed_volt("V0", 1.0)
        .add_fixed_volt("V1", 2.0)
        .add_fixed_volt("V2", 3.0)
        .add_fixed_volt("V3", 4.0)
        .add_fixed_volt("A0", 0.0)
        .add_fixed_volt("A1", 5.0)
        .add_fixed_volt("EN", 0.0) // Active low enable
        .add_analog_mux("MUX1", 4)
        .add_resistor("RLOAD", 100_000.0)
        .add_ground("GND");

    c.connect("V0-outnod", "MUX1-pinY0")
        .connect("V1-outnod", "MUX1-pinY1")
        .connect("V2-outnod", "MUX1-pinY2")
        .connect("V3-outnod", "MUX1-pinY3")
        .connect("A0-outnod", "MUX1-pinAddr0")
        .connect("A1-outnod", "MUX1-pinAddr1")
        .connect("EN-outnod", "MUX1-PinEnable")
        .connect("MUX1-PinInput", "RLOAD-lPin")
        .connect("RLOAD-rPin", "GND-Gnd");

    c.solve().unwrap();
    let vz = c.pin_voltage("MUX1-PinInput").unwrap();
    approx(vz, 3.0, 0.05, "Analog MUX channel 2 selected -> Vout=3.0V");
}

// =========================================================================
// 7. PASSIVES, SENSORS, AND BRIDGE NETWORKS
// =========================================================================

#[test]
fn test_potentiometer_sweep() {
    for &wiper in &[0.0, 0.25, 0.5, 0.75, 1.0] {
        let mut c = Circuit::new();
        c.add_fixed_volt("VIN", 10.0)
            .add_potentiometer("POT1", 10_000.0, wiper)
            .add_resistor("RLOAD", 1e9)
            .add_ground("GND");

        c.connect("VIN-outnod", "POT1-PinB")
            .connect("POT1-PinA", "GND-Gnd")
            .connect("POT1-PinM", "RLOAD-lPin")
            .connect("RLOAD-rPin", "GND-Gnd");

        c.solve().unwrap();
        let vw = c.pin_voltage("POT1-PinM").unwrap();
        let expected = 10.0 * wiper;
        approx(vw, expected, 0.05, &format!("Potentiometer wiper={wiper}"));
    }
}

#[test]
fn test_wheatstone_bridge() {
    // Wheatstone Bridge: R1 = R2 = R3 = 10k. R4 = 10k (balanced) -> Vbridge = 0V
    // When R4 changes to 12k (unbalanced) -> Vbridge = 10V * (12/22 - 10/20) = 10V * (0.5455 - 0.5) = 0.455V
    let mut c = Circuit::new();
    c.add_fixed_volt("VSUP", 10.0)
        .add_resistor("R1", 10_000.0)
        .add_resistor("R2", 10_000.0)
        .add_resistor("R3", 10_000.0)
        .add_resistor("R4", 12_000.0)
        .add_ground("GND");

    // Left branch: R1 (top) and R2 (bottom)
    c.connect("VSUP-outnod", "R1-lPin")
        .connect("R1-rPin", "R2-lPin")
        .connect("R2-rPin", "GND-Gnd");

    // Right branch: R3 (top) and R4 (bottom)
    c.connect("VSUP-outnod", "R3-lPin")
        .connect("R3-rPin", "R4-lPin")
        .connect("R4-rPin", "GND-Gnd");

    c.solve().unwrap();
    let v_left = c.pin_voltage("R1-rPin").unwrap();
    let v_right = c.pin_voltage("R3-rPin").unwrap();
    approx(v_left, 5.0, 0.01, "Wheatstone left node");
    approx(
        v_right,
        10.0 * (12_000.0 / 22_000.0),
        0.01,
        "Wheatstone right node",
    );
    let v_diff = v_right - v_left;
    approx(
        v_diff,
        10.0 * (12.0 / 22.0 - 0.5),
        0.01,
        "Bridge differential voltage",
    );
}

#[test]
fn test_bjt_current_mirror() {
    // Current mirror: Q1 diode-connected (collector tied to base) as reference.
    // Q2 base tied to Q1 base. Emitters to GND.
    // I_ref = (VCC - VBE) / R_ref = (10V - 0.7V) / 10k ≈ 0.93mA.
    // Output branch has R_load = 5k.
    // VC2 = VCC - I_ref * R_load ≈ 10V - 0.93mA * 5k ≈ 5.35V.
    let mut c = Circuit::new();
    c.add_fixed_volt("VCC", 10.0)
        .add_bjt("Q1", false) // Reference NPN
        .add_bjt("Q2", false) // Output NPN
        .add_resistor("RREF", 10_000.0)
        .add_resistor("RLOAD", 5000.0)
        .add_ground("GND");

    // Reference branch
    c.connect("VCC-outnod", "RREF-lPin")
        .connect("RREF-rPin", "Q1-collector")
        .connect("Q1-collector", "Q1-base")
        .connect("Q1-emiter", "GND-Gnd");

    // Mirror connection
    c.connect("Q1-base", "Q2-base")
        .connect("Q2-emiter", "GND-Gnd")
        .connect("VCC-outnod", "RLOAD-lPin")
        .connect("RLOAD-rPin", "Q2-collector");

    c.solve().unwrap();
    let vc1 = c.pin_voltage("Q1-collector").unwrap();
    let vc2 = c.pin_voltage("Q2-collector").unwrap();
    approx(
        vc1,
        0.6,
        0.15,
        "Diode-connected Q1 base/collector drop ~0.6V",
    );
    // Current in Q2 should be very close to Q1
    let ic1 = (10.0 - vc1) / 10_000.0;
    let ic2 = (10.0 - vc2) / 5000.0;
    approx(
        ic2,
        ic1,
        0.15 * ic1,
        "Current mirror output branch matches reference branch",
    );
}

#[test]
fn test_cmos_inverter() {
    // CMOS Inverter: PMOS on top (Source to VDD=5V), NMOS on bottom (Source to GND).
    // Gates tied together to VIN. Drains tied together to VOUT.
    // 1. VIN = 0V -> NMOS OFF, PMOS ON -> VOUT = 5V
    let mut c_hi = Circuit::new();
    c_hi.add_fixed_volt("VDD", 5.0)
        .add_fixed_volt("VIN", 0.0)
        .add_mosfet_with("MP", true, false, 1.0, 2.0) // PMOS
        .add_mosfet_with("MN", false, false, 1.0, 2.0) // NMOS
        .add_resistor("RLOAD", 1_000_000.0)
        .add_ground("GND");

    c_hi.connect("VDD-outnod", "MP-Sour")
        .connect("VIN-outnod", "MP-Gate")
        .connect("VIN-outnod", "MN-Gate")
        .connect("MN-Sour", "GND-Gnd")
        .connect("MP-Dren", "MN-Dren")
        .connect("MP-Dren", "RLOAD-lPin")
        .connect("RLOAD-rPin", "GND-Gnd");

    c_hi.solve().unwrap();
    let vo_hi = c_hi.pin_voltage("MP-Dren").unwrap();
    approx(vo_hi, 5.0, 0.05, "CMOS Inverter input 0V -> output 5V");

    // 2. VIN = 5V -> NMOS ON, PMOS OFF -> VOUT = 0V
    let mut c_lo = Circuit::new();
    c_lo.add_fixed_volt("VDD", 5.0)
        .add_fixed_volt("VIN", 5.0)
        .add_mosfet_with("MP", true, false, 1.0, 2.0) // PMOS
        .add_mosfet_with("MN", false, false, 1.0, 2.0) // NMOS
        .add_resistor("RLOAD", 1_000_000.0)
        .add_ground("GND");

    c_lo.connect("VDD-outnod", "MP-Sour")
        .connect("VIN-outnod", "MP-Gate")
        .connect("VIN-outnod", "MN-Gate")
        .connect("MN-Sour", "GND-Gnd")
        .connect("MP-Dren", "MN-Dren")
        .connect("MP-Dren", "RLOAD-lPin")
        .connect("RLOAD-rPin", "GND-Gnd");

    c_lo.solve().unwrap();
    let vo_lo = c_lo.pin_voltage("MP-Dren").unwrap();
    approx(
        vo_lo,
        CERO_DOUB,
        0.05,
        "CMOS Inverter input 5V -> output 0V",
    );
}

#[test]
fn test_diac_breakover_switching() {
    // DIAC: Breakover voltage V_BO = 30V
    // 1. Voltage below breakover (20V): DIAC does not conduct -> output stays at 0V
    let mut c_off = Circuit::new();
    c_off
        .add_fixed_volt("VSUP", 20.0)
        .add_diac("D1")
        .add_resistor("RLOAD", 1000.0)
        .add_ground("GND");

    c_off
        .connect("VSUP-outnod", "D1-lPin")
        .connect("D1-rPin", "RLOAD-lPin")
        .connect("RLOAD-rPin", "GND-Gnd");

    c_off.solve().unwrap();
    let vo_off = c_off.pin_voltage("RLOAD-lPin").unwrap();
    approx(vo_off, 0.0, 0.05, "DIAC below breakover voltage: Vout = 0V");

    // 2. Voltage above breakover (35V): DIAC breaks over and conducts -> output voltage rises
    let mut c_on = Circuit::new();
    c_on.add_fixed_volt("VSUP", 35.0)
        .add_diac("D1")
        .add_resistor("RLOAD", 1000.0)
        .add_ground("GND");

    c_on.connect("VSUP-outnod", "D1-lPin")
        .connect("D1-rPin", "RLOAD-lPin")
        .connect("RLOAD-rPin", "GND-Gnd");

    c_on.solve().unwrap();
    let vo_on = c_on.pin_voltage("RLOAD-lPin").unwrap();
    assert!(
        vo_on > 15.0,
        "DIAC conducts after breakover: Vout={vo_on:.2} > 15V"
    );
}

#[test]
fn test_voltage_regulator_with_resistor_divider() {
    // LM317-style adjustable regulator circuit:
    // Vout = Vref * (1 + R2 / R1) = 1.25V * (1 + 3k / 1k) = 5.0V
    let mut c = Circuit::new();
    c.add_fixed_volt("VIN", 12.0)
        .add_volt_reg("VR1")
        .add_resistor("R1", 1000.0)
        .add_resistor("R2", 3000.0)
        .add_ground("GND");

    // V_set on VR1 is 1.25V
    if let Kind::VoltReg { ref mut state } = c.comp_mut("VR1").unwrap().kind {
        state.set_out_volt(1.25);
    }

    c.connect("VIN-outnod", "VR1-input")
        .connect("VR1-output", "R1-lPin")
        .connect("R1-rPin", "VR1-ref")
        .connect("VR1-ref", "R2-lPin")
        .connect("R2-rPin", "GND-Gnd");

    c.solve().unwrap();
    let vo = c.pin_voltage("VR1-output").unwrap();
    approx(
        vo,
        5.0,
        0.1,
        "Adjustable regulator with R1=1k, R2=3k -> Vout=5.0V",
    );
}
