//! Per-component visual rendering trait.

use crate::canvas::Rect;
use crate::canvas::draw::{Align, Color, Draw, PaintCtx, Palette, parse_hex};
use crate::plot::{Channel, PlotBuffer};
use crate::theme::{COMPONENT_BORDER_WIDTH, ColorId, ColorTheme};

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

/// Chassis, labels, screen and reticle. `traces` are live samples (sim);
/// `None` leaves the screen empty. Frequency text is overlaid, not cached.
pub fn paint_oscope(
    d: &mut dyn Draw,
    pal: &Palette,
    tunnels: &[&str],
    freqs: &[&str],
    traces: Option<&PlotBuffer>,
    volt_divs: &[f64; 4],
    volt_pos: &[f64; 4],
    tracks: i32,
    hidden: &[bool; 4],
    zoom: f64,
) {
    paint_oscope_chrome(d, pal, tunnels);
    paint_oscope_reticle(d, tracks, zoom);
    paint_oscope_freq_labels(d, pal, freqs);
    paint_oscope_waves(d, traces, volt_divs, volt_pos, tracks, hidden, zoom);
}

fn paint_oscope_chrome(d: &mut dyn Draw, pal: &Palette, tunnels: &[&str]) {
    // 1. Chassis
    d.fill_round_rect(-80.0, -72.0, 213.0, 144.0, 4.0, pal.body.fade(0.9));
    d.stroke_round_rect(
        -80.0,
        -72.0,
        213.0,
        144.0,
        4.0,
        pal.border,
        COMPONENT_BORDER_WIDTH,
    );

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
    let screen_x = -6.0;
    let screen_y = -64.0;
    let screen_w = 130.0;
    let screen_h = 128.0;
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

fn paint_oscope_reticle(d: &mut dyn Draw, tracks: i32, zoom: f64) {
    let px = 1.0 / zoom.max(0.1);
    paint_scope_grid(
        d,
        oscope_inner_rect(),
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

const OSCOPE_SCREEN_X: f64 = -6.0;
const OSCOPE_SCREEN_Y: f64 = -64.0;
const OSCOPE_SCREEN_W: f64 = 130.0;
const OSCOPE_SCREEN_H: f64 = 128.0;

fn oscope_inner_rect() -> (f64, f64, f64, f64) {
    (
        OSCOPE_SCREEN_X + 4.0,
        OSCOPE_SCREEN_Y + 4.0,
        OSCOPE_SCREEN_W - 8.0,
        OSCOPE_SCREEN_H - 8.0,
    )
}

fn scope_track_count(tracks: i32) -> usize {
    match tracks {
        2 => 2,
        4 => 4,
        _ => 1,
    }
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
        let c = if is_center { accent } else { grid };
        let lw = px * if is_center { 1.2 } else { 0.75 };
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

fn trace_points(ch: &Channel, x0: f64, width: f64, y_of: impl Fn(f64) -> f64) -> Vec<[f64; 2]> {
    if ch.plot.len() >= 2 && ch.plot.len() == ch.plot_x.len() {
        let mut pts = Vec::with_capacity(ch.plot.len());
        for (&x, &v) in ch.plot_x.iter().zip(ch.plot.iter()) {
            pts.push([x0 + x * width, y_of(v)]);
        }
        return pts;
    }
    let last = ch.samples.len().saturating_sub(1).max(1) as f64;
    let mut pts = Vec::with_capacity(ch.samples.len());
    for (k, &v) in ch.samples.iter().enumerate() {
        pts.push([x0 + (k as f64 / last) * width, y_of(v)]);
    }
    pts
}

fn paint_oscope_waves(
    d: &mut dyn Draw,
    traces: Option<&PlotBuffer>,
    volt_divs: &[f64; 4],
    volt_pos: &[f64; 4],
    tracks: i32,
    hidden: &[bool; 4],
    zoom: f64,
) {
    paint_scope_waves(
        d,
        oscope_inner_rect(),
        traces,
        volt_divs,
        volt_pos,
        tracks,
        hidden,
        1.0 / zoom.max(0.1),
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
    d.push_clip_rect(x0, y0, w, h);

    for (c, ch) in buf.channels.iter().enumerate().take(4) {
        if hidden.get(c).copied().unwrap_or(false) || !ch.connected || ch.samples.len() < 2 {
            continue;
        }
        let track_idx = c % num_tracks;
        let track_h = h / (num_tracks as f64);
        let track_center_y = y0 + (track_idx as f64 + 0.5) * track_h;
        let vd = volt_divs.get(c).copied().unwrap_or(1.0).max(1e-12);
        let vp = volt_pos.get(c).copied().unwrap_or(0.0);
        let scale_y = track_h / (vd * 10.0);
        let color = parse_hex(&ch.color);
        let y_of = |v: f64| track_center_y - (v - vp) * scale_y;

        let pts = trace_points(ch, x0, w, y_of);
        d.polyline(&pts, color, px * 1.5, false);

        if expanded {
            if let Some((min_v, max_v)) = channel_extent(ch) {
                let end_x = x0 + w;
                let y_max = y_of(max_v);
                let y_min = y_of(min_v);
                let label_x = x0 + if c % 2 == 0 { 4.0 * px } else { w - 50.0 * px };
                if (y0..=y0 + h).contains(&y_max) {
                    d.polyline(&[[x0, y_max], [end_x, y_max]], color, px * 0.5, true);
                    d.text_bold(
                        label_x,
                        (y_max - 12.0 * px).max(y0),
                        &format!("{max_v:.2}V"),
                        9.0 * px,
                        color,
                        Align::TopLeft,
                    );
                }
                if (y0..=y0 + h).contains(&y_min) {
                    d.polyline(&[[x0, y_min], [end_x, y_min]], color, px * 0.5, true);
                    d.text_bold(
                        label_x,
                        (y_min + 2.0 * px).min(y0 + h - 10.0 * px),
                        &format!("{min_v:.2}V"),
                        9.0 * px,
                        color,
                        Align::TopLeft,
                    );
                }
            }
        }
    }

    d.pop_clip();
}

/// Scope window picture: graticule, traces, and the expanded-view min/max marks.
/// Coordinates are the item's pixel rectangle. The caller clears with `ScopePaper`.
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
    paint_scope_grid(d, rect, tracks, grid, accent, px, 1.5 * px);
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
        d, rect, traces, volt_divs, volt_pos, tracks, hidden, px, true,
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
    let row_h = h / 8.0;
    for (c, ch) in buf.channels.iter().enumerate().take(8) {
        if !ch.connected {
            continue;
        }
        let has_plot = ch.plot.len() >= 2 && ch.plot.len() == ch.plot_x.len();
        if !has_plot && ch.samples.len() < 2 {
            continue;
        }
        let row_y = y0 + (c as f64) * row_h;
        let pts = trace_points(ch, x0, w, |v| {
            let bit = v.clamp(0.0, 1.0);
            row_y + row_h - inset - bit * (row_h - 2.0 * inset)
        });
        d.polyline(&pts, parse_hex(&ch.color), trace_w, false);
    }
}

/// Logic-analyzer window: eight rows, ten columns, digital traces.
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
    paint_la_grid(d, rect, scope_color(ColorId::ScopeWindowGrid), px);
    paint_la_waves(d, rect, traces, 3.0 * px, px * 1.5);
}

pub fn paint_lanalizer(
    d: &mut dyn Draw,
    pal: &Palette,
    tunnels: &[&str],
    traces: Option<&PlotBuffer>,
    zoom: f64,
) {
    // 1. Chassis
    d.fill_round_rect(-80.0, -72.0, 213.0, 144.0, 4.0, pal.body.fade(0.9));
    d.stroke_round_rect(
        -80.0,
        -72.0,
        213.0,
        144.0,
        4.0,
        pal.border,
        COMPONENT_BORDER_WIDTH,
    );

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
    let screen_x = -6.0;
    let screen_y = -64.0;
    let screen_w = 130.0;
    let screen_h = 128.0;
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

    let inner = oscope_inner_rect();
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
    d.fill_round_rect(
        r.x,
        r.y,
        r.w,
        r.h,
        2.0,
        pal.body.fade(crate::theme::COMPONENT_FILL_ALPHA),
    );
    d.stroke_round_rect(r.x, r.y, r.w, r.h, 2.0, pal.border, COMPONENT_BORDER_WIDTH);
    if !ic_label.is_empty() {
        if rotate {
            d.push(0.0, 0.0, -90.0, 1.0, 1.0);
            d.text(
                0.0,
                0.0,
                ic_label,
                7.0,
                pal.border,
                crate::canvas::draw::Align::Center,
            );
            d.pop();
        } else {
            d.text(
                0.0,
                0.0,
                ic_label,
                7.0,
                pal.border,
                crate::canvas::draw::Align::Center,
            );
        }
    }
}

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
    let fill = if custom_color && !bckgnd_color.is_empty() {
        crate::canvas::draw::parse_hex(bckgnd_color)
    } else if logic_symbol {
        crate::canvas::draw::parse_hex("#ebf0ff")
    } else {
        crate::canvas::draw::parse_hex("#141e3c")
    };
    d.fill_round_rect(0.0, 0.0, w, h, 1.0, fill);
    d.stroke_round_rect(0.0, 0.0, w, h, 1.0, pal.border, COMPONENT_BORDER_WIDTH);

    if !logic_symbol {
        let notch_color = crate::canvas::draw::parse_hex("#aaaa96");
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
