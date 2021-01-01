//! PNG / JPEG / BMP / SVG from the canvas scene. No Qt.
//!
//! Raster matches C++ `CircuitCanvas::saveImage`: visible rect stretched to
//! the current view size, paper + painted grid, then scene primitives.
//! SVG uses the same primitives with a scene-rect viewBox.

pub mod paint;
pub mod raster;
pub mod svg;
pub mod text;

#[cfg(test)]
mod tests;

use std::io::Write;
use std::path::Path;
use tiny_skia::Transform;

use self::paint::{
    paint_component_rect, paint_grid, paint_item, paint_scene, paint_wire, paint_wire_chevrons,
};
use self::raster::{Raster, sk_color};
use self::svg::Svg;
use super::Canvas;
use super::geom::Rect;
use super::scene::Scene;
use crate::Error;

pub use super::draw::{Align, Color, Draw, PaintCtx, Palette, parse_hex};
pub use text::{font, font_bold, text_width};
pub use tiny_skia::Pixmap;

pub fn save_image(canvas: &Canvas, path: &Path, palette: &Palette) -> crate::Result<()> {
    match format_of(path) {
        ImageKind::Svg => {
            let s = svg_string(canvas, palette);
            std::fs::write(path, s)?;
            Ok(())
        }
        kind => {
            let pm = raster(canvas, palette)?;
            match kind {
                ImageKind::Png => pm
                    .save_png(path)
                    .map_err(|e| Error::Export(format!("png: {e}"))),
                ImageKind::Jpeg => encode_jpeg(&pm, path),
                ImageKind::Bmp => encode_bmp(&pm, path),
                ImageKind::Svg => unreachable!(),
            }
        }
    }
}

pub fn svg_string(canvas: &Canvas, palette: &Palette) -> String {
    let sr = canvas.viewport().scene_rect();
    let mut svg = Svg::new(sr);
    paint_scene(
        &mut svg,
        &PaintCtx {
            canvas,
            pal: palette,
            scale: 1.0,
            item_id: "",
        },
    );
    svg.finish()
}

pub fn png_bytes(canvas: &Canvas, palette: &Palette) -> crate::Result<Vec<u8>> {
    raster(canvas, palette)?
        .encode_png()
        .map_err(|e| Error::Export(format!("png: {e}")))
}

pub fn render_viewport(
    canvas: &Canvas,
    palette: &Palette,
    width: u32,
    height: u32,
    dpr: f64,
    draw_background: bool,
) -> Option<Pixmap> {
    let mut pm = Pixmap::new(width.max(1), height.max(1))?;
    render_viewport_mut(canvas, palette, &mut pm, dpr, draw_background);
    Some(pm)
}

pub fn render_viewport_mut(
    canvas: &Canvas,
    palette: &Palette,
    pixmap: &mut Pixmap,
    dpr: f64,
    draw_background: bool,
) {
    let w = pixmap.width();
    let h = pixmap.height();
    let vp = canvas.viewport();
    let zoom = vp.zoom();
    let center = vp.center();
    let s = (zoom * dpr) as f32;
    let tx = (w as f64 * 0.5 - center.x * zoom * dpr) as f32;
    let ty = (h as f64 * 0.5 - center.y * zoom * dpr) as f32;
    let world = Transform::from_row(s, 0.0, 0.0, s, tx, ty);
    if draw_background {
        pixmap.fill(sk_color(palette.canvas));
    } else {
        pixmap.fill(tiny_skia::Color::TRANSPARENT);
    }
    let mut rast = Raster::new(pixmap, world);
    if draw_background && canvas.show_grid() {
        paint_grid(&mut rast, vp.scene_rect(), zoom, palette.grid);
    }
    let pixmap_world_w = (w as f64) / (zoom * dpr).max(1e-9);
    let pixmap_world_h = (h as f64) / (zoom * dpr).max(1e-9);
    let cull = Rect::new(
        center.x - pixmap_world_w * 0.5 - 80.0,
        center.y - pixmap_world_h * 0.5 - 80.0,
        pixmap_world_w + 160.0,
        pixmap_world_h + 160.0,
    );
    let connected_pins = canvas.scene().connected_pins_set();
    paint_culled(&mut rast, canvas, palette, zoom, cull, &connected_pins);
}

/// Re-raster `scene_rects` into an existing pixmap. Returns false when the
/// caller should fall back to a full [`render_viewport_mut`] (empty, or the
/// dirty area covers most of the surface).
pub fn render_viewport_regions_mut(
    canvas: &Canvas,
    palette: &Palette,
    pixmap: &mut Pixmap,
    dpr: f64,
    scene_rects: &[Rect],
) -> bool {
    if scene_rects.is_empty() {
        return false;
    }
    let w = pixmap.width();
    let h = pixmap.height();
    if w == 0 || h == 0 {
        return false;
    }
    let vp = canvas.viewport();
    let zoom = vp.zoom();
    let center = vp.center();
    let s = zoom * dpr.max(1.0);
    if s <= 1e-9 {
        return false;
    }
    let tx = w as f64 * 0.5 - center.x * s;
    let ty = h as f64 * 0.5 - center.y * s;

    let mut device_rects: Vec<Rect> = Vec::new();
    for sr in scene_rects {
        if let Some(dr) = scene_to_device_rect(*sr, s, tx, ty, 2.0, w, h) {
            device_rects.push(dr);
        }
    }
    if device_rects.is_empty() {
        return false;
    }
    let merged = merge_device_rects(device_rects, 6, 4000.0);
    let dirty_area: f64 = merged.iter().map(Rect::area).sum();
    if dirty_area > (w as f64) * (h as f64) * 0.5 {
        return false;
    }

    let connected_pins = canvas.scene().connected_pins_set();
    let pw = pixmap.width() as i32;
    let ph = pixmap.height() as i32;
    for dr in merged {
        let x0 = dr.x.round() as i32;
        let y0 = dr.y.round() as i32;
        let rw = dr.w.round().max(1.0) as u32;
        let rh = dr.h.round().max(1.0) as u32;
        let Some(mut sub) = Pixmap::new(rw, rh) else {
            continue;
        };
        // Live canvas pixmap is transparent paper (GPU grid shader sits
        // underneath). Do not fill canvas color or software grid here —
        // Source blit of an opaque patch would stain the background.
        let world = Transform::from_row(
            s as f32,
            0.0,
            0.0,
            s as f32,
            (tx - x0 as f64) as f32,
            (ty - y0 as f64) as f32,
        );
        let mut rast = Raster::new(&mut sub, world);
        let cull = Rect::new(
            (x0 as f64 - tx) / s - 8.0,
            (y0 as f64 - ty) / s - 8.0,
            rw as f64 / s + 16.0,
            rh as f64 / s + 16.0,
        );
        paint_culled(&mut rast, canvas, palette, zoom, cull, &connected_pins);
        let dst_data = pixmap.data_mut();
        let src_data = sub.data();
        for row in 0..rh as i32 {
            let dst_y = y0 + row;
            if dst_y < 0 || dst_y >= ph {
                continue;
            }
            let dst_x_start = x0.max(0);
            let dst_x_end = (x0 + rw as i32).min(pw);
            if dst_x_start >= dst_x_end {
                continue;
            }
            let src_x_offset = (dst_x_start - x0) as usize;
            let copy_pixels = (dst_x_end - dst_x_start) as usize;
            let src_row_start = (row as usize * rw as usize + src_x_offset) * 4;
            let dst_row_start = (dst_y as usize * pw as usize + dst_x_start as usize) * 4;
            let byte_len = copy_pixels * 4;
            dst_data[dst_row_start..dst_row_start + byte_len]
                .copy_from_slice(&src_data[src_row_start..src_row_start + byte_len]);
        }
    }
    true
}

fn paint_culled(
    rast: &mut Raster<'_>,
    canvas: &Canvas,
    palette: &Palette,
    zoom: f64,
    cull: Rect,
    connected_pins: &rustc_hash::FxHashSet<String>,
) {
    let scene: &Scene = canvas.scene();
    let ctx = PaintCtx {
        canvas,
        pal: palette,
        scale: zoom,
        item_id: "",
    };
    for w in scene.wires() {
        if w.points.len() >= 2 && !cull.intersects(&w.bounding_rect()) {
            continue;
        }
        paint_wire(rast, &ctx, w);
    }
    for it in scene.items() {
        if !it.total_bounding_rect().intersects(&cull) {
            continue;
        }
        paint_item(rast, &ctx, scene, it, connected_pins);
        if canvas.show_component_rect() {
            paint_component_rect(rast, &ctx, it);
        }
    }
    for w in scene.wires() {
        if w.points.len() >= 2 && !cull.intersects(&w.bounding_rect()) {
            continue;
        }
        paint_wire_chevrons(rast, &ctx, w);
    }
}

fn scene_to_device_rect(
    sr: Rect,
    s: f64,
    tx: f64,
    ty: f64,
    pad: f64,
    pw: u32,
    ph: u32,
) -> Option<Rect> {
    let x0 = (sr.x * s + tx - pad).floor();
    let y0 = (sr.y * s + ty - pad).floor();
    let x1 = ((sr.x + sr.w) * s + tx + pad).ceil();
    let y1 = ((sr.y + sr.h) * s + ty + pad).ceil();
    let x0 = x0.max(0.0);
    let y0 = y0.max(0.0);
    let x1 = x1.min(pw as f64);
    let y1 = y1.min(ph as f64);
    if x1 <= x0 || y1 <= y0 {
        return None;
    }
    Some(Rect::new(x0, y0, x1 - x0, y1 - y0))
}

fn merge_device_rects(mut parts: Vec<Rect>, max_parts: usize, free_merge: f64) -> Vec<Rect> {
    while parts.len() > 1 {
        let mut bi = None;
        let mut bj = None;
        let mut best_extra = f64::MAX;
        for i in 0..parts.len() {
            for j in (i + 1)..parts.len() {
                let u = parts[i].united(parts[j]);
                let extra = u.area() - parts[i].area() - parts[j].area();
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

fn format_of(path: &Path) -> ImageKind {
    match path
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_ascii_lowercase())
        .as_deref()
    {
        Some("svg") => ImageKind::Svg,
        Some("jpg") | Some("jpeg") => ImageKind::Jpeg,
        Some("bmp") => ImageKind::Bmp,
        _ => ImageKind::Png,
    }
}

enum ImageKind {
    Png,
    Jpeg,
    Bmp,
    Svg,
}

fn raster(canvas: &Canvas, palette: &Palette) -> crate::Result<Pixmap> {
    let (vw, vh) = canvas.viewport().view_size();
    let w = vw.round().max(1.0) as u32;
    let h = vh.round().max(1.0) as u32;
    let mut pm =
        Pixmap::new(w, h).ok_or_else(|| Error::Export("could not allocate image".into()))?;
    let vis = canvas.viewport().visible_rect();
    if vis.w <= 0.0 || vis.h <= 0.0 {
        return Ok(pm);
    }
    let sx = w as f32 / vis.w as f32;
    let sy = h as f32 / vis.h as f32;
    let world = Transform::from_scale(sx, sy)
        .pre_concat(Transform::from_translate(-vis.x as f32, -vis.y as f32));
    let mut rast = Raster::new(&mut pm, world);
    paint_scene(
        &mut rast,
        &PaintCtx {
            canvas,
            pal: palette,
            scale: canvas.viewport().zoom(),
            item_id: "",
        },
    );
    Ok(pm)
}

fn encode_jpeg(pm: &Pixmap, path: &Path) -> crate::Result<()> {
    let (w, h, rgb) = pixmap_rgb(pm);
    let mut file = std::fs::File::create(path)?;
    let mut enc = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut file, 90);
    enc.encode(&rgb, w, h, image::ExtendedColorType::Rgb8)
        .map_err(|e| Error::Export(format!("jpeg: {e}")))?;
    file.flush()?;
    Ok(())
}

fn encode_bmp(pm: &Pixmap, path: &Path) -> crate::Result<()> {
    let (w, h, rgb) = pixmap_rgb(pm);
    let mut file = std::fs::File::create(path)?;
    let mut enc = image::codecs::bmp::BmpEncoder::new(&mut file);
    enc.encode(&rgb, w, h, image::ExtendedColorType::Rgb8)
        .map_err(|e| Error::Export(format!("bmp: {e}")))?;
    file.flush()?;
    Ok(())
}

fn pixmap_rgb(pm: &Pixmap) -> (u32, u32, Vec<u8>) {
    let w = pm.width();
    let h = pm.height();
    let mut rgb = Vec::with_capacity((w * h * 3) as usize);
    for px in pm.pixels() {
        let a = px.alpha();
        if a == 0 {
            rgb.extend_from_slice(&[0, 0, 0]);
        } else if a == 255 {
            rgb.extend_from_slice(&[px.red(), px.green(), px.blue()]);
        } else {
            let d = px.demultiply();
            rgb.extend_from_slice(&[d.red(), d.green(), d.blue()]);
        }
    }
    (w, h, rgb)
}
