//! Canvas drawing primitives: Color, Palette, Draw trait, and PaintCtx.

use crate::canvas::Canvas;
use crate::canvas::geom::Rect;
use crate::theme::{ColorId, ColorTheme};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    pub fn with_alpha(self, a: f64) -> Self {
        Self {
            a: (a.clamp(0.0, 1.0) * 255.0).round() as u8,
            ..self
        }
    }

    pub fn fade(self, alpha: f64) -> Self {
        self.with_alpha((self.a as f64 / 255.0) * alpha)
    }

    pub fn css(self) -> String {
        if self.a == 255 {
            format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
        } else {
            format!(
                "rgba({},{},{},{:.3})",
                self.r,
                self.g,
                self.b,
                self.a as f64 / 255.0
            )
        }
    }
}

/// Hex colours matching `CircuitCanvas` (CSS/hex, optional 8-digit alpha).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Palette {
    pub canvas: Color,
    pub grid: Color,
    pub body: Color,
    pub border: Color,
    pub wire: Color,
    pub pin_high: Color,
    pub pin_low: Color,
    pub band: Color,
    pub tunnel_color1: Color,
    pub tunnel_color2: Color,
    pub tunnel_color3: Color,
    pub pin_open_high: Color,
    pub meter_display: Color,
    pub msg_warn: Color,
    pub msg_error: Color,
}

impl Palette {
    pub fn light() -> Self {
        let to_color = |id| {
            let (r, g, b, a) = ColorTheme::get_rgba(id, false);
            Color { r, g, b, a }
        };
        Self {
            canvas: to_color(ColorId::CanvasBackground),
            grid: to_color(ColorId::CanvasGridDots),
            body: to_color(ColorId::ComponentBody),
            border: to_color(ColorId::ComponentBorder),
            wire: to_color(ColorId::WireOpen),
            pin_high: to_color(ColorId::PinHigh),
            pin_low: to_color(ColorId::PinLow),
            band: to_color(ColorId::ItemHovered),
            tunnel_color1: to_color(ColorId::TunnelColor1),
            tunnel_color2: to_color(ColorId::TunnelColor2),
            tunnel_color3: to_color(ColorId::TunnelColor3),
            pin_open_high: to_color(ColorId::PinOpenHigh),
            meter_display: to_color(ColorId::MeterDisplayText),
            msg_warn: to_color(ColorId::MsgWarnBg),
            msg_error: to_color(ColorId::MsgErrorBg),
        }
    }

    pub fn dark() -> Self {
        let to_color = |id| {
            let (r, g, b, a) = ColorTheme::get_rgba(id, true);
            Color { r, g, b, a }
        };
        Self {
            canvas: to_color(ColorId::CanvasBackground),
            grid: to_color(ColorId::CanvasGridDots),
            body: to_color(ColorId::ComponentBody),
            border: to_color(ColorId::ComponentBorder),
            wire: to_color(ColorId::WireOpen),
            pin_high: to_color(ColorId::PinHigh),
            pin_low: to_color(ColorId::PinLow),
            band: to_color(ColorId::ItemHovered),
            tunnel_color1: to_color(ColorId::TunnelColor1),
            tunnel_color2: to_color(ColorId::TunnelColor2),
            tunnel_color3: to_color(ColorId::TunnelColor3),
            pin_open_high: to_color(ColorId::PinOpenHigh),
            meter_display: to_color(ColorId::MeterDisplayText),
            msg_warn: to_color(ColorId::MsgWarnBg),
            msg_error: to_color(ColorId::MsgErrorBg),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn from_hex(
        canvas: &str,
        grid: &str,
        body: &str,
        border: &str,
        wire: &str,
        pin_high: &str,
        pin_low: &str,
        band: &str,
    ) -> Self {
        let (mr, mg, mb, ma) = ColorTheme::get_rgba(ColorId::MeterDisplayText, false);
        let (wr, wg, wb, wa) = ColorTheme::get_rgba(ColorId::MsgWarnBg, false);
        let (er, eg, eb, ea) = ColorTheme::get_rgba(ColorId::MsgErrorBg, false);
        Self {
            canvas: parse_hex(canvas),
            grid: parse_hex(grid),
            body: parse_hex(body),
            border: parse_hex(border),
            wire: parse_hex(wire),
            pin_high: parse_hex(pin_high),
            pin_low: parse_hex(pin_low),
            band: parse_hex(band),
            tunnel_color1: Color::rgb(80, 180, 80),
            tunnel_color2: Color::rgb(50, 50, 50),
            tunnel_color3: Color::rgb(100, 100, 120),
            pin_open_high: Color::rgb(255, 180, 0),
            meter_display: Color {
                r: mr,
                g: mg,
                b: mb,
                a: ma,
            },
            msg_warn: Color {
                r: wr,
                g: wg,
                b: wb,
                a: wa,
            },
            msg_error: Color {
                r: er,
                g: eg,
                b: eb,
                a: ea,
            },
        }
    }
}

pub fn parse_hex(s: &str) -> Color {
    let h = s.trim().trim_start_matches('#');
    let n = |i: usize| u8::from_str_radix(h.get(i..i + 2).unwrap_or("00"), 16).unwrap_or(0);
    match h.len() {
        8 => Color {
            r: n(0),
            g: n(2),
            b: n(4),
            a: n(6),
        },
        6 => Color::rgb(n(0), n(2), n(4)),
        4 => Color {
            r: nibble(h, 0),
            g: nibble(h, 1),
            b: nibble(h, 2),
            a: nibble(h, 3),
        },
        3 => Color::rgb(nibble(h, 0), nibble(h, 1), nibble(h, 2)),
        _ => Color::rgb(0, 0, 0),
    }
}

fn nibble(h: &str, i: usize) -> u8 {
    let c = h.as_bytes().get(i).copied().unwrap_or(b'0');
    let v = match c {
        b'0'..=b'9' => c - b'0',
        b'a'..=b'f' => c - b'a' + 10,
        b'A'..=b'F' => c - b'A' + 10,
        _ => 0,
    };
    v * 17
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Align {
    TopLeft,
    Center,
    HCenter,
    Right,
}

#[allow(clippy::too_many_arguments)]
pub trait Draw {
    fn fill_rect(&mut self, x: f64, y: f64, w: f64, h: f64, c: Color);
    fn stroke_rect(&mut self, x: f64, y: f64, w: f64, h: f64, c: Color, width: f64);
    fn fill_round_rect(&mut self, x: f64, y: f64, w: f64, h: f64, r: f64, c: Color);
    fn stroke_round_rect(&mut self, x: f64, y: f64, w: f64, h: f64, r: f64, c: Color, width: f64);
    fn fill_circle(&mut self, cx: f64, cy: f64, r: f64, c: Color);
    fn stroke_circle(&mut self, cx: f64, cy: f64, r: f64, c: Color, width: f64);
    fn fill_ellipse(&mut self, x: f64, y: f64, w: f64, h: f64, c: Color);
    fn stroke_ellipse(&mut self, x: f64, y: f64, w: f64, h: f64, c: Color, width: f64);
    fn fill_poly(&mut self, pts: &[[f64; 2]], c: Color);
    fn stroke_poly(&mut self, pts: &[[f64; 2]], c: Color, width: f64, close: bool);
    fn polyline(&mut self, pts: &[[f64; 2]], c: Color, width: f64, dash: bool);
    fn polylines(&mut self, list: &[&[[f64; 2]]], c: Color, width: f64, dash: bool) {
        for pts in list {
            self.polyline(pts, c, width, dash);
        }
    }
    fn line(&mut self, x0: f64, y0: f64, x1: f64, y1: f64, c: Color, width: f64);
    fn arc(&mut self, cx: f64, cy: f64, r: f64, a0: f64, a1: f64, c: Color, width: f64);
    fn text(&mut self, x: f64, y: f64, s: &str, size: f64, c: Color, align: Align);
    fn text_bold(&mut self, x: f64, y: f64, s: &str, size: f64, c: Color, align: Align) {
        self.text(x, y, s, size, c, align);
    }
    fn grid_dots(&mut self, scene: Rect, step: i32, c: Color);
    fn push(&mut self, x: f64, y: f64, rot: f64, sx: f64, sy: f64);
    fn pop(&mut self);
    fn push_clip_rect(&mut self, _x: f64, _y: f64, _w: f64, _h: f64) {}
    fn pop_clip(&mut self) {}
    fn device_scale(&self) -> f64 {
        1.0
    }
    /// Blit a pixmap whose pixels were rasterized at `pm_scale` local units per
    /// pixel, with pixmap (0,0) at local (`x`,`y`). Returns false if unsupported.
    fn blit_pixmap(&mut self, _x: f64, _y: f64, _pm: &tiny_skia::Pixmap, _pm_scale: f64) -> bool {
        false
    }
    /// Draw and scale a pixmap to fit the rectangle (x, y, w, h) with opacity. Returns false if unsupported.
    fn draw_pixmap_rect(
        &mut self,
        _x: f64,
        _y: f64,
        _w: f64,
        _h: f64,
        _pm: &tiny_skia::Pixmap,
        _opacity: f64,
    ) -> bool {
        false
    }
}

pub struct PaintCtx<'a> {
    pub canvas: &'a Canvas,
    pub pal: &'a Palette,
    pub scale: f64,
    pub item_id: &'a str,
}

impl<'a> PaintCtx<'a> {
    #[inline]
    pub fn pin_voltage(&self, suffix: &str) -> Option<f64> {
        self.canvas.item_pin_voltage(self.item_id, suffix)
    }

    #[inline]
    pub fn pin_current(&self, suffix: &str) -> Option<f64> {
        self.canvas.item_pin_current(self.item_id, suffix)
    }

    #[inline]
    pub fn is_active(&self) -> bool {
        self.canvas.is_item_active(self.item_id)
    }
}
