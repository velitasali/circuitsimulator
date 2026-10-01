//! Records canvas `Draw` commands into a Vello scene.

use std::sync::{Arc, OnceLock};

use super::export::arc;
use super::export::paint;
use super::export::text;
use super::{Align, Canvas, Color, Draw, ImageBuf, PaintCtx, Palette, Point, Rect};
use skrifa::{FontRef, GlyphId, MetadataProvider, instance::LocationRef, instance::Size};
use vello::kurbo::{Affine, BezPath, Circle, Ellipse, Rect as KRect, RoundedRect, Shape, Stroke};
use vello::peniko::{Blob, Fill, Font, Image, ImageFormat, ImageQuality, Mix};
use vello::{Glyph, Scene};

const FONT_REGULAR: &[u8] = include_bytes!("../../../../resources/fonts/Ubuntu-R.ttf");
const FONT_BOLD: &[u8] = include_bytes!("../../../../resources/fonts/Ubuntu-B.ttf");

pub fn color_of(c: Color) -> vello::peniko::Color {
    vello::peniko::Color::from_rgba8(c.r, c.g, c.b, c.a)
}

pub fn record(
    scene: &mut Scene,
    canvas: &Canvas,
    palette: &Palette,
    width: u32,
    height: u32,
    dpr: f64,
) {
    record_at(
        scene, canvas, palette, width, height, 0, 0, width, height, dpr,
    );
}

/// Record the circuit into a texture that covers one rectangle of the full frame.
/// `(origin_x, origin_y)` is that rectangle's top-left in full-frame pixels.
pub fn record_at(
    scene: &mut Scene,
    canvas: &Canvas,
    palette: &Palette,
    full_w: u32,
    full_h: u32,
    origin_x: u32,
    origin_y: u32,
    patch_w: u32,
    patch_h: u32,
    dpr: f64,
) {
    let vp = canvas.viewport();
    record_scene(
        scene,
        canvas,
        palette,
        full_w,
        full_h,
        origin_x,
        origin_y,
        patch_w,
        patch_h,
        dpr,
        true,
        vp.center(),
        vp.zoom(),
    );
}

/// Same rectangle as [`record_at`], without the paper fill or the grid.
/// A patch copied onto an existing frame uses this.
pub fn record_items_at(
    scene: &mut Scene,
    canvas: &Canvas,
    palette: &Palette,
    full_w: u32,
    full_h: u32,
    origin_x: u32,
    origin_y: u32,
    patch_w: u32,
    patch_h: u32,
    dpr: f64,
) {
    let vp = canvas.viewport();
    record_items_view(
        scene,
        canvas,
        palette,
        full_w,
        full_h,
        origin_x,
        origin_y,
        patch_w,
        patch_h,
        dpr,
        vp.center(),
        vp.zoom(),
    );
}

/// [`record_items_at`] with an explicit projection. Region export uses this when
/// the pixmap center and zoom differ from the canvas viewport.
pub(crate) fn record_items_view(
    scene: &mut Scene,
    canvas: &Canvas,
    palette: &Palette,
    full_w: u32,
    full_h: u32,
    origin_x: u32,
    origin_y: u32,
    patch_w: u32,
    patch_h: u32,
    dpr: f64,
    center: Point,
    zoom: f64,
) {
    record_scene(
        scene, canvas, palette, full_w, full_h, origin_x, origin_y, patch_w, patch_h, dpr, false,
        center, zoom,
    );
}

fn record_scene(
    scene: &mut Scene,
    canvas: &Canvas,
    palette: &Palette,
    full_w: u32,
    full_h: u32,
    origin_x: u32,
    origin_y: u32,
    patch_w: u32,
    patch_h: u32,
    dpr: f64,
    paper: bool,
    center: Point,
    zoom: f64,
) {
    scene.reset();
    let vp = canvas.viewport();
    let s = (zoom * dpr).max(1e-6);
    let tx = full_w as f64 * 0.5 - origin_x as f64 - center.x * s;
    let ty = full_h as f64 * 0.5 - origin_y as f64 - center.y * s;
    let world = Affine::new([s, 0.0, 0.0, s, tx, ty]);

    let mut draw = VelloDraw {
        scene,
        stack: vec![world],
        clips: Vec::new(),
        view_w: patch_w as f64,
        view_h: patch_h as f64,
        zoom,
    };
    if paper {
        draw.stack.push(Affine::IDENTITY);
        draw.fill_rect(0.0, 0.0, patch_w as f64, patch_h as f64, palette.canvas);
        draw.stack.pop();
        if canvas.show_grid() {
            paint::paint_grid(&mut draw, vp.scene_rect(), zoom, palette.grid);
        }
    }

    let scene_x = (origin_x as f64 - full_w as f64 * 0.5) / s + center.x;
    let scene_y = (origin_y as f64 - full_h as f64 * 0.5) / s + center.y;
    let scene_w = patch_w as f64 / s;
    let scene_h = patch_h as f64 / s;
    let cull = Rect::new(
        scene_x - 80.0,
        scene_y - 80.0,
        scene_w + 160.0,
        scene_h + 160.0,
    );
    let circuit = canvas.scene();
    let ctx = PaintCtx {
        canvas,
        pal: palette,
        scale: zoom,
        item_id: "",
    };
    let connected = circuit.connected_pins_set();
    for wire in circuit.wires() {
        if wire.points.len() >= 2 && cull.intersects(&wire.bounding_rect()) {
            paint::paint_wire(&mut draw, &ctx, wire);
        }
    }
    for item in circuit.items() {
        if cull.intersects(&item.total_bounding_rect()) {
            paint::paint_item(&mut draw, &ctx, circuit, item, &connected);
            if canvas.show_component_rect() {
                paint::paint_component_rect(&mut draw, &ctx, item);
            }
        }
    }
    for wire in circuit.wires() {
        if wire.points.len() >= 2 && cull.intersects(&wire.bounding_rect()) {
            paint::paint_wire_chevrons(&mut draw, &ctx, wire);
        }
    }
    while let Some(open) = draw.clips.pop() {
        if open {
            draw.scene.pop_layer();
        }
    }
}

struct VelloDraw<'a> {
    scene: &'a mut Scene,
    stack: Vec<Affine>,
    clips: Vec<bool>,
    view_w: f64,
    view_h: f64,
    zoom: f64,
}

impl VelloDraw<'_> {
    fn xf(&self) -> Affine {
        self.stack.last().copied().unwrap_or(Affine::IDENTITY)
    }

    fn fill_shape(&mut self, shape: &impl vello::kurbo::Shape, c: Color) {
        if c.a == 0 {
            return;
        }
        self.scene
            .fill(Fill::NonZero, self.xf(), color_of(c), None, shape);
    }

    fn stroke_shape(&mut self, shape: &impl vello::kurbo::Shape, c: Color, width: f64, dash: bool) {
        if c.a == 0 {
            return;
        }
        let mut style = Stroke::new(width.max(0.05));
        if dash {
            style = style.with_dashes(0.0, [4.0, 3.0]);
        }
        self.scene
            .stroke(&style, self.xf(), color_of(c), None, shape);
    }
}

impl Draw for VelloDraw<'_> {
    fn fill_rect(&mut self, x: f64, y: f64, w: f64, h: f64, c: Color) {
        if let Some(r) = krect(x, y, w, h) {
            self.fill_shape(&r, c);
        }
    }

    fn stroke_rect(&mut self, x: f64, y: f64, w: f64, h: f64, c: Color, width: f64) {
        if let Some(r) = krect(x, y, w, h) {
            self.stroke_shape(&r, c, width, false);
        }
    }

    fn fill_round_rect(&mut self, x: f64, y: f64, w: f64, h: f64, r: f64, c: Color) {
        if let Some(rect) = krect(x, y, w, h) {
            if r <= 0.0 {
                self.fill_shape(&rect, c);
            } else {
                let round = RoundedRect::from_rect(rect, r);
                self.fill_shape(&round, c);
            }
        }
    }

    fn stroke_round_rect(&mut self, x: f64, y: f64, w: f64, h: f64, r: f64, c: Color, width: f64) {
        if let Some(rect) = krect(x, y, w, h) {
            if r <= 0.0 {
                self.stroke_shape(&rect, c, width, false);
            } else {
                let round = RoundedRect::from_rect(rect, r);
                self.stroke_shape(&round, c, width, false);
            }
        }
    }

    fn fill_circle(&mut self, cx: f64, cy: f64, r: f64, c: Color) {
        if r > 0.0 {
            let circle = Circle::new((cx, cy), r);
            self.fill_shape(&circle, c);
        }
    }

    fn stroke_circle(&mut self, cx: f64, cy: f64, r: f64, c: Color, width: f64) {
        if r > 0.0 {
            let circle = Circle::new((cx, cy), r);
            self.stroke_shape(&circle, c, width, false);
        }
    }

    fn fill_ellipse(&mut self, x: f64, y: f64, w: f64, h: f64, c: Color) {
        if let Some(rect) = krect(x, y, w, h) {
            let ellipse = Ellipse::from_rect(rect);
            self.fill_shape(&ellipse, c);
        }
    }

    fn stroke_ellipse(&mut self, x: f64, y: f64, w: f64, h: f64, c: Color, width: f64) {
        if let Some(rect) = krect(x, y, w, h) {
            let ellipse = Ellipse::from_rect(rect);
            self.stroke_shape(&ellipse, c, width, false);
        }
    }

    fn fill_poly(&mut self, pts: &[[f64; 2]], c: Color) {
        if let Some(path) = bez_lines(pts, true) {
            self.fill_shape(&path, c);
        }
    }

    fn stroke_poly(&mut self, pts: &[[f64; 2]], c: Color, width: f64, close: bool) {
        if let Some(path) = bez_lines(pts, close) {
            self.stroke_shape(&path, c, width, false);
        }
    }

    fn polyline(&mut self, pts: &[[f64; 2]], c: Color, width: f64, dash: bool) {
        if let Some(path) = bez_lines(pts, false) {
            self.stroke_shape(&path, c, width, dash);
        }
    }

    fn line(&mut self, x0: f64, y0: f64, x1: f64, y1: f64, c: Color, width: f64) {
        self.polyline(&[[x0, y0], [x1, y1]], c, width, false);
    }

    fn arc(&mut self, cx: f64, cy: f64, r: f64, a0: f64, a1: f64, c: Color, width: f64) {
        let pts = arc::arc_pts(cx, cy, r, a0, a1);
        self.polyline(&pts, c, width, false);
    }

    fn text(&mut self, x: f64, y: f64, s: &str, size: f64, c: Color, align: Align) {
        let xf = self.xf();
        draw_text(self.scene, xf, false, x, y, s, size, c, align);
    }

    fn text_bold(&mut self, x: f64, y: f64, s: &str, size: f64, c: Color, align: Align) {
        let xf = self.xf();
        draw_text(self.scene, xf, true, x, y, s, size, c, align);
    }

    fn grid_dots(&mut self, scene: Rect, step: i32, c: Color) {
        let Some(visible) = self.visible_scene_bounds() else {
            return;
        };
        let left = scene.left().max(visible.left() - 8.0);
        let right = scene.right().min(visible.right() + 8.0);
        let top = scene.top().max(visible.top() - 8.0);
        let bottom = scene.bottom().min(visible.bottom() + 8.0);
        if left > right || top > bottom {
            return;
        }
        let mut step = step.max(1);
        let mut start_nx = ((left - 4.0) / 8.0).floor() as i32;
        let end_nx = ((right - 4.0) / 8.0).ceil() as i32;
        let mut start_ny = ((top - 4.0) / 8.0).floor() as i32;
        let end_ny = ((bottom - 4.0) / 8.0).ceil() as i32;
        start_nx = start_nx.div_euclid(step) * step;
        start_ny = start_ny.div_euclid(step) * step;
        let span = |a: i32, b: i32, step: i32| ((b - a) / step).saturating_add(1).max(0) as i64;
        while span(start_nx, end_nx, step) * span(start_ny, end_ny, step) > 64_000 {
            step *= 2;
            start_nx = start_nx.div_euclid(step) * step;
            start_ny = start_ny.div_euclid(step) * step;
        }
        // grid.frag: device radius `0.75 * dpr * max(1, zoom)`, which is 0.75
        // scene units from 1× up and 0.75 logical px when zoomed out.
        let radius = shader_dot_radius(self.zoom, 0.75);
        self.fill_dot_lattice(
            start_nx, end_nx, start_ny, end_ny, step, left, right, top, bottom, 4.0, radius, c,
        );
        // Half-step dots (x = 8n+8) appear on the main canvas at zoom >= 2.
        if self.zoom >= 2.0 && step == 1 {
            let half = shader_dot_radius(self.zoom, 0.5);
            let mut hx0 = ((left - 8.0) / 8.0).floor() as i32;
            let hx1 = ((right - 8.0) / 8.0).ceil() as i32;
            let mut hy0 = ((top - 8.0) / 8.0).floor() as i32;
            let hy1 = ((bottom - 8.0) / 8.0).ceil() as i32;
            hx0 = hx0.div_euclid(step) * step;
            hy0 = hy0.div_euclid(step) * step;
            self.fill_dot_lattice(
                hx0,
                hx1,
                hy0,
                hy1,
                step,
                left,
                right,
                top,
                bottom,
                8.0,
                half,
                c.fade(0.45),
            );
        }
    }

    fn push(&mut self, x: f64, y: f64, rot: f64, sx: f64, sy: f64) {
        // Draw rotation is degrees, matching tiny-skia and SVG. kurbo rotates in radians.
        let extra = Affine::translate((x, y))
            * Affine::rotate(rot.to_radians())
            * Affine::scale_non_uniform(sx, sy);
        self.stack.push(self.xf() * extra);
    }

    fn pop(&mut self) {
        if self.stack.len() > 1 {
            self.stack.pop();
        }
    }

    fn push_clip_rect(&mut self, x: f64, y: f64, w: f64, h: f64) {
        if let Some(r) = krect(x, y, w, h) {
            self.scene.push_layer(Mix::Clip, 1.0, self.xf(), &r);
            self.clips.push(true);
        } else {
            self.clips.push(false);
        }
    }

    fn pop_clip(&mut self) {
        if self.clips.pop() == Some(true) {
            self.scene.pop_layer();
        }
    }

    fn device_scale(&self) -> f64 {
        let c = self.xf().as_coeffs();
        let sx = (c[0] * c[0] + c[1] * c[1]).sqrt();
        let sy = (c[2] * c[2] + c[3] * c[3]).sqrt();
        ((sx + sy) * 0.5).max(0.1)
    }

    fn blit_pixmap(&mut self, x: f64, y: f64, image: &ImageBuf, pm_scale: f64) -> bool {
        if image.width() == 0 || image.height() == 0 {
            return false;
        }
        let scale = 1.0 / pm_scale.max(0.1);
        self.paint_image(
            x,
            y,
            image.width() as f64 * scale,
            image.height() as f64 * scale,
            image,
            1.0,
            ImageQuality::Low,
        )
    }

    fn draw_pixmap_rect(
        &mut self,
        x: f64,
        y: f64,
        w: f64,
        h: f64,
        image: &ImageBuf,
        opacity: f64,
    ) -> bool {
        self.paint_image(x, y, w, h, image, opacity, ImageQuality::Medium)
    }
}

impl VelloDraw<'_> {
    fn paint_image(
        &mut self,
        x: f64,
        y: f64,
        w: f64,
        h: f64,
        image: &ImageBuf,
        opacity: f64,
        quality: ImageQuality,
    ) -> bool {
        if image.width() == 0 || image.height() == 0 || w <= 0.0 || h <= 0.0 || opacity <= 0.0 {
            return false;
        }
        let blob = Blob::new(Arc::new(image.pixels().to_vec()));
        let mut picture = Image::new(blob, ImageFormat::Rgba8, image.width(), image.height());
        picture.quality = quality;
        picture.alpha = opacity.clamp(0.0, 1.0) as f32;
        let sx = w / image.width() as f64;
        let sy = h / image.height() as f64;
        let local = Affine::new([sx, 0.0, 0.0, sy, x, y]);
        self.scene.draw_image(&picture, self.xf() * local);
        true
    }
}

/// Scene-space radius matching `grid.frag`: device radius is
/// `logical_px * dpr * max(1, zoom)`.
fn shader_dot_radius(zoom: f64, logical_px: f64) -> f64 {
    let zoom = zoom.max(1e-6);
    logical_px * zoom.max(1.0) / zoom
}

impl VelloDraw<'_> {
    fn fill_dot_lattice(
        &mut self,
        start_nx: i32,
        end_nx: i32,
        start_ny: i32,
        end_ny: i32,
        step: i32,
        left: f64,
        right: f64,
        top: f64,
        bottom: f64,
        origin: f64,
        radius: f64,
        c: Color,
    ) {
        if radius <= 0.0 || c.a == 0 || step < 1 {
            return;
        }
        let mut path = BezPath::new();
        for nx in (start_nx..=end_nx).step_by(step as usize) {
            let x = f64::from(8 * nx) + origin;
            if x < left || x > right {
                continue;
            }
            for ny in (start_ny..=end_ny).step_by(step as usize) {
                let y = f64::from(8 * ny) + origin;
                if y < top || y > bottom {
                    continue;
                }
                path.extend(Circle::new((x, y), radius).path_elements(0.25));
            }
        }
        if !path.is_empty() {
            self.fill_shape(&path, c);
        }
    }

    fn visible_scene_bounds(&self) -> Option<Rect> {
        let xf = self.xf();
        let det = xf.determinant();
        if !det.is_finite() || det.abs() < 1e-9 {
            return None;
        }
        let inv = xf.inverse();
        let corners = [
            (0.0, 0.0),
            (self.view_w, 0.0),
            (0.0, self.view_h),
            (self.view_w, self.view_h),
        ];
        let mut min_x = f64::INFINITY;
        let mut min_y = f64::INFINITY;
        let mut max_x = f64::NEG_INFINITY;
        let mut max_y = f64::NEG_INFINITY;
        for (x, y) in corners {
            let p = inv * vello::kurbo::Point::new(x, y);
            if !p.x.is_finite() || !p.y.is_finite() {
                return None;
            }
            min_x = min_x.min(p.x);
            min_y = min_y.min(p.y);
            max_x = max_x.max(p.x);
            max_y = max_y.max(p.y);
        }
        if max_x <= min_x || max_y <= min_y {
            return None;
        }
        Some(Rect::new(min_x, min_y, max_x - min_x, max_y - min_y))
    }
}

fn krect(x: f64, y: f64, w: f64, h: f64) -> Option<KRect> {
    if w <= 0.0 || h <= 0.0 || !x.is_finite() || !y.is_finite() {
        return None;
    }
    Some(KRect::new(x, y, x + w, y + h))
}

fn bez_lines(pts: &[[f64; 2]], close: bool) -> Option<BezPath> {
    if pts.len() < 2 {
        return None;
    }
    let mut path = BezPath::new();
    path.move_to((pts[0][0], pts[0][1]));
    for p in &pts[1..] {
        path.line_to((p[0], p[1]));
    }
    if close {
        path.close_path();
    }
    Some(path)
}

fn font_data(bold: bool) -> &'static Font {
    static REGULAR: OnceLock<Font> = OnceLock::new();
    static BOLD: OnceLock<Font> = OnceLock::new();
    let slot = if bold { &BOLD } else { &REGULAR };
    slot.get_or_init(|| {
        let bytes: &'static [u8] = if bold { FONT_BOLD } else { FONT_REGULAR };
        Font::new(Blob::new(Arc::new(bytes)), 0)
    })
}

fn draw_text(
    scene: &mut Scene,
    xf: Affine,
    bold: bool,
    x: f64,
    y: f64,
    text: &str,
    size: f64,
    c: Color,
    align: Align,
) {
    if text.is_empty() || c.a == 0 || size <= 0.0 {
        return;
    }
    let clean = text::strip_tags(text);
    let bytes = if bold { FONT_BOLD } else { FONT_REGULAR };
    let Ok(font_ref) = FontRef::new(bytes) else {
        return;
    };
    let px = size as f32;
    let metrics = font_ref.metrics(Size::new(px), LocationRef::default());
    let glyph_metrics = font_ref.glyph_metrics(Size::new(px), LocationRef::default());
    let charmap = font_ref.charmap();
    let ascent = if metrics.ascent.abs() > 0.01 {
        metrics.ascent
    } else {
        px
    };
    let line_height = (ascent - metrics.descent + metrics.leading).max(px);
    let lines: Vec<&str> = clean.split('\n').collect();
    let num_lines = lines.len();
    let mut glyphs = Vec::new();
    for (line_idx, line) in lines.into_iter().enumerate() {
        if line.is_empty() {
            continue;
        }
        let width = if align != Align::TopLeft {
            line.chars()
                .map(|ch| {
                    let gid = charmap.map(u32::from(ch)).unwrap_or(GlyphId::NOTDEF);
                    glyph_metrics.advance_width(gid).unwrap_or(0.0)
                })
                .sum::<f32>()
        } else {
            0.0
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
            let gid = charmap.map(u32::from(ch)).unwrap_or(GlyphId::NOTDEF);
            glyphs.push(Glyph {
                id: gid.to_u32(),
                x: pen_x,
                y: baseline,
            });
            pen_x += glyph_metrics.advance_width(gid).unwrap_or(0.0);
        }
    }
    if glyphs.is_empty() {
        return;
    }
    scene
        .draw_glyphs(font_data(bold))
        .transform(xf)
        .font_size(px)
        .brush(color_of(c))
        .draw(Fill::NonZero, glyphs.into_iter());
}
