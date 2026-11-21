//! Shared Vello GPU device and headless frame renderer.
//!
//! One GPU device and `vello::Renderer` are initialized once for the process
//! and shared by the schematic presenter, the instrument windows (oscilloscope,
//! logic analyzer), Save Image, and the pixel tests.
//!
//! Offscreen callers share a small target pool: the two most recent pixel sizes
//! each keep a texture and a readback buffer, and a third size replaces the
//! older one.

use std::env;
use std::future::Future;
use std::num::NonZeroUsize;
use std::pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock, mpsc};
use std::task::{Context, Poll, Wake, Waker};
use std::thread::{self, Thread};
use std::time::Duration;

use vello::util::RenderContext;
use vello::wgpu::{
    self, Buffer, BufferDescriptor, BufferUsages, CommandEncoderDescriptor, Device, Extent3d,
    Maintain, MapMode, Origin3d, Queue, TexelCopyBufferInfo, TexelCopyBufferLayout,
    TexelCopyTextureInfo, Texture, TextureAspect, TextureDescriptor, TextureDimension,
    TextureFormat, TextureUsages, TextureView, TextureViewDescriptor,
};
#[cfg(windows)]
use vello::wgpu::{BackendOptions, Backends, Instance, InstanceDescriptor, InstanceFlags};
use vello::{AaConfig, AaSupport, RenderParams, Renderer, RendererOptions, Scene};

use super::image_buf::ImageBuf;
use super::vello_draw::{self, color_of};
use super::{Canvas, Color, Draw, Palette, Point};

const MAX_DIM: u32 = 8192;
/// Scope and logic-analyzer windows, or one of those plus Save Image.
const TARGET_SLOTS: usize = 2;

/// Process-wide Vello GPU device and renderer shared across all renderers.
pub struct VelloGpu {
    _cx: RenderContext,
    device: Device,
    queue: Queue,
    renderer: Mutex<Renderer>,
}

struct Target {
    width: u32,
    height: u32,
    texture: Texture,
    view: TextureView,
    /// Reused for every readback at this size. Mapped only for the copy, then unmapped.
    readback: Buffer,
}

struct OffscreenPool {
    scene: Scene,
    /// Newest size is last. A hit moves that target to the end.
    targets: Vec<Target>,
}

static SHARED: OnceLock<Result<VelloGpu, String>> = OnceLock::new();
static OFFSCREEN: Mutex<Option<OffscreenPool>> = Mutex::new(None);

struct ThreadWaker(Thread);

impl Wake for ThreadWaker {
    fn wake(self: Arc<Self>) {
        self.0.unpark();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        self.0.unpark();
    }
}

fn block_on<F: Future>(fut: F) -> F::Output {
    let mut fut = pin::pin!(fut);
    let waker = Waker::from(Arc::new(ThreadWaker(thread::current())));
    let mut cx = Context::from_waker(&waker);
    loop {
        match fut.as_mut().poll(&mut cx) {
            Poll::Ready(value) => return value,
            Poll::Pending => {
                thread::park_timeout(Duration::from_millis(1));
            }
        }
    }
}

impl VelloGpu {
    /// Return the shared Vello GPU device, initializing it once on first use.
    pub fn get() -> Result<&'static Self, &'static str> {
        SHARED
            .get_or_init(Self::open)
            .as_ref()
            .map_err(String::as_str)
    }

    pub fn device(&self) -> &Device {
        &self.device
    }

    pub fn queue(&self) -> &Queue {
        &self.queue
    }

    /// Submit `scene` into `view` and return a completion flag that turns `true`
    /// once the GPU finishes that submission (driven by `device.poll`).
    pub fn render_to_texture(
        &self,
        scene: &Scene,
        view: &TextureView,
        params: &RenderParams,
    ) -> Result<Arc<AtomicBool>, String> {
        let done = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&done);
        let mut renderer = self.renderer.lock().unwrap_or_else(|err| err.into_inner());
        renderer
            .render_to_texture(&self.device, &self.queue, scene, view, params)
            .map_err(|err| err.to_string())?;
        self.queue.on_submitted_work_done(move || {
            flag.store(true, Ordering::Release);
        });
        Ok(done)
    }

    fn open() -> Result<Self, String> {
        if env::var("CS_NO_GPU").is_ok() {
            return Err("GPU disabled by CS_NO_GPU".to_string());
        }
        // On Windows, construct the DX12 instance directly without creating a
        // throwaway default instance first.
        #[cfg(windows)]
        let mut cx = RenderContext {
            instance: Instance::new(&InstanceDescriptor {
                backends: Backends::from_env().unwrap_or(Backends::DX12),
                flags: InstanceFlags::from_build_config().with_env(),
                backend_options: BackendOptions::from_env_or_default(),
            }),
            devices: Vec::new(),
        };
        #[cfg(not(windows))]
        let mut cx = RenderContext::new();

        let dev_id = block_on(cx.device(None)).ok_or_else(|| "no gpu device".to_string())?;
        let device = cx.devices[dev_id].device.clone();
        let queue = cx.devices[dev_id].queue.clone();
        let renderer = Renderer::new(
            &device,
            RendererOptions {
                use_cpu: false,
                antialiasing_support: AaSupport::area_only(),
                num_init_threads: if cfg!(target_os = "macos") {
                    NonZeroUsize::new(1)
                } else {
                    None
                },
                pipeline_cache: None,
            },
        )
        .map_err(|err| err.to_string())?;
        Ok(Self {
            _cx: cx,
            device,
            queue,
            renderer: Mutex::new(renderer),
        })
    }
}

fn with_offscreen<T>(f: impl FnOnce(&VelloGpu, &mut OffscreenPool) -> Option<T>) -> Option<T> {
    let gpu = VelloGpu::get().ok()?;
    let mut slot = OFFSCREEN.lock().unwrap_or_else(|err| err.into_inner());
    let pool = slot.get_or_insert_with(|| OffscreenPool {
        scene: Scene::new(),
        targets: Vec::new(),
    });
    f(gpu, pool)
}

fn dim(value: u32) -> Option<u32> {
    if value == 0 || value > MAX_DIM {
        None
    } else {
        Some(value)
    }
}

impl OffscreenPool {
    fn make_target(device: &Device, width: u32, height: u32) -> Target {
        let texture = device.create_texture(&TextureDescriptor {
            label: Some("cs-export-frame"),
            size: Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba8Unorm,
            usage: TextureUsages::STORAGE_BINDING | TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = texture.create_view(&TextureViewDescriptor::default());
        let readback = device.create_buffer(&BufferDescriptor {
            label: Some("cs-export-readback"),
            size: u64::from(readback_stride(width)) * u64::from(height),
            usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        Target {
            width,
            height,
            texture,
            view,
            readback,
        }
    }

    fn prepare_target(&mut self, device: &Device, width: u32, height: u32) {
        if let Some(index) = self
            .targets
            .iter()
            .position(|target| target.width == width && target.height == height)
        {
            if index + 1 != self.targets.len() {
                let target = self.targets.remove(index);
                self.targets.push(target);
            }
            return;
        }
        let target = Self::make_target(device, width, height);
        if self.targets.len() >= TARGET_SLOTS {
            self.targets.remove(0);
        }
        self.targets.push(target);
    }

    fn draw(&mut self, gpu: &VelloGpu, width: u32, height: u32, clear: Color) -> Option<ImageBuf> {
        self.prepare_target(gpu.device(), width, height);
        let target = self.targets.last()?;
        let params = RenderParams {
            base_color: color_of(clear),
            width,
            height,
            antialiasing_method: AaConfig::Area,
        };
        gpu.render_to_texture(&self.scene, &target.view, &params)
            .ok()?;
        read_texture(
            gpu.device(),
            gpu.queue(),
            &target.texture,
            &target.readback,
            width,
            height,
        )
    }
}

/// Rasterize one `Draw` pass. Coordinates are pixels. `clear` is the base color.
pub fn render_draw(
    width: u32,
    height: u32,
    clear: Color,
    draw: impl FnOnce(&mut dyn Draw),
) -> Option<ImageBuf> {
    let width = dim(width)?;
    let height = dim(height)?;
    with_offscreen(|gpu, pool| {
        vello_draw::record_direct(&mut pool.scene, width as f64, height as f64, draw);
        pool.draw(gpu, width, height, clear)
    })
}

/// Full frame. `background` paints the paper and the grid.
pub(crate) fn render_view(
    canvas: &Canvas,
    palette: &Palette,
    width: u32,
    height: u32,
    dpr: f64,
    background: bool,
) -> Option<ImageBuf> {
    let width = dim(width)?;
    let height = dim(height)?;
    with_offscreen(|gpu, pool| {
        if background {
            vello_draw::record(&mut pool.scene, canvas, palette, width, height, dpr);
            pool.draw(gpu, width, height, palette.canvas)
        } else {
            vello_draw::record_items_at(
                &mut pool.scene,
                canvas,
                palette,
                width,
                height,
                0,
                0,
                width,
                height,
                dpr,
            );
            pool.draw(gpu, width, height, Color::rgba(0, 0, 0, 0))
        }
    })
}

/// Items only, in one rectangle of a full frame. The image is `patch_w` by `patch_h`.
pub(crate) fn render_items_patch(
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
) -> Option<ImageBuf> {
    let full_w = dim(full_w)?;
    let full_h = dim(full_h)?;
    let patch_w = dim(patch_w)?;
    let patch_h = dim(patch_h)?;
    with_offscreen(|gpu, pool| {
        vello_draw::record_items_view(
            &mut pool.scene,
            canvas,
            palette,
            full_w,
            full_h,
            origin_x,
            origin_y,
            patch_w,
            patch_h,
            dpr,
            center,
            zoom,
        );
        pool.draw(gpu, patch_w, patch_h, Color::rgba(0, 0, 0, 0))
    })
}

/// Row stride of a mapped RGBA8 readback. `copy_texture_to_buffer` rounds each
/// row up to `COPY_BYTES_PER_ROW_ALIGNMENT`.
pub fn readback_stride(width: u32) -> u32 {
    let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
    let row = width.saturating_mul(4);
    row.div_ceil(align).saturating_mul(align).max(align)
}

/// Copy an RGBA8 texture into an `ImageBuf`.
///
/// `buffer` is kept by the caller and reused. It must be
/// `readback_stride(width) * height` bytes, created with `COPY_DST | MAP_READ`,
/// and it must not be mapped.
pub fn read_texture(
    device: &Device,
    queue: &Queue,
    texture: &Texture,
    buffer: &Buffer,
    width: u32,
    height: u32,
) -> Option<ImageBuf> {
    let padded = readback_stride(width);
    let row = width.saturating_mul(4);
    let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor {
        label: Some("cs-export-readback"),
    });
    encoder.copy_texture_to_buffer(
        TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: Origin3d::ZERO,
            aspect: TextureAspect::All,
        },
        TexelCopyBufferInfo {
            buffer,
            layout: TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(padded),
                rows_per_image: Some(height),
            },
        },
        Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );
    let submission = queue.submit([encoder.finish()]);
    let slice = buffer.slice(..);
    let (tx, rx) = mpsc::channel();
    slice.map_async(MapMode::Read, move |result| {
        let _ = tx.send(result);
    });
    let _ = device.poll(Maintain::WaitForSubmissionIndex(submission));
    rx.recv().ok()?.ok()?;
    let mapped = slice.get_mapped_range();
    let mut pixels = vec![0u8; row as usize * height as usize];
    let row_n = row as usize;
    let padded_n = padded as usize;
    for y in 0..height as usize {
        let src = y * padded_n;
        let dst = y * row_n;
        pixels[dst..dst + row_n].copy_from_slice(&mapped[src..src + row_n]);
    }
    drop(mapped);
    buffer.unmap();
    ImageBuf::from_rgba(width, height, pixels)
}

#[cfg(test)]
mod tests {
    use std::ptr;
    use std::sync::atomic::Ordering;
    use std::thread;
    use std::time::{Duration, Instant};

    use vello::wgpu::{
        Extent3d, Maintain, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
        TextureViewDescriptor,
    };
    use vello::{AaConfig, RenderParams, Scene};

    use super::{VelloGpu, render_draw};
    use crate::canvas::Color;
    use crate::canvas::vello_draw::{self, color_of};

    #[test]
    fn shared_gpu_reused_between_direct_and_offscreen_renders() {
        let Ok(gpu_a) = VelloGpu::get() else {
            eprintln!("skipping shared_gpu test: vello renderer unavailable");
            return;
        };
        let gpu_b = VelloGpu::get().expect("second VelloGpu::get should succeed");
        assert!(ptr::eq(gpu_a, gpu_b));

        // Simulate concurrent presenter (`render_to_texture`) + instrument (`render_draw`).
        let bg = Color::rgba(16, 32, 48, 255);
        let worker = thread::spawn(move || {
            render_draw(128, 64, bg, |draw| {
                draw.line(0.0, 32.0, 128.0, 32.0, Color::rgba(0, 255, 0, 255), 2.0);
            })
        });

        let tex = gpu_a.device().create_texture(&TextureDescriptor {
            label: Some("cs-test-shared"),
            size: Extent3d {
                width: 64,
                height: 64,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba8Unorm,
            usage: TextureUsages::STORAGE_BINDING | TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = tex.create_view(&TextureViewDescriptor::default());
        let mut scene = Scene::new();
        vello_draw::record_direct(&mut scene, 64.0, 64.0, |draw| {
            draw.line(0.0, 0.0, 64.0, 64.0, Color::rgba(255, 0, 0, 255), 2.0);
        });
        let params = RenderParams {
            base_color: color_of(bg),
            width: 64,
            height: 64,
            antialiasing_method: AaConfig::Area,
        };
        let done = gpu_a
            .render_to_texture(&scene, &view, &params)
            .expect("render_to_texture should succeed");
        let start = Instant::now();
        while !done.load(Ordering::Acquire) {
            let _ = gpu_a.device().poll(Maintain::Poll);
            assert!(
                start.elapsed() < Duration::from_secs(5),
                "shared render_to_texture completion flag did not fire"
            );
            thread::sleep(Duration::from_micros(200));
        }

        let offscreen = worker
            .join()
            .expect("worker thread panicked")
            .expect("offscreen render_draw should succeed");
        assert_eq!(offscreen.width(), 128);
        assert_eq!(offscreen.height(), 64);
    }
}
