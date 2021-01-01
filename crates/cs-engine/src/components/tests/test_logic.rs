//! Tests for digital logic gates, flip-flops, multiplexers, and logic IC components.

use super::recorder::record_part_paint;
use crate::components::and_gate::AndGate;
use crate::components::full_adder::FullAdder;
use crate::components::half_adder::HalfAdder;
use crate::components::*;

#[test]
fn test_and_gate_input_scaling_and_pins() {
    let mut gate = AndGate::default();
    assert_eq!(gate.num_inputs, 2);

    // Default 2-input AND gate has 2 in pins + 1 out pin
    let pins2 = gate.pin_geoms();
    assert_eq!(pins2.len(), 3);
    assert_eq!(pins2[0].suffix, "-in0");
    assert_eq!(pins2[1].suffix, "-in1");
    assert_eq!(pins2[2].suffix, "-out");

    let rec2 = record_part_paint(&Part::AndGate(gate.clone()));
    assert!(rec2.is_all_finite());

    // Scale to 4 inputs
    gate.set_prop_text("NumInputs", "4").unwrap();
    assert_eq!(gate.num_inputs, 4);

    let pins4 = gate.pin_geoms();
    assert_eq!(
        pins4.len(),
        5,
        "4-input AND gate must have 4 inputs + 1 output pin"
    );
    assert_eq!(pins4[0].suffix, "-in0");
    assert_eq!(pins4[3].suffix, "-in3");
    assert_eq!(pins4[4].suffix, "-out");

    let rec4 = record_part_paint(&Part::AndGate(gate.clone()));
    assert!(rec4.is_all_finite());
    assert_ne!(
        rec2.ops, rec4.ops,
        "Gate polygon height must expand with more inputs"
    );

    // Add tristate enable pin
    gate.set_prop_text("Tristate", "true").unwrap();
    assert!(gate.tristate);
    let pins_tri = gate.pin_geoms();
    assert_eq!(pins_tri.len(), 6, "Tristate gate must have enable pin");
    assert!(pins_tri.iter().any(|p| p.suffix == "-Pin_outEnable"));

    let rec_tri = record_part_paint(&Part::AndGate(gate));
    assert!(rec_tri.is_all_finite());
    assert_ne!(rec4.ops, rec_tri.ops, "Tristate gate must draw enable line");
}

#[test]
fn test_logic_gates_family() {
    // OR Gate
    let mut or_gate = OrGate::default();
    or_gate.set_prop_text("NumInputs", "3").unwrap();
    assert_eq!(or_gate.pin_geoms().len(), 4);
    let rec_or = record_part_paint(&Part::OrGate(or_gate));
    assert!(rec_or.is_all_finite());

    // XOR Gate
    let mut xor_gate = XorGate::default();
    xor_gate.set_prop_text("NumInputs", "2").unwrap();
    assert_eq!(xor_gate.pin_geoms().len(), 3);
    let rec_xor = record_part_paint(&Part::XorGate(xor_gate));
    assert!(rec_xor.is_all_finite());

    // NOT Gate
    let not_gate = NotGate::default();
    assert_eq!(not_gate.pin_geoms().len(), 2, "NOT gate has 1 in, 1 out");
    let rec_not = record_part_paint(&Part::NotGate(not_gate));
    assert!(rec_not.is_all_finite());

    // Buffer Gate
    let buf_gate = BufferGate::default();
    assert_eq!(buf_gate.pin_geoms().len(), 2);
    let rec_buf = record_part_paint(&Part::BufferGate(buf_gate));
    assert!(rec_buf.is_all_finite());
}

#[test]
fn test_flip_flops_and_latches() {
    let ff = FlipFlop::default();
    assert!(
        ff.pin_geoms().len() >= 4,
        "FlipFlop has clock, data, Q, Q_not pins"
    );
    let rec_ff = record_part_paint(&Part::FlipFlop(ff));
    assert!(rec_ff.is_all_finite());

    let latch = Latch::default();
    assert!(latch.pin_geoms().len() >= 3);
    let rec_latch = record_part_paint(&Part::Latch(latch));
    assert!(rec_latch.is_all_finite());
}

#[test]
fn test_multiplexers_and_decoders() {
    // MUX
    let mux = Mux::default();
    assert!(mux.pin_geoms().len() >= 4);
    let rec_mux = record_part_paint(&Part::Mux(mux));
    assert!(rec_mux.is_all_finite());

    // DEMUX
    let demux = Demux::default();
    assert!(demux.pin_geoms().len() >= 4);
    let rec_demux = record_part_paint(&Part::Demux(demux));
    assert!(rec_demux.is_all_finite());

    // BCD to 7-Segment
    let bcd7 = BcdTo7Segment::default();
    assert!(
        bcd7.pin_geoms().len() >= 11,
        "BCD to 7S has 4 inputs + 7 segment outputs"
    );
    let rec_bcd7 = record_part_paint(&Part::BcdTo7Segment(bcd7));
    assert!(rec_bcd7.is_all_finite());
}

#[test]
fn test_adders_and_counters() {
    // Half Adder (1-bit)
    let mut ha1 = HalfAdder::default();
    ha1.set_prop_text("Bits", "1").unwrap();
    assert_eq!(
        ha1.pin_geoms().len(),
        4,
        "1-bit Half Adder has A, B, Sum, Carry"
    );
    let rec_ha1 = record_part_paint(&Part::HalfAdder(ha1));
    assert!(rec_ha1.is_all_finite());

    // Half Adder (4-bit default)
    let ha4 = HalfAdder::default();
    assert_eq!(
        ha4.pin_geoms().len(),
        13,
        "4-bit Half Adder has 4 A, 4 B, 4 Sum, 1 Carry = 13 pins"
    );

    // Full Adder (1-bit)
    let mut fa1 = FullAdder::default();
    fa1.set_prop_text("Bits", "1").unwrap();
    assert_eq!(
        fa1.pin_geoms().len(),
        5,
        "1-bit Full Adder has A, B, Cin, Sum, Cout"
    );
    let rec_fa1 = record_part_paint(&Part::FullAdder(fa1));
    assert!(rec_fa1.is_all_finite());

    // Binary Counter
    let bc = BinCounter::default();
    assert!(bc.pin_geoms().len() >= 4);
    let rec_bc = record_part_paint(&Part::BinCounter(bc));
    assert!(rec_bc.is_all_finite());
}

#[test]
fn test_timer_555_pinout() {
    let timer = Lm555::default();
    let pins = timer.pin_geoms();
    assert_eq!(
        pins.len(),
        8,
        "LM555 timer IC must have exactly 8 pins (DIP-8)"
    );
    let rec = record_part_paint(&Part::Lm555(timer));
    assert!(rec.is_all_finite());
}
