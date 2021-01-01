//! Repaint tracking, overscan management, coordinate culling, and canvas render state synchronization.

use cs_engine::canvas::{Canvas, Change, DirtySet, Palette, Rect};
use serde_json::{Value, json};

pub(super) fn cull_margin(zoom: f64) -> f64 {
    (200.0 / zoom.max(0.1)).max(150.0)
}

pub(super) fn cull_min_x(vr: Rect, zoom: f64) -> f64 {
    vr.x - cull_margin(zoom)
}

pub(super) fn cull_max_x(vr: Rect, zoom: f64) -> f64 {
    vr.x + vr.w + cull_margin(zoom)
}

pub(super) fn cull_min_y(vr: Rect, zoom: f64) -> f64 {
    vr.y - cull_margin(zoom)
}

pub(super) fn cull_max_y(vr: Rect, zoom: f64) -> f64 {
    vr.y + vr.h + cull_margin(zoom)
}

pub(super) fn v_scroll_pos(sr: Rect, vr: Rect) -> f64 {
    if sr.h > 0.0 {
        ((vr.y - sr.y) / sr.h).clamp(0.0, 1.0)
    } else {
        0.0
    }
}

pub(super) fn v_scroll_size(sr: Rect, vr: Rect) -> f64 {
    if sr.h > 0.0 {
        (vr.h / sr.h).clamp(0.0, 1.0)
    } else {
        1.0
    }
}

pub(super) fn h_scroll_pos(sr: Rect, vr: Rect) -> f64 {
    if sr.w > 0.0 {
        ((vr.x - sr.x) / sr.w).clamp(0.0, 1.0)
    } else {
        0.0
    }
}

pub(super) fn h_scroll_size(sr: Rect, vr: Rect) -> f64 {
    if sr.w > 0.0 {
        (vr.w / sr.w).clamp(0.0, 1.0)
    } else {
        1.0
    }
}

pub(super) fn rect_json(r: Rect) -> Value {
    json!({ "x": r.x, "y": r.y, "w": r.w, "h": r.h })
}

pub(super) fn render_canvas(canvas: &Canvas, dark: bool, dpr: f64) {
    let (w, h) = canvas.viewport().view_size();
    if w <= 0.0 || h <= 0.0 {
        return;
    }
    let palette = if dark {
        Palette::dark()
    } else {
        Palette::light()
    };
    crate::native_canvas::update_render_state(
        canvas,
        &palette,
        w.round().max(1.0) as u32,
        h.round().max(1.0) as u32,
        dpr.max(1.0),
    );
}

pub(super) fn render_canvas_regions(
    canvas: &Canvas,
    dark: bool,
    dpr: f64,
    scene_rects: &[Rect],
) -> bool {
    let (w, h) = canvas.viewport().view_size();
    if w <= 0.0 || h <= 0.0 {
        return false;
    }
    let palette = if dark {
        Palette::dark()
    } else {
        Palette::light()
    };
    crate::native_canvas::update_render_regions(
        canvas,
        &palette,
        w.round().max(1.0) as u32,
        h.round().max(1.0) as u32,
        dpr.max(1.0),
        scene_rects,
    )
}

pub(super) fn merge_screen_rects(
    mut parts: Vec<Rect>,
    max_parts: usize,
    free_merge: f64,
) -> Vec<Rect> {
    let area = |r: &Rect| r.w * r.h;
    while parts.len() > 1 {
        let mut bi = None;
        let mut bj = None;
        let mut best_extra = f64::MAX;

        for i in 0..parts.len() {
            for j in (i + 1)..parts.len() {
                let u = parts[i].united(parts[j]);
                let extra = area(&u) - area(&parts[i]) - area(&parts[j]);
                if bi.is_none() || extra < best_extra {
                    bi = Some(i);
                    bj = Some(j);
                    best_extra = extra;
                }
            }
        }

        if let (Some(i), Some(j)) = (bi, bj) {
            if parts.len() <= max_parts && best_extra > free_merge {
                break;
            }
            let u = parts[i].united(parts[j]);
            parts[i] = u;
            parts.remove(j);
        } else {
            break;
        }
    }
    parts
}

fn view_rect_json(x: f64, y: f64, w: f64, h: f64) -> Value {
    json!({ "x": x, "y": y, "width": w, "height": h })
}

fn scene_rects_to_screen(
    canvas: &Canvas,
    scene_rects: &[Rect],
    view_w: f64,
    view_h: f64,
) -> Vec<Rect> {
    let vp = canvas.viewport();
    let surface = Rect::new(0.0, 0.0, view_w, view_h);
    let mut screen_rects: Vec<Rect> = Vec::new();
    for sr in scene_rects {
        let scr = vp.map_rect_from_circuit(*sr).adjust(-2.0, -2.0, 2.0, 2.0);
        if scr.intersects(&surface) {
            let clamped = Rect::new(
                scr.x.max(0.0),
                scr.y.max(0.0),
                (scr.right().min(view_w) - scr.x.max(0.0)).max(0.0),
                (scr.bottom().min(view_h) - scr.y.max(0.0)).max(0.0),
            );
            if !clamped.is_empty() {
                screen_rects.push(clamped);
            }
        }
    }
    screen_rects
}

fn screen_rects_json(rects: Vec<Rect>) -> Value {
    Value::Array(
        rects
            .into_iter()
            .map(|r| view_rect_json(r.x, r.y, r.w, r.h))
            .collect(),
    )
}

pub(super) fn check_and_record_repaints(
    canvas: &Canvas,
    band_item_rect: Rect,
    c: Change,
    dirty: &DirtySet,
    painted_full: bool,
) -> Option<(Value, bool, Value)> {
    if !cs_engine::settings::with(|s| s.repaint_overlay) {
        return None;
    }

    let vp = canvas.viewport();
    let (view_w, view_h) = vp.view_size();
    if view_w <= 0.0 || view_h <= 0.0 {
        return None;
    }

    if c.band && !painted_full && dirty.is_empty() {
        let br = band_item_rect;
        if br.w > 0.0 && br.h > 0.0 {
            return Some((
                Value::Array(vec![view_rect_json(br.x, br.y, br.w, br.h)]),
                false,
                Value::Array(vec![]),
            ));
        }
    }

    let source_scene = dirty.scene_rects(canvas);
    let source_screen = scene_rects_to_screen(canvas, &source_scene, view_w, view_h);
    let source_json = screen_rects_json(merge_screen_rects(source_screen.clone(), 6, 4000.0));

    if painted_full {
        let full = view_rect_json(0.0, 0.0, view_w, view_h);
        return Some((Value::Array(vec![full]), true, source_json));
    }

    if source_screen.is_empty() {
        return None;
    }

    let merged = merge_screen_rects(source_screen, 6, 4000.0);
    Some((screen_rects_json(merged), false, Value::Array(vec![])))
}
