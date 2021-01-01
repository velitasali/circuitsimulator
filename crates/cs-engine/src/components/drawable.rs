//! Per-component visual rendering trait.

use crate::canvas::Rect;
use crate::canvas::draw::{Color, Draw, PaintCtx, Palette, parse_hex};
use crate::plot::PlotBuffer;
use crate::theme::COMPONENT_BORDER_WIDTH;

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
) {
    paint_oscope_chrome(d, pal, tunnels);
    paint_oscope_reticle(d, tracks);
    paint_oscope_freq_labels(d, pal, freqs);
    paint_oscope_waves(d, traces, volt_divs, volt_pos, tracks, hidden);
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
    let ch_colors = [
        crate::canvas::draw::parse_hex("#ffff00"),
        crate::canvas::draw::parse_hex("#00ff00"),
        crate::canvas::draw::parse_hex("#00ffff"),
        crate::canvas::draw::parse_hex("#ff00ff"),
    ];

    for i in 0..4 {
        let y_center = -48.0 + 32.0 * (i as f64);

        // Color swatch box (C++ QRectF(-76, yCenter - 6, 8, 12))
        d.fill_round_rect(-76.0, y_center - 6.0, 8.0, 12.0, 1.0, ch_colors[i]);
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
        crate::canvas::draw::Color::rgb(13, 17, 20),
    );
    d.stroke_round_rect(
        screen_x,
        screen_y,
        screen_w,
        screen_h,
        4.0,
        crate::canvas::draw::Color::rgb(42, 49, 56),
        COMPONENT_BORDER_WIDTH,
    );
}

fn paint_oscope_reticle(d: &mut dyn Draw, tracks: i32) {
    let (inner_x, inner_y, inner_w, inner_h) = oscope_inner_rect();
    let end_x = inner_x + inner_w;
    let end_y = inner_y + inner_h;
    let h_center = inner_x + inner_w * 0.5;

    let num_tracks = match tracks {
        2 => 2,
        4 => 4,
        _ => 1,
    };
    let v_divs = 10 * num_tracks;

    let mut track_centers = Vec::with_capacity(num_tracks);
    for t in 0..num_tracks {
        track_centers.push(inner_y + (t as f64 + 0.5) * inner_h / (num_tracks as f64));
    }

    let grid_pen = crate::canvas::draw::Color::rgb(55, 62, 70);
    let accent_pen = crate::canvas::draw::Color::rgb(100, 110, 125);

    // Horizontal grid lines
    for gy in 1..v_divs {
        let y = inner_y + inner_h * (gy as f64) / (v_divs as f64);
        let is_center = track_centers.iter().any(|&tc| (y - tc).abs() < 0.5);
        let c = if is_center { accent_pen } else { grid_pen };
        let w = if is_center { 1.2 } else { 0.75 };
        d.line(inner_x, y, end_x, y, c, w);
    }

    // Vertical grid lines (10 divisions)
    for gx in 1..10 {
        let x = inner_x + inner_w * (gx as f64) / 10.0;
        let is_center = (x - h_center).abs() < 0.5;
        let c = if is_center { accent_pen } else { grid_pen };
        let w = if is_center { 1.2 } else { 0.75 };
        d.line(x, inner_y, x, end_y, c, w);
    }

    // Center crosshair tick marks
    let tick = 1.5;
    for tc in &track_centers {
        for tx in 0..=50 {
            let x = inner_x + inner_w * (tx as f64) / 50.0;
            d.line(x, tc - tick, x, tc + tick, accent_pen, 0.75);
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
        let y = inner_y + inner_h * (ty as f64) / (ym as f64);
        d.line(h_center - tick, y, h_center + tick, y, accent_pen, 0.75);
    }
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

fn paint_oscope_waves(
    d: &mut dyn Draw,
    traces: Option<&PlotBuffer>,
    volt_divs: &[f64; 4],
    volt_pos: &[f64; 4],
    tracks: i32,
    hidden: &[bool; 4],
) {
    let Some(buf) = traces else {
        return;
    };
    let (inner_x, inner_y, inner_w, inner_h) = oscope_inner_rect();
    let num_tracks = match tracks {
        2 => 2,
        4 => 4,
        _ => 1,
    };
    let n_out = 96usize;

    for (c, ch) in buf.channels.iter().enumerate().take(4) {
        if hidden.get(c).copied().unwrap_or(false) || !ch.connected || ch.samples.len() < 2 {
            continue;
        }
        let track_idx = c % num_tracks;
        let track_h = inner_h / (num_tracks as f64);
        let track_center_y = inner_y + (track_idx as f64 + 0.5) * track_h;
        let vd = volt_divs.get(c).copied().unwrap_or(1.0).max(1e-12);
        let vp = volt_pos.get(c).copied().unwrap_or(0.0);
        let span = vd * 10.0;
        let scale_y = track_h / span;

        let last = ch.samples.len() - 1;
        let mut pts = Vec::with_capacity(n_out);
        for k in 0..n_out {
            let src = k * last / (n_out - 1);
            let t = k as f64 / (n_out - 1) as f64;
            let v = ch.samples[src];
            let px = inner_x + t * inner_w;
            let py = (track_center_y - (v - vp) * scale_y).clamp(inner_y, inner_y + inner_h);
            pts.push([px, py]);
        }
        d.polyline(&pts, parse_hex(&ch.color), 1.5, false);
    }
}

pub fn paint_lanalizer(
    d: &mut dyn Draw,
    pal: &Palette,
    tunnels: &[&str],
    traces: Option<&PlotBuffer>,
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
    let la_colors = [
        crate::canvas::draw::parse_hex("#e74c3c"),
        crate::canvas::draw::parse_hex("#e67e22"),
        crate::canvas::draw::parse_hex("#f1c40f"),
        crate::canvas::draw::parse_hex("#2ecc71"),
        crate::canvas::draw::parse_hex("#1abc9c"),
        crate::canvas::draw::parse_hex("#3498db"),
        crate::canvas::draw::parse_hex("#9b59b6"),
        crate::canvas::draw::parse_hex("#ecf0f1"),
    ];

    for i in 0..8 {
        let y_center = -64.0 + 16.0 * (i as f64);
        // Swatch box (C++ QRectF(-76, yCenter - 5, 8, 10))
        d.fill_round_rect(-76.0, y_center - 5.0, 8.0, 10.0, 1.0, la_colors[i]);
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
        crate::canvas::draw::Color::rgb(13, 17, 20),
    );
    d.stroke_round_rect(
        screen_x,
        screen_y,
        screen_w,
        screen_h,
        4.0,
        crate::canvas::draw::Color::rgb(42, 49, 56),
        COMPONENT_BORDER_WIDTH,
    );

    // Reticle Grid (8 rows, 10 columns)
    let inner_x = screen_x + 4.0;
    let inner_y = screen_y + 4.0;
    let inner_w = screen_w - 8.0;
    let inner_h = screen_h - 8.0;
    let end_x = inner_x + inner_w;
    let end_y = inner_y + inner_h;

    let grid_pen = crate::canvas::draw::Color::rgb(55, 62, 70);

    for gy in 1..8 {
        let y = inner_y + inner_h * (gy as f64) / 8.0;
        d.line(inner_x, y, end_x, y, grid_pen, 0.75);
    }
    for gx in 1..10 {
        let x = inner_x + inner_w * (gx as f64) / 10.0;
        d.line(x, inner_y, x, end_y, grid_pen, 0.75);
    }

    // Live Digital Waveforms
    if let Some(buf) = traces {
        let row_h = inner_h / 8.0;
        for (c, ch) in buf.channels.iter().enumerate().take(8) {
            if !ch.connected || ch.samples.len() < 2 {
                continue;
            }
            let row_y = inner_y + (c as f64) * row_h;
            let last = ch.samples.len() - 1;
            let mut pts = Vec::with_capacity(ch.samples.len());
            for (i, &v) in ch.samples.iter().enumerate() {
                let px = inner_x + (i as f64 / last as f64) * inner_w;
                let bit = v.clamp(0.0, 1.0);
                let py = row_y + row_h - 3.0 - bit * (row_h - 6.0);
                pts.push([px, py]);
            }
            let color = parse_hex(&ch.color);
            d.polyline(&pts, color, 1.2, false);
        }
    }
}

#[allow(dead_code)]
pub fn paint_plot(d: &mut dyn Draw, pal: &Palette) {
    paint_oscope(d, pal, &[], &[], None, &[1.0; 4], &[0.0; 4], 1, &[false; 4]);
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
