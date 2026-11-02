//! Plots, traces, overload log, and simulation instrumentation data serialization.

use cs_engine::canvas::Canvas;
use serde_json::{Value, json};

pub(super) fn overload_log_json(canvas: &Canvas) -> Value {
    let log = canvas.overload_log();
    let mut arr = Vec::with_capacity(log.len());
    for entry in log {
        arr.push(json!({
            "text": entry.text,
            "compUid": entry.comp_uid,
            "crashed": entry.crashed,
        }));
    }
    Value::Array(arr)
}

pub(super) fn warnings_text(canvas: &Canvas) -> String {
    let count = canvas.overload_log().len();
    if count > 0 {
        format!("{}", count)
    } else {
        String::new()
    }
}

pub(super) fn warnings_crashed(canvas: &Canvas) -> bool {
    canvas.overload_log().iter().any(|e| e.crashed)
}

pub(super) fn memory_table_rows_json(canvas: &Canvas, uid: &str) -> Value {
    json!(canvas.memory_table_rows(uid))
}

pub(super) fn test_unit_truth_json(canvas: &Canvas, uid: &str) -> Value {
    json!(canvas.test_unit_truth(uid))
}

pub(super) fn subcircuit_tree_json(canvas: &Canvas) -> Value {
    json!(canvas.snapshot_subcircuits())
}

pub(super) fn programmable_devices_json(canvas: &Canvas, active_device_id: Option<&str>) -> Value {
    json!(canvas.collect_programmable_devices(active_device_id))
}

pub(super) fn ac_sources_json(canvas: &Canvas) -> Value {
    json!(canvas.scene().to_circuit().ac_sources())
}

pub(super) fn ac_probes_json(canvas: &Canvas) -> Value {
    json!(canvas.scene().to_circuit().ac_probes())
}

pub(super) fn dc_sources_json(canvas: &Canvas) -> Value {
    json!(canvas.scene().to_circuit().dc_sources())
}

pub(super) fn dc_probes_json(canvas: &Canvas) -> Value {
    json!(canvas.scene().to_circuit().dc_probes())
}

pub(super) fn dc_sweep_json(
    canvas: &Canvas,
    primary_source: &str,
    primary_start: f64,
    primary_stop: f64,
    primary_step: f64,
    secondary_source: &str,
    secondary_start: f64,
    secondary_stop: f64,
    secondary_step: f64,
    probe_pin: &str,
) -> Value {
    let circuit = canvas.scene().to_circuit();
    let sec_source = if secondary_source.trim().is_empty() {
        None
    } else {
        Some(secondary_source.to_string())
    };
    let params = cs_engine::simulation::DcSweepParams {
        primary_source: primary_source.to_string(),
        primary_start,
        primary_stop,
        primary_step,
        secondary_source: sec_source,
        secondary_start,
        secondary_stop,
        secondary_step,
        probe_pin: probe_pin.to_string(),
    };
    match circuit.dc_sweep(&params) {
        Ok(result) => json!(result),
        Err(e) => json!({ "error": e.to_string() }),
    }
}

/// Small-signal sweep of a copy of the schematic. The running simulator is left alone.
pub(super) fn ac_sweep_json(
    canvas: &Canvas,
    source: &str,
    probe: &str,
    f_start: f64,
    f_stop: f64,
    per_decade: i32,
) -> Value {
    let mut circuit = canvas.scene().to_circuit();
    let ppd = u32::try_from(per_decade).unwrap_or(1).clamp(1, 10_000);
    match circuit.ac_sweep(source, probe, f_start, f_stop, ppd) {
        Ok(points) => {
            let rows: Vec<Value> = points
                .iter()
                .map(|p| {
                    let db = if p.gain > 1e-15 {
                        20.0 * p.gain.log10()
                    } else {
                        -300.0
                    };
                    json!({
                        "hz": p.hz,
                        "mag": p.mag,
                        "phase": p.phase_deg,
                        "gain": p.gain,
                        "db": db,
                    })
                })
                .collect();
            json!({ "points": rows })
        }
        Err(e) => json!({ "error": e.to_string() }),
    }
}
