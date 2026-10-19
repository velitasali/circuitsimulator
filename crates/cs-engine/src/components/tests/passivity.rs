//! Every external pin pulled to ground, then one pin excited, then KCL.
//!
//! `passivity_expected.txt` lists drivers, missing pin currents, parts whose
//! return path is implicit ground, and the default variable resistor (0 Ω
//! stamps 1e12 S and the matrix is singular). A new line, or a line that
//! disappears, changes that file. A panic fails the test outright.

use std::collections::BTreeSet;
use std::panic::{AssertUnwindSafe, catch_unwind};

use crate::Circuit;
use crate::components::Part;
use crate::components::harness::CASES;

const ID: &str = "U1";
const PULL_OHMS: f64 = 1_000.0;
const EXCITE_VOLTS: f64 = 5.0;
/// A 100 kΩ pull-up into the 1 kΩ shunt sits near 50 mV. Logic highs are volts.
const IDLE_VOLT: f64 = 0.1;
/// Absolute KCL tolerance, or this fraction of the sum of |I|.
const KCL_ABS: f64 = 1e-6;
const KCL_REL: f64 = 1e-4;

const EXPECTED: &str = include_str!("passivity_expected.txt");

fn pins_of(part: &Part) -> Result<Vec<String>, String> {
    let mut circuit = Circuit::new();
    part.add_to_circuit(ID, &mut circuit);
    let Some(comp) = circuit.comp(ID) else {
        return Err("not-inserted".to_string());
    };
    let mut seen = BTreeSet::new();
    let mut pins = Vec::new();
    for id in comp.pin_ids() {
        if id.is_empty() || !seen.insert(id.clone()) {
            continue;
        }
        pins.push(id);
    }
    if pins.is_empty() {
        return Err("no-pins".to_string());
    }
    Ok(pins)
}

fn pull_down(circuit: &mut Circuit, pins: &[String]) {
    circuit.add_ground("GND");
    for (i, pin) in pins.iter().enumerate() {
        let rid = format!("Rd{i}");
        circuit.add_resistor(&rid, PULL_OHMS);
        circuit.connect(pin.as_str(), format!("{rid}-lPin"));
        circuit.connect(format!("{rid}-rPin"), "GND-Gnd");
    }
}

fn max_abs_volt(circuit: &Circuit, pins: &[String]) -> f64 {
    pins.iter()
        .filter_map(|pin| circuit.pin_voltage(pin))
        .map(f64::abs)
        .fold(0.0, f64::max)
}

fn kcl(circuit: &Circuit, pins: &[String]) -> Result<(), String> {
    let mut sum = 0.0;
    let mut abs = 0.0;
    let mut missing = Vec::new();
    for pin in pins {
        match circuit.current_out_of_pin(pin) {
            Some(i) => {
                sum += i;
                abs += i.abs();
            }
            None => missing.push(pin.as_str()),
        }
    }
    if !missing.is_empty() {
        return Err("current-gap".to_string());
    }
    let tol = KCL_ABS.max(KCL_REL * abs);
    if sum.abs() > tol {
        // The return path is implicit ground (a source, a logic rail, or an
        // instrument Norton), so the external pins do not sum to zero.
        return Err("implicit-return".to_string());
    }
    Ok(())
}

fn check(part: &Part) -> Result<String, String> {
    let pins = pins_of(part)?;
    let mut idle = Circuit::new();
    part.add_to_circuit(ID, &mut idle);
    pull_down(&mut idle, &pins);
    idle.solve().map_err(|err| format!("idle-solve {err}"))?;
    let idle_v = max_abs_volt(&idle, &pins);
    let mut flags = Vec::new();
    if idle_v > IDLE_VOLT {
        flags.push(format!("driver {idle_v:.3} V"));
    }

    let mut excited = Circuit::new();
    part.add_to_circuit(ID, &mut excited);
    pull_down(&mut excited, &pins);
    let excite = &pins[0];
    excited.add_fixed_volt("VEX", EXCITE_VOLTS);
    excited.add_resistor("Rex", PULL_OHMS);
    excited.connect("VEX-outnod", "Rex-lPin");
    excited.connect("Rex-rPin", excite.as_str());
    excited
        .solve()
        .map_err(|err| format!("excited-solve {err}"))?;
    // Meters copy the reading into `last_out` after the stamp. One more solve
    // uses that value so the output Norton matches the node voltage.
    excited
        .re_solve()
        .map_err(|err| format!("excited-resolve {err}"))?;
    match kcl(&excited, &pins) {
        Ok(()) => {}
        Err(flag) => flags.push(flag),
    }
    Ok(flags.join(" "))
}

fn check_catch(part: &Part) -> String {
    match catch_unwind(AssertUnwindSafe(|| check(part))) {
        Ok(Ok(flags)) => flags,
        Ok(Err(flag)) => flag,
        Err(_) => "panic".to_string(),
    }
}

fn build_report() -> String {
    let mut lines = Vec::new();
    for case in CASES {
        let part = (case.make)();
        let flags = check_catch(&part);
        if !flags.is_empty() {
            lines.push(format!("{}: {flags}", case.itemtype));
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

fn hard_failures(report: &str) -> Vec<&str> {
    report
        .lines()
        .filter(|line| line.ends_with("panic"))
        .collect()
}

#[test]
fn passive_pins_rest_near_ground_and_obey_kcl() {
    let report = build_report();
    let hard = hard_failures(&report);
    if !hard.is_empty() || report != EXPECTED {
        let path = std::env::temp_dir().join("passivity_report.txt");
        std::fs::write(&path, &report).expect("write passivity report");
        panic!(
            "passivity inventory changed\nfull report: {}\nhard failures: {}\n{report}",
            path.display(),
            hard.join(" | ")
        );
    }
}
