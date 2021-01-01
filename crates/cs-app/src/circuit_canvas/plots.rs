//! Plots, traces, overload log, and simulation instrumentation data serialization.

use cs_engine::canvas::Canvas;
use serde_json::{Value, json};

pub(super) fn traces_json(buf: &cs_engine::plot::PlotBuffer) -> Value {
    let arr: Vec<Value> = buf
        .channels
        .iter()
        .map(|ch| {
            json!({
                "color": ch.color,
                "samples": ch.samples,
                "connected": ch.connected,
            })
        })
        .collect();
    Value::Array(arr)
}

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
