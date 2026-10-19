//! An isolated part must stamp the matrix, unless it is a known empty part.
//!
//! `stamp_expected.txt` lists parts that contribute nothing. Reactive parts
//! set `has_reactive`. Clock and WaveGen set `has_time_sources`. VoltReg is a
//! DC regulator: it is nonlinear and it is not a time source.

use std::panic::{AssertUnwindSafe, catch_unwind};

use rustc_hash::FxHashMap;

use crate::Circuit;
use crate::components::Part;
use crate::components::harness::CASES;
use crate::elements::ANALOG_DT_DEFAULT;
use crate::net::ENode;

const ID: &str = "U1";
const EXPECTED: &str = include_str!("stamp_expected.txt");

const REACTIVE: &[&str] = &[
    "Capacitor",
    "ElCapacitor",
    "Inductor",
    "VarCapacitor",
    "VarInductor",
    "Transformer",
    "Lamp",
];

fn contributes(part: &Part) -> Result<bool, &'static str> {
    let mut circuit = Circuit::new();
    part.add_to_circuit(ID, &mut circuit);
    let Some(comp) = circuit.comp(ID) else {
        return Err("not-inserted");
    };
    let mut pin_net = FxHashMap::default();
    let mut nodes = Vec::new();
    for pin in comp.pin_ids() {
        if pin.is_empty() || pin_net.contains_key(&pin) {
            continue;
        }
        let index = nodes.len();
        nodes.push(ENode::new(pin.clone(), index));
        pin_net.insert(pin, index);
    }
    if nodes.is_empty() {
        return Err("no-pins");
    }
    let cache = comp.build_pin_cache(&pin_net);
    comp.stamp_with_cache(&cache, &pin_net, &mut nodes, ANALOG_DT_DEFAULT);
    Ok(nodes.iter().any(ENode::has_stamp))
}

fn classify(part: &Part) -> String {
    match catch_unwind(AssertUnwindSafe(|| contributes(part))) {
        Ok(Ok(true)) => String::new(),
        Ok(Ok(false)) => "no-stamp".to_string(),
        Ok(Err(flag)) => flag.to_string(),
        Err(_) => "panic".to_string(),
    }
}

fn build_report() -> String {
    let mut lines = Vec::new();
    for case in CASES {
        let flag = classify(&(case.make)());
        if !flag.is_empty() {
            lines.push(format!("{}: {flag}", case.itemtype));
        }
    }
    if lines.is_empty() {
        String::new()
    } else {
        let mut report = lines.join("\n");
        report.push('\n');
        report
    }
}

fn placed(itemtype: &str) -> Circuit {
    let case = CASES
        .iter()
        .find(|c| c.itemtype == itemtype)
        .unwrap_or_else(|| panic!("missing {itemtype}"));
    let mut circuit = Circuit::new();
    (case.make)().add_to_circuit(ID, &mut circuit);
    assert!(circuit.comp(ID).is_some(), "{itemtype} not inserted");
    circuit
}

#[test]
fn isolated_parts_register_a_stamp() {
    let report = build_report();
    let hard: Vec<_> = report
        .lines()
        .filter(|line| line.ends_with("panic"))
        .collect();
    if !hard.is_empty() || report != EXPECTED {
        let path = std::env::temp_dir().join("stamp_report.txt");
        std::fs::write(&path, &report).expect("write stamp report");
        panic!(
            "stamp inventory changed\nfull report: {}\nhard: {}\n{report}",
            path.display(),
            hard.join(" | ")
        );
    }
}

#[test]
fn reactive_and_time_sources_are_classified() {
    for itemtype in REACTIVE {
        let circuit = placed(itemtype);
        assert!(circuit.has_reactive(), "{itemtype} should be reactive");
        assert!(
            !circuit.has_time_sources(),
            "{itemtype} is not a time source"
        );
    }

    let clock = placed("Clock");
    assert!(clock.has_time_sources());
    assert!(!clock.has_reactive());

    let wave = placed("WaveGen");
    assert!(wave.has_time_sources());

    let reg = placed("VoltReg");
    assert!(!reg.has_time_sources(), "VoltReg is a DC regulator");
    assert!(reg.scan_has_nonlinear());
}

#[test]
fn clock_toggles_over_a_half_period() {
    let mut circuit = placed("Clock");
    circuit
        .add_resistor("RL", 10_000.0)
        .add_ground("GND")
        .connect(format!("{ID}-outnod"), "RL-lPin")
        .connect("RL-rPin", "GND-Gnd");
    circuit.solve().unwrap();
    let low = circuit.pin_voltage(&format!("{ID}-outnod")).unwrap();
    assert!(low.abs() < 0.2, "clock starts low, got {low}");
    circuit.run_ps(500_000_000).unwrap();
    let high = circuit.pin_voltage(&format!("{ID}-outnod")).unwrap();
    assert!(high > 4.0, "clock should be high, got {high}");
}
