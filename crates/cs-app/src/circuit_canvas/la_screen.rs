//! Logic-analyzer window. The GUI thread snapshots the plot. A worker paints it.

use cs_engine::canvas::{self, Canvas, Color, ImageBuf};
use cs_engine::components;
use cs_engine::plot::PlotBuffer;
use cs_engine::theme::{ColorId, ColorTheme};
use serde_json::{Value, json};

use super::screen::{Frame, Host};
use crate::image_frame::Slot;

fn snapshot(canvas: &Canvas) -> PlotBuffer {
    canvas.la_traces().clone()
}

fn paint(job: &Frame<PlotBuffer>) -> Option<ImageBuf> {
    let (r, g, b, a) = ColorTheme::get_rgba(ColorId::ScopePaper, true);
    canvas::render_draw(job.pw, job.ph, Color { r, g, b, a }, |draw| {
        components::paint_la_screen(
            draw,
            job.pw as f64,
            job.ph as f64,
            Some(&job.payload),
            job.dpr,
        );
    })
}

static HOST: Host<PlotBuffer> = Host::new("la-screen", Slot::Logic, paint);

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

/// The analyzer item is gone. Drop a queued job and ignore the one already rendering.
#[unsafe(no_mangle)]
pub extern "C" fn cs_logic_screen_closed() {
    HOST.close();
}

/// Logic-analyzer levels at logical time `x`. Each hit's `y` is in logical pixels.
pub fn signal_at(canvas: &Canvas, x: f64, w: f64, h: f64, dpr: f64) -> Value {
    if w < 2.0 || h < 2.0 {
        return Value::Null;
    }
    let (pw, ph, dpr) = super::screen::device_size(w, h, dpr);
    let fraction = (x / w).clamp(0.0, 1.0);
    let hits = components::la_signal_at(
        pw,
        ph,
        fraction,
        components::LA_TRACE_INSET * dpr,
        canvas.la_traces(),
    );
    let hits = hits
        .iter()
        .map(|hit| {
            json!({
                "channel": hit.channel,
                "value": hit.value,
                "y": hit.y * h / ph,
            })
        })
        .collect::<Vec<_>>();
    json!({
        "fraction": fraction,
        "x": fraction * w,
        "hits": hits,
    })
}
