//! Oscilloscope window. The GUI thread snapshots the plot. A worker paints it.

use cs_engine::canvas::{self, Canvas, Color, ImageBuf};
use cs_engine::components;
use cs_engine::plot::{Channel, PlotBuffer};
use cs_engine::theme::{ColorId, ColorTheme};
use serde_json::{Value, json};

use super::screen::{Frame, Host};
use crate::image_frame::Slot;

struct Payload {
    traces: PlotBuffer,
    volt_div: [f64; 4],
    volt_pos: [f64; 4],
    tracks: i32,
    hidden: [bool; 4],
}

fn snapshot(canvas: &Canvas) -> Payload {
    Payload {
        traces: canvas.scope_traces().clone(),
        volt_div: canvas.scope_volt_div(),
        volt_pos: canvas.scope_volt_pos(),
        tracks: canvas.scope_tracks(),
        hidden: canvas.scope_hidden(),
    }
}

fn paint(job: &Frame<Payload>) -> Option<ImageBuf> {
    let (r, g, b, a) = ColorTheme::get_rgba(ColorId::ScopePaper, true);
    let payload = &job.payload;
    canvas::render_draw(job.pw, job.ph, Color { r, g, b, a }, |draw| {
        components::paint_scope_screen(
            draw,
            job.pw as f64,
            job.ph as f64,
            Some(&payload.traces),
            &payload.volt_div,
            &payload.volt_pos,
            payload.tracks,
            &payload.hidden,
            job.dpr,
        );
    })
}

static HOST: Host<Payload> = Host::new("scope-screen", Slot::Scope, paint);

/// Remember the window size and paint. A closed or tiny window is left unchanged.
pub fn render(canvas: &Canvas, w: f64, h: f64, dpr: f64) {
    HOST.render(w, h, dpr, || snapshot(canvas));
}

/// Queue one paint for the current window. Returns false when no window is open
/// or a flush is already waiting, so a burst of setters posts a single event.
pub fn schedule() -> bool {
    HOST.schedule()
}

pub fn cancel_flush() {
    HOST.cancel_flush();
}

/// Snapshot after the current Qt event. Setters in that event share this call.
pub fn flush(canvas: &Canvas) {
    HOST.flush(|| snapshot(canvas));
}

/// The scope item is gone. Drop a queued job and ignore the one already rendering.
#[unsafe(no_mangle)]
pub extern "C" fn cs_scope_screen_closed() {
    HOST.close();
}

pub fn voltage_at(canvas: &Canvas, fraction: f64) -> Value {
    let hidden = canvas.scope_hidden();
    let buf = canvas.scope_traces();
    let mut out = Vec::with_capacity(4);
    for c in 0..4 {
        let ch = buf.channels.get(c);
        let value = if hidden[c] {
            None
        } else {
            ch.filter(|ch| ch.connected)
                .and_then(|ch| held_voltage(ch, fraction))
        };
        out.push(match value {
            Some(v) => json!(v),
            None => Value::Null,
        });
    }
    Value::Array(out)
}

fn held_voltage(ch: &Channel, fraction: f64) -> Option<f64> {
    let fraction = fraction.clamp(0.0, 1.0);
    if ch.plot.len() >= 2 && ch.plot.len() == ch.plot_x.len() {
        let mut held = ch.plot[0];
        for (&x, &v) in ch.plot_x.iter().zip(ch.plot.iter()) {
            if x <= fraction + 1e-6 {
                held = v;
            } else {
                break;
            }
        }
        return Some(held);
    }
    let n = ch.samples.len();
    if n == 0 {
        return None;
    }
    let idx = ((fraction * (n - 1) as f64).round() as usize).min(n - 1);
    Some(ch.samples[idx])
}
