use crate::canvas::Scene;
use crate::canvas::draw::{Align, Color, Draw, PaintCtx, parse_hex};
use crate::canvas::geom::{Point, Rect};
use crate::canvas::pin::Pin;
use crate::canvas::scene::Item;
use crate::canvas::wire::Wire;
use crate::theme::{BUS_WIRE_WIDTH, COMPONENT_BORDER_WIDTH, WIRE_WIDTH};

pub fn paint_scene(d: &mut dyn Draw, ctx: &PaintCtx<'_>) {
    let sr = ctx.canvas.viewport().scene_rect();
    d.fill_rect(sr.x, sr.y, sr.w, sr.h, ctx.pal.canvas);
    if ctx.canvas.show_grid() {
        paint_grid(d, sr, ctx.scale, ctx.pal.grid);
    }
    let scene = ctx.canvas.scene();
    for w in scene.wires() {
        paint_wire(d, ctx, w);
    }
    let connected_pins = scene.connected_pins_set();
    for it in scene.items() {
        paint_item(d, ctx, scene, it, &connected_pins);
        if ctx.canvas.show_component_rect() {
            paint_component_rect(d, ctx, it);
        }
    }
    for w in scene.wires() {
        paint_wire_chevrons(d, ctx, w);
    }
}

pub fn paint_grid(d: &mut dyn Draw, sr: Rect, scale: f64, color: Color) {
    const NICE: [i32; 14] = [
        1, 2, 5, 10, 20, 50, 100, 200, 500, 1000, 2000, 5000, 10000, 20000,
    ];
    let mut grid_k = *NICE.last().unwrap();
    for n in NICE {
        if 8.0 * f64::from(n) * scale >= 7.0 {
            grid_k = n;
            break;
        }
    }
    d.grid_dots(sr, grid_k, color);
}

pub fn paint_wire(d: &mut dyn Draw, ctx: &PaintCtx<'_>, w: &Wire) {
    if w.points.len() < 2 {
        return;
    }
    let pts: Vec<[f64; 2]> = w.points.iter().map(|p| [p.x, p.y]).collect();
    let color = if w.selected {
        ctx.pal.band
    } else if ctx.canvas.sim_running() && ctx.canvas.circ_settings().animate_logic {
        let v = ctx.canvas.pin_voltage(&w.start_pin).unwrap_or(0.0);
        if v > 2.5 {
            ctx.pal.pin_high
        } else {
            ctx.pal.pin_low
        }
    } else {
        ctx.pal.wire
    };
    let width = if w.is_bus {
        BUS_WIRE_WIDTH
    } else if w.drawing() {
        WIRE_WIDTH * 1.2
    } else {
        WIRE_WIDTH
    };
    d.polyline(&pts, color, width, w.drawing());
}

pub fn paint_wire_chevrons(d: &mut dyn Draw, ctx: &PaintCtx<'_>, w: &Wire) {
    let chevrons = ctx.canvas.sim_running()
        && ctx.canvas.circ_settings().animate_curr
        && !w.drawing()
        && !w.is_bus;
    if chevrons {
        let current = ctx.canvas.wire_current(&w.id);
        if current.abs() > 1e-12 {
            paint_chevrons(
                d,
                &w.points,
                current,
                ctx.canvas.chevron_step(&w.id),
                ctx.pal.border,
            );
        }
    }
}

/// Spacing between chevrons along a wire, matching the C++ `m_step` wrap of 8.
const CHEVRON_SPACING: f64 = 8.0;
/// C++ `fadeDist = 0.35` with 1-unit spacing; scale to [`CHEVRON_SPACING`].
const CHEVRON_FADE_DIST: f64 = 0.35 * CHEVRON_SPACING;

struct ChevronMark {
    x: f64,
    y: f64,
    ux: f64,
    uy: f64,
    px: f64,
    py: f64,
    along: f64,
}

pub fn paint_chevrons(d: &mut dyn Draw, pts: &[Point], current: f64, step: f64, color: Color) {
    let mag = current.abs();
    if mag <= 1e-12 || pts.len() < 2 {
        return;
    }
    let reverse = current < 0.0;
    let mut offset = step;
    if offset < 0.0 {
        offset += CHEVRON_SPACING;
    }

    let mut total_len = 0.0;
    let mut marks = Vec::new();
    let mut remain = offset;
    for w in pts.windows(2) {
        let x0 = w[0].x;
        let y0 = w[0].y;
        let dx = w[1].x - x0;
        let dy = w[1].y - y0;
        let len = dx.hypot(dy);
        if len < 1e-6 {
            continue;
        }
        let mut ux = dx / len;
        let mut uy = dy / len;
        if reverse {
            ux = -ux;
            uy = -uy;
        }
        let px = -uy;
        let py = ux;
        let mut dist = remain;
        while dist < len {
            marks.push(ChevronMark {
                x: x0 + (dx / len) * dist,
                y: y0 + (dy / len) * dist,
                ux,
                uy,
                px,
                py,
                along: total_len + dist,
            });
            dist += CHEVRON_SPACING;
        }
        remain = dist - len;
        total_len += len;
    }
    if marks.is_empty() {
        return;
    }

    // C++ `ConnectorLine::collectChevrons`: alpha from |current|, then fade
    // the first mark in and the last mark out at the connector ends.
    let alpha_frac = (192.45 * mag / 4.0).min(1.0);
    let base_alpha = 0.25 + 0.75 * alpha_frac;
    let last_idx = marks.len() - 1;
    let mut first_alpha = (marks[0].along / CHEVRON_FADE_DIST).clamp(0.0, 1.0);
    let mut last_alpha = ((total_len - marks[last_idx].along) / CHEVRON_FADE_DIST).clamp(0.0, 1.0);
    if last_idx == 0 {
        first_alpha = first_alpha.min(last_alpha);
        last_alpha = first_alpha;
    }

    let wing = 2.0;
    let back = 1.0;
    let make_chevron = |m: &ChevronMark| -> [[f64; 2]; 3] {
        [
            [
                m.x - m.ux * back + m.px * wing,
                m.y - m.uy * back + m.py * wing,
            ],
            [m.x + m.ux * back, m.y + m.uy * back],
            [
                m.x - m.ux * back - m.px * wing,
                m.y - m.uy * back - m.py * wing,
            ],
        ]
    };

    if last_idx == 0 {
        let pts = make_chevron(&marks[0]);
        d.polyline(
            &pts,
            color.fade(base_alpha * first_alpha),
            WIRE_WIDTH,
            false,
        );
        return;
    }

    let first_pts = make_chevron(&marks[0]);
    d.polyline(
        &first_pts,
        color.fade(base_alpha * first_alpha),
        WIRE_WIDTH,
        false,
    );

    if marks.len() > 2 {
        let middle_pts: Vec<[[f64; 2]; 3]> = marks[1..last_idx].iter().map(make_chevron).collect();
        let slices: Vec<&[[f64; 2]]> = middle_pts.iter().map(|p| &p[..]).collect();
        d.polylines(&slices, color.fade(base_alpha), WIRE_WIDTH, false);
    }

    let last_pts = make_chevron(&marks[last_idx]);
    d.polyline(
        &last_pts,
        color.fade(base_alpha * last_alpha),
        WIRE_WIDTH,
        false,
    );
}

pub fn paint_component_rect(d: &mut dyn Draw, ctx: &PaintCtx<'_>, it: &Item) {
    d.push(it.x, it.y, it.rotation, it.hflip as f64, it.vflip as f64);

    let body = it.body_rect();
    let body_pts = [
        [body.x, body.y],
        [body.x + body.w, body.y],
        [body.x + body.w, body.y + body.h],
        [body.x, body.y + body.h],
        [body.x, body.y],
    ];
    let body_color = ctx.pal.band;
    d.polyline(&body_pts, body_color, 1.2, true);

    // Origin crosshair at (0, 0)
    let cross_color = Color::rgba(body_color.r, body_color.g, body_color.b, 160);
    d.line(-3.0, 0.0, 3.0, 0.0, cross_color, 0.8);
    d.line(0.0, -3.0, 0.0, 3.0, cross_color, 0.8);

    // Dimension label above the top-left of body
    let dim_text = format!("{:.0}×{:.0}", body.w, body.h);
    d.text(
        body.x,
        body.y - 6.0,
        &dim_text,
        5.5,
        body_color,
        Align::TopLeft,
    );

    // Total bounding box with pins (if larger than body)
    let total = it.local_total_bounding_rect();
    if total.w > body.w + 0.5 || total.h > body.h + 0.5 {
        let total_pts = [
            [total.x, total.y],
            [total.x + total.w, total.y],
            [total.x + total.w, total.y + total.h],
            [total.x, total.y + total.h],
            [total.x, total.y],
        ];
        let total_color = Color::rgba(body_color.r, body_color.g, body_color.b, 100);
        d.polyline(&total_pts, total_color, 0.8, true);
    }

    d.pop();
}

pub fn paint_item(
    d: &mut dyn Draw,
    ctx: &PaintCtx<'_>,
    scene: &Scene,
    it: &Item,
    connected_pins: &rustc_hash::FxHashSet<String>,
) {
    d.push(it.x, it.y, it.rotation, it.hflip as f64, it.vflip as f64);
    let mut item_pal = *ctx.pal;
    if it.selected {
        item_pal.border = ctx.pal.band;
    }
    let item_ctx = PaintCtx {
        canvas: ctx.canvas,
        pal: &item_pal,
        scale: ctx.scale,
        item_id: &it.id,
    };
    let pins = if it.is_node() { Vec::new() } else { it.pins() };
    for pin in &pins {
        paint_pin(d, ctx, scene, it, pin, connected_pins.contains(&pin.id));
    }
    it.kind.paint(d, &item_ctx);
    if !matches!(
        it.kind,
        crate::components::Part::Subcircuit(_)
            | crate::components::Part::Mcu(_)
            | crate::components::Part::SubPackage(_)
            | crate::components::Part::Node(_)
    ) && it.show_id
        && !it.label.is_empty()
    {
        let (lx, ly) = it.label_pos();
        d.push(
            lx,
            ly,
            it.label_rot as f64,
            it.hflip as f64,
            it.vflip as f64,
        );
        d.text(
            0.0,
            0.0,
            &it.label,
            9.0,
            item_ctx.pal.border,
            Align::TopLeft,
        );
        d.pop();
    }
    if !it.is_node() && it.show_val {
        let val_text = it.val_label_text();
        if !val_text.is_empty() {
            let (vx, vy) = it.val_pos();
            d.push(vx, vy, it.val_rot as f64, it.hflip as f64, it.vflip as f64);
            d.text(
                0.0,
                0.0,
                &val_text,
                9.0,
                item_ctx.pal.border,
                Align::TopLeft,
            );
            d.pop();
        }
    }
    if matches!(
        it.kind,
        crate::components::Part::Subcircuit(_)
            | crate::components::Part::Mcu(_)
            | crate::components::Part::QemuDevice(_)
            | crate::components::Part::SubPackage(_)
    ) {
        paint_chip_pin_labels(d, it);
    }
    if ctx.canvas.sim_running() {
        if let Some(st) = ctx.canvas.item_overload_state(&it.id) {
            if st.warning || st.crashed {
                let body = it.body_rect();
                if body.w > 0.0 && body.h > 0.0 {
                    let phase = (ctx.canvas.anim_tick() as f64 * 0.15).sin() * 0.5 + 0.5;
                    let alpha = 0.15 + 0.40 * phase;
                    let color = if st.crashed {
                        ctx.pal.msg_error
                    } else {
                        ctx.pal.msg_warn
                    };
                    d.fill_round_rect(
                        body.x - 2.0,
                        body.y - 2.0,
                        body.w + 4.0,
                        body.h + 4.0,
                        4.0,
                        color.fade(alpha),
                    );
                    d.stroke_round_rect(
                        body.x - 2.0,
                        body.y - 2.0,
                        body.w + 4.0,
                        body.h + 4.0,
                        4.0,
                        color.fade((alpha * 1.5).min(1.0)),
                        1.2,
                    );
                }
            }
        }
    }
    d.pop();
}

pub fn paint_chip_pin_labels(d: &mut dyn Draw, it: &Item) {
    let ls = it.logic_symbol();
    let color = if ls {
        parse_hex("#555555")
    } else {
        parse_hex("#cccccc")
    };
    for pin in it.pins() {
        if pin.label.is_empty() {
            continue;
        }
        let len = if pin.length == 0.0 { 8.0 } else { pin.length };
        match pin.angle {
            0 => {
                d.text(
                    pin.local.x - len - 2.0,
                    pin.local.y - 3.5,
                    &pin.label,
                    5.0,
                    color,
                    Align::Right,
                );
            }
            180 => {
                d.text(
                    pin.local.x + len + 2.0,
                    pin.local.y - 3.5,
                    &pin.label,
                    5.0,
                    color,
                    Align::TopLeft,
                );
            }
            90 => {
                d.push(pin.local.x + 3.0, pin.local.y + len + 2.0, 90.0, 1.0, 1.0);
                d.text(0.0, 0.0, &pin.label, 5.0, color, Align::TopLeft);
                d.pop();
            }
            270 => {
                d.push(pin.local.x - 3.0, pin.local.y - len - 2.0, -90.0, 1.0, 1.0);
                d.text(0.0, 0.0, &pin.label, 5.0, color, Align::TopLeft);
                d.pop();
            }
            _ => {
                d.text(
                    pin.local.x - 6.0,
                    pin.local.y - 4.0,
                    &pin.label,
                    5.0,
                    color,
                    Align::TopLeft,
                );
            }
        }
    }
}

pub fn paint_pin(
    d: &mut dyn Draw,
    ctx: &PaintCtx<'_>,
    _scene: &Scene,
    it: &Item,
    pin: &Pin,
    connected: bool,
) {
    let stroke = if ctx.canvas.sim_running() {
        match ctx.canvas.pin_voltage(&pin.id) {
            Some(v) if v > 2.5 => ctx.pal.pin_high,
            Some(_) => ctx.pal.pin_low,
            None => ctx.pal.border,
        }
    } else {
        ctx.pal.border
    };
    // Derive local pin id directly from the owning item — no O(N) scan.
    let pin_id_local = pin
        .id
        .strip_prefix(it.id.as_str())
        .unwrap_or("")
        .strip_prefix('-')
        .unwrap_or(&pin.id);
    let direction = ctx
        .canvas
        .pin_direction(&pin.id)
        .or_else(|| it.default_pin_direction(pin_id_local));
    // Check pullup: canvas runtime state first, then Part method.
    let has_pullup = ctx.canvas.is_pin_pullup(&pin.id)
        || it.default_pin_pullup(pin_id_local)
        || it.kind.is_pin_default_pullup(pin_id_local);

    d.push(pin.local.x, pin.local.y, (180 - pin.angle) as f64, 1.0, 1.0);
    let x = if connected { 0.0 } else { 2.0 };
    let w = (pin.length - if connected { 0.0 } else { 2.0 }).max(0.5);
    d.fill_rect(x, -0.5, w, COMPONENT_BORDER_WIDTH, stroke);

    let is_directional =
        !pin.unused && ctx.canvas.circ_settings().animate_logic && direction.is_some();
    if is_directional {
        match direction {
            Some(crate::canvas::PinDirection::In) => {
                let pts = [[0.0, -2.0], [2.0, 0.0], [0.0, 2.0]];
                d.stroke_poly(&pts, stroke, COMPONENT_BORDER_WIDTH, false);
            }
            Some(crate::canvas::PinDirection::OpenCo) => {
                d.line(2.0, -2.0, 0.0, 0.0, stroke, COMPONENT_BORDER_WIDTH);
            }
            Some(crate::canvas::PinDirection::Out) => {
                let pts = [[2.0, -2.0], [0.0, 0.0], [2.0, 2.0]];
                d.stroke_poly(&pts, stroke, COMPONENT_BORDER_WIDTH, false);
            }
            None => {}
        }
    } else if !connected {
        d.stroke_circle(0.0, 0.0, 2.0, stroke, COMPONENT_BORDER_WIDTH);
        if pin.unused {
            d.fill_circle(0.0, 0.0, 0.8, stroke);
        }
    }

    if has_pullup {
        let px = (pin.length - 3.9).max(0.0) + 1.5;
        d.fill_circle(px, 0.0, 1.5, ctx.pal.pin_open_high);
    }

    d.pop();
}
