//! Registers the Qt canvas item and asks Vello to paint it.

use cs_engine::canvas::{Canvas, Palette};
use std::ffi::{self, CStr};

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
