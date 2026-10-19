//! Live `update_component_kind` keeps solver memory and takes new properties.
//!
//! Asserted before any solve. `re_solve` still re-inits gate, flip-flop, and
//! latch output pins; flip-flop `q0` is what `stamp_init` restores from.

use crate::Circuit;
use crate::components::Part;
use crate::components::harness::CASES;
use crate::elements::Kind;

const ID: &str = "U1";

fn case(itemtype: &str) -> Part {
    let case = CASES
        .iter()
        .find(|c| c.itemtype == itemtype)
        .unwrap_or_else(|| panic!("missing harness case {itemtype}"));
    (case.make)()
}

fn inserted(part: &Part) -> Circuit {
    let mut circuit = Circuit::new();
    part.add_to_circuit(ID, &mut circuit);
    assert!(circuit.comp(ID).is_some(), "not inserted");
    circuit
}

fn refresh(circuit: &mut Circuit, part: &Part) {
    let kind = part
        .to_element_kind()
        .unwrap_or_else(|| panic!("no element kind"));
    assert!(circuit.update_component_kind(ID, &kind));
}

#[test]
fn memory_byte_survives_same_property_refresh() {
    let part = case("Memory");
    let mut circuit = inserted(&part);
    match &mut circuit.comp_mut(ID).unwrap().kind {
        Kind::Memory(mem) => mem.data[0] = 0xA5,
        other => panic!("expected memory, got {other:?}"),
    }
    refresh(&mut circuit, &part);
    match &circuit.comp(ID).unwrap().kind {
        Kind::Memory(mem) => assert_eq!(mem.data[0], 0xA5),
        other => panic!("expected memory, got {other:?}"),
    }
}

#[test]
fn counter_stepper_and_lamp_keep_runtime() {
    let part = case("Counter");
    let mut circuit = inserted(&part);
    match &mut circuit.comp_mut(ID).unwrap().kind {
        Kind::Counter(counter) => counter.count = 7,
        other => panic!("expected counter, got {other:?}"),
    }
    refresh(&mut circuit, &part);
    match &circuit.comp(ID).unwrap().kind {
        Kind::Counter(counter) => assert_eq!(counter.count, 7),
        other => panic!("expected counter, got {other:?}"),
    }

    let part = case("Stepper");
    let mut circuit = inserted(&part);
    match &mut circuit.comp_mut(ID).unwrap().kind {
        Kind::Stepper { angle, .. } => *angle = 123.0,
        other => panic!("expected stepper, got {other:?}"),
    }
    refresh(&mut circuit, &part);
    match &circuit.comp(ID).unwrap().kind {
        Kind::Stepper { angle, .. } => assert!((*angle - 123.0).abs() < 1e-9),
        other => panic!("expected stepper, got {other:?}"),
    }

    let part = case("Lamp");
    let mut circuit = inserted(&part);
    match &mut circuit.comp_mut(ID).unwrap().kind {
        Kind::Lamp {
            heat, resistance, ..
        } => {
            *heat = 0.5;
            *resistance = 42.0;
        }
        other => panic!("expected lamp, got {other:?}"),
    }
    refresh(&mut circuit, &part);
    match &circuit.comp(ID).unwrap().kind {
        Kind::Lamp {
            heat, resistance, ..
        } => {
            assert!((*heat - 0.5).abs() < 1e-12);
            assert!((*resistance - 42.0).abs() < 1e-12);
        }
        other => panic!("expected lamp, got {other:?}"),
    }
}

#[test]
fn lamp_heat_survives_a_voltage_edit_and_resistance_follows_it() {
    let mut part = case("Lamp");
    let mut circuit = inserted(&part);
    let cold = match &circuit.comp(ID).unwrap().kind {
        Kind::Lamp { r_cold, .. } => *r_cold,
        _ => panic!("lamp"),
    };
    match &mut circuit.comp_mut(ID).unwrap().kind {
        Kind::Lamp {
            heat, resistance, ..
        } => {
            *heat = 0.5;
            *resistance = 42.0;
        }
        _ => panic!("lamp"),
    }
    part.set_prop_text("Voltage", "6").unwrap();
    refresh(&mut circuit, &part);
    match &circuit.comp(ID).unwrap().kind {
        Kind::Lamp {
            voltage,
            heat,
            resistance,
            ..
        } => {
            assert!((*voltage - 6.0).abs() < 1e-9);
            assert!((*heat - 0.5).abs() < 1e-12);
            assert!((*resistance - cold).abs() < 1e-9);
        }
        _ => panic!("lamp"),
    }
}

#[test]
fn relay_stays_pulled_in_when_coil_resistance_changes() {
    let mut part = case("Relay");
    let mut circuit = inserted(&part);
    match &mut circuit.comp_mut(ID).unwrap().kind {
        Kind::Relay { active, .. } => *active = true,
        other => panic!("expected relay, got {other:?}"),
    }
    part.set_prop_text("Rcoil", "50").unwrap();
    refresh(&mut circuit, &part);
    match &circuit.comp(ID).unwrap().kind {
        Kind::Relay { active, r_coil, .. } => {
            assert!(*active);
            assert!((*r_coil - 50.0).abs() < 1e-9);
        }
        other => panic!("expected relay, got {other:?}"),
    }
}

#[test]
fn capacitor_voltage_survives_a_capacitance_edit() {
    let mut part = case("Capacitor");
    let mut circuit = inserted(&part);
    match &mut circuit.comp_mut(ID).unwrap().kind {
        Kind::Capacitor { volt, .. } => *volt = 2.5,
        other => panic!("expected capacitor, got {other:?}"),
    }
    part.set_prop_text("Capacitance", "1 µF").unwrap();
    refresh(&mut circuit, &part);
    match &circuit.comp(ID).unwrap().kind {
        Kind::Capacitor { capacitance, volt } => {
            assert!((*capacitance - 1e-6).abs() < 1e-12);
            assert!((*volt - 2.5).abs() < 1e-12);
        }
        other => panic!("expected capacitor, got {other:?}"),
    }
}

#[test]
fn functional_properties_reach_the_solver_kind() {
    let mut ldr = case("Ldr");
    let default_r = match ldr.to_element_kind().unwrap() {
        Kind::Ldr { resistance, .. } => resistance,
        _ => panic!("ldr"),
    };
    ldr.set_prop_text("Lux", "1000").unwrap();
    let circuit = inserted(&ldr);
    match &circuit.comp(ID).unwrap().kind {
        Kind::Ldr { resistance, lux } => {
            assert!((*lux - 1000.0).abs() < 1e-9);
            assert!((*resistance - default_r).abs() > 1.0);
        }
        other => panic!("expected ldr, got {other:?}"),
    }

    let mut dip = case("ResistorDip");
    dip.set_prop_text("Bussed", "true").unwrap();
    let circuit = inserted(&dip);
    match &circuit.comp(ID).unwrap().kind {
        Kind::ResistorDip { bussed, .. } => assert!(*bussed),
        other => panic!("expected resistor dip, got {other:?}"),
    }
    assert!(
        circuit
            .comp(ID)
            .unwrap()
            .pin_ids()
            .iter()
            .any(|id| id == &format!("{ID}-com"))
    );

    let mut gate = case("AndGate");
    gate.set_prop_text("InvertInputs", "true").unwrap();
    let circuit = inserted(&gate);
    match &circuit.comp(ID).unwrap().kind {
        Kind::Gate(gate) => assert!(gate.inputs[0].inverted),
        other => panic!("expected gate, got {other:?}"),
    }
}
