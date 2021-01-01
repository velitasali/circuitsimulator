use tiny_skia::{
    FillRule, LineCap, LineJoin, Paint, Path as SkPath, PathBuilder, Pixmap, PixmapPaint,
    PremultipliedColorU8, Rect as SkRect, Stroke, StrokeDash, Transform,
};

use super::text::{blend_over, font, font_bold, get_cached_glyph, strip_tags};
use crate::canvas::draw::{Align, Color, Draw};
use crate::canvas::geom::Rect;

pub fn sk_color(c: Color) -> tiny_skia::Color {
    tiny_skia::Color::from_rgba8(c.r, c.g, c.b, c.a)
}

pub fn sk_paint(c: Color) -> Paint<'static> {
    sk_paint_aa(c, true)
}

fn sk_paint_aa(c: Color, anti_alias: bool) -> Paint<'static> {
    let mut p = Paint::default();
    p.set_color_rgba8(c.r, c.g, c.b, c.a);
    p.anti_alias = anti_alias;
    p
}

pub fn sk_stroke(width: f64, dash: bool) -> Stroke {
    Stroke {
        width: width.max(0.05) as f32,
        miter_limit: 4.0,
        line_cap: LineCap::Round,
        line_join: LineJoin::Round,
        dash: if dash {
            StrokeDash::new(vec![4.0, 3.0], 0.0)
        } else {
            None
        },
    }
}

pub fn sk_rect(x: f64, y: f64, w: f64, h: f64) -> Option<SkRect> {
    SkRect::from_xywh(x as f32, y as f32, w as f32, h as f32)
}

pub fn poly_path(pts: &[[f64; 2]], close: bool) -> Option<SkPath> {
    if pts.is_empty() {
        return None;
    }
    let mut pb = PathBuilder::new();
    pb.move_to(pts[0][0] as f32, pts[0][1] as f32);
    for p in &pts[1..] {
        pb.line_to(p[0] as f32, p[1] as f32);
    }
    if close {
        pb.close();
    }
    pb.finish()
}

pub fn round_rect_path(x: f64, y: f64, w: f64, h: f64, mut r: f64) -> Option<SkPath> {
    r = r.min(w * 0.5).min(h * 0.5);
    if r <= 0.05 {
        return Some(PathBuilder::from_rect(sk_rect(x, y, w, h)?));
    }
    let (x, y, w, h, r) = (x as f32, y as f32, w as f32, h as f32, r as f32);
    let k = 0.5523 * r;
    let x1 = x + w;
    let y1 = y + h;
    let mut pb = PathBuilder::new();
    pb.move_to(x + r, y);
    pb.line_to(x1 - r, y);
    pb.cubic_to(x1 - r + k, y, x1, y + r - k, x1, y + r);
    pb.line_to(x1, y1 - r);
    pb.cubic_to(x1, y1 - r + k, x1 - r + k, y1, x1 - r, y1);
    pb.line_to(x + r, y1);
    pb.cubic_to(x + r - k, y1, x, y1 - r + k, x, y1 - r);
    pb.line_to(x, y + r);
    pb.cubic_to(x, y + r - k, x + r - k, y, x + r, y);
    pb.close();
    pb.finish()
}

pub fn ellipse_path(x: f64, y: f64, w: f64, h: f64) -> Option<SkPath> {
    let mut pb = PathBuilder::new();
    pb.push_oval(sk_rect(x, y, w, h)?);
    pb.finish()
}

pub fn circle_path(cx: f64, cy: f64, r: f64) -> Option<SkPath> {
    if r <= 0.0 {
        return None;
    }
    let mut pb = PathBuilder::new();
    pb.push_circle(cx as f32, cy as f32, r as f32);
    pb.finish()
}

pub fn arc_pts(cx: f64, cy: f64, r: f64, a0: f64, a1: f64) -> Vec<[f64; 2]> {
    let sweep = a1 - a0;
    let n = ((sweep.abs() / std::f64::consts::PI) * 16.0)
        .ceil()
        .max(4.0) as usize;
    (0..=n)
        .map(|i| {
            let a = a0 + sweep * (i as f64 / n as f64);
            [cx + r * a.cos(), cy + r * a.sin()]
        })
        .collect()
}

pub struct Raster<'a> {
    pub pixmap: &'a mut Pixmap,
    pub stack: Vec<Transform>,
}

impl<'a> Raster<'a> {
    pub fn new(pixmap: &'a mut Pixmap, initial_transform: Transform) -> Self {
        Self {
            pixmap,
            stack: vec![initial_transform],
        }
    }

    pub fn xf(&self) -> Transform {
        self.stack
            .last()
            .copied()
            .unwrap_or_else(Transform::identity)
    }

    fn transform_scale(&self) -> f64 {
        let xf = self.xf();
        let sx = (xf.sx * xf.sx + xf.ky * xf.ky).sqrt() as f64;
        let sy = (xf.kx * xf.kx + xf.sy * xf.sy).sqrt() as f64;
        ((sx + sy) * 0.5).max(0.1)
    }

    pub fn fill_path(&mut self, path: &SkPath, c: Color) {
        self.pixmap
            .fill_path(path, &sk_paint(c), FillRule::Winding, self.xf(), None);
    }

    fn fill_path_maybe_aliased(&mut self, path: &SkPath, c: Color, device_max_edge: f64) {
        // Large fills spend their time in the AA blitter; aliased fill is
        // visually identical past ~1 CSS pixel of coverage at typical DPR.
        let aa = device_max_edge < 64.0;
        self.pixmap.fill_path(
            path,
            &sk_paint_aa(c, aa),
            FillRule::Winding,
            self.xf(),
            None,
        );
    }

    pub fn stroke_path(&mut self, path: &SkPath, c: Color, width: f64, dash: bool) {
        self.pixmap
            .stroke_path(path, &sk_paint(c), &sk_stroke(width, dash), self.xf(), None);
    }

    fn draw_text_with_font(
        &mut self,
        bold: bool,
        x: f64,
        y: f64,
        s: &str,
        size: f64,
        c: Color,
        align: Align,
    ) {
        if s.is_empty() {
            return;
        }
        let clean = strip_tags(s);

        // Calculate the effective scale factor of the current transform stack
        let xf = self.xf();
        let scale_x = (xf.sx * xf.sx + xf.ky * xf.ky).sqrt();
        let scale_y = (xf.kx * xf.kx + xf.sy * xf.sy).sqrt();
        let scale = ((scale_x + scale_y) * 0.5).max(0.1);

        let scene_px = size as f32;
        // Rasterize the font directly at physical device pixel size so text is razor-sharp on Retina / High-DPI
        let raster_px = (scene_px * scale).max(1.0);

        let font_ref = if bold { font_bold() } else { font() };
        let metrics = font_ref.horizontal_line_metrics(raster_px);
        let ascent = metrics.map(|m| m.ascent).unwrap_or(raster_px) / scale;
        let line_height = metrics.map(|m| m.new_line_size).unwrap_or(raster_px * 1.2) / scale;

        let lines: Vec<&str> = clean.split('\n').collect();
        let num_lines = lines.len();

        let is_axis_aligned = xf.kx == 0.0 && xf.ky == 0.0 && xf.sx > 0.0 && xf.sy > 0.0;
        let paint = PixmapPaint {
            quality: tiny_skia::FilterQuality::Bilinear,
            ..PixmapPaint::default()
        };

        for (line_idx, line) in lines.into_iter().enumerate() {
            if line.is_empty() {
                continue;
            }
            let width = if align != Align::TopLeft {
                let mut w = 0.0f32;
                for ch in line.chars() {
                    let glyph = get_cached_glyph(bold, ch, raster_px);
                    w += glyph.metrics.advance_width / scale;
                }
                w
            } else {
                0.0f32
            };
            let line_y = match align {
                Align::Center => {
                    y as f32 - (num_lines as f32 - 1.0) * line_height * 0.5
                        + line_idx as f32 * line_height
                }
                _ => y as f32 + line_idx as f32 * line_height,
            };
            let (mut pen_x, baseline) = match align {
                Align::TopLeft => (x as f32, line_y + ascent),
                Align::HCenter => (x as f32 - width * 0.5, line_y + ascent),
                Align::Center => (x as f32 - width * 0.5, line_y + ascent * 0.35),
                Align::Right => (x as f32 - width, line_y + ascent),
            };
            for ch in line.chars() {
                let glyph = get_cached_glyph(bold, ch, raster_px);
                let gm = &glyph.metrics;
                let bitmap = &glyph.bitmap;
                if gm.width > 0 && gm.height > 0 {
                    let char_xmin = gm.xmin as f32 / scale;
                    let char_ymin = gm.ymin as f32 / scale;
                    let char_height = gm.height as f32 / scale;
                    let gx = pen_x + char_xmin;
                    let gy = baseline - (char_ymin + char_height);

                    if is_axis_aligned {
                        let dx = (xf.sx * gx + xf.tx).round() as i32;
                        let dy = (xf.sy * gy + xf.ty).round() as i32;
                        let pw = self.pixmap.width() as i32;
                        let ph = self.pixmap.height() as i32;
                        let gw = gm.width as i32;
                        let gh = gm.height as i32;

                        if dx + gw > 0 && dx < pw && dy + gh > 0 && dy < ph {
                            let start_y = dy.max(0);
                            let end_y = (dy + gh).min(ph);
                            let start_x = dx.max(0);
                            let end_x = (dx + gw).min(pw);

                            let pixels = self.pixmap.pixels_mut();
                            for row in start_y..end_y {
                                let glyph_row = (row - dy) as usize;
                                let canvas_row_idx = (row as usize) * (pw as usize);
                                for col in start_x..end_x {
                                    let glyph_col = (col - dx) as usize;
                                    let a = bitmap[glyph_row * (gw as usize) + glyph_col];
                                    if a == 0 {
                                        continue;
                                    }
                                    let src_a = ((a as u16 * c.a as u16) / 255) as u8;
                                    let pr = ((c.r as u16 * src_a as u16) / 255) as u8;
                                    let pg = ((c.g as u16 * src_a as u16) / 255) as u8;
                                    let pb = ((c.b as u16 * src_a as u16) / 255) as u8;
                                    if let Some(src_px) =
                                        PremultipliedColorU8::from_rgba(pr, pg, pb, src_a)
                                    {
                                        let target_idx = canvas_row_idx + col as usize;
                                        pixels[target_idx] = blend_over(src_px, pixels[target_idx]);
                                    }
                                }
                            }
                        }
                    } else if let Some(mut gpm) = Pixmap::new(gm.width as u32, gm.height as u32) {
                        for (i, a) in bitmap.iter().enumerate() {
                            if *a == 0 {
                                continue;
                            }
                            let src_a = ((*a as u16 * c.a as u16) / 255) as u8;
                            let pr = ((c.r as u16 * src_a as u16) / 255) as u8;
                            let pg = ((c.g as u16 * src_a as u16) / 255) as u8;
                            let pb = ((c.b as u16 * src_a as u16) / 255) as u8;
                            if let Some(px) = PremultipliedColorU8::from_rgba(pr, pg, pb, src_a) {
                                gpm.pixels_mut()[i] = px;
                            }
                        }
                        let t = xf
                            .pre_concat(Transform::from_translate(gx, gy))
                            .pre_concat(Transform::from_scale(1.0 / scale, 1.0 / scale));
                        self.pixmap.draw_pixmap(0, 0, gpm.as_ref(), &paint, t, None);
                    }
                }
                pen_x += gm.advance_width / scale;
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
impl<'a> Draw for Raster<'a> {
    fn fill_rect(&mut self, x: f64, y: f64, w: f64, h: f64, c: Color) {
        if let Some(r) = sk_rect(x, y, w, h) {
            self.pixmap.fill_rect(r, &sk_paint(c), self.xf(), None);
        }
    }

    fn stroke_rect(&mut self, x: f64, y: f64, w: f64, h: f64, c: Color, width: f64) {
        if let Some(r) = sk_rect(x, y, w, h) {
            let path = PathBuilder::from_rect(r);
            self.stroke_path(&path, c, width, false);
        }
    }

    fn fill_round_rect(&mut self, x: f64, y: f64, w: f64, h: f64, r: f64, c: Color) {
        let scale = self.transform_scale();
        if r * scale < 1.5 {
            self.fill_rect(x, y, w, h, c);
            return;
        }
        if let Some(p) = round_rect_path(x, y, w, h, r) {
            let device_max_edge = w.max(h) * scale;
            self.fill_path_maybe_aliased(&p, c, device_max_edge);
        }
    }

    fn stroke_round_rect(&mut self, x: f64, y: f64, w: f64, h: f64, r: f64, c: Color, width: f64) {
        if let Some(p) = round_rect_path(x, y, w, h, r) {
            self.stroke_path(&p, c, width, false);
        }
    }

    fn fill_circle(&mut self, cx: f64, cy: f64, r: f64, c: Color) {
        if let Some(p) = circle_path(cx, cy, r) {
            self.fill_path(&p, c);
        }
    }

    fn stroke_circle(&mut self, cx: f64, cy: f64, r: f64, c: Color, width: f64) {
        if let Some(p) = circle_path(cx, cy, r) {
            self.stroke_path(&p, c, width, false);
        }
    }

    fn fill_ellipse(&mut self, x: f64, y: f64, w: f64, h: f64, c: Color) {
        if let Some(p) = ellipse_path(x, y, w, h) {
            self.fill_path(&p, c);
        }
    }

    fn stroke_ellipse(&mut self, x: f64, y: f64, w: f64, h: f64, c: Color, width: f64) {
        if let Some(p) = ellipse_path(x, y, w, h) {
            self.stroke_path(&p, c, width, false);
        }
    }

    fn fill_poly(&mut self, pts: &[[f64; 2]], c: Color) {
        if let Some(p) = poly_path(pts, true) {
            self.fill_path(&p, c);
        }
    }

    fn stroke_poly(&mut self, pts: &[[f64; 2]], c: Color, width: f64, close: bool) {
        if let Some(p) = poly_path(pts, close) {
            self.stroke_path(&p, c, width, false);
        }
    }

    fn polyline(&mut self, pts: &[[f64; 2]], c: Color, width: f64, dash: bool) {
        if let Some(p) = poly_path(pts, false) {
            self.stroke_path(&p, c, width, dash);
        }
    }

    fn polylines(&mut self, list: &[&[[f64; 2]]], c: Color, width: f64, dash: bool) {
        if list.is_empty() {
            return;
        }
        let mut pb = tiny_skia::PathBuilder::new();
        let mut any = false;
        for pts in list {
            if pts.len() >= 2 {
                pb.move_to(pts[0][0] as f32, pts[0][1] as f32);
                for p in &pts[1..] {
                    pb.line_to(p[0] as f32, p[1] as f32);
                }
                any = true;
            }
        }
        if any {
            if let Some(p) = pb.finish() {
                self.stroke_path(&p, c, width, dash);
            }
        }
    }

    fn line(&mut self, x0: f64, y0: f64, x1: f64, y1: f64, c: Color, width: f64) {
        self.polyline(&[[x0, y0], [x1, y1]], c, width, false);
    }

    fn arc(&mut self, cx: f64, cy: f64, r: f64, a0: f64, a1: f64, c: Color, width: f64) {
        let pts = arc_pts(cx, cy, r, a0, a1);
        self.polyline(&pts, c, width, false);
    }

    fn text(&mut self, x: f64, y: f64, s: &str, size: f64, c: Color, align: Align) {
        self.draw_text_with_font(false, x, y, s, size, c, align);
    }

    fn text_bold(&mut self, x: f64, y: f64, s: &str, size: f64, c: Color, align: Align) {
        self.draw_text_with_font(true, x, y, s, size, c, align);
    }

    fn grid_dots(&mut self, scene: Rect, step: i32, c: Color) {
        let xf = self.xf();
        let is_axis_aligned = xf.kx == 0.0 && xf.ky == 0.0 && xf.sx > 0.0 && xf.sy > 0.0;
        if is_axis_aligned {
            let pw = self.pixmap.width() as f32;
            let ph = self.pixmap.height() as f32;
            let vis_left = ((-xf.tx) / xf.sx) as f64;
            let vis_right = ((pw - xf.tx) / xf.sx) as f64;
            let vis_top = ((-xf.ty) / xf.sy) as f64;
            let vis_bottom = ((ph - xf.ty) / xf.sy) as f64;

            let left = scene.left().max(vis_left - 8.0);
            let right = scene.right().min(vis_right + 8.0);
            let top = scene.top().max(vis_top - 8.0);
            let bottom = scene.bottom().min(vis_bottom + 8.0);

            if left > right || top > bottom {
                return;
            }

            let start_nx = ((left - 4.0) / 8.0).floor() as i32;
            let end_nx = ((right - 4.0) / 8.0).ceil() as i32;
            let start_ny = ((top - 4.0) / 8.0).floor() as i32;
            let end_ny = ((bottom - 4.0) / 8.0).ceil() as i32;
            let step = step.max(1);
            let first_nx = start_nx.div_euclid(step) * step;
            let first_ny = start_ny.div_euclid(step) * step;

            let pw_i = self.pixmap.width() as i32;
            let ph_i = self.pixmap.height() as i32;
            let r_px = (0.75 * xf.sx as f64).clamp(0.65, 1.6) as f32;

            let src_a = c.a;
            let pr = ((c.r as u16 * src_a as u16) / 255) as u8;
            let pg = ((c.g as u16 * src_a as u16) / 255) as u8;
            let pb = ((c.b as u16 * src_a as u16) / 255) as u8;
            let center_color = match PremultipliedColorU8::from_rgba(pr, pg, pb, src_a) {
                Some(cc) => cc,
                None => return,
            };

            let pixels = self.pixmap.pixels_mut();
            let stride = pw_i as usize;

            if r_px <= 0.85 {
                for nx in (first_nx..=end_nx).step_by(step as usize) {
                    let x = 8.0 * (nx as f32) + 4.0;
                    let dx = (xf.sx * x + xf.tx).round() as i32;
                    if dx < 0 || dx >= pw_i {
                        continue;
                    }
                    let col = dx as usize;
                    for ny in (first_ny..=end_ny).step_by(step as usize) {
                        let y = 8.0 * (ny as f32) + 4.0;
                        let dy = (xf.sy * y + xf.ty).round() as i32;
                        if dy >= 0 && dy < ph_i {
                            let idx = (dy as usize) * stride + col;
                            pixels[idx] = blend_over(center_color, pixels[idx]);
                        }
                    }
                }
            } else {
                let cov_ortho = ((r_px + 0.5 - 1.0) * 0.75).clamp(0.15, 0.75);
                let ortho_a = ((src_a as f32 * cov_ortho).round() as u16).min(255) as u8;
                let ortho_r = ((c.r as u16 * ortho_a as u16) / 255) as u8;
                let ortho_g = ((c.g as u16 * ortho_a as u16) / 255) as u8;
                let ortho_b = ((c.b as u16 * ortho_a as u16) / 255) as u8;
                let ortho_color =
                    PremultipliedColorU8::from_rgba(ortho_r, ortho_g, ortho_b, ortho_a)
                        .unwrap_or(center_color);

                let diag_color = if r_px > 1.25 {
                    let cov_diag = ((r_px + 0.5 - 1.414) * 0.4).clamp(0.05, 0.35);
                    let diag_a = ((src_a as f32 * cov_diag).round() as u16).min(255) as u8;
                    let diag_r = ((c.r as u16 * diag_a as u16) / 255) as u8;
                    let diag_g = ((c.g as u16 * diag_a as u16) / 255) as u8;
                    let diag_b = ((c.b as u16 * diag_a as u16) / 255) as u8;
                    PremultipliedColorU8::from_rgba(diag_r, diag_g, diag_b, diag_a)
                } else {
                    None
                };

                for nx in (first_nx..=end_nx).step_by(step as usize) {
                    let x = 8.0 * (nx as f32) + 4.0;
                    let dx = (xf.sx * x + xf.tx).round() as i32;
                    if dx < 1 || dx >= pw_i - 1 {
                        if dx >= 0 && dx < pw_i {
                            let col = dx as usize;
                            for ny in (first_ny..=end_ny).step_by(step as usize) {
                                let y = 8.0 * (ny as f32) + 4.0;
                                let dy = (xf.sy * y + xf.ty).round() as i32;
                                if dy >= 0 && dy < ph_i {
                                    let idx = (dy as usize) * stride + col;
                                    pixels[idx] = blend_over(center_color, pixels[idx]);
                                }
                            }
                        }
                        continue;
                    }
                    let col = dx as usize;
                    for ny in (first_ny..=end_ny).step_by(step as usize) {
                        let y = 8.0 * (ny as f32) + 4.0;
                        let dy = (xf.sy * y + xf.ty).round() as i32;
                        if dy < 1 || dy >= ph_i - 1 {
                            if dy >= 0 && dy < ph_i {
                                let idx = (dy as usize) * stride + col;
                                pixels[idx] = blend_over(center_color, pixels[idx]);
                            }
                            continue;
                        }
                        let center_idx = (dy as usize) * stride + col;
                        pixels[center_idx] = blend_over(center_color, pixels[center_idx]);
                        pixels[center_idx - 1] = blend_over(ortho_color, pixels[center_idx - 1]);
                        pixels[center_idx + 1] = blend_over(ortho_color, pixels[center_idx + 1]);
                        pixels[center_idx - stride] =
                            blend_over(ortho_color, pixels[center_idx - stride]);
                        pixels[center_idx + stride] =
                            blend_over(ortho_color, pixels[center_idx + stride]);
                        if let Some(dc) = diag_color {
                            pixels[center_idx - stride - 1] =
                                blend_over(dc, pixels[center_idx - stride - 1]);
                            pixels[center_idx - stride + 1] =
                                blend_over(dc, pixels[center_idx - stride + 1]);
                            pixels[center_idx + stride - 1] =
                                blend_over(dc, pixels[center_idx + stride - 1]);
                            pixels[center_idx + stride + 1] =
                                blend_over(dc, pixels[center_idx + stride + 1]);
                        }
                    }
                }
            }
            return;
        }

        let start_nx = ((scene.left() - 4.0) / 8.0).floor() as i32;
        let end_nx = ((scene.right() - 4.0) / 8.0).ceil() as i32;
        let start_ny = ((scene.top() - 4.0) / 8.0).floor() as i32;
        let end_ny = ((scene.bottom() - 4.0) / 8.0).ceil() as i32;
        let first_nx = start_nx.div_euclid(step) * step;
        let first_ny = start_ny.div_euclid(step) * step;
        let r = 0.75;
        for nx in (first_nx..=end_nx).step_by(step.max(1) as usize) {
            let x = f64::from(8 * nx + 4);
            if x < scene.left() || x > scene.right() {
                continue;
            }
            for ny in (first_ny..=end_ny).step_by(step.max(1) as usize) {
                let y = f64::from(8 * ny + 4);
                if y < scene.top() || y > scene.bottom() {
                    continue;
                }
                self.fill_circle(x, y, r, c);
            }
        }
    }

    fn push(&mut self, x: f64, y: f64, rot: f64, sx: f64, sy: f64) {
        let extra = Transform::from_translate(x as f32, y as f32)
            .pre_concat(Transform::from_rotate(rot as f32))
            .pre_concat(Transform::from_scale(sx as f32, sy as f32));
        self.stack.push(self.xf().pre_concat(extra));
    }

    fn pop(&mut self) {
        if self.stack.len() > 1 {
            self.stack.pop();
        }
    }

    fn device_scale(&self) -> f64 {
        self.transform_scale()
    }

    fn blit_pixmap(&mut self, x: f64, y: f64, pm: &Pixmap, pm_scale: f64) -> bool {
        let inv = 1.0 / pm_scale.max(0.1) as f32;
        let local = Transform::from_row(inv, 0.0, 0.0, inv, x as f32, y as f32);
        let t = self.xf().pre_concat(local);
        let paint = PixmapPaint {
            quality: tiny_skia::FilterQuality::Nearest,
            ..PixmapPaint::default()
        };
        self.pixmap.draw_pixmap(0, 0, pm.as_ref(), &paint, t, None);
        true
    }

    fn draw_pixmap_rect(
        &mut self,
        x: f64,
        y: f64,
        w: f64,
        h: f64,
        pm: &Pixmap,
        opacity: f64,
    ) -> bool {
        if pm.width() == 0 || pm.height() == 0 || w <= 0.0 || h <= 0.0 {
            return false;
        }
        let sx = (w as f32) / (pm.width() as f32);
        let sy = (h as f32) / (pm.height() as f32);
        let local = Transform::from_row(sx, 0.0, 0.0, sy, x as f32, y as f32);
        let t = self.xf().pre_concat(local);
        let paint = PixmapPaint {
            opacity: opacity.clamp(0.0, 1.0) as f32,
            quality: tiny_skia::FilterQuality::Bilinear,
            ..PixmapPaint::default()
        };
        self.pixmap.draw_pixmap(0, 0, pm.as_ref(), &paint, t, None);
        true
    }
}
