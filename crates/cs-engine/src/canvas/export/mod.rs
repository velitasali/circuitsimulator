//! PNG / JPEG / BMP / SVG from the canvas scene. No Qt.
//!
//! Raster formats record the same Vello scene as the window and read the
//! frame back. SVG stays vectors.

pub(crate) mod arc;
pub mod paint;
pub mod svg;
pub mod text;

#[cfg(test)]
mod tests;

use std::io::Write;
use std::path::Path;

#[cfg(test)]
use self::paint::paint_item;
use self::paint::paint_scene;
use self::svg::Svg;
use super::Canvas;
use super::ImageBuf;
use super::geom::{Point, Rect};
use super::vello_gpu;
use crate::Error;

pub use super::draw::{Align, Color, Draw, PaintCtx, Palette, parse_hex};
pub use text::{font, font_bold, text_width};

/// Straight RGBA8 frame. Kept so existing `Pixmap` call sites keep compiling.
pub type Pixmap = ImageBuf;

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
                ImageKind::Png => {
                    let bytes = pm
                        .encode_png()
                        .ok_or_else(|| Error::Export("png encode failed".into()))?;
                    std::fs::write(path, bytes)?;
                    Ok(())
                }
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
        .ok_or_else(|| Error::Export("png encode failed".into()))
}

pub fn render_viewport(
    canvas: &Canvas,
    palette: &Palette,
    width: u32,
    height: u32,
    dpr: f64,
    draw_background: bool,
) -> Option<Pixmap> {
    vello_gpu::render_view(
        canvas,
        palette,
        width.max(1),
        height.max(1),
        dpr,
        draw_background,
    )
}

pub fn render_viewport_mut(
    canvas: &Canvas,
    palette: &Palette,
    pixmap: &mut Pixmap,
    dpr: f64,
    draw_background: bool,
) {
    let Some(frame) = vello_gpu::render_view(
        canvas,
        palette,
        pixmap.width(),
        pixmap.height(),
        dpr,
        draw_background,
    ) else {
        return;
    };
    *pixmap = frame;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RegionRenderStatus {
    /// At least one dirty region intersected the pixmap and was painted directly.
    Painted(usize),
    /// All dirty regions were completely outside the pixmap; nothing on screen was dirty.
    Offscreen,
    /// Region rendering could not be performed (e.g. invalid pixmap dimensions or scale).
    Failed,
}

impl RegionRenderStatus {
    #[inline]
    pub fn is_success(&self) -> bool {
        !matches!(self, Self::Failed)
    }

    #[inline]
    pub fn is_painted(&self) -> bool {
        matches!(self, Self::Painted(_))
    }
}

/// Re-raster `scene_rects` directly into an existing pixmap.
/// Each merged device rectangle is rendered on its own and copied over.
pub fn render_viewport_regions_mut(
    canvas: &Canvas,
    palette: &Palette,
    pixmap: &mut Pixmap,
    dpr: f64,
    scene_rects: &[Rect],
) -> bool {
    let vp = canvas.viewport();
    render_viewport_regions_projected_mut(
        canvas,
        palette,
        pixmap,
        dpr,
        vp.center(),
        vp.zoom(),
        scene_rects,
    )
    .is_success()
}

/// Variant of [`render_viewport_regions_mut`] that accepts an explicit projection center and zoom.
/// Used when patching into an overscan-padded backing pixmap during active pan gestures.
pub fn render_viewport_regions_projected_mut(
    canvas: &Canvas,
    palette: &Palette,
    pixmap: &mut Pixmap,
    dpr: f64,
    center: Point,
    zoom: f64,
    scene_rects: &[Rect],
) -> RegionRenderStatus {
    if scene_rects.is_empty() {
        return RegionRenderStatus::Offscreen;
    }
    let w = pixmap.width();
    let h = pixmap.height();
    if w == 0 || h == 0 {
        return RegionRenderStatus::Failed;
    }
    let s = zoom * dpr.max(1.0);
    if s <= 1e-9 {
        return RegionRenderStatus::Failed;
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
        return RegionRenderStatus::Offscreen;
    }
    let merged = merge_device_rects(device_rects, 6, 4000.0);
    let count = merged.len();
    let dpr = dpr.max(1.0);

    let mut patches = Vec::with_capacity(count);
    for dr in &merged {
        let x0 = (dr.x.round() as u32).min(w);
        let y0 = (dr.y.round() as u32).min(h);
        let x1 = (dr.right().round() as u32).min(w);
        let y1 = (dr.bottom().round() as u32).min(h);
        if x1 <= x0 || y1 <= y0 {
            continue;
        }
        let Some(patch) = vello_gpu::render_items_patch(
            canvas,
            palette,
            w,
            h,
            x0,
            y0,
            x1 - x0,
            y1 - y0,
            dpr,
            center,
            zoom,
        ) else {
            return RegionRenderStatus::Failed;
        };
        patches.push((x0, y0, patch));
    }
    for (x, y, patch) in &patches {
        pixmap.copy_at(*x, *y, patch);
    }
    RegionRenderStatus::Painted(count)
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
    render_viewport(canvas, palette, w, h, 1.0, true)
        .ok_or_else(|| Error::Export("could not render image".into()))
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
    for px in pm.pixels().chunks_exact(4) {
        if px[3] == 0 {
            rgb.extend_from_slice(&[0, 0, 0]);
        } else {
            rgb.extend_from_slice(&[px[0], px[1], px[2]]);
        }
    }
    (w, h, rgb)
}
