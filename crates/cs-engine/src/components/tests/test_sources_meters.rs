//! Tests for power sources, generators, measurement meters, and connectors.

use super::recorder::{DrawOp, record_part_paint};
use crate::components::battery::Battery;
use crate::components::clock::Clock;
use crate::components::ground::Ground;
use crate::components::*;

#[test]
fn test_battery_and_ground() {
    let mut bat = Battery::default();
    bat.set_prop_text("Voltage", "9 V").unwrap();
    assert_eq!(bat.voltage, 9.0);

    let pins_bat = bat.pin_geoms();
    assert_eq!(pins_bat.len(), 2, "Battery has 2 terminal pins (+ and -)");
    let rec_bat = record_part_paint(&Part::Battery(bat));
    assert!(rec_bat.is_all_finite());
    // Battery draws parallel plate lines using fill_rect
    assert!(
        rec_bat
            .ops
            .iter()
            .any(|op| matches!(op, DrawOp::FillRect { .. }))
    );

    let gnd = Ground::default();
    let pins_gnd = gnd.pin_geoms();
    assert_eq!(pins_gnd.len(), 1, "Ground has 1 pin");
    let rec_gnd = record_part_paint(&Part::Ground(gnd));
    assert!(rec_gnd.is_all_finite());
}

#[test]
fn test_clock_and_wave_generator() {
    let mut clk = Clock::default();
    clk.set_prop_text("Frequency", "1 kHz").unwrap();
    assert_eq!(clk.frequency, 1000.0);

    let pins_clk = clk.pin_geoms();
    assert_eq!(pins_clk.len(), 1, "Clock generator has 1 output pin");
    let rec_clk = record_part_paint(&Part::Clock(clk));
    assert!(rec_clk.is_all_finite());

    // Wave Generator (1-pin single ended by default; 2-pin when bipolar)
    let mut wg = WaveGen::default();
    wg.set_prop_text("Frequency", "500 Hz").unwrap();
    wg.set_prop_text("Amplitude", "2.5 V").unwrap();
    assert_eq!(
        wg.pin_geoms().len(),
        1,
        "Wave generator default has 1 output pin"
    );

    wg.set_prop_text("Bipolar", "true").unwrap();
    assert_eq!(
        wg.pin_geoms().len(),
        2,
        "Bipolar Wave generator has 2 output pins"
    );

    let rec_wg = record_part_paint(&Part::WaveGen(wg));
    assert!(rec_wg.is_all_finite());
}

#[test]
fn test_meters_voltmeter_ammeter_freqmeter() {
    // Voltmeter has 3 pins (+, -, outnod)
    let vm = Voltmeter::default();
    let pins_vm = vm.pin_geoms();
    assert_eq!(pins_vm.len(), 3, "Voltmeter has 3 pins (+, -, outnod)");
    let rec_vm = record_part_paint(&Part::Voltmeter(vm));
    assert!(rec_vm.is_all_finite());

    // Ammeter has 3 pins (+, -, outnod)
    let am = Ammeter::default();
    let pins_am = am.pin_geoms();
    assert_eq!(pins_am.len(), 3, "Ammeter has 3 pins (+, -, outnod)");
    let rec_am = record_part_paint(&Part::Ammeter(am));
    assert!(rec_am.is_all_finite());

    // Frequency Meter has 1 probe pin
    let fm = FreqMeter::default();
    assert_eq!(
        fm.pin_geoms().len(),
        1,
        "Frequency meter has 1 probe input pin"
    );
    let rec_fm = record_part_paint(&Part::FreqMeter(fm));
    assert!(rec_fm.is_all_finite());
}

#[test]
fn test_oscilloscope_and_logic_analyzer() {
    // Oscilloscope
    let mut osc = Oscope::default();
    osc.set_prop_text("InputImped", "1 MΩ").unwrap();
    let pins_osc = osc.pin_geoms();
    assert!(
        pins_osc.len() >= 4,
        "Oscilloscope has 4 input channels + GND pin"
    );
    let rec_osc = record_part_paint(&Part::Oscope(osc.clone()));
    assert!(rec_osc.is_all_finite());
    // Should draw reticle grid lines, chassis, and screen
    assert!(
        rec_osc
            .ops
            .iter()
            .any(|op| matches!(op, DrawOp::FillRoundRect { .. }))
    );

    // Logic Analyzer
    let la = LogicAnalyzer::default();
    let pins_la = la.pin_geoms();
    assert!(pins_la.len() >= 8, "Logic Analyzer has 8 channels");
    let rec_la = record_part_paint(&Part::LogicAnalyzer(la));
    assert!(rec_la.is_all_finite());
}

#[test]
fn test_connectors_tunnel_bus_header_socket() {
    // Tunnel
    let mut tun = Tunnel::default();
    tun.set_prop_text("Name", "NetA").unwrap();
    assert_eq!(tun.name, "NetA");
    let pins_tun = tun.pin_geoms();
    assert_eq!(pins_tun.len(), 1, "Tunnel has 1 pin");
    let rec_tun = record_part_paint(&Part::Tunnel(tun));
    assert!(rec_tun.is_all_finite());

    // Bus
    let bus = Bus::default();
    let pins_bus = bus.pin_geoms();
    assert!(pins_bus.len() >= 2);
    let rec_bus = record_part_paint(&Part::Bus(bus));
    assert!(rec_bus.is_all_finite());

    // Header & Socket (Pins prop specifies pin pairs)
    let mut hdr = Header::default();
    hdr.set_prop_text("Pins", "6").unwrap();
    assert_eq!(
        hdr.pin_geoms().len(),
        12,
        "6-position header has 12 pins (left & right)"
    );
    let rec_hdr = record_part_paint(&Part::Header(hdr));
    assert!(rec_hdr.is_all_finite());

    let mut sock = Socket::default();
    sock.set_prop_text("Pins", "8").unwrap();
    assert_eq!(sock.pin_geoms().len(), 16, "8-position socket has 16 pins");
    let rec_sock = record_part_paint(&Part::Socket(sock));
    assert!(rec_sock.is_all_finite());
}
