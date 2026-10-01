//! Registers the Qt canvas item. Vello paints the picture. On Windows and
//! Linux this module copies the finished frame into the buffer Qt uploads.

use cs_engine::canvas::{Canvas, Palette};
use std::ffi::{self, CStr};
use std::sync::Mutex;

struct StagingBuffer {
    data: Vec<u8>,
    stride: u32,
    width: u32,
    height: u32,
}

static STAGING_BUFFER: Mutex<StagingBuffer> = Mutex::new(StagingBuffer {
    data: Vec::new(),
    stride: 0,
    width: 0,
    height: 0,
});

unsafe extern "C" {
    fn cs_set_native_text_rendering();
    fn cs_register_canvas_item();
    fn cs_qt_version() -> *const ffi::c_char;
}

pub fn qt_version() -> String {
    unsafe {
        let ptr = cs_qt_version();
        if ptr.is_null() {
            String::new()
        } else {
            CStr::from_ptr(ptr).to_string_lossy().into_owned()
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

pub fn update_render_state(
    canvas: &Canvas,
    palette: &Palette,
    width: u32,
    height: u32,
    dpr: f64,
) -> bool {
    crate::vello_preview::present(canvas, palette, width, height, dpr)
}

/// Latest Vello frame for the non-macOS scene-graph upload. macOS samples the
/// IOSurface instead, so this returns null there.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cs_canvas_render(
    out_width: *mut u32,
    out_height: *mut u32,
    out_stride: *mut u32,
) -> *const u8 {
    let mut staging = match STAGING_BUFFER.lock() {
        Ok(guard) => guard,
        Err(_) => return std::ptr::null(),
    };
    let Some((width, height, stride)) = crate::vello_preview::copy_cpu_frame(&mut staging.data)
    else {
        return std::ptr::null();
    };
    staging.width = width;
    staging.height = height;
    staging.stride = stride;
    if !out_width.is_null() {
        unsafe {
            *out_width = width;
        }
    }
    if !out_height.is_null() {
        unsafe {
            *out_height = height;
        }
    }
    if !out_stride.is_null() {
        unsafe {
            *out_stride = stride;
        }
    }
    staging.data.as_ptr()
}

#[unsafe(no_mangle)]
pub extern "C" fn cs_canvas_generation() -> u64 {
    crate::vello_preview::cpu_frame_generation()
}
