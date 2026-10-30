//! CPU image handed to Qt. One slot per view.
//!
//! The schematic's shared GPU texture is a different path. Slot `Canvas` is
//! only the read-back used when that share is not open. Slot `Scope` is the
//! oscilloscope window. Slot `Logic` is the logic-analyzer window.

use std::sync::Mutex;

use cs_engine::canvas::ImageBuf;

#[repr(i32)]
#[derive(Clone, Copy)]
pub enum Slot {
    Canvas = 0,
    Scope = 1,
    Logic = 2,
}

const SLOTS: usize = 3;

/// Logical size of the item the image was painted for. `dpr` is 0 when Qt
/// should use the window's device pixel ratio at upload time.
pub struct View {
    pub logical_w: f64,
    pub logical_h: f64,
    pub dpr: f64,
}

impl View {
    pub fn at(logical_w: f64, logical_h: f64, dpr: f64) -> Self {
        Self {
            logical_w,
            logical_h,
            dpr,
        }
    }

    pub fn device() -> Self {
        Self {
            logical_w: 0.0,
            logical_h: 0.0,
            dpr: 0.0,
        }
    }
}

struct SlotFrame {
    logical_w: f64,
    logical_h: f64,
    dpr: f64,
    width: u32,
    height: u32,
    stride: u32,
    generation: u64,
    pixels: Vec<u8>,
}

impl SlotFrame {
    const fn empty() -> Self {
        Self {
            logical_w: 0.0,
            logical_h: 0.0,
            dpr: 0.0,
            width: 0,
            height: 0,
            stride: 0,
            generation: 0,
            pixels: Vec::new(),
        }
    }
}

static FRAMES: Mutex<[SlotFrame; SLOTS]> =
    Mutex::new([SlotFrame::empty(), SlotFrame::empty(), SlotFrame::empty()]);

fn lock() -> std::sync::MutexGuard<'static, [SlotFrame; SLOTS]> {
    FRAMES.lock().unwrap_or_else(|err| err.into_inner())
}

fn index(slot: Slot) -> usize {
    slot as usize
}

/// Store `image` and ask the item for this slot to upload it.
pub fn publish(slot: Slot, image: ImageBuf, view: View) {
    let width = image.width();
    let height = image.height();
    let stride = width.saturating_mul(4);
    let expected = (stride as usize).saturating_mul(height as usize);
    if width == 0 || height == 0 || image.pixels().len() != expected {
        return;
    }
    let pixels = image.into_pixels();
    {
        let mut frames = lock();
        let frame = &mut frames[index(slot)];
        frame.logical_w = view.logical_w;
        frame.logical_h = view.logical_h;
        frame.dpr = view.dpr;
        frame.width = width;
        frame.height = height;
        frame.stride = stride;
        frame.pixels = pixels;
        frame.generation = frame.generation.wrapping_add(1);
    }
    notify(slot);
}

fn clear(slot: Slot) {
    let mut frames = lock();
    let frame = &mut frames[index(slot)];
    let generation = frame.generation;
    *frame = SlotFrame::empty();
    frame.generation = generation;
}

fn notify(slot: Slot) {
    unsafe extern "C" {
        fn cs_image_frame_request_update(slot: i32);
    }
    unsafe {
        cs_image_frame_request_update(slot as i32);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn cs_image_frame_clear(slot: i32) {
    if let Some(slot) = slot_from_i32(slot) {
        clear(slot);
    }
}

/// Copy the latest frame into `dst`. A null `dst` writes the size and returns the byte count.
/// `dst_stride` is the destination row pitch. `0` means the packed source stride.
/// Returns 0 when no frame exists, the slot is unknown, or `dst` is too small.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cs_image_frame_copy(
    slot: i32,
    dst: *mut u8,
    dst_len: usize,
    dst_stride: u32,
    out_w: *mut u32,
    out_h: *mut u32,
    out_stride: *mut u32,
    out_gen: *mut u64,
    out_dpr: *mut f64,
) -> i32 {
    let Some(slot) = slot_from_i32(slot) else {
        return 0;
    };
    let frames = lock();
    let frame = &frames[index(slot)];
    if frame.width == 0 || frame.pixels.is_empty() {
        return 0;
    }
    unsafe {
        if !out_w.is_null() {
            *out_w = frame.width;
        }
        if !out_h.is_null() {
            *out_h = frame.height;
        }
        if !out_stride.is_null() {
            *out_stride = frame.stride;
        }
        if !out_gen.is_null() {
            *out_gen = frame.generation;
        }
        if !out_dpr.is_null() {
            *out_dpr = frame.dpr;
        }
        if dst.is_null() {
            return frame.pixels.len() as i32;
        }
        let src_stride = frame.stride as usize;
        let height = frame.height as usize;
        let packed = src_stride.saturating_mul(height);
        if frame.pixels.len() < packed {
            return 0;
        }
        let dst_stride = if dst_stride == 0 {
            src_stride
        } else {
            dst_stride as usize
        };
        if dst_stride < src_stride {
            return 0;
        }
        let need = dst_stride.saturating_mul(height);
        if need == 0 || dst_len < need {
            return 0;
        }
        let src = frame.pixels.as_ptr();
        if dst_stride == src_stride {
            std::ptr::copy_nonoverlapping(src, dst, packed);
        } else {
            for y in 0..height {
                std::ptr::copy_nonoverlapping(
                    src.add(y * src_stride),
                    dst.add(y * dst_stride),
                    src_stride,
                );
            }
        }
    }
    1
}

fn slot_from_i32(slot: i32) -> Option<Slot> {
    if slot == Slot::Canvas as i32 {
        Some(Slot::Canvas)
    } else if slot == Slot::Scope as i32 {
        Some(Slot::Scope)
    } else if slot == Slot::Logic as i32 {
        Some(Slot::Logic)
    } else {
        None
    }
}
