//! Paints the circuit view with Vello on every platform.
//! macOS renders straight into an IOSurface that Qt samples.
//! Windows renders into a shared Direct3D12 texture that Qt samples.
//! Linux renders into a Vulkan image that Qt samples through OpenGL or Vulkan.
//! If that share cannot be opened, the finished image is copied back instead.
//! A small change is drawn into a patch and copied onto the frame already on screen.

use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

use cs_engine::canvas::{Canvas, Palette, Rect};

struct CpuFrame {
    pixels: Vec<u8>,
    width: u32,
    height: u32,
    stride: u32,
}

static CPU_FRAME: Mutex<Option<CpuFrame>> = Mutex::new(None);
static CPU_FRAME_GEN: AtomicU64 = AtomicU64::new(1);

pub fn cpu_frame_generation() -> u64 {
    CPU_FRAME_GEN.load(Ordering::Acquire)
}

/// Copy the latest read-back frame. macOS does not fill this; Qt samples the
/// IOSurface instead.
pub fn copy_cpu_frame(dst: &mut Vec<u8>) -> Option<(u32, u32, u32)> {
    let frame = CPU_FRAME.lock().unwrap_or_else(|err| err.into_inner());
    let frame = frame.as_ref()?;
    dst.resize(frame.pixels.len(), 0);
    dst.copy_from_slice(&frame.pixels);
    Some((frame.width, frame.height, frame.stride))
}

#[cfg_attr(target_os = "macos", allow(dead_code))]
pub(super) fn store_cpu_frame(pixels: Vec<u8>, width: u32, height: u32, stride: u32) {
    let mut frame = CPU_FRAME.lock().unwrap_or_else(|err| err.into_inner());
    *frame = Some(CpuFrame {
        pixels,
        width,
        height,
        stride,
    });
    CPU_FRAME_GEN.fetch_add(1, Ordering::Release);
}

pub enum RegionPresent {
    Painted,
    Busy,
    Fallback,
    Skipped,
}

/// Queue a full frame. `false` means the presenter was busy or the GPU is
/// unavailable; the caller keeps its dirty set and tries again later.
pub fn present(canvas: &Canvas, palette: &Palette, width: u32, height: u32, dpr: f64) -> bool {
    gpu::present(canvas, palette, width, height, dpr)
}

pub fn present_regions(
    canvas: &Canvas,
    palette: &Palette,
    width: u32,
    height: u32,
    dpr: f64,
    scene_rects: &[Rect],
) -> RegionPresent {
    gpu::present_regions(canvas, palette, width, height, dpr, scene_rects)
}

#[unsafe(no_mangle)]
pub extern "C" fn cs_vello_preview_release() {
    gpu::release();
}

mod gpu {
    use super::{Rect, RegionPresent};
    use cs_engine::canvas::vello_draw::{self, color_of};
    use cs_engine::canvas::{Canvas, Palette};
    #[cfg(target_os = "macos")]
    use metal::foreign_types::ForeignType;
    #[cfg(target_os = "macos")]
    use std::ffi::c_void;
    use std::future::Future;
    use std::num::NonZeroUsize;
    #[cfg(not(target_os = "macos"))]
    use std::sync::atomic::AtomicU32;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Condvar, Mutex, MutexGuard};
    use std::thread::JoinHandle;
    use std::time::{Duration, Instant};
    use vello::util::RenderContext;
    use vello::wgpu;
    #[cfg(target_os = "macos")]
    use vello::wgpu::hal::api::Metal;
    use vello::{AaConfig, AaSupport, RenderParams, Renderer, RendererOptions, Scene};

    unsafe extern "C" {
        fn cs_canvas_item_request_update();
    }

    #[cfg(target_os = "macos")]
    unsafe extern "C" {
        fn cs_vello_shared_reset_needed(mtl_device: *mut c_void, width: i32, height: i32) -> i32;
        fn cs_vello_shared_begin(
            mtl_device: *mut c_void,
            width: i32,
            height: i32,
            out_slot: *mut i32,
            out_epoch: *mut i32,
        ) -> *mut c_void;
        fn cs_vello_shared_commit(slot: i32);
        fn cs_vello_shared_has_base(width: i32, height: i32) -> i32;
        fn cs_vello_shared_patch(patch_tex: *mut c_void, x: i32, y: i32, w: i32, h: i32) -> i32;
    }

    #[cfg(any(windows, target_os = "linux"))]
    unsafe extern "C" {
        fn cs_vello_share_reset_needed(kind: i32, device: u64, width: i32, height: i32) -> i32;
        fn cs_vello_share_begin(
            kind: i32,
            device: u64,
            queue: u64,
            instance: u64,
            physical: u64,
            family: u32,
            width: i32,
            height: i32,
            out_slot: *mut i32,
            out_epoch: *mut i32,
        ) -> i32;
        fn cs_vello_share_resource(slot: i32) -> u64;
        fn cs_vello_share_commit(slot: i32);
        fn cs_vello_share_has_base(width: i32, height: i32) -> i32;
        fn cs_vello_share_patch_dest(
            out_dst: *mut i32,
            out_src: *mut i32,
            out_copy_full: *mut i32,
        ) -> i32;
        fn cs_vello_share_blocked() -> i32;
        fn cs_vello_share_block();
        fn cs_vello_share_release();
    }

    #[cfg(target_os = "macos")]
    struct ImportedSurface {
        epoch: i32,
        ptr: usize,
        /// Owns wgpu's retain of the IOSurface texture for as long as we sample it.
        _texture: wgpu::Texture,
        view: wgpu::TextureView,
    }

    struct Scratch {
        width: u32,
        height: u32,
        texture: wgpu::Texture,
        view: wgpu::TextureView,
    }

    /// A frame Vello shares with Qt. Dropping it releases wgpu's reference.
    /// The native image stays alive until `cs_vello_share_release`.
    #[cfg(any(windows, target_os = "linux"))]
    struct SharedSurface {
        epoch: i32,
        width: u32,
        height: u32,
        texture: wgpu::Texture,
        view: wgpu::TextureView,
    }

    #[cfg(any(windows, target_os = "linux"))]
    struct ShareHandles {
        kind: i32,
        device: u64,
        queue: u64,
        instance: u64,
        physical: u64,
        family: u32,
    }

    struct Gpu {
        renderer: Renderer,
        scene: Scene,
        #[cfg(target_os = "macos")]
        surfaces: [Option<ImportedSurface>; 2],
        /// Private frame used when the shared texture cannot be opened.
        #[cfg(not(target_os = "macos"))]
        frame: Option<Scratch>,
        #[cfg(any(windows, target_os = "linux"))]
        surfaces: [Option<SharedSurface>; 2],
        #[cfg(any(windows, target_os = "linux"))]
        shared: bool,
        scratch: Option<Scratch>,
        dev_id: usize,
        cx: RenderContext,
    }

    /// A frame the UI thread has recorded. The present thread submits it and
    /// waits for the GPU, so `tick` does not sleep in wgpu's Metal poll.
    struct Pending {
        width: u32,
        height: u32,
        started: Instant,
        clear: cs_engine::canvas::Color,
        slot: i32,
        /// Top-left of a partial patch. Absent for a full frame.
        patch_origin: Option<(u32, u32)>,
    }

    enum Phase {
        Idle,
        /// Scene is recorded. The present thread still has to submit it.
        Recorded(Pending),
        /// Commands are on the GPU. The present thread is polling completion.
        Waiting(Pending),
    }

    struct Slot {
        gpu: Option<Gpu>,
        failed: bool,
        phase: Phase,
        stop: bool,
    }

    static GPU: Mutex<Slot> = Mutex::new(Slot {
        gpu: None,
        failed: false,
        phase: Phase::Idle,
        stop: false,
    });
    static WAKE: Condvar = Condvar::new();
    static PRESENTER: Mutex<Option<JoinHandle<()>>> = Mutex::new(None);
    static LOGGED: AtomicBool = AtomicBool::new(false);
    #[cfg(not(target_os = "macos"))]
    static FRAME_W: AtomicU32 = AtomicU32::new(0);
    #[cfg(not(target_os = "macos"))]
    static FRAME_H: AtomicU32 = AtomicU32::new(0);
    /// Once the share fails, later frames copy pixels instead of trying again.
    #[cfg(any(windows, target_os = "linux"))]
    static SHARE_OFF: AtomicBool = AtomicBool::new(false);
    #[cfg(any(windows, target_os = "linux"))]
    static SHARE_LOGGED: AtomicBool = AtomicBool::new(false);

    struct NopWaker;
    impl std::task::Wake for NopWaker {
        fn wake(self: Arc<Self>) {}
    }

    fn block_on<F: Future>(fut: F) -> F::Output {
        let mut fut = std::pin::pin!(fut);
        let waker = std::task::Waker::from(Arc::new(NopWaker));
        let mut cx = std::task::Context::from_waker(&waker);
        loop {
            match fut.as_mut().poll(&mut cx) {
                std::task::Poll::Ready(value) => return value,
                std::task::Poll::Pending => {
                    std::thread::sleep(std::time::Duration::from_millis(1));
                }
            }
        }
    }

    fn lock_gpu() -> MutexGuard<'static, Slot> {
        GPU.lock().unwrap_or_else(|err| err.into_inner())
    }

    pub fn release() {
        {
            let mut slot = lock_gpu();
            slot.stop = true;
        }
        WAKE.notify_one();
        if let Some(thread) = PRESENTER
            .lock()
            .unwrap_or_else(|err| err.into_inner())
            .take()
        {
            let _ = thread.join();
        }
        // Drop imported textures before the native images, and destroy those
        // images while the wgpu device is still alive.
        #[cfg(not(target_os = "macos"))]
        {
            let mut slot = lock_gpu();
            if let Some(gpu) = slot.gpu.as_mut() {
                gpu.frame = None;
                gpu.scratch = None;
                #[cfg(any(windows, target_os = "linux"))]
                {
                    gpu.surfaces = [None, None];
                    gpu.shared = false;
                }
            }
        }
        #[cfg(any(windows, target_os = "linux"))]
        unsafe {
            cs_vello_share_release();
        }
        let mut slot = lock_gpu();
        slot.gpu = None;
        slot.phase = Phase::Idle;
    }

    fn ensure_presenter() {
        let mut presenter = PRESENTER.lock().unwrap_or_else(|err| err.into_inner());
        if presenter.is_some() {
            return;
        }
        *presenter = std::thread::Builder::new()
            .name("vello-present".into())
            .spawn(presenter_loop)
            .ok();
    }

    pub fn present(canvas: &Canvas, palette: &Palette, width: u32, height: u32, dpr: f64) -> bool {
        ensure_presenter();
        let dpr = if dpr.is_finite() { dpr.max(1.0) } else { 1.0 };
        let width = ((width as f64) * dpr).round() as u32;
        let height = ((height as f64) * dpr).round() as u32;
        let width = width.clamp(1, 8192);
        let height = height.clamp(1, 8192);

        // try_lock: the present thread may still be finishing the previous frame.
        // The UI thread must not wait. A false return leaves the dirty set for
        // the next tick, same as present_regions' Busy.
        let mut slot = match GPU.try_lock() {
            Ok(slot) => slot,
            Err(std::sync::TryLockError::WouldBlock) => return false,
            Err(std::sync::TryLockError::Poisoned(err)) => err.into_inner(),
        };
        if slot.failed || slot.stop || !matches!(slot.phase, Phase::Idle) {
            return false;
        }
        if slot.gpu.is_none() {
            match init_gpu(width, height) {
                Ok(gpu) => slot.gpu = Some(gpu),
                Err(err) => {
                    slot.failed = true;
                    log_once(&err);
                    return false;
                }
            }
        }
        let Some(gpu) = slot.gpu.as_mut() else {
            return false;
        };
        vello_draw::record(&mut gpu.scene, canvas, palette, width, height, dpr);
        slot.phase = Phase::Recorded(Pending {
            width,
            height,
            started: Instant::now(),
            clear: palette.canvas,
            slot: 0,
            patch_origin: None,
        });
        drop(slot);
        WAKE.notify_one();
        true
    }

    pub fn present_regions(
        canvas: &Canvas,
        palette: &Palette,
        width: u32,
        height: u32,
        dpr: f64,
        scene_rects: &[Rect],
    ) -> RegionPresent {
        if scene_rects.is_empty() {
            return RegionPresent::Skipped;
        }
        let dpr = if dpr.is_finite() { dpr.max(1.0) } else { 1.0 };
        let full_w = (((width as f64) * dpr).round() as u32).clamp(1, 8192);
        let full_h = (((height as f64) * dpr).round() as u32).clamp(1, 8192);
        if !has_base(full_w, full_h) {
            return RegionPresent::Fallback;
        }
        let Some((x, y, patch_w, patch_h)) = patch_bounds(canvas, scene_rects, full_w, full_h, dpr)
        else {
            return RegionPresent::Skipped;
        };
        let area = (patch_w as f64) * (patch_h as f64);
        let frame = (full_w as f64) * (full_h as f64);
        if area > frame * 0.5 {
            return RegionPresent::Fallback;
        }
        ensure_presenter();
        let mut slot = match GPU.try_lock() {
            Ok(slot) => slot,
            Err(std::sync::TryLockError::WouldBlock) => return RegionPresent::Busy,
            Err(std::sync::TryLockError::Poisoned(err)) => err.into_inner(),
        };
        if slot.failed || slot.stop || !matches!(slot.phase, Phase::Idle) {
            return RegionPresent::Busy;
        }
        if slot.gpu.is_none() {
            return RegionPresent::Fallback;
        }
        let Some(gpu) = slot.gpu.as_mut() else {
            return RegionPresent::Fallback;
        };
        vello_draw::record_at(
            &mut gpu.scene,
            canvas,
            palette,
            full_w,
            full_h,
            x,
            y,
            patch_w,
            patch_h,
            dpr,
        );
        slot.phase = Phase::Recorded(Pending {
            width: patch_w,
            height: patch_h,
            started: Instant::now(),
            clear: palette.canvas,
            slot: 0,
            patch_origin: Some((x, y)),
        });
        drop(slot);
        WAKE.notify_one();
        RegionPresent::Painted
    }

    fn presenter_loop() {
        loop {
            let mut slot = lock_gpu();
            if slot.stop {
                return;
            }
            if !matches!(slot.phase, Phase::Recorded(_)) {
                slot = WAKE.wait(slot).unwrap_or_else(|err| err.into_inner());
                if slot.stop {
                    return;
                }
            }
            if !matches!(slot.phase, Phase::Recorded(_)) {
                continue;
            }
            if let Err(err) = submit_recorded(&mut slot) {
                #[cfg(any(windows, target_os = "linux"))]
                if slot.gpu.as_ref().is_some_and(|gpu| gpu.shared) {
                    forget_share(&mut slot);
                    slot.phase = Phase::Idle;
                    log_share_fallback(&err);
                    continue;
                }
                slot.phase = Phase::Idle;
                log_once(&err);
                continue;
            }
            let pending = match &slot.phase {
                Phase::Waiting(pending) => Pending {
                    width: pending.width,
                    height: pending.height,
                    started: pending.started,
                    clear: pending.clear,
                    slot: pending.slot,
                    patch_origin: pending.patch_origin,
                },
                _ => continue,
            };
            // Poll without wgpu's 1 ms sleep. The lock is released between
            // checks; phase stays Waiting so tick() returns instead of waiting.
            let (mut slot, ready) = wait_until_gpu_idle(slot, pending.started);
            if slot.stop {
                return;
            }
            if !ready {
                slot.phase = Phase::Idle;
                log_once("GPU frame was not finished after 500 ms");
                continue;
            }
            if pending.patch_origin.is_some() {
                publish_patch(&mut slot, &pending);
            } else {
                publish_target(&mut slot, &pending);
            }
            slot.phase = Phase::Idle;
            drop(slot);
            unsafe { cs_canvas_item_request_update() };
        }
    }

    fn submit_recorded(slot: &mut Slot) -> Result<(), String> {
        let Phase::Recorded(pending) = &slot.phase else {
            return Ok(());
        };
        let width = pending.width;
        let height = pending.height;
        let started = pending.started;
        let clear = pending.clear;
        let gpu = slot
            .gpu
            .as_mut()
            .ok_or_else(|| "GPU released before submit".to_string())?;
        let patch = pending.patch_origin;
        let surface_slot = if patch.is_none() {
            bind_surface(gpu, width, height)?
        } else {
            0
        };
        let view = if patch.is_some() {
            ensure_scratch(gpu, width, height);
            gpu.scratch
                .as_ref()
                .ok_or_else(|| "patch target missing".to_string())?
                .view
                .clone()
        } else {
            full_frame_view(gpu, surface_slot)?
        };
        let dev_id = gpu.dev_id;
        let device = &gpu.cx.devices[dev_id].device;
        let queue = &gpu.cx.devices[dev_id].queue;
        let params = RenderParams {
            base_color: color_of(clear),
            width,
            height,
            antialiasing_method: AaConfig::Area,
        };
        gpu.renderer
            .render_to_texture(device, queue, &gpu.scene, &view, &params)
            .map_err(|err| err.to_string())?;
        slot.phase = Phase::Waiting(Pending {
            width,
            height,
            started,
            clear,
            slot: surface_slot,
            patch_origin: patch,
        });
        Ok(())
    }

    fn wait_until_gpu_idle(
        mut slot: MutexGuard<'static, Slot>,
        started: Instant,
    ) -> (MutexGuard<'static, Slot>, bool) {
        loop {
            if slot.stop {
                return (slot, false);
            }
            let done = slot.gpu.as_ref().is_some_and(|gpu| {
                let device = &gpu.cx.devices[gpu.dev_id].device;
                device.poll(wgpu::Maintain::Poll).is_queue_empty()
            });
            if done {
                return (slot, true);
            }
            if started.elapsed() > Duration::from_millis(500) {
                return (slot, false);
            }
            drop(slot);
            std::thread::sleep(Duration::from_micros(200));
            slot = lock_gpu();
        }
    }

    fn publish_target(slot: &mut Slot, pending: &Pending) {
        #[cfg(target_os = "macos")]
        {
            let _ = slot;
            unsafe { cs_vello_shared_commit(pending.slot) };
        }
        #[cfg(not(target_os = "macos"))]
        {
            #[cfg(any(windows, target_os = "linux"))]
            if slot.gpu.as_ref().is_some_and(|gpu| gpu.shared) {
                unsafe { cs_vello_share_commit(pending.slot) };
                return;
            }
            let _ = pending.slot;
            let copied = {
                let Some(gpu) = slot.gpu.as_ref() else {
                    return;
                };
                let Some(frame) = gpu.frame.as_ref() else {
                    log_once("frame target missing");
                    return;
                };
                (frame.texture.clone(), frame.width, frame.height)
            };
            let (texture, width, height) = copied;
            let Some(gpu) = slot.gpu.as_ref() else {
                return;
            };
            readback(gpu, &texture, width, height);
        }
    }

    fn publish_patch(slot: &mut Slot, pending: &Pending) {
        let Some((x, y)) = pending.patch_origin else {
            return;
        };
        #[cfg(target_os = "macos")]
        {
            let Some(gpu) = slot.gpu.as_ref() else {
                log_once("GPU released before patch");
                return;
            };
            let Some(scratch) = gpu.scratch.as_ref() else {
                log_once("patch target missing");
                return;
            };
            let tex = match metal_texture(&scratch.texture) {
                Ok(tex) => tex,
                Err(err) => {
                    log_once(&err);
                    return;
                }
            };
            let ok = unsafe {
                cs_vello_shared_patch(
                    tex,
                    x as i32,
                    y as i32,
                    pending.width as i32,
                    pending.height as i32,
                )
            };
            if ok == 0 {
                log_once("IOSurface patch failed");
            }
        }
        #[cfg(not(target_os = "macos"))]
        {
            #[cfg(any(windows, target_os = "linux"))]
            if slot.gpu.as_ref().is_some_and(|gpu| gpu.shared) {
                if !composite_shared(slot, x, y, pending.width, pending.height) {
                    log_share_fallback("shared patch failed");
                    forget_share(slot);
                }
                return;
            }
            if !composite_patch(slot, x, y, pending.width, pending.height) {
                log_once("frame patch failed");
            }
        }
    }

    fn init_gpu(width: u32, height: u32) -> Result<Gpu, String> {
        let mut cx = RenderContext::new();
        let dev_id = block_on(cx.device(None)).ok_or_else(|| "no gpu device".to_string())?;
        let renderer = {
            let device = &cx.devices[dev_id].device;
            Renderer::new(
                device,
                RendererOptions {
                    use_cpu: false,
                    antialiasing_support: AaSupport::area_only(),
                    num_init_threads: NonZeroUsize::new(1),
                    pipeline_cache: None,
                },
            )
            .map_err(|err| err.to_string())?
        };
        #[cfg(not(target_os = "macos"))]
        let _ = (width, height);
        #[cfg(target_os = "macos")]
        log_ready(width, height, "direct");
        Ok(Gpu {
            renderer,
            scene: Scene::new(),
            #[cfg(target_os = "macos")]
            surfaces: [None, None],
            #[cfg(not(target_os = "macos"))]
            frame: None,
            #[cfg(any(windows, target_os = "linux"))]
            surfaces: [None, None],
            #[cfg(any(windows, target_os = "linux"))]
            shared: false,
            scratch: None,
            dev_id,
            cx,
        })
    }

    fn ensure_scratch(gpu: &mut Gpu, width: u32, height: u32) {
        if gpu
            .scratch
            .as_ref()
            .is_some_and(|scratch| scratch.width == width && scratch.height == height)
        {
            return;
        }
        let device = &gpu.cx.devices[gpu.dev_id].device;
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("vello-patch"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::STORAGE_BINDING
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        gpu.scratch = Some(Scratch {
            width,
            height,
            texture,
            view,
        });
    }

    fn patch_bounds(
        canvas: &Canvas,
        scene_rects: &[Rect],
        full_w: u32,
        full_h: u32,
        dpr: f64,
    ) -> Option<(u32, u32, u32, u32)> {
        let vp = canvas.viewport();
        let center = vp.center();
        let s = (vp.zoom() * dpr).max(1e-6);
        let tx = full_w as f64 * 0.5 - center.x * s;
        let ty = full_h as f64 * 0.5 - center.y * s;
        let mut x0 = f64::MAX;
        let mut y0 = f64::MAX;
        let mut x1 = f64::MIN;
        let mut y1 = f64::MIN;
        for rect in scene_rects {
            x0 = x0.min(rect.left() * s + tx);
            y0 = y0.min(rect.top() * s + ty);
            x1 = x1.max(rect.right() * s + tx);
            y1 = y1.max(rect.bottom() * s + ty);
        }
        if !x0.is_finite() || !y0.is_finite() || !x1.is_finite() || !y1.is_finite() {
            return None;
        }
        x0 = (x0 - 2.0).floor().max(0.0);
        y0 = (y0 - 2.0).floor().max(0.0);
        x1 = (x1 + 2.0).ceil().min(full_w as f64);
        y1 = (y1 + 2.0).ceil().min(full_h as f64);
        let x = x0.round() as u32;
        let y = y0.round() as u32;
        if x >= full_w || y >= full_h {
            return None;
        }
        let w = ((x1 - x0).round() as u32).min(full_w - x);
        let h = ((y1 - y0).round() as u32).min(full_h - y);
        if w == 0 || h == 0 {
            return None;
        }
        Some((x, y, w, h))
    }

    fn has_base(width: u32, height: u32) -> bool {
        #[cfg(target_os = "macos")]
        {
            unsafe { cs_vello_shared_has_base(width as i32, height as i32) != 0 }
        }
        #[cfg(not(target_os = "macos"))]
        {
            #[cfg(any(windows, target_os = "linux"))]
            if !SHARE_OFF.load(Ordering::Acquire)
                && unsafe { cs_vello_share_has_base(width as i32, height as i32) } != 0
            {
                return true;
            }
            FRAME_W.load(Ordering::Acquire) == width && FRAME_H.load(Ordering::Acquire) == height
        }
    }

    #[cfg(target_os = "macos")]
    fn full_frame_view(gpu: &Gpu, surface_slot: i32) -> Result<wgpu::TextureView, String> {
        gpu.surfaces
            .get(surface_slot as usize)
            .and_then(|surface| surface.as_ref())
            .map(|surface| surface.view.clone())
            .ok_or_else(|| "IOSurface target missing".to_string())
    }

    #[cfg(not(target_os = "macos"))]
    fn full_frame_view(gpu: &Gpu, surface_slot: i32) -> Result<wgpu::TextureView, String> {
        #[cfg(any(windows, target_os = "linux"))]
        if gpu.shared {
            return gpu
                .surfaces
                .get(surface_slot as usize)
                .and_then(|surface| surface.as_ref())
                .map(|surface| surface.view.clone())
                .ok_or_else(|| "shared target missing".to_string());
        }
        let _ = surface_slot;
        gpu.frame
            .as_ref()
            .map(|frame| frame.view.clone())
            .ok_or_else(|| "frame target missing".to_string())
    }

    #[cfg(target_os = "macos")]
    fn bind_surface(gpu: &mut Gpu, width: u32, height: u32) -> Result<i32, String> {
        let dev_ptr = {
            let device = &gpu.cx.devices[gpu.dev_id].device;
            metal_device(device)?
        };
        if unsafe { cs_vello_shared_reset_needed(dev_ptr, width as i32, height as i32) } != 0 {
            gpu.surfaces = [None, None];
        }
        let mut surface_slot = 0i32;
        let mut epoch = 0i32;
        let ptr = unsafe {
            cs_vello_shared_begin(
                dev_ptr,
                width as i32,
                height as i32,
                &mut surface_slot,
                &mut epoch,
            )
        };
        if ptr.is_null() || !(0..=1).contains(&surface_slot) {
            return Err("IOSurface target failed".to_string());
        }
        let idx = surface_slot as usize;
        let ptr_bits = ptr as usize;
        let reuse = gpu.surfaces[idx]
            .as_ref()
            .is_some_and(|surface| surface.epoch == epoch && surface.ptr == ptr_bits);
        if !reuse {
            let device = &gpu.cx.devices[gpu.dev_id].device;
            let (texture, view) = import_surface(device, ptr, width, height);
            gpu.surfaces[idx] = Some(ImportedSurface {
                epoch,
                ptr: ptr_bits,
                _texture: texture,
                view,
            });
        }
        Ok(surface_slot)
    }

    #[cfg(not(target_os = "macos"))]
    fn bind_surface(gpu: &mut Gpu, width: u32, height: u32) -> Result<i32, String> {
        #[cfg(any(windows, target_os = "linux"))]
        if !SHARE_OFF.load(Ordering::Acquire) {
            match bind_shared(gpu, width, height) {
                Ok(slot) => {
                    gpu.shared = true;
                    log_ready(width, height, "shared");
                    return Ok(slot);
                }
                Err(err) => {
                    gpu.shared = false;
                    gpu.surfaces = [None, None];
                    SHARE_OFF.store(true, Ordering::Release);
                    unsafe { cs_vello_share_release() };
                    log_share_fallback(&err);
                }
            }
        }
        ensure_frame(gpu, width, height);
        if gpu.frame.is_none() {
            return Err("frame target missing".to_string());
        }
        FRAME_W.store(width, Ordering::Release);
        FRAME_H.store(height, Ordering::Release);
        log_ready(width, height, "readback");
        Ok(0)
    }

    #[cfg(any(windows, target_os = "linux"))]
    fn bind_shared(gpu: &mut Gpu, width: u32, height: u32) -> Result<i32, String> {
        if unsafe { cs_vello_share_blocked() } != 0 {
            return Err("shared texture is unavailable".to_string());
        }
        let handles = share_handles(gpu)?;
        if unsafe {
            cs_vello_share_reset_needed(handles.kind, handles.device, width as i32, height as i32)
        } != 0
        {
            // Drop wgpu's references before begin() destroys the native images.
            gpu.surfaces = [None, None];
        }
        let mut surface_slot = 0i32;
        let mut epoch = 0i32;
        let ok = unsafe {
            cs_vello_share_begin(
                handles.kind,
                handles.device,
                handles.queue,
                handles.instance,
                handles.physical,
                handles.family,
                width as i32,
                height as i32,
                &mut surface_slot,
                &mut epoch,
            )
        };
        if ok == 0 || !(0..=1).contains(&surface_slot) {
            return Err("shared texture failed".to_string());
        }
        let ready = gpu.surfaces.iter().all(|surface| {
            surface.as_ref().is_some_and(|surface| {
                surface.epoch == epoch && surface.width == width && surface.height == height
            })
        });
        if ready {
            return Ok(surface_slot);
        }
        let device = gpu.cx.devices[gpu.dev_id].device.clone();
        for index in 0..2 {
            let reuse = gpu.surfaces[index].as_ref().is_some_and(|surface| {
                surface.epoch == epoch && surface.width == width && surface.height == height
            });
            if reuse {
                continue;
            }
            gpu.surfaces[index] = None;
            // AddRef once per epoch. Calling this every frame would leak.
            let bits = unsafe { cs_vello_share_resource(index as i32) };
            if bits == 0 {
                return Err("shared texture handle failed".to_string());
            }
            let (texture, view) = import_shared(&device, bits, width, height);
            gpu.surfaces[index] = Some(SharedSurface {
                epoch,
                width,
                height,
                texture,
                view,
            });
        }
        Ok(surface_slot)
    }

    #[cfg(windows)]
    fn share_handles(gpu: &Gpu) -> Result<ShareHandles, String> {
        let device = &gpu.cx.devices[gpu.dev_id].device;
        unsafe {
            device.as_hal::<wgpu::hal::api::Dx12, _, _>(|dev| {
                let dev = dev.ok_or_else(|| "not a Direct3D12 device".to_string())?;
                use windows::core::Interface;
                Ok(ShareHandles {
                    kind: 1,
                    device: dev.raw_device().as_raw() as usize as u64,
                    queue: dev.raw_queue().as_raw() as usize as u64,
                    instance: 0,
                    physical: 0,
                    family: 0,
                })
            })
        }
    }

    #[cfg(target_os = "linux")]
    fn share_handles(gpu: &Gpu) -> Result<ShareHandles, String> {
        use ash::vk::Handle;
        let device = &gpu.cx.devices[gpu.dev_id].device;
        unsafe {
            device.as_hal::<wgpu::hal::api::Vulkan, _, _>(|dev| {
                let dev = dev.ok_or_else(|| "not a Vulkan device".to_string())?;
                Ok(ShareHandles {
                    kind: 2,
                    device: dev.raw_device().handle().as_raw(),
                    queue: dev.raw_queue().as_raw(),
                    instance: dev.shared_instance().raw_instance().handle().as_raw(),
                    physical: dev.raw_physical_device().as_raw(),
                    family: dev.queue_family_index(),
                })
            })
        }
    }

    #[cfg(any(windows, target_os = "linux"))]
    fn share_texture_desc(width: u32, height: u32) -> wgpu::TextureDescriptor<'static> {
        wgpu::TextureDescriptor {
            label: Some("vello-share"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::STORAGE_BINDING
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        }
    }

    #[cfg(windows)]
    fn import_shared(
        device: &wgpu::Device,
        bits: u64,
        width: u32,
        height: u32,
    ) -> (wgpu::Texture, wgpu::TextureView) {
        use windows::core::Interface;
        // from_raw takes the AddRef from cs_vello_share_resource and does not AddRef again.
        let resource = unsafe {
            windows::Win32::Graphics::Direct3D12::ID3D12Resource::from_raw(
                bits as *mut std::ffi::c_void,
            )
        };
        let hal_tex = unsafe {
            wgpu::hal::dx12::Device::texture_from_raw(
                resource,
                wgpu::TextureFormat::Rgba8Unorm,
                wgpu::TextureDimension::D2,
                wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                1,
                1,
            )
        };
        let desc = share_texture_desc(width, height);
        let texture =
            unsafe { device.create_texture_from_hal::<wgpu::hal::api::Dx12>(hal_tex, &desc) };
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        (texture, view)
    }

    #[cfg(target_os = "linux")]
    fn import_shared(
        device: &wgpu::Device,
        bits: u64,
        width: u32,
        height: u32,
    ) -> (wgpu::Texture, wgpu::TextureView) {
        use ash::vk::Handle;
        let image = ash::vk::Image::from_raw(bits);
        let hal_desc = wgpu::hal::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::hal::TextureUses::COPY_SRC
                | wgpu::hal::TextureUses::COPY_DST
                | wgpu::hal::TextureUses::RESOURCE
                | wgpu::hal::TextureUses::STORAGE_READ_WRITE,
            memory_flags: wgpu::hal::MemoryFlags::empty(),
            view_formats: Vec::new(),
        };
        // The empty callback leaves destruction to cs_vello_share_release.
        let hal_tex = unsafe {
            wgpu::hal::vulkan::Device::texture_from_raw(image, &hal_desc, Some(Box::new(|| {})))
        };
        let desc = share_texture_desc(width, height);
        let texture =
            unsafe { device.create_texture_from_hal::<wgpu::hal::api::Vulkan>(hal_tex, &desc) };
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        (texture, view)
    }

    #[cfg(any(windows, target_os = "linux"))]
    #[allow(deprecated)]
    fn composite_shared(slot: &Slot, x: u32, y: u32, width: u32, height: u32) -> bool {
        let mut dst = 0i32;
        let mut src = 0i32;
        let mut copy_full = 0i32;
        if unsafe { cs_vello_share_patch_dest(&mut dst, &mut src, &mut copy_full) } == 0 {
            return false;
        }
        if !(0..=1).contains(&dst) || !(0..=1).contains(&src) {
            return false;
        }
        let Some(gpu) = slot.gpu.as_ref() else {
            return false;
        };
        let Some(dst_surf) = gpu.surfaces[dst as usize].as_ref() else {
            return false;
        };
        let Some(src_surf) = gpu.surfaces[src as usize].as_ref() else {
            return false;
        };
        let Some(scratch) = gpu.scratch.as_ref() else {
            return false;
        };
        let dst_tex = dst_surf.texture.clone();
        let src_tex = src_surf.texture.clone();
        let scratch_tex = scratch.texture.clone();
        let frame_w = dst_surf.width;
        let frame_h = dst_surf.height;
        let dev_id = gpu.dev_id;
        let device = gpu.cx.devices[dev_id].device.clone();
        let queue = gpu.cx.devices[dev_id].queue.clone();
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("vello-share-patch"),
        });
        if copy_full != 0 && dst != src {
            encoder.copy_texture_to_texture(
                wgpu::ImageCopyTexture {
                    texture: &src_tex,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                wgpu::ImageCopyTexture {
                    texture: &dst_tex,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                wgpu::Extent3d {
                    width: frame_w,
                    height: frame_h,
                    depth_or_array_layers: 1,
                },
            );
        }
        encoder.copy_texture_to_texture(
            wgpu::ImageCopyTexture {
                texture: &scratch_tex,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::ImageCopyTexture {
                texture: &dst_tex,
                mip_level: 0,
                origin: wgpu::Origin3d { x, y, z: 0 },
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
        queue.submit([encoder.finish()]);
        let _ = device.poll(wgpu::Maintain::Wait);
        unsafe { cs_vello_share_commit(dst) };
        true
    }

    #[cfg(any(windows, target_os = "linux"))]
    fn forget_share(slot: &mut Slot) {
        if let Some(gpu) = slot.gpu.as_mut() {
            gpu.shared = false;
            gpu.surfaces = [None, None];
        }
        SHARE_OFF.store(true, Ordering::Release);
        // Leave the native images alive. Qt may still be sampling the last
        // wrapped texture; release() destroys them after that wrapper is gone.
        unsafe { cs_vello_share_block() };
    }

    #[cfg(target_os = "macos")]
    fn import_surface(
        device: &wgpu::Device,
        ptr: *mut c_void,
        width: u32,
        height: u32,
    ) -> (wgpu::Texture, wgpu::TextureView) {
        // `from_ptr` borrows the texture ObjC already owns. Clone retains a
        // second reference for wgpu, which releases it when the texture drops.
        let borrowed = unsafe { metal::Texture::from_ptr(ptr.cast()) };
        let owned = borrowed.clone();
        std::mem::forget(borrowed);
        let hal_tex = unsafe {
            wgpu::hal::metal::Device::texture_from_raw(
                owned,
                wgpu::TextureFormat::Rgba8Unorm,
                metal::MTLTextureType::D2,
                1,
                1,
                wgpu::hal::CopyExtent {
                    width,
                    height,
                    depth: 1,
                },
            )
        };
        let texture = unsafe {
            device.create_texture_from_hal::<Metal>(
                hal_tex,
                &wgpu::TextureDescriptor {
                    label: Some("vello-iosurface"),
                    size: wgpu::Extent3d {
                        width,
                        height,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    usage: wgpu::TextureUsages::STORAGE_BINDING
                        | wgpu::TextureUsages::TEXTURE_BINDING,
                    view_formats: &[],
                },
            )
        };
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        (texture, view)
    }

    #[cfg(target_os = "macos")]
    fn metal_device(device: &wgpu::Device) -> Result<*mut c_void, String> {
        unsafe {
            device.as_hal::<Metal, _, _>(|dev| {
                dev.map(|d| d.raw_device().lock().as_ptr() as *mut c_void)
                    .ok_or_else(|| "not a metal device".to_string())
            })
        }
    }

    #[cfg(target_os = "macos")]
    fn metal_texture(texture: &wgpu::Texture) -> Result<*mut c_void, String> {
        unsafe {
            texture.as_hal::<Metal, _, _>(|tex| {
                tex.map(|t| t.raw_handle().as_ptr() as *mut c_void)
                    .ok_or_else(|| "not a metal texture".to_string())
            })
        }
    }

    #[cfg(not(target_os = "macos"))]
    fn ensure_frame(gpu: &mut Gpu, width: u32, height: u32) {
        if gpu
            .frame
            .as_ref()
            .is_some_and(|frame| frame.width == width && frame.height == height)
        {
            return;
        }
        let device = &gpu.cx.devices[gpu.dev_id].device;
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("vello-frame"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::STORAGE_BINDING
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        gpu.frame = Some(Scratch {
            width,
            height,
            texture,
            view,
        });
    }

    #[cfg(not(target_os = "macos"))]
    #[allow(deprecated)]
    fn composite_patch(slot: &mut Slot, x: u32, y: u32, width: u32, height: u32) -> bool {
        let Some(gpu) = slot.gpu.as_ref() else {
            return false;
        };
        let dev_id = gpu.dev_id;
        let device = gpu.cx.devices[dev_id].device.clone();
        let queue = gpu.cx.devices[dev_id].queue.clone();
        let Some(frame) = gpu.frame.as_ref() else {
            return false;
        };
        let frame_tex = frame.texture.clone();
        let frame_w = frame.width;
        let frame_h = frame.height;
        let Some(scratch) = gpu.scratch.as_ref() else {
            return false;
        };
        let scratch_tex = scratch.texture.clone();
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("vello-patch-copy"),
        });
        encoder.copy_texture_to_texture(
            wgpu::ImageCopyTexture {
                texture: &scratch_tex,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::ImageCopyTexture {
                texture: &frame_tex,
                mip_level: 0,
                origin: wgpu::Origin3d { x, y, z: 0 },
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
        queue.submit([encoder.finish()]);
        let _ = device.poll(wgpu::Maintain::Wait);
        readback(gpu, &frame_tex, frame_w, frame_h);
        true
    }

    #[cfg(not(target_os = "macos"))]
    #[allow(deprecated)]
    fn readback(gpu: &Gpu, texture: &wgpu::Texture, width: u32, height: u32) {
        let device = &gpu.cx.devices[gpu.dev_id].device;
        let queue = &gpu.cx.devices[gpu.dev_id].queue;
        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let row = width.saturating_mul(4);
        let padded = row.div_ceil(align).saturating_mul(align).max(align);
        let size = padded as u64 * height as u64;
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("vello-readback"),
            size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("vello-readback"),
        });
        encoder.copy_texture_to_buffer(
            wgpu::ImageCopyTexture {
                texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::ImageCopyBuffer {
                buffer: &buffer,
                layout: wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(padded),
                    rows_per_image: Some(height),
                },
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
        queue.submit([encoder.finish()]);
        let slice = buffer.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = tx.send(result);
        });
        let _ = device.poll(wgpu::Maintain::Wait);
        if rx.recv().ok().and_then(|result| result.ok()).is_none() {
            log_once("GPU readback failed");
            return;
        }
        let mapped = slice.get_mapped_range();
        let mut packed = vec![0u8; (row as usize).saturating_mul(height as usize)];
        for y in 0..height as usize {
            let src = y * padded as usize;
            let dst = y * row as usize;
            let n = row as usize;
            packed[dst..dst + n].copy_from_slice(&mapped[src..src + n]);
        }
        drop(mapped);
        buffer.unmap();
        super::store_cpu_frame(packed, width, height, row);
    }

    fn log_ready(width: u32, height: u32, handoff: &str) {
        static HANDED: AtomicBool = AtomicBool::new(false);
        if !HANDED.swap(true, Ordering::AcqRel) {
            eprintln!("[vello] canvas ready ({width}x{height}, {handoff})");
        }
    }

    #[cfg(any(windows, target_os = "linux"))]
    fn log_share_fallback(msg: &str) {
        if !SHARE_LOGGED.swap(true, Ordering::AcqRel) {
            eprintln!("[vello] {msg}. Copying the frame instead.");
        }
    }

    fn log_once(msg: &str) {
        if !LOGGED.swap(true, Ordering::AcqRel) {
            eprintln!("[vello] canvas unavailable: {msg}");
        }
    }
}
