//! Schematic pin ids versus solver pin ids, and live kind updates.
//!
//! Inventory of current failures is `pin_parity_expected.txt` beside this file.
//! A component that starts matching, or a new mismatch, changes that file.
//! See `docs/component_audit_and_verification.md` (invariants 1 and 2).

use std::collections::BTreeSet;
use std::panic::AssertUnwindSafe;

use crate::Circuit;
use crate::components::Part;
use crate::components::harness::CASES;

const COMP_ID: &str = "U1";
const EXPECTED: &str = include_str!("pin_parity_expected.txt");

/// Companion nodes the solver owns and the schematic does not draw.
fn is_internal_node(pin_id: &str) -> bool {
    pin_id == format!("{COMP_ID}-mid")
        || pin_id == format!("{COMP_ID}-nodeX")
        || pin_id == format!("{COMP_ID}-nodeY")
}

fn schematic_pin_ids(part: &Part) -> BTreeSet<String> {
    part.pin_geoms()
        .into_iter()
        .map(|pin| {
            if pin.suffix.starts_with('-') {
                format!("{COMP_ID}{}", pin.suffix)
            } else {
                format!("{COMP_ID}-{}", pin.suffix)
            }
        })
        .collect()
}

fn show(pin_id: &str) -> String {
    if pin_id.is_empty() {
        return "<empty>".to_string();
    }
    pin_id
        .strip_prefix(&format!("{COMP_ID}-"))
        .unwrap_or(pin_id)
        .to_string()
}

fn show_list(pins: &BTreeSet<String>) -> String {
    pins.iter()
        .map(|id| show(id))
        .collect::<Vec<_>>()
        .join(", ")
}

fn check(part: &Part) -> String {
    let schematic_n = part.pin_geoms().len();
    let schematic = schematic_pin_ids(part);
    let mut circuit = Circuit::new();
    part.add_to_circuit(COMP_ID, &mut circuit);
    let Some(comp) = circuit.comp(COMP_ID) else {
        if schematic.is_empty() {
            return String::new();
        }
        return format!("not-in-circuit schematic=[{}]", show_list(&schematic));
    };

    let solver_list = comp.pin_ids();
    let solver: BTreeSet<String> = solver_list.iter().cloned().collect();
    let bare: BTreeSet<String> = solver
        .iter()
        .filter(|id| id.starts_with('-'))
        .cloned()
        .collect();
    let only_schematic: BTreeSet<String> = schematic.difference(&solver).cloned().collect();
    let only_solver: BTreeSet<String> = solver
        .difference(&schematic)
        .filter(|id| !is_internal_node(id) && !id.starts_with('-'))
        .cloned()
        .collect();

    let mut flags = Vec::new();
    if !bare.is_empty() {
        flags.push(format!("bare=[{}]", show_list(&bare)));
    }
    if !only_schematic.is_empty() {
        flags.push(format!("only-schematic=[{}]", show_list(&only_schematic)));
    }
    if !only_solver.is_empty() {
        flags.push(format!("only-solver=[{}]", show_list(&only_solver)));
    }

    let solver_external_n = solver_list
        .iter()
        .filter(|id| !is_internal_node(id))
        .count();
    let solver_external_unique = solver.iter().filter(|id| !is_internal_node(id)).count();
    if schematic_n != schematic.len() || solver_external_n != solver_external_unique {
        flags.push(format!(
            "duplicate-ids schematic={schematic_n} unique={} solver={solver_external_n} unique={solver_external_unique}",
            schematic.len()
        ));
    }

    match part.to_element_kind() {
        None => flags.push("live:no-kind".to_string()),
        Some(kind) => {
            let before: BTreeSet<String> = solver_list.into_iter().collect();
            if !circuit.update_component_kind(COMP_ID, &kind) {
                flags.push("live:missing".to_string());
            } else if let Some(updated) = circuit.comp(COMP_ID) {
                let after: BTreeSet<String> = updated.pin_ids().into_iter().collect();
                if before != after {
                    let stripped: BTreeSet<String> = before
                        .iter()
                        .map(|id| id.strip_prefix(COMP_ID).unwrap_or(id).to_string())
                        .collect();
                    if after == stripped {
                        flags.push("live:prefix-cleared".to_string());
                    } else {
                        let lost: BTreeSet<String> = before.difference(&after).cloned().collect();
                        let gained: BTreeSet<String> = after.difference(&before).cloned().collect();
                        flags.push(format!(
                            "live:lost=[{}] gained=[{}]",
                            show_list(&lost),
                            show_list(&gained)
                        ));
                    }
                }
            }
        }
    }

    flags.join(" ")
}

fn check_catch(part: &Part) -> String {
    match std::panic::catch_unwind(AssertUnwindSafe(|| check(part))) {
        Ok(flags) => flags,
        Err(_) => "panic".to_string(),
    }
}

fn build_report() -> String {
    let mut lines = Vec::new();
    for case in CASES {
        let default_part = (case.make)();
        let default_flags = check_catch(&default_part);
        if !default_flags.is_empty() {
            lines.push(format!("{}: {default_flags}", case.itemtype));
        }
        for (prop, left, right) in case.extras {
            for value in [*left, *right] {
                let mut variant = (case.make)();
                let flags = match variant.set_prop_text(prop, value) {
                    Ok(_) => check_catch(&variant),
                    Err(err) => format!("prop-error: {err}"),
                };
                if flags != default_flags {
                    let body = if flags.is_empty() {
                        "ok"
                    } else {
                        flags.as_str()
                    };
                    lines.push(format!("{} {prop}={value}: {body}", case.itemtype));
                }
            }
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

fn first_diff(got: &str, expected: &str) -> String {
    let mut out = String::new();
    let mut shown = 0;
    for (index, (got_line, expected_line)) in got.lines().zip(expected.lines()).enumerate() {
        if got_line != expected_line {
            out.push_str(&format!(
                "line {}: got `{got_line}`\n     expected `{expected_line}`\n",
                index + 1
            ));
            shown += 1;
            if shown >= 8 {
                break;
            }
        }
    }
    let got_n = got.lines().count();
    let expected_n = expected.lines().count();
    if got_n != expected_n {
        out.push_str(&format!("line count got {got_n} expected {expected_n}\n"));
    }
    out
}

#[test]
fn pin_parity_and_live_prefix() {
    let report = build_report();
    if report != EXPECTED {
        let path = std::env::temp_dir().join("pin_parity_report.txt");
        std::fs::write(&path, &report).expect("write pin parity report");
        panic!(
            "pin parity inventory changed\nfull report: {}\nUpdate pin_parity_expected.txt when a mismatch is fixed or a new one appears.\n{}",
            path.display(),
            first_diff(&report, EXPECTED)
        );
    }
}
