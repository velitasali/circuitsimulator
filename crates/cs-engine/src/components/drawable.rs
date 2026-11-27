//! Per-component visual rendering trait.

use crate::canvas::draw::{Align, Color, Draw, PaintCtx, Palette, parse_hex};
use crate::canvas::{ImageBuf, Rect};
use crate::plot::{Channel, PlotBuffer};
use crate::theme::{COMPONENT_BORDER_WIDTH, COMPONENT_FILL_ALPHA, ColorId, ColorTheme};

fn scope_color(id: ColorId) -> Color {
    let (r, g, b, a) = ColorTheme::get_rgba(id, true);
    Color { r, g, b, a }
}

/// Extra radius the warning pulse adds on top of `radius * 2`.
const BULB_HALO_PULSE: f64 = 6.0;

/// Largest halo [`paint_indicator_bulb`] draws (warning pulse at peak).
pub fn indicator_bulb_halo_radius(radius: f64) -> f64 {
    radius * 2.0 + BULB_HALO_PULSE
}

/// Local AABB of the glow halo around `(cx, cy)`.
pub fn indicator_bulb_visual_rect(cx: f64, cy: f64, radius: f64) -> Rect {
    let r = indicator_bulb_halo_radius(radius);
    Rect::new(cx - r, cy - r, r * 2.0, r * 2.0)
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BulbState {
    pub is_lit: bool,
    pub intensity: f64,
    pub is_warning: bool,
    pub is_crashed: bool,
}

impl BulbState {
    pub const fn unlit() -> Self {
        Self {
            is_lit: false,
            intensity: 0.0,
            is_warning: false,
            is_crashed: false,
        }
    }
}

pub fn paint_indicator_bulb(
    d: &mut dyn Draw,
    ctx: &PaintCtx,
    cx: f64,
    cy: f64,
    radius: f64,
    color: Color,
    state: &BulbState,
) {
    if state.is_crashed {
        // Blown / destroyed bulb (no glow, dark body)
        d.fill_circle(cx, cy, radius, Color::rgb(25, 25, 25));
        d.stroke_circle(cx, cy, radius, ctx.pal.border, COMPONENT_BORDER_WIDTH);
    } else if state.is_warning {
        // Warning orange body fill underneath (lower z-index)
        let phase = (ctx.canvas.anim_tick() as f64 * 0.25).sin() * 0.5 + 0.5; // 0.0 .. 1.0
        let r = (255.0 * (0.85 + 0.15 * phase)).round() as u8;
        let g = (140.0 * (0.85 + 0.15 * phase)).round() as u8;
        d.fill_circle(cx, cy, radius, Color::rgb(r, g, 0));

        // Overcurrent warning: intense pulsing warning glow halo ON TOP (higher z-index)
        let r_halo = radius * 2.0 + BULB_HALO_PULSE * phase;
        let alpha_mod = 0.7 + 0.3 * phase;
        d.fill_circle(
            cx,
            cy,
            r_halo,
            Color::rgb(255, 100, 0).fade(0.18 * alpha_mod),
        );
        d.fill_circle(
            cx,
            cy,
            r_halo * 0.75,
            Color::rgb(255, 140, 0).fade(0.35 * alpha_mod),
        );
        d.fill_circle(
            cx,
            cy,
            radius * 1.5,
            Color::rgb(255, 160, 0).fade(0.55 * alpha_mod),
        );
        d.fill_circle(
            cx,
            cy,
            radius * 1.2,
            Color::rgb(255, 180, 0).fade(0.75 * alpha_mod),
        );
    } else if state.is_lit {
        let halo_scale = state.intensity.clamp(0.2, 1.0);

        // 1. Lower z-index: Lit bulb disk base underneath
        d.fill_circle(cx, cy, radius, color);

        // 2. Higher z-index: Glow halo drawn on top of the disk
        d.fill_circle(cx, cy, radius * 2.25, color.fade(0.12 * halo_scale));
        d.fill_circle(cx, cy, radius * 1.875, color.fade(0.25 * halo_scale));
        d.fill_circle(cx, cy, radius * 1.5, color.fade(0.42 * halo_scale));
        d.fill_circle(cx, cy, radius * 1.2, color.fade(0.65 * halo_scale));
    } else {
        // Unlit bulb body
        d.fill_circle(cx, cy, radius, ctx.pal.body.fade(0.55));
        d.stroke_circle(cx, cy, radius, ctx.pal.border, COMPONENT_BORDER_WIDTH);
    }
}

pub trait Drawable {
    fn paint(&self, _d: &mut dyn Draw, _ctx: &PaintCtx) -> bool {
        false
    }
}

pub fn paint_dial(d: &mut dyn Draw, pal: &Palette, val: f64, min_val: f64, max_val: f64) {
    let r_track = 13.0;
    d.arc(
        0.0,
        0.0,
        r_track,
        135.0_f64.to_radians(),
        405.0_f64.to_radians(),
        pal.border.fade(0.6),
        1.0,
    );
    for &deg in &[135.0, 202.5, 270.0, 337.5, 405.0] {
        let rad = (deg as f64).to_radians();
        let c = rad.cos();
        let s = rad.sin();
        d.line(
            (r_track - 1.5) * c,
            (r_track - 1.5) * s,
            (r_track + 1.5) * c,
            (r_track + 1.5) * s,
            pal.border.fade(0.7),
            1.0,
        );
    }
    d.fill_circle(0.0, 0.0, 10.0, pal.body.fade(0.5));
    d.stroke_circle(0.0, 0.0, 10.0, pal.border, COMPONENT_BORDER_WIDTH);
    let span = (max_val - min_val).max(1e-6);
    let norm = ((val - min_val) / span).clamp(0.0, 1.0);
    let angle = (norm * 270.0 - 225.0).to_radians();
    d.line(
        2.0 * angle.cos(),
        2.0 * angle.sin(),
        8.5 * angle.cos(),
        8.5 * angle.sin(),
        pal.border,
        COMPONENT_BORDER_WIDTH * 1.5,
    );
}

pub fn paint_var_source_body(
    d: &mut dyn Draw,
    pal: &Palette,
    val: f64,
    min_val: f64,
    max_val: f64,
    running: bool,
    unit: &str,
) {
    d.fill_round_rect(-20.0, -28.0, 40.0, 56.0, 2.0, pal.body.fade(0.3));
    d.stroke_round_rect(
        -20.0,
        -28.0,
        40.0,
        56.0,
        2.0,
        pal.border,
        COMPONENT_BORDER_WIDTH,
    );

    let cy = -8.0;
    let r_track = 13.0;
    d.arc(
        0.0,
        cy,
        r_track,
        135.0_f64.to_radians(),
        405.0_f64.to_radians(),
        pal.border.fade(0.6),
        1.0,
    );
    for &deg in &[135.0, 202.5, 270.0, 337.5, 405.0] {
        let rad = (deg as f64).to_radians();
        let c = rad.cos();
        let s = rad.sin();
        d.line(
            (r_track - 1.5) * c,
            cy + (r_track - 1.5) * s,
            (r_track + 1.5) * c,
            cy + (r_track + 1.5) * s,
            pal.border.fade(0.7),
            1.0,
        );
    }
    d.fill_circle(
        0.0,
        cy,
        10.0,
        pal.body.fade(crate::theme::COMPONENT_FILL_ALPHA),
    );
    d.stroke_circle(0.0, cy, 10.0, pal.border, COMPONENT_BORDER_WIDTH);

    let span = (max_val - min_val).abs().max(1e-6);
    let lo = min_val.min(max_val);
    let norm = ((val - lo) / span).clamp(0.0, 1.0);
    let angle = (norm * 270.0 - 225.0).to_radians();
    d.line(
        2.0 * angle.cos(),
        cy + 2.0 * angle.sin(),
        8.5 * angle.cos(),
        cy + 8.5 * angle.sin(),
        pal.border,
        COMPONENT_BORDER_WIDTH * 1.5,
    );

    let btn_bg = if running {
        pal.body.fade(0.6)
    } else {
        pal.body.fade(0.15)
    };
    d.fill_round_rect(-16.0, 10.0, 32.0, 14.0, 2.0, btn_bg);
    let btn_border = if running {
        pal.border
    } else {
        pal.border.fade(0.4)
    };
    d.stroke_round_rect(
        -16.0,
        10.0,
        32.0,
        14.0,
        2.0,
        btn_border,
        COMPONENT_BORDER_WIDTH,
    );

    let label = if running {
        crate::units::format_si_precision(val, unit, 2)
    } else {
        format!("--- {}", unit)
    };
    let text_color = if running {
        pal.border
    } else {
        pal.border.fade(0.5)
    };
    d.text(
        0.0,
        17.0,
        &label,
        7.0,
        text_color,
        crate::canvas::draw::Align::Center,
    );
}

pub fn paint_diac_body(d: &mut dyn Draw, pal: &Palette) {
    let t1 = [[8.0, -8.0], [-8.0, -15.0], [-8.0, 0.0]];
    let t2 = [[-8.0, 8.0], [8.0, 0.0], [8.0, 15.0]];
    d.fill_poly(&t1, pal.body.fade(crate::theme::COMPONENT_FILL_ALPHA));
    d.stroke_poly(&t1, pal.border, COMPONENT_BORDER_WIDTH, true);
    d.fill_poly(&t2, pal.body.fade(crate::theme::COMPONENT_FILL_ALPHA));
    d.stroke_poly(&t2, pal.border, COMPONENT_BORDER_WIDTH, true);
    d.line(-8.0, -16.0, -8.0, 16.0, pal.border, COMPONENT_BORDER_WIDTH);
    d.line(8.0, -16.0, 8.0, 16.0, pal.border, COMPONENT_BORDER_WIDTH);
}

/// Layout geometry for on-canvas plot instruments (`Oscope` and `LogicAnalyzer`).
/// Preserves the default `135×135` silhouette while scaling chassis and screen
/// dynamically with `Basic_X` and `Basic_Y`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlotLayout {
    pub chassis: Rect,
    pub screen: (f64, f64, f64, f64),
    pub inner: (f64, f64, f64, f64),
}

pub fn plot_layout(basic_x: i32, basic_y: i32) -> PlotLayout {
    let screen_w = (basic_x as f64 - 5.0).max(16.0);
    let screen_h = (basic_y as f64 - 7.0).max(16.0);
    let screen_x = -6.0;
    let screen_y = -screen_h * 0.5;
    let chassis_x = -80.0;
    let chassis_w = 83.0 + screen_w;
    let chassis_h = (screen_h + 16.0).max(144.0);
    let chassis_y = -chassis_h * 0.5;
    let screen = (screen_x, screen_y, screen_w, screen_h);
    PlotLayout {
        chassis: Rect::new(chassis_x, chassis_y, chassis_w, chassis_h),
        screen,
        inner: oscope_inner_rect(screen),
    }
}

/// Chassis, labels, screen and reticle. `traces` are live samples (sim);
/// `None` leaves the screen empty. Frequency text is overlaid, not cached.
#[allow(clippy::too_many_arguments)]
pub fn paint_oscope(
    d: &mut dyn Draw,
    pal: &Palette,
    basic_x: i32,
    basic_y: i32,
    tunnels: &[&str],
    freqs: &[&str],
    traces: Option<&PlotBuffer>,
    volt_divs: &[f64; 4],
    volt_pos: &[f64; 4],
    tracks: i32,
    hidden: &[bool; 4],
    zoom: f64,
) {
    let layout = plot_layout(basic_x, basic_y);
    paint_oscope_chrome(d, pal, &layout, tunnels);
    paint_oscope_reticle(d, layout.inner, tracks, zoom);
    paint_oscope_freq_labels(d, pal, freqs);
    paint_oscope_waves(
        d,
        layout.inner,
        traces,
        volt_divs,
        volt_pos,
        tracks,
        hidden,
        zoom,
    );
}

fn paint_oscope_chrome(d: &mut dyn Draw, pal: &Palette, layout: &PlotLayout, tunnels: &[&str]) {
    // 1. Chassis
    let c = layout.chassis;
    d.fill_round_rect(c.x, c.y, c.w, c.h, 4.0, pal.body.fade(0.9));
    d.stroke_round_rect(c.x, c.y, c.w, c.h, 4.0, pal.border, COMPONENT_BORDER_WIDTH);

    // 2. Sidebar (Channels 0..3)
    for i in 0..4 {
        let y_center = -48.0 + 32.0 * (i as f64);
        let swatch = scope_color(ColorTheme::scope_channel_color_id(i));

        // Color swatch box (C++ QRectF(-76, yCenter - 6, 8, 12))
        d.fill_round_rect(-76.0, y_center - 6.0, 8.0, 12.0, 1.0, swatch);
        d.stroke_round_rect(-76.0, y_center - 6.0, 8.0, 12.0, 1.0, pal.border, 0.5);

        // Channel / tunnel label (C++ QRectF(-64, yCenter - 14, 54, 12))
        let ch_name = tunnels
            .get(i)
            .copied()
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("Ch{i}"));
        d.text(
            -64.0,
            y_center - 14.0,
            &ch_name,
            8.0,
            pal.border,
            crate::canvas::draw::Align::TopLeft,
        );
    }

    // Ground label (C++ QRectF(-76, 58, 60, 12))
    d.text(
        -76.0,
        58.0,
        "GND",
        8.0,
        pal.border,
        crate::canvas::draw::Align::TopLeft,
    );

    // 3. Screen
    let (screen_x, screen_y, screen_w, screen_h) = layout.screen;
    d.fill_round_rect(
        screen_x,
        screen_y,
        screen_w,
        screen_h,
        4.0,
        scope_color(ColorId::ScopeTube),
    );
    d.stroke_round_rect(
        screen_x,
        screen_y,
        screen_w,
        screen_h,
        4.0,
        scope_color(ColorId::ScopeBezel),
        COMPONENT_BORDER_WIDTH,
    );
}

/// Width in scene units that renders as `css_px` logical pixels regardless of
/// canvas zoom. The canvas transform multiplies scene-unit widths by zoom, so
/// dividing by the viewport zoom keeps logical width constant. `zoom` is
/// `PaintCtx::scale`.
fn scope_screen_width(zoom: f64, css_px: f64) -> f64 {
    css_px / zoom.max(0.1)
}

fn paint_oscope_reticle(d: &mut dyn Draw, inner: (f64, f64, f64, f64), tracks: i32, zoom: f64) {
    let px = 1.0 / zoom.max(0.1);
    paint_scope_grid(
        d,
        inner,
        tracks,
        scope_color(ColorId::ScopeSymbolGrid),
        scope_color(ColorId::ScopeSymbolAccent),
        px,
        1.5,
    );
}

fn paint_oscope_freq_labels(d: &mut dyn Draw, pal: &Palette, freqs: &[&str]) {
    for i in 0..4 {
        let Some(&freq) = freqs.get(i) else {
            continue;
        };
        let trimmed = freq.trim();
        if trimmed.is_empty() || trimmed == "0 Hz" {
            continue;
        }
        let y_center = -48.0 + 32.0 * (i as f64);
        d.text(
            -64.0,
            y_center - 1.0,
            trimmed,
            7.0,
            pal.border.fade(0.8),
            crate::canvas::draw::Align::TopLeft,
        );
    }
}

fn oscope_inner_rect(screen: (f64, f64, f64, f64)) -> (f64, f64, f64, f64) {
    let (screen_x, screen_y, screen_w, screen_h) = screen;
    (
        screen_x + 4.0,
        screen_y + 4.0,
        (screen_w - 8.0).max(2.0),
        (screen_h - 8.0).max(2.0),
    )
}

fn scope_track_count(tracks: i32) -> usize {
    match tracks {
        2 => 2,
        4 => 4,
        _ => 1,
    }
}

/// Vertical division height in the expanded oscilloscope window. Scaling by
/// `1 / tracks` keeps the same number of vertical divisions per track across
/// 1-, 2-, and 4-track modes, matching the on-canvas oscilloscope symbol.
fn scope_v_div_px(cell: f64, tracks: i32) -> f64 {
    cell / scope_track_count(tracks) as f64
}

/// Graticule in `rect` = `(x, y, w, h)`. `px` scales line widths. `tick` is the
/// crosshair tick length in the same units as `rect`.
fn paint_scope_grid(
    d: &mut dyn Draw,
    rect: (f64, f64, f64, f64),
    tracks: i32,
    grid: Color,
    accent: Color,
    px: f64,
    tick: f64,
) {
    let (x0, y0, w, h) = rect;
    if w <= 1.0 || h <= 1.0 {
        return;
    }
    let end_x = x0 + w;
    let end_y = y0 + h;
    let h_center = x0 + w * 0.5;
    let num_tracks = scope_track_count(tracks);
    let v_divs = 10 * num_tracks;
    let mut centers = [0.0; 4];
    for t in 0..num_tracks {
        centers[t] = y0 + (t as f64 + 0.5) * h / (num_tracks as f64);
    }
    let track_centers = &centers[..num_tracks];

    for gy in 1..v_divs {
        let y = y0 + h * (gy as f64) / (v_divs as f64);
        let is_center = track_centers.iter().any(|&tc| (y - tc).abs() < 0.5);
        let is_boundary = num_tracks > 1 && gy % 10 == 0;
        let c = if is_center || is_boundary {
            accent
        } else {
            grid
        };
        let lw = px * if is_center || is_boundary { 1.2 } else { 0.75 };
        d.line(x0, y, end_x, y, c, lw);
    }
    for gx in 1..10 {
        let x = x0 + w * (gx as f64) / 10.0;
        let is_center = (x - h_center).abs() < 0.5;
        let c = if is_center { accent } else { grid };
        let lw = px * if is_center { 1.2 } else { 0.75 };
        d.line(x, y0, x, end_y, c, lw);
    }

    let tick_w = px * 0.75;
    for &tc in track_centers {
        for tx in 0..=50 {
            let x = x0 + w * (tx as f64) / 50.0;
            d.line(x, tc - tick, x, tc + tick, accent, tick_w);
        }
    }
    let ym = if num_tracks == 1 {
        50
    } else if num_tracks == 2 {
        40
    } else {
        20
    };
    for ty in 0..=ym {
        let y = y0 + h * (ty as f64) / (ym as f64);
        d.line(h_center - tick, y, h_center + tick, y, accent, tick_w);
    }
}

/// `(fraction, value)` vertices the stroke actually draws. Timed `plot` points
/// win. Otherwise the uniform `samples` grid is stretched across the width.
fn trace_vertices(ch: &Channel) -> Vec<(f64, f64)> {
    if ch.plot.len() >= 2 && ch.plot.len() == ch.plot_x.len() {
        return ch
            .plot_x
            .iter()
            .copied()
            .zip(ch.plot.iter().copied())
            .collect();
    }
    let last = ch.samples.len().saturating_sub(1).max(1) as f64;
    ch.samples
        .iter()
        .enumerate()
        .map(|(k, &v)| (k as f64 / last, v))
        .collect()
}

fn trace_points(ch: &Channel, x0: f64, width: f64, y_of: impl Fn(f64) -> f64) -> Vec<[f64; 2]> {
    let verts = trace_vertices(ch);
    let mut pts = Vec::with_capacity(verts.len());
    for (x, v) in verts {
        pts.push([x0 + x * width, y_of(v)]);
    }
    pts
}

/// Scope trace Y. `div_px` is one division in the same units as `track_center_y`.
fn scope_trace_y(v: f64, track_center_y: f64, volt_pos: f64, div_px: f64, volt_div: f64) -> f64 {
    let scale_y = div_px.max(1e-6) / volt_div.max(1e-12);
    track_center_y - (v - volt_pos) * scale_y
}

/// Logical pixels from a logic-analyzer row edge to the drawn high or low level.
/// The window multiplies this by the device pixel ratio.
pub const LA_TRACE_INSET: f64 = 3.0;

fn la_trace_y(v: f64, row_y: f64, row_h: f64, inset: f64) -> f64 {
    let bit = v.clamp(0.0, 1.0);
    row_y + row_h - inset - bit * (row_h - 2.0 * inset)
}

#[allow(clippy::too_many_arguments)]
fn paint_oscope_waves(
    d: &mut dyn Draw,
    rect: (f64, f64, f64, f64),
    traces: Option<&PlotBuffer>,
    volt_divs: &[f64; 4],
    volt_pos: &[f64; 4],
    tracks: i32,
    hidden: &[bool; 4],
    zoom: f64,
) {
    let track_h = rect.3 / scope_track_count(tracks) as f64;
    paint_scope_waves(
        d,
        rect,
        traces,
        volt_divs,
        volt_pos,
        tracks,
        hidden,
        1.0 / zoom.max(0.1),
        track_h / 10.0,
        false,
    );
}

fn channel_extent(ch: &Channel) -> Option<(f64, f64)> {
    let src = if ch.plot.len() >= 2 {
        &ch.plot
    } else {
        &ch.samples
    };
    let mut min_v = f64::INFINITY;
    let mut max_v = f64::NEG_INFINITY;
    for &v in src {
        if v < min_v {
            min_v = v;
        }
        if v > max_v {
            max_v = v;
        }
    }
    if min_v <= max_v {
        Some((min_v, max_v))
    } else {
        None
    }
}

fn paint_scope_waves(
    d: &mut dyn Draw,
    rect: (f64, f64, f64, f64),
    traces: Option<&PlotBuffer>,
    volt_divs: &[f64; 4],
    volt_pos: &[f64; 4],
    tracks: i32,
    hidden: &[bool; 4],
    px: f64,
    div_px: f64,
    expanded: bool,
) {
    let Some(buf) = traces else {
        return;
    };
    let (x0, y0, w, h) = rect;
    if w <= 1.0 || h <= 1.0 {
        return;
    }
    let num_tracks = scope_track_count(tracks);
    let track_h = h / (num_tracks as f64);

    for (c, ch) in buf.channels.iter().enumerate().take(4) {
        if hidden.get(c).copied().unwrap_or(false) || !ch.connected || ch.samples.len() < 2 {
            continue;
        }
        let track_idx = c % num_tracks;
        let track_top = y0 + (track_idx as f64) * track_h;
        let track_bot = track_top + track_h;
        let track_center_y = track_top + 0.5 * track_h;
        let vd = volt_divs.get(c).copied().unwrap_or(1.0);
        let vp = volt_pos.get(c).copied().unwrap_or(0.0);
        let color = parse_hex(&ch.color);
        let y_of = |v: f64| scope_trace_y(v, track_center_y, vp, div_px, vd);

        d.push_clip_rect(x0, track_top, w, track_h);

        let pts = trace_points(ch, x0, w, y_of);
        d.polyline(&pts, color, px * 1.5, false);

        if expanded {
            if let Some((min_v, max_v)) = channel_extent(ch) {
                let end_x = x0 + w;
                let y_max = y_of(max_v);
                let y_min = y_of(min_v);
                let label_x = x0 + if c % 2 == 0 { 4.0 * px } else { w - 50.0 * px };
                if (track_top..=track_bot).contains(&y_max) {
                    d.polyline(&[[x0, y_max], [end_x, y_max]], color, px * 0.5, true);
                    d.text_bold(
                        label_x,
                        (y_max - 12.0 * px).max(track_top),
                        &format!("{max_v:.2}V"),
                        9.0 * px,
                        color,
                        Align::TopLeft,
                    );
                }
                if (track_top..=track_bot).contains(&y_min) {
                    d.polyline(&[[x0, y_min], [end_x, y_min]], color, px * 0.5, true);
                    d.text_bold(
                        label_x,
                        (y_min + 2.0 * px).min(track_bot - 10.0 * px).max(track_top),
                        &format!("{min_v:.2}V"),
                        9.0 * px,
                        color,
                        Align::TopLeft,
                    );
                }
            }
        }

        d.pop_clip();
    }
}

/// Positions `origin + k * step` that fall inside `(min, max)`. `k = 0` is the center.
fn for_each_grid_pos(origin: f64, step: f64, min: f64, max: f64, mut f: impl FnMut(f64, bool)) {
    if !step.is_finite() || step <= 0.0 {
        return;
    }
    if origin > min && origin < max {
        f(origin, true);
    }
    let mut k = 1i32;
    while k < 8192 {
        let neg = origin - f64::from(k) * step;
        let pos = origin + f64::from(k) * step;
        let neg_in = neg > min && neg < max;
        let pos_in = pos > min && pos < max;
        if neg_in {
            f(neg, false);
        }
        if pos_in {
            f(pos, false);
        }
        if !neg_in && !pos_in {
            break;
        }
        k += 1;
    }
}

/// Expanded-scope graticule. Horizontal divisions are `cell` pixels wide,
/// anchored on the trigger at the horizontal center. Vertical divisions scale
/// by `1 / tracks` so each track keeps the same division count across 1-, 2-,
/// and 4-track modes, anchored on each track's volt-position line.
fn paint_scope_window_grid(
    d: &mut dyn Draw,
    rect: (f64, f64, f64, f64),
    tracks: i32,
    grid: Color,
    accent: Color,
    px: f64,
    tick: f64,
    cell: f64,
) {
    let (x0, y0, w, h) = rect;
    if w <= 1.0 || h <= 1.0 || cell <= 0.0 {
        return;
    }
    let end_x = x0 + w;
    let end_y = y0 + h;
    let h_center = x0 + w * 0.5;
    let num_tracks = scope_track_count(tracks);
    let v_cell = scope_v_div_px(cell, tracks);
    let mut centers = [0.0; 4];
    for t in 0..num_tracks {
        centers[t] = y0 + (t as f64 + 0.5) * h / (num_tracks as f64);
    }
    let track_centers = &centers[..num_tracks];
    let major = px * 0.75;
    let center_w = px * 1.2;
    let band_h = h / num_tracks as f64;

    for_each_grid_pos(h_center, cell, x0, end_x, |x, center| {
        let c = if center { accent } else { grid };
        let lw = if center { center_w } else { major };
        d.line(x, y0, x, end_y, c, lw);
    });
    for (t, &tc) in track_centers.iter().enumerate() {
        let band_top = y0 + t as f64 * band_h;
        let band_bot = band_top + band_h;
        for_each_grid_pos(tc, v_cell, band_top, band_bot, |y, center| {
            let c = if center { accent } else { grid };
            let lw = if center { center_w } else { major };
            d.line(x0, y, end_x, y, c, lw);
        });
    }
    for t in 1..num_tracks {
        let y = y0 + t as f64 * band_h;
        d.line(x0, y, end_x, y, accent, center_w);
    }

    let tick_w = px * 0.75;
    let h_minor = cell / 5.0;
    let v_sub = if num_tracks >= 4 { 2.0 } else { 5.0 };
    let v_minor = v_cell / v_sub;
    for &tc in track_centers {
        for_each_grid_pos(h_center, h_minor, x0, end_x, |x, _| {
            d.line(x, tc - tick, x, tc + tick, accent, tick_w);
        });
    }
    for (t, &tc) in track_centers.iter().enumerate() {
        let band_top = y0 + t as f64 * band_h;
        let band_bot = band_top + band_h;
        for_each_grid_pos(tc, v_minor, band_top, band_bot, |y, _| {
            d.line(h_center - tick, y, h_center + tick, y, accent, tick_w);
        });
    }
}

/// Scope window picture: graticule, traces, and the expanded-view min/max marks.
/// Coordinates are the item's device-pixel rectangle. `px` is the device pixel
/// ratio, so one horizontal division stays [`crate::instruments::SCOPE_DIV_PX`]
/// logical pixels and vertical divisions scale by `1 / tracks`. The caller clears
/// with `ScopePaper`.
pub fn paint_scope_screen(
    d: &mut dyn Draw,
    width: f64,
    height: f64,
    traces: Option<&PlotBuffer>,
    volt_divs: &[f64; 4],
    volt_pos: &[f64; 4],
    tracks: i32,
    hidden: &[bool; 4],
    px: f64,
) {
    let px = px.max(0.1);
    let rect = (0.0, 0.0, width, height);
    let grid = scope_color(ColorId::ScopeWindowGrid);
    let accent = scope_color(ColorId::ScopeWindowAccent);
    let cell = crate::instruments::SCOPE_DIV_PX * px;
    let v_cell = scope_v_div_px(cell, tracks);
    paint_scope_window_grid(d, rect, tracks, grid, accent, px, 1.5 * px, cell);
    let mid = width * 0.5;
    d.line(
        mid,
        0.0,
        mid,
        height,
        scope_color(ColorId::ScopeCursor),
        px * 1.5,
    );
    paint_scope_waves(
        d, rect, traces, volt_divs, volt_pos, tracks, hidden, px, v_cell, true,
    );
}

/// Eight-row, ten-column graticule in `rect` = `(x, y, w, h)`.
fn paint_la_grid(d: &mut dyn Draw, rect: (f64, f64, f64, f64), grid: Color, grid_w: f64) {
    let (x0, y0, w, h) = rect;
    if w <= 1.0 || h <= 1.0 {
        return;
    }
    let end_x = x0 + w;
    let end_y = y0 + h;
    for gy in 1..8 {
        let y = y0 + h * (gy as f64) / 8.0;
        d.line(x0, y, end_x, y, grid, grid_w);
    }
    for gx in 1..10 {
        let x = x0 + w * (gx as f64) / 10.0;
        d.line(x, y0, x, end_y, grid, grid_w);
    }
}

/// Digital traces in `rect`. `inset` is the gap from a row edge to the high or low level.
/// A channel with a timed `plot` is drawn even when `samples` is shorter than two points.
fn paint_la_waves(
    d: &mut dyn Draw,
    rect: (f64, f64, f64, f64),
    traces: Option<&PlotBuffer>,
    inset: f64,
    trace_w: f64,
) {
    let Some(buf) = traces else {
        return;
    };
    let (x0, y0, w, h) = rect;
    if w <= 1.0 || h <= 1.0 {
        return;
    }
    let rows = crate::instruments::LA_WINDOW_ROWS;
    let row_h = h / rows as f64;
    for (c, ch) in buf.channels.iter().enumerate().take(rows) {
        if !ch.connected {
            continue;
        }
        let has_plot = ch.plot.len() >= 2 && ch.plot.len() == ch.plot_x.len();
        if !has_plot && ch.samples.len() < 2 {
            continue;
        }
        let row_y = y0 + (c as f64) * row_h;
        let pts = trace_points(ch, x0, w, |v| la_trace_y(v, row_y, row_h, inset));
        d.polyline(&pts, parse_hex(&ch.color), trace_w, false);
    }
}

/// Expanded logic-analyzer graticule. Eight rows fill the height. Time columns
/// are as wide as a row and are centered on the trigger.
fn paint_la_window_grid(d: &mut dyn Draw, rect: (f64, f64, f64, f64), grid: Color, grid_w: f64) {
    let (x0, y0, w, h) = rect;
    if w <= 1.0 || h <= 1.0 {
        return;
    }
    let rows = crate::instruments::LA_WINDOW_ROWS;
    let end_x = x0 + w;
    let end_y = y0 + h;
    let row_h = h / rows as f64;
    for gy in 1..rows {
        let y = y0 + h * (gy as f64) / (rows as f64);
        d.line(x0, y, end_x, y, grid, grid_w);
    }
    let mid = x0 + w * 0.5;
    for_each_grid_pos(mid, row_h, x0, end_x, |x, _| {
        d.line(x, y0, x, end_y, grid, grid_w);
    });
}

/// Logic-analyzer window. Eight rows fill the height and time columns match
/// that row height, so the window shows as much time as fits.
/// Coordinates are the item's pixel rectangle. `px` is the device pixel ratio,
/// so a 1.5 px trace stays 1.5 logical pixels. The caller clears with `ScopePaper`.
pub fn paint_la_screen(
    d: &mut dyn Draw,
    width: f64,
    height: f64,
    traces: Option<&PlotBuffer>,
    px: f64,
) {
    let px = px.max(0.1);
    let rect = (0.0, 0.0, width, height);
    paint_la_window_grid(d, rect, scope_color(ColorId::ScopeWindowGrid), px);
    paint_la_waves(d, rect, traces, LA_TRACE_INSET * px, px * 1.5);
}

/// One point on a painted trace. `x` and `y` use the same coordinates as the
/// width and height passed to the query. For a window that is the device-pixel
/// picture, which the façade maps back to logical pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TraceHit {
    pub channel: usize,
    /// Time across the window, 0 at the left edge and 1 at the right.
    pub fraction: f64,
    /// Volts on the scope. Drawn 0 or 1 level on the logic analyzer.
    pub value: f64,
    pub x: f64,
    pub y: f64,
}

const TIME_EPS: f64 = 1e-9;

/// Levels of `vertices` at `fraction`. A slope or a hold contributes one value.
/// A min/max band contributes its low and high edges. A vertical step contributes
/// the level the stroke continues with on the right of that instant.
fn levels_at(vertices: &[(f64, f64)], fraction: f64) -> Vec<f64> {
    let fraction = fraction.clamp(0.0, 1.0);
    let mut spans = Vec::new();
    for pair in vertices.windows(2) {
        let (f0, v0) = pair[0];
        let (f1, v1) = pair[1];
        if (f1 - f0).abs() <= TIME_EPS {
            continue;
        }
        let lo = f0.min(f1);
        let hi = f0.max(f1);
        if fraction < lo - TIME_EPS || fraction > hi + TIME_EPS {
            continue;
        }
        spans.push((f0, f1, v0, v1));
    }
    let interior = spans.iter().any(|&(f0, f1, _, _)| {
        let lo = f0.min(f1);
        let hi = f0.max(f1);
        fraction > lo + TIME_EPS && fraction < hi - TIME_EPS
    });
    let any_right = spans
        .iter()
        .any(|&(f0, f1, _, _)| f0.max(f1) > fraction + TIME_EPS);
    let mut values = Vec::new();
    for &(f0, f1, v0, v1) in &spans {
        let lo = f0.min(f1);
        let hi = f0.max(f1);
        let covers = if interior {
            fraction > lo + TIME_EPS && fraction < hi - TIME_EPS
        } else if any_right {
            hi > fraction + TIME_EPS
        } else {
            true
        };
        if !covers {
            continue;
        }
        let t = ((fraction - f0) / (f1 - f0)).clamp(0.0, 1.0);
        let value = v0 + t * (v1 - v0);
        if values.iter().all(|v: &f64| (v - value).abs() > 1e-6) {
            values.push(value);
        }
    }
    if values.is_empty() {
        let mut held = None;
        for &(f, v) in vertices {
            if f <= fraction + TIME_EPS {
                held = Some(v);
            }
        }
        if let Some(v) = held {
            values.push(v);
        }
    }
    // A step on the right edge has no span past that instant. The vertical's
    // end is the level the stroke finishes on. A band still has two edges, so
    // a single incoming level is what gets replaced.
    if !interior && !any_right && values.len() == 1 {
        let pre = values[0];
        let mut rise = None;
        for pair in vertices.windows(2) {
            let (f0, v0) = pair[0];
            let (f1, v1) = pair[1];
            if (f1 - f0).abs() <= TIME_EPS
                && (f0 - fraction).abs() <= TIME_EPS
                && (v0 - pre).abs() <= 1e-6
                && (v1 - v0).abs() > 1e-6
            {
                rise = Some(v1);
            }
        }
        if let Some(v) = rise {
            values.clear();
            values.push(v);
        }
    }
    values.sort_by(f64::total_cmp);
    values
}

fn push_levels(
    out: &mut Vec<TraceHit>,
    channel: usize,
    fraction: f64,
    width: f64,
    values: &[f64],
    y_of: impl Fn(f64) -> f64,
) {
    let fraction = fraction.clamp(0.0, 1.0);
    let x = fraction * width;
    for &value in values {
        out.push(TraceHit {
            channel,
            fraction,
            value,
            x,
            y: y_of(value),
        });
    }
}

/// Every visible scope level at `fraction` (0 at the left edge, 1 at the right).
/// Coordinates match [`paint_scope_screen`]: `div_px` is one division, and a
/// channel drawn on a track uses that track's center. Hidden, disconnected, and
/// short channels are skipped the same way as the painter. A min/max band
/// returns both edges. `y` may fall outside the window when the volts do.
pub fn scope_signal_at(
    width: f64,
    height: f64,
    fraction: f64,
    div_px: f64,
    traces: &PlotBuffer,
    volt_divs: &[f64; 4],
    volt_pos: &[f64; 4],
    tracks: i32,
    hidden: &[bool; 4],
) -> Vec<TraceHit> {
    if width <= 1.0 || height <= 1.0 {
        return Vec::new();
    }
    let num_tracks = scope_track_count(tracks);
    let track_div_px = scope_v_div_px(div_px, tracks);
    let mut out = Vec::new();
    for (c, ch) in traces.channels.iter().enumerate().take(4) {
        if hidden.get(c).copied().unwrap_or(false) || !ch.connected || ch.samples.len() < 2 {
            continue;
        }
        let track_idx = c % num_tracks;
        let track_h = height / num_tracks as f64;
        let track_center_y = (track_idx as f64 + 0.5) * track_h;
        let vd = volt_divs.get(c).copied().unwrap_or(1.0);
        let vp = volt_pos.get(c).copied().unwrap_or(0.0);
        let y_of = |v: f64| scope_trace_y(v, track_center_y, vp, track_div_px, vd);
        let values = levels_at(&trace_vertices(ch), fraction);
        push_levels(&mut out, c, fraction, width, &values, y_of);
    }
    out
}

/// Every visible logic-analyzer level at `fraction`. Coordinates and `inset`
/// match [`paint_la_screen`]. The value is the drawn level in 0..1. A step
/// uses the level the stroke continues with. A band returns both edges.
pub fn la_signal_at(
    width: f64,
    height: f64,
    fraction: f64,
    inset: f64,
    traces: &PlotBuffer,
) -> Vec<TraceHit> {
    if width <= 1.0 || height <= 1.0 {
        return Vec::new();
    }
    let rows = crate::instruments::LA_WINDOW_ROWS;
    let row_h = height / rows as f64;
    let mut out = Vec::new();
    for (c, ch) in traces.channels.iter().enumerate().take(rows) {
        if !ch.connected {
            continue;
        }
        let has_plot = ch.plot.len() >= 2 && ch.plot.len() == ch.plot_x.len();
        if !has_plot && ch.samples.len() < 2 {
            continue;
        }
        let row_y = c as f64 * row_h;
        let domain: Vec<(f64, f64)> = trace_vertices(ch)
            .into_iter()
            .map(|(fraction, v)| (fraction, v.clamp(0.0, 1.0)))
            .collect();
        let y_of = |bit: f64| la_trace_y(bit, row_y, row_h, inset);
        let values = levels_at(&domain, fraction);
        push_levels(&mut out, c, fraction, width, &values, y_of);
    }
    out
}

pub fn paint_lanalizer(
    d: &mut dyn Draw,
    pal: &Palette,
    basic_x: i32,
    basic_y: i32,
    tunnels: &[&str],
    traces: Option<&PlotBuffer>,
    zoom: f64,
) {
    let layout = plot_layout(basic_x, basic_y);
    let c = layout.chassis;

    // 1. Chassis
    d.fill_round_rect(c.x, c.y, c.w, c.h, 4.0, pal.body.fade(0.9));
    d.stroke_round_rect(c.x, c.y, c.w, c.h, 4.0, pal.border, COMPONENT_BORDER_WIDTH);

    // 2. Sidebar (Channels 0..7)
    for i in 0..8 {
        let y_center = -64.0 + 16.0 * (i as f64);
        let swatch = scope_color(ColorTheme::la_channel_color_id(i));
        // Swatch box (C++ QRectF(-76, yCenter - 5, 8, 10))
        d.fill_round_rect(-76.0, y_center - 5.0, 8.0, 10.0, 1.0, swatch);
        d.stroke_round_rect(-76.0, y_center - 5.0, 8.0, 10.0, 1.0, pal.border, 0.5);

        // Channel / tunnel label
        let ch_name = tunnels
            .get(i)
            .copied()
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("Ch{i}"));
        d.text(
            -64.0,
            y_center - 6.0,
            &ch_name,
            7.0,
            pal.border,
            crate::canvas::draw::Align::TopLeft,
        );
    }

    // 3. Screen
    let (screen_x, screen_y, screen_w, screen_h) = layout.screen;
    d.fill_round_rect(
        screen_x,
        screen_y,
        screen_w,
        screen_h,
        4.0,
        scope_color(ColorId::ScopeTube),
    );
    d.stroke_round_rect(
        screen_x,
        screen_y,
        screen_w,
        screen_h,
        4.0,
        scope_color(ColorId::ScopeBezel),
        COMPONENT_BORDER_WIDTH,
    );

    let inner = layout.inner;
    paint_la_grid(
        d,
        inner,
        scope_color(ColorId::ScopeSymbolGrid),
        scope_screen_width(zoom, 0.75),
    );
    paint_la_waves(d, inner, traces, 3.0, scope_screen_width(zoom, 1.2));
}

#[allow(dead_code)]
pub fn paint_plot(d: &mut dyn Draw, pal: &Palette) {
    paint_oscope(
        d,
        pal,
        135,
        135,
        &[],
        &[],
        None,
        &[1.0; 4],
        &[0.0; 4],
        1,
        &[false; 4],
        1.0,
    );
}

pub fn paint_chip_body(
    d: &mut dyn Draw,
    pal: &Palette,
    r: crate::canvas::Rect,
    ic_label: &str,
    rotate: bool,
) {
    d.fill_round_rect(r.x, r.y, r.w, r.h, 2.0, pal.body.fade(COMPONENT_FILL_ALPHA));
    d.stroke_round_rect(r.x, r.y, r.w, r.h, 2.0, pal.border, COMPONENT_BORDER_WIDTH);
    if !ic_label.is_empty() {
        if rotate {
            d.push(0.0, 0.0, -90.0, 1.0, 1.0);
            d.text(0.0, 0.0, ic_label, 7.0, pal.border, Align::Center);
            d.pop();
        } else {
            d.text(0.0, 0.0, ic_label, 7.0, pal.border, Align::Center);
        }
    }
}

pub fn paint_trapezoid_body(
    d: &mut dyn Draw,
    pal: &Palette,
    pts: &[[f64; 2]; 4],
    ic_label: &str,
    rotate: bool,
) {
    d.fill_poly(pts, pal.body.fade(COMPONENT_FILL_ALPHA));
    d.stroke_poly(pts, pal.border, COMPONENT_BORDER_WIDTH, true);
    if !ic_label.is_empty() {
        if rotate {
            d.push(0.0, 0.0, -90.0, 1.0, 1.0);
            d.text(0.0, 0.0, ic_label, 7.0, pal.border, Align::Center);
            d.pop();
        } else {
            d.text(0.0, 0.0, ic_label, 7.0, pal.border, Align::Center);
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn paint_dip_package(
    d: &mut dyn Draw,
    pal: &Palette,
    w: f64,
    h: f64,
    label: &str,
    logic_symbol: bool,
    custom_color: bool,
    bckgnd_color: &str,
    is_active: bool,
) {
    paint_dip_package_with_bg(
        d,
        pal,
        w,
        h,
        label,
        logic_symbol,
        custom_color,
        bckgnd_color,
        is_active,
        None,
        false,
    );
}

#[allow(clippy::too_many_arguments)]
pub fn paint_dip_package_with_bg(
    d: &mut dyn Draw,
    pal: &Palette,
    w: f64,
    h: f64,
    label: &str,
    logic_symbol: bool,
    custom_color: bool,
    bckgnd_color: &str,
    is_active: bool,
    background: Option<&ImageBuf>,
    border: bool,
) {
    if let Some(bg) = background {
        d.draw_pixmap_rect(0.0, 0.0, w, h, bg, 1.0);
        if border {
            d.stroke_round_rect(0.0, 0.0, w, h, 1.0, pal.border, COMPONENT_BORDER_WIDTH);
        }
    } else {
        let fill = if custom_color && !bckgnd_color.is_empty() {
            parse_hex(bckgnd_color)
        } else if logic_symbol {
            parse_hex("#ebf0ff")
        } else {
            parse_hex("#141e3c")
        };
        d.fill_round_rect(0.0, 0.0, w, h, 1.0, fill);
        d.stroke_round_rect(0.0, 0.0, w, h, 1.0, pal.border, COMPONENT_BORDER_WIDTH);
    }

    if !logic_symbol && background.is_none() {
        let notch_color = parse_hex("#aaaa96");
        if (w - h).abs() < 0.5 {
            if !is_active {
                d.stroke_circle(6.0, 6.0, 2.0, notch_color, COMPONENT_BORDER_WIDTH);
            }
        } else {
            d.arc(
                w * 0.5,
                0.0,
                4.0,
                0.0,
                std::f64::consts::PI,
                notch_color,
                COMPONENT_BORDER_WIDTH,
            );
        }
    }

    if is_active {
        let yellow = Color::rgb(255, 255, 0);
        if (w - h).abs() < 0.5 {
            d.fill_round_rect(4.0, 4.0, 4.0, 4.0, 2.0, yellow);
            d.stroke_round_rect(4.0, 4.0, 4.0, 4.0, 2.0, pal.border, 0.5);
        } else {
            d.fill_round_rect(w * 0.5 - 2.0, -1.0, 4.0, 4.0, 2.0, yellow);
            d.stroke_round_rect(w * 0.5 - 2.0, -1.0, 4.0, 4.0, 2.0, pal.border, 0.5);
        }
    }

    if !label.is_empty() {
        let color = if logic_symbol {
            crate::canvas::draw::parse_hex("#878778")
        } else {
            crate::canvas::draw::parse_hex("#c8c8b4")
        };
        if (w - h).abs() < 0.5 {
            d.text(
                w * 0.5,
                h * 0.5,
                label,
                5.0,
                color,
                crate::canvas::draw::Align::Center,
            );
        } else {
            d.push(w * 0.5, h * 0.5, -90.0, 1.0, 1.0);
            d.text(
                0.0,
                0.0,
                label,
                5.0,
                color,
                crate::canvas::draw::Align::Center,
            );
            d.pop();
        }
    }
}

#[cfg(test)]
mod trace_hit_tests {
    use super::{LA_TRACE_INSET, TraceHit, la_signal_at, scope_signal_at};
    use crate::instruments::SCOPE_DIV_PX;
    use crate::plot::{Channel, PlotBuffer};

    fn near(a: f64, b: f64) {
        assert!((a - b).abs() < 1e-6, "{a} != {b}");
    }

    fn channel(plot_x: &[f64], plot: &[f64]) -> Channel {
        Channel {
            color: "#ffffff".into(),
            samples: vec![0.0, 0.0],
            plot: plot.to_vec(),
            plot_x: plot_x.to_vec(),
            connected: true,
        }
    }

    fn scope(
        channels: Vec<Channel>,
        fraction: f64,
        tracks: i32,
        hidden: [bool; 4],
        volt_pos: [f64; 4],
    ) -> Vec<TraceHit> {
        scope_signal_at(
            400.0,
            200.0,
            fraction,
            SCOPE_DIV_PX,
            &PlotBuffer { channels, n: 2 },
            &[1.0; 4],
            &volt_pos,
            tracks,
            &hidden,
        )
    }

    #[test]
    fn slope_lands_on_the_line() {
        // y = 100 - volts * 48. From 0 V to 1 V the stroke runs (0, 100) → (400, 52).
        let hits = scope(
            vec![channel(&[0.0, 1.0], &[0.0, 1.0])],
            0.75,
            1,
            [false; 4],
            [0.0; 4],
        );
        assert_eq!(hits.len(), 1);
        near(hits[0].fraction, 0.75);
        near(hits[0].value, 0.75);
        near(hits[0].x, 300.0);
        near(hits[0].y, 64.0);
    }

    #[test]
    fn horizontal_hold_uses_the_trace_y() {
        let hits = scope(
            vec![channel(&[0.0, 1.0], &[1.5, 1.5])],
            0.25,
            1,
            [false; 4],
            [0.0; 4],
        );
        assert_eq!(hits.len(), 1);
        near(hits[0].value, 1.5);
        near(hits[0].y, 28.0);
        near(hits[0].x, 100.0);
    }

    #[test]
    fn step_keeps_the_level_that_continues() {
        let step = channel(&[0.0, 0.5, 0.5, 1.0], &[0.0, 0.0, 1.0, 1.0]);
        let at_edge = scope(vec![step.clone()], 0.5, 1, [false; 4], [0.0; 4]);
        assert_eq!(at_edge.len(), 1);
        near(at_edge[0].value, 1.0);
        near(at_edge[0].y, 52.0);
        let before = scope(vec![step], 0.49, 1, [false; 4], [0.0; 4]);
        assert_eq!(before.len(), 1);
        near(before[0].value, 0.0);
        near(before[0].y, 100.0);
    }

    #[test]
    fn step_at_the_right_edge_uses_the_final_level() {
        let hits = scope(
            vec![channel(&[0.0, 1.0, 1.0], &[0.0, 0.0, 1.0])],
            1.0,
            1,
            [false; 4],
            [0.0; 4],
        );
        assert_eq!(hits.len(), 1);
        near(hits[0].value, 1.0);
        near(hits[0].y, 52.0);
    }

    #[test]
    fn band_at_the_right_edge_keeps_both_edges() {
        let outline = channel(
            &[0.8, 0.8, 1.0, 1.0, 0.8, 1.0, 1.0],
            &[0.0, 1.0, 1.0, 0.0, 0.0, 0.0, 0.0],
        );
        let hits = scope(vec![outline], 1.0, 1, [false; 4], [0.0; 4]);
        assert_eq!(hits.len(), 2);
        near(hits[0].value, 0.0);
        near(hits[1].value, 1.0);
    }

    #[test]
    fn band_marks_both_edges() {
        // emit_band outline: left, top, right, bottom, step back, then the exit.
        let outline = channel(
            &[0.4, 0.4, 0.6, 0.6, 0.4, 0.6, 0.6],
            &[0.0, 1.0, 1.0, 0.0, 0.0, 0.0, 0.0],
        );
        let hits = scope(vec![outline.clone()], 0.5, 1, [false; 4], [0.0; 4]);
        assert_eq!(hits.len(), 2);
        near(hits[0].value, 0.0);
        near(hits[0].y, 100.0);
        near(hits[1].value, 1.0);
        near(hits[1].y, 52.0);
        let edge = scope(vec![outline], 0.4, 1, [false; 4], [0.0; 4]);
        assert_eq!(edge.len(), 2);
        near(edge[0].value, 0.0);
        near(edge[1].value, 1.0);
    }

    #[test]
    fn every_visible_channel_is_marked() {
        let flat = channel(&[0.0, 1.0], &[0.0, 0.0]);
        let hits = scope(vec![flat.clone(), flat], 0.2, 2, [false; 4], [0.0; 4]);
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].channel, 0);
        near(hits[0].y, 50.0);
        assert_eq!(hits[1].channel, 1);
        near(hits[1].y, 150.0);
    }

    #[test]
    fn hidden_channel_is_skipped() {
        let flat = channel(&[0.0, 1.0], &[0.0, 0.0]);
        let hits = scope(
            vec![flat.clone(), flat],
            0.2,
            2,
            [true, false, false, false],
            [0.0; 4],
        );
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].channel, 1);
        assert!(
            scope(
                vec![channel(&[0.0, 1.0], &[0.0, 0.0])],
                0.2,
                1,
                [true, false, false, false],
                [0.0; 4],
            )
            .is_empty()
        );
    }

    #[test]
    fn volt_pos_shifts_the_level() {
        let hits = scope(
            vec![channel(&[0.0, 1.0], &[1.0, 1.0])],
            0.1,
            1,
            [false; 4],
            [1.0, 0.0, 0.0, 0.0],
        );
        assert_eq!(hits.len(), 1);
        near(hits[0].value, 1.0);
        near(hits[0].y, 100.0);
    }

    #[test]
    fn samples_grid_interpolates_between_columns() {
        let mut ch = channel(&[], &[]);
        ch.plot.clear();
        ch.plot_x.clear();
        ch.samples = vec![0.0, 0.0, 0.0, 0.0, 2.0];
        let hits = scope(vec![ch], 0.875, 1, [false; 4], [0.0; 4]);
        assert_eq!(hits.len(), 1);
        near(hits[0].value, 1.0);
        near(hits[0].y, 52.0);
    }

    #[test]
    fn multitrack_scales_divisions_and_clips_per_track() {
        use super::paint_scope_screen;
        use crate::canvas::Rect;
        use crate::canvas::draw::{Align, Color, Draw};

        struct ClipRecorder {
            clips: Vec<(f64, f64, f64, f64)>,
            lines: Vec<(f64, f64, f64, f64)>,
            polylines: Vec<(Vec<[f64; 2]>, bool)>,
            texts: Vec<(f64, f64, String)>,
        }

        impl Draw for ClipRecorder {
            fn fill_rect(&mut self, _: f64, _: f64, _: f64, _: f64, _: Color) {}
            fn stroke_rect(&mut self, _: f64, _: f64, _: f64, _: f64, _: Color, _: f64) {}
            fn fill_round_rect(&mut self, _: f64, _: f64, _: f64, _: f64, _: f64, _: Color) {}
            fn stroke_round_rect(
                &mut self,
                _: f64,
                _: f64,
                _: f64,
                _: f64,
                _: f64,
                _: Color,
                _: f64,
            ) {
            }
            fn fill_circle(&mut self, _: f64, _: f64, _: f64, _: Color) {}
            fn stroke_circle(&mut self, _: f64, _: f64, _: f64, _: Color, _: f64) {}
            fn fill_ellipse(&mut self, _: f64, _: f64, _: f64, _: f64, _: Color) {}
            fn stroke_ellipse(&mut self, _: f64, _: f64, _: f64, _: f64, _: Color, _: f64) {}
            fn fill_poly(&mut self, _: &[[f64; 2]], _: Color) {}
            fn stroke_poly(&mut self, _: &[[f64; 2]], _: Color, _: f64, _: bool) {}
            fn polyline(&mut self, pts: &[[f64; 2]], _: Color, _: f64, dash: bool) {
                self.polylines.push((pts.to_vec(), dash));
            }
            fn line(&mut self, x0: f64, y0: f64, x1: f64, y1: f64, _: Color, _: f64) {
                self.lines.push((x0, y0, x1, y1));
            }
            fn arc(&mut self, _: f64, _: f64, _: f64, _: f64, _: f64, _: Color, _: f64) {}
            fn text(&mut self, x: f64, y: f64, s: &str, _: f64, _: Color, _: Align) {
                self.texts.push((x, y, s.to_string()));
            }
            fn text_bold(&mut self, x: f64, y: f64, s: &str, _: f64, _: Color, _: Align) {
                self.texts.push((x, y, s.to_string()));
            }
            fn grid_dots(&mut self, _: Rect, _: i32, _: Color) {}
            fn push(&mut self, _: f64, _: f64, _: f64, _: f64, _: f64) {}
            fn pop(&mut self) {}
            fn push_clip_rect(&mut self, x: f64, y: f64, w: f64, h: f64) {
                self.clips.push((x, y, w, h));
            }
        }

        // 480 px tall => 10 vertical divisions per track for 1, 2, or 4 tracks.
        let h = SCOPE_DIV_PX * 10.0;
        let w = 400.0;
        let ch0 = channel(&[0.0, 1.0], &[4.0, 4.0]); // fits inside track 0 (+4 div)
        let ch1 = channel(&[0.0, 1.0], &[8.0, 8.0]); // overflows above track 1 (+8 div > +5 div)
        let buf = PlotBuffer {
            channels: vec![ch0, ch1],
            n: 2,
        };

        let mut rec = ClipRecorder {
            clips: Vec::new(),
            lines: Vec::new(),
            polylines: Vec::new(),
            texts: Vec::new(),
        };
        paint_scope_screen(
            &mut rec,
            w,
            h,
            Some(&buf),
            &[1.0; 4],
            &[0.0; 4],
            2,
            &[false; 4],
            1.0,
        );

        // Each channel clips to its own 240 px band.
        assert_eq!(
            rec.clips,
            vec![(0.0, 0.0, w, 240.0), (0.0, 240.0, w, 240.0)]
        );

        // Track divider is drawn at y = 240.0 across the full width.
        assert!(
            rec.lines.iter().any(|&(x0, y0, x1, y1)| x0 == 0.0
                && x1 == w
                && (y0 - 240.0).abs() < 1e-6
                && (y1 - 240.0).abs() < 1e-6),
            "expected a track divider at y = 240"
        );

        // Track 0 (ch0 = 4 V): center 120, v_div = 24 => y = 120 - 4*24 = 24 (inside track 0).
        // Track 1 (ch1 = 8 V): center 360, v_div = 24 => y = 360 - 8*24 = 168 (above track 1, so no min/max label leaks into track 0).
        let solid: Vec<&Vec<[f64; 2]>> = rec
            .polylines
            .iter()
            .filter(|(_, dash)| !dash)
            .map(|(pts, _)| pts)
            .collect();
        assert_eq!(solid.len(), 2);
        near(solid[0][0][1], 24.0);
        near(solid[1][0][1], 168.0);
        assert!(rec.texts.iter().any(|(_, _, s)| s == "4.00V"));
        assert!(
            rec.texts.iter().all(|(_, _, s)| s != "8.00V"),
            "overflowing track 1 marker must not render into track 0"
        );

        // Cursor hit y matches the painted trace y for both tracks.
        let hits = scope_signal_at(
            w,
            h,
            0.5,
            SCOPE_DIV_PX,
            &buf,
            &[1.0; 4],
            &[0.0; 4],
            2,
            &[false; 4],
        );
        assert_eq!(hits.len(), 2);
        near(hits[0].y, 24.0);
        near(hits[1].y, 168.0);

        // 4-track mode: 480 px => 120 px/track, v_div = 12 px, +5 V stays inside track 0 (60 - 5*12 = 0).
        let buf4 = PlotBuffer {
            channels: vec![
                channel(&[0.0, 1.0], &[5.0, 5.0]),
                channel(&[0.0, 1.0], &[5.0, 5.0]),
                channel(&[0.0, 1.0], &[5.0, 5.0]),
                channel(&[0.0, 1.0], &[5.0, 5.0]),
            ],
            n: 2,
        };
        let hits4 = scope_signal_at(
            w,
            h,
            0.5,
            SCOPE_DIV_PX,
            &buf4,
            &[1.0; 4],
            &[0.0; 4],
            4,
            &[false; 4],
        );
        assert_eq!(hits4.len(), 4);
        near(hits4[0].y, 0.0);
        near(hits4[1].y, 120.0);
        near(hits4[2].y, 240.0);
        near(hits4[3].y, 360.0);
    }

    #[test]
    fn offscreen_slope_reports_the_true_voltage() {
        let hits = scope(
            vec![channel(&[0.0, 1.0], &[0.0, 4.0])],
            0.75,
            1,
            [false; 4],
            [0.0; 4],
        );
        assert_eq!(hits.len(), 1);
        near(hits[0].value, 3.0);
        near(hits[0].y, -44.0);
    }

    #[test]
    fn la_marks_every_row() {
        let mut channels = Vec::new();
        for row in 0..8 {
            let level = if row == 2 { 1.0 } else { 0.0 };
            channels.push(channel(&[0.0, 1.0], &[level, level]));
        }
        let hits = la_signal_at(
            320.0,
            160.0,
            0.5,
            LA_TRACE_INSET,
            &PlotBuffer { channels, n: 2 },
        );
        assert_eq!(hits.len(), 8);
        let high = hits.iter().find(|hit| hit.channel == 2).unwrap();
        near(high.value, 1.0);
        near(high.y, 43.0);
        assert!(
            hits.iter()
                .filter(|hit| hit.channel != 2)
                .all(|hit| (hit.value - 0.0).abs() < 1e-6)
        );
    }

    #[test]
    fn la_step_keeps_the_level_that_continues() {
        let hits = la_signal_at(
            320.0,
            160.0,
            0.5,
            LA_TRACE_INSET,
            &PlotBuffer {
                channels: vec![channel(&[0.0, 0.5, 0.5, 1.0], &[0.0, 0.0, 1.0, 1.0])],
                n: 2,
            },
        );
        assert_eq!(hits.len(), 1);
        near(hits[0].value, 1.0);
        near(hits[0].y, 3.0);
    }

    #[test]
    fn la_timed_plot_is_hit_without_a_sample_grid() {
        let mut ch = channel(&[0.0, 1.0], &[1.0, 1.0]);
        ch.samples.clear();
        let hits = la_signal_at(
            320.0,
            160.0,
            0.25,
            LA_TRACE_INSET,
            &PlotBuffer {
                channels: vec![ch],
                n: 0,
            },
        );
        assert_eq!(hits.len(), 1);
        near(hits[0].value, 1.0);
        near(hits[0].y, 3.0);
    }
}
