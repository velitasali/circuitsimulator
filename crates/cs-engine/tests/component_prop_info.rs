//! Verification that component properties expose non-empty docstrings (info) for PropDialog.

use cs_engine::components::{
    AndGate, Battery, Bjt, Capacitor, Clock, Component, DHT22, Diode, Inductor, Mosfet, OpAmp,
    Resistor, Servo, WaveGen,
};

#[test]
fn test_resistor_property_info() {
    let r = Resistor::default();
    let rows = r.prop_rows();

    let res_row = rows
        .iter()
        .find(|r| r.name == "Resistance")
        .expect("Resistance row");
    assert_eq!(res_row.info, "Resistance value, in ohms.");

    let bands_row = rows
        .iter()
        .find(|r| r.name == "ShowBands")
        .expect("ShowBands row");
    assert_eq!(
        bands_row.info,
        "Show the resistance as color bands instead of a value label."
    );
}

#[test]
fn test_diode_property_info() {
    let d = Diode::default();
    let rows = d.prop_rows();

    let thresh_row = rows
        .iter()
        .find(|r| r.name == "Threshold")
        .expect("Threshold row");
    assert_eq!(thresh_row.info, "Voltage drop when forward biased.");

    let res_row = rows
        .iter()
        .find(|r| r.name == "Resistance")
        .expect("Resistance row");
    assert_eq!(res_row.info, "Series resistance.");

    let em_row = rows
        .iter()
        .find(|r| r.name == "EmCoef")
        .expect("EmCoef row");
    assert_eq!(em_row.info, "Ideality factor.");
}

#[test]
fn test_active_and_passive_component_property_info() {
    let bjt = Bjt::default();
    let rows = bjt.prop_rows();
    assert_eq!(
        rows.iter().find(|r| r.name == "Gain").unwrap().info,
        "Current gain."
    );
    assert_eq!(
        rows.iter().find(|r| r.name == "PNP").unwrap().info,
        "PNP or NPN."
    );

    let mosfet = Mosfet::default();
    let rows = mosfet.prop_rows();
    assert_eq!(
        rows.iter().find(|r| r.name == "Threshold").unwrap().info,
        "Gate-Source Voltage to start conducting."
    );

    let opamp = OpAmp::default();
    let rows = opamp.prop_rows();
    assert_eq!(
        rows.iter().find(|r| r.name == "Gain").unwrap().info,
        "Voltage gain."
    );

    let cap = Capacitor::default();
    let rows = cap.prop_rows();
    assert_eq!(
        rows.iter().find(|r| r.name == "Capacitance").unwrap().info,
        "Capacitance value."
    );

    let ind = Inductor::default();
    let rows = ind.prop_rows();
    assert_eq!(
        rows.iter().find(|r| r.name == "Inductance").unwrap().info,
        "Inductance value."
    );
}

#[test]
fn test_sources_sensors_and_logic_property_info() {
    let bat = Battery::default();
    let rows = bat.prop_rows();
    assert_eq!(
        rows.iter().find(|r| r.name == "Voltage").unwrap().info,
        "Battery voltage."
    );

    let clk = Clock::default();
    let rows = clk.prop_rows();
    assert_eq!(
        rows.iter().find(|r| r.name == "Frequency").unwrap().info,
        "Set output frequency."
    );

    let wg = WaveGen::default();
    let rows = wg.prop_rows();
    assert_eq!(
        rows.iter().find(|r| r.name == "Frequency").unwrap().info,
        "Set output frequency."
    );

    let servo = Servo::default();
    let rows = servo.prop_rows();
    assert_eq!(
        rows.iter().find(|r| r.name == "Speed").unwrap().info,
        "Time to rotate 60º."
    );

    let dht = DHT22::default();
    let rows = dht.prop_rows();
    assert_eq!(
        rows.iter().find(|r| r.name == "Temp").unwrap().info,
        "Current temperature."
    );

    let and_gate = AndGate::default();
    let rows = and_gate.prop_rows();
    assert_eq!(
        rows.iter().find(|r| r.name == "Small").unwrap().info,
        "Use a smaller body."
    );
}
