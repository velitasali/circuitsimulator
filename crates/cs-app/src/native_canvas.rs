//! Native canvas rendering and Qt Quick Scene Graph item integration.

use cs_engine::canvas::{Canvas, Palette, Pixmap, Rect};
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

/// Logical pixels of overscan on each side of the circuit pixmap.
/// Fast GPU pan/zoom samples within this pad; `settle_pan` re-rasters after the gesture.
const OVERSCAN_PAD_PX: f64 = 256.0;
/// Round physical pixmap edges up so 1-pixel view/DPR jitter does not realloc.
const PIXMAP_QUANTUM: u32 = 64;

fn quantize_dim(n: u32) -> u32 {
    let q = PIXMAP_QUANTUM.max(1);
    n.max(1).saturating_add(q - 1) / q * q
}

fn alloc_dim(need: u32) -> u32 {
    let rounded = quantize_dim(need);
    // Exact quantum multiples have no headroom; take the next bucket so 1-pixel
    // view jitter does not realloc.
    if rounded == need.max(1) {
        rounded.saturating_add(PIXMAP_QUANTUM)
    } else {
        rounded
    }
}

fn needed_pixmap_size(view_w: f64, view_h: f64, dpr: f64) -> (u32, u32) {
    let d = dpr.max(1.0);
    let pw = ((view_w + 2.0 * OVERSCAN_PAD_PX) * d).ceil().max(1.0) as u32;
    let ph = ((view_h + 2.0 * OVERSCAN_PAD_PX) * d).ceil().max(1.0) as u32;
    (pw, ph)
}

fn take_or_alloc_circuit_pixmap(
    existing: Option<Pixmap>,
    need_w: u32,
    need_h: u32,
) -> Option<Pixmap> {
    let alloc_w = alloc_dim(need_w);
    let alloc_h = alloc_dim(need_h);
    if let Some(pm) = existing {
        let ew = pm.width();
        let eh = pm.height();
        let fits = ew >= need_w && eh >= need_h;
        let not_huge = ew <= alloc_w.saturating_add(PIXMAP_QUANTUM)
            && eh <= alloc_h.saturating_add(PIXMAP_QUANTUM);
        if fits && not_huge {
            return Some(pm);
        }
    }
    Pixmap::new(alloc_w, alloc_h)
}

#[derive(Clone, Copy, Debug)]
pub struct CanvasRenderMeta {
    pub center_x: f64,
    pub center_y: f64,
    pub zoom: f64,
    pub pad_x: f64,
    pub pad_y: f64,
    pub dpr: f64,
    pub view_w: f64,
    pub view_h: f64,
}

impl Default for CanvasRenderMeta {
    fn default() -> Self {
        Self {
            center_x: 0.0,
            center_y: 0.0,
            zoom: 1.0,
            pad_x: 0.0,
            pad_y: 0.0,
            dpr: 1.0,
            view_w: 0.0,
            view_h: 0.0,
        }
    }
}

static CANVAS_META: Mutex<CanvasRenderMeta> = Mutex::new(CanvasRenderMeta {
    center_x: 0.0,
    center_y: 0.0,
    zoom: 1.0,
    pad_x: 0.0,
    pad_y: 0.0,
    dpr: 1.0,
    view_w: 0.0,
    view_h: 0.0,
});

pub fn current_canvas_meta() -> CanvasRenderMeta {
    CANVAS_META.lock().map(|m| *m).unwrap_or_default()
}

/// True when the last software raster still covers the requested view after a
/// GPU pan/zoom (same source-rect math as `CircuitCanvasItem::updatePaintNode`).
pub fn pixmap_covers_view(
    center_x: f64,
    center_y: f64,
    zoom: f64,
    view_w: f64,
    view_h: f64,
) -> bool {
    let meta = current_canvas_meta();
    if meta.view_w <= 1.0 || meta.view_h <= 1.0 || meta.zoom <= 0.0 || zoom <= 0.0 {
        return false;
    }
    if (view_w - meta.view_w).abs() > 0.5 || (view_h - meta.view_h).abs() > 0.5 {
        return false;
    }
    let dpr = meta.dpr.max(1.0);
    let zoom_ratio = meta.zoom / zoom;
    let dx = (center_x - meta.center_x) * meta.zoom * dpr;
    let dy = (center_y - meta.center_y) * meta.zoom * dpr;
    let view_w_px = meta.view_w * dpr;
    let view_h_px = meta.view_h * dpr;
    let src_w = view_w_px * zoom_ratio;
    let src_h = view_h_px * zoom_ratio;
    let src_x = meta.pad_x * dpr + dx - (src_w - view_w_px) * 0.5;
    let src_y = meta.pad_y * dpr + dy - (src_h - view_h_px) * 0.5;
    let tex_w = (meta.view_w + 2.0 * meta.pad_x) * dpr;
    let tex_h = (meta.view_h + 2.0 * meta.pad_y) * dpr;
    const MARGIN: f64 = 1.0;
    src_x >= MARGIN
        && src_y >= MARGIN
        && src_x + src_w <= tex_w - MARGIN
        && src_y + src_h <= tex_h - MARGIN
}

struct RenderBuffer {
    pixmap: Pixmap,
    stride: u32,
    width: u32,
    height: u32,
}

static RENDER_BUFFER: Mutex<Option<RenderBuffer>> = Mutex::new(None);
static CANVAS_GENERATION: AtomicU64 = AtomicU64::new(1);

unsafe extern "C" {
    fn cs_set_native_text_rendering();
    fn cs_register_canvas_item();
    fn cs_canvas_item_request_update();
    fn cs_qt_version() -> *const std::ffi::c_char;
}

pub fn qt_version() -> String {
    unsafe {
        let ptr = cs_qt_version();
        if ptr.is_null() {
            String::new()
        } else {
            std::ffi::CStr::from_ptr(ptr).to_string_lossy().into_owned()
        }
    }
}

pub fn set_native_text_rendering() {
    unsafe {
        cs_set_native_text_rendering();
    }
}

pub fn register_canvas_item() {
    unsafe {
        cs_register_canvas_item();
    }
}

pub fn update_render_state(canvas: &Canvas, palette: &Palette, width: u32, height: u32, dpr: f64) {
    let mut lock = match RENDER_BUFFER.lock() {
        Ok(guard) => guard,
        Err(_) => return,
    };
    let d = dpr.max(1.0);
    let vp = canvas.viewport();
    let center = vp.center();
    let zoom = vp.zoom();

    let view_w = width as f64;
    let view_h = height as f64;

    let (need_w, need_h) = needed_pixmap_size(view_w, view_h, d);
    let mut pm =
        match take_or_alloc_circuit_pixmap(lock.take().map(|buf| buf.pixmap), need_w, need_h) {
            Some(p) => p,
            None => return,
        };
    cs_engine::canvas::render_viewport_mut(canvas, palette, &mut pm, d, false);
    let stride = pm.width() * 4;
    let pm_w = pm.width();
    let pm_h = pm.height();

    // Actual pad includes quantization slack, so GPU pan can use the extra pixels.
    let pad_x = (pm_w as f64 / d - view_w).max(0.0) * 0.5;
    let pad_y = (pm_h as f64 / d - view_h).max(0.0) * 0.5;

    let meta = CanvasRenderMeta {
        center_x: center.x,
        center_y: center.y,
        zoom,
        pad_x,
        pad_y,
        dpr: d,
        view_w,
        view_h,
    };
    if let Ok(mut m) = CANVAS_META.lock() {
        *m = meta;
    }

    *lock = Some(RenderBuffer {
        pixmap: pm,
        stride,
        width: pm_w,
        height: pm_h,
    });
    CANVAS_GENERATION.fetch_add(1, Ordering::Release);
    drop(lock);
    unsafe {
        cs_canvas_item_request_update();
    }
}

/// Patch dirty scene rects into the existing pixmap. Returns false when the
/// buffer is missing, the view has moved, or the dirty area is too large —
/// caller should fall back to [`update_render_state`].
pub fn update_render_regions(
    canvas: &Canvas,
    palette: &Palette,
    width: u32,
    height: u32,
    dpr: f64,
    scene_rects: &[Rect],
) -> bool {
    if scene_rects.is_empty() {
        return false;
    }
    let mut lock = match RENDER_BUFFER.lock() {
        Ok(guard) => guard,
        Err(_) => return false,
    };
    let Some(buf) = lock.as_mut() else {
        return false;
    };
    let d = dpr.max(1.0);
    let vp = canvas.viewport();
    let center = vp.center();
    let zoom = vp.zoom();
    let meta = current_canvas_meta();
    if (meta.zoom - zoom).abs() > 1e-4
        || (meta.center_x - center.x).abs() > 1e-4
        || (meta.center_y - center.y).abs() > 1e-4
        || (meta.dpr - d).abs() > 1e-4
        || (meta.view_w - width as f64).abs() > 0.5
        || (meta.view_h - height as f64).abs() > 0.5
    {
        return false;
    }
    let painted = cs_engine::canvas::render_viewport_regions_mut(
        canvas,
        palette,
        &mut buf.pixmap,
        d,
        scene_rects,
    );
    if !painted {
        return false;
    }
    CANVAS_GENERATION.fetch_add(1, Ordering::Release);
    drop(lock);
    unsafe {
        cs_canvas_item_request_update();
    }
    true
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cs_canvas_render(
    out_width: *mut u32,
    out_height: *mut u32,
    out_stride: *mut u32,
) -> *const u8 {
    let lock = match RENDER_BUFFER.lock() {
        Ok(guard) => guard,
        Err(_) => return std::ptr::null(),
    };
    match lock.as_ref() {
        Some(buf) => {
            if !out_width.is_null() {
                unsafe {
                    *out_width = buf.width;
                }
            }
            if !out_height.is_null() {
                unsafe {
                    *out_height = buf.height;
                }
            }
            if !out_stride.is_null() {
                unsafe {
                    *out_stride = buf.stride;
                }
            }
            buf.pixmap.data().as_ptr()
        }
        None => std::ptr::null(),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn cs_canvas_generation() -> u64 {
    CANVAS_GENERATION.load(Ordering::Acquire)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cs_canvas_meta(
    out_cx: *mut f64,
    out_cy: *mut f64,
    out_zoom: *mut f64,
    out_pad_x: *mut f64,
    out_pad_y: *mut f64,
    out_dpr: *mut f64,
    out_view_w: *mut f64,
    out_view_h: *mut f64,
) {
    if let Ok(meta) = CANVAS_META.lock() {
        if !out_cx.is_null() {
            unsafe {
                *out_cx = meta.center_x;
            }
        }
        if !out_cy.is_null() {
            unsafe {
                *out_cy = meta.center_y;
            }
        }
        if !out_zoom.is_null() {
            unsafe {
                *out_zoom = meta.zoom;
            }
        }
        if !out_pad_x.is_null() {
            unsafe {
                *out_pad_x = meta.pad_x;
            }
        }
        if !out_pad_y.is_null() {
            unsafe {
                *out_pad_y = meta.pad_y;
            }
        }
        if !out_dpr.is_null() {
            unsafe {
                *out_dpr = meta.dpr;
            }
        }
        if !out_view_w.is_null() {
            unsafe {
                *out_view_w = meta.view_w;
            }
        }
        if !out_view_h.is_null() {
            unsafe {
                *out_view_h = meta.view_h;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        OVERSCAN_PAD_PX, PIXMAP_QUANTUM, alloc_dim, needed_pixmap_size, quantize_dim,
        take_or_alloc_circuit_pixmap,
    };
    use cs_engine::canvas::Pixmap;

    #[test]
    fn quantize_dim_rounds_up_to_quantum() {
        assert_eq!(quantize_dim(1), PIXMAP_QUANTUM);
        assert_eq!(quantize_dim(PIXMAP_QUANTUM), PIXMAP_QUANTUM);
        assert_eq!(quantize_dim(PIXMAP_QUANTUM + 1), PIXMAP_QUANTUM * 2);
        assert_eq!(quantize_dim(3600), 3648);
        assert_eq!(quantize_dim(3603), 3648);
        assert_eq!(quantize_dim(2624), 2624);
        assert_eq!(quantize_dim(2626), 2688);
    }

    #[test]
    fn alloc_dim_adds_headroom_on_exact_multiples() {
        assert_eq!(alloc_dim(3392), 3456);
        assert_eq!(alloc_dim(3394), 3456);
        assert_eq!(alloc_dim(3600), 3648);
        assert_eq!(alloc_dim(3603), 3648);
    }

    #[test]
    fn one_pixel_view_jitter_reuses_buffer() {
        let (need_a_w, need_a_h) = needed_pixmap_size(1440.0, 838.0, 2.0);
        let (need_b_w, need_b_h) = needed_pixmap_size(1441.0, 839.0, 2.0);
        let first = take_or_alloc_circuit_pixmap(None, need_a_w, need_a_h).unwrap();
        let w = first.width();
        let h = first.height();
        assert_eq!(w % PIXMAP_QUANTUM, 0);
        assert_eq!(h % PIXMAP_QUANTUM, 0);
        assert!(w >= need_a_w && w >= need_b_w);
        assert!(h >= need_a_h && h >= need_b_h);
        assert!(w * h < 4320 * 2700);

        let second = take_or_alloc_circuit_pixmap(Some(first), need_b_w, need_b_h).unwrap();
        assert_eq!(second.width(), w);
        assert_eq!(second.height(), h);
    }

    #[test]
    fn overscan_is_at_least_the_configured_pad() {
        let (need_w, need_h) = needed_pixmap_size(1200.0, 800.0, 2.0);
        let pm = take_or_alloc_circuit_pixmap(None, need_w, need_h).unwrap();
        let pad_x = (pm.width() as f64 / 2.0 - 1200.0) * 0.5;
        let pad_y = (pm.height() as f64 / 2.0 - 800.0) * 0.5;
        assert!(pad_x >= OVERSCAN_PAD_PX);
        assert!(pad_y >= OVERSCAN_PAD_PX);
    }

    #[test]
    fn shrinks_when_buffer_is_much_larger() {
        let huge = Pixmap::new(4096, 4096).unwrap();
        let (need_w, need_h) = needed_pixmap_size(400.0, 300.0, 1.0);
        let pm = take_or_alloc_circuit_pixmap(Some(huge), need_w, need_h).unwrap();
        assert!(pm.width() < 4096);
        assert!(pm.height() < 4096);
        assert!(pm.width() >= need_w);
        assert!(pm.height() >= need_h);
    }

    fn set_meta(meta: super::CanvasRenderMeta) {
        *super::CANVAS_META.lock().unwrap() = meta;
    }

    fn sample_meta() -> super::CanvasRenderMeta {
        super::CanvasRenderMeta {
            center_x: 0.0,
            center_y: 0.0,
            zoom: 1.0,
            pad_x: 256.0,
            pad_y: 256.0,
            dpr: 1.0,
            view_w: 800.0,
            view_h: 600.0,
        }
    }

    #[test]
    fn pixmap_covers_pan_zoom_in_but_not_zoom_out_or_resize() {
        set_meta(sample_meta());
        assert!(super::pixmap_covers_view(50.0, 40.0, 1.0, 800.0, 600.0));
        assert!(super::pixmap_covers_view(0.0, 0.0, 2.0, 800.0, 600.0));
        assert!(super::pixmap_covers_view(0.0, 0.0, 1.0, 800.0, 600.0));
        assert!(!super::pixmap_covers_view(400.0, 0.0, 1.0, 800.0, 600.0));
        assert!(!super::pixmap_covers_view(0.0, 0.0, 0.5, 800.0, 600.0));
        assert!(!super::pixmap_covers_view(0.0, 0.0, 1.0, 1200.0, 600.0));
    }
}
