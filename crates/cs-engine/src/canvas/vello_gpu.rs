//! Headless Vello frame shared by Save Image and the pixel tests.
//!
//! One GPU device is cached for the process. Callers take turns on a mutex.

use std::future::Future;
use std::num::NonZeroUsize;
use std::sync::{Arc, Mutex};

use vello::util::RenderContext;
use vello::wgpu;
use vello::{AaConfig, AaSupport, RenderParams, Renderer, RendererOptions, Scene};

use super::image_buf::ImageBuf;
use super::vello_draw::{self, color_of};
use super::{Canvas, Color, Palette, Point};

const MAX_DIM: u32 = 8192;

struct Target {
    width: u32,
    height: u32,
    texture: wgpu::Texture,
    view: wgpu::TextureView,
}

struct Gpu {
    cx: RenderContext,
    dev_id: usize,
    renderer: Renderer,
    scene: Scene,
    target: Option<Target>,
}

enum Slot {
    Ready(Gpu),
    Unavailable,
}

static GPU: Mutex<Option<Slot>> = Mutex::new(None);

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

fn with_gpu<T>(f: impl FnOnce(&mut Gpu) -> Option<T>) -> Option<T> {
    let mut slot = GPU.lock().unwrap_or_else(|err| err.into_inner());
    if slot.is_none() {
        *slot = Some(match Gpu::open() {
            Some(gpu) => Slot::Ready(gpu),
            None => Slot::Unavailable,
        });
    }
    match slot.as_mut()? {
        Slot::Ready(gpu) => f(gpu),
        Slot::Unavailable => None,
    }
}

fn dim(value: u32) -> Option<u32> {
    if value == 0 || value > MAX_DIM {
        None
    } else {
        Some(value)
    }
}

impl Gpu {
    fn open() -> Option<Self> {
        let mut cx = RenderContext::new();
        let dev_id = block_on(cx.device(None))?;
        let renderer = Renderer::new(
            &cx.devices[dev_id].device,
            RendererOptions {
                use_cpu: false,
                antialiasing_support: AaSupport::area_only(),
                num_init_threads: NonZeroUsize::new(1),
                pipeline_cache: None,
            },
        )
        .ok()?;
        Some(Self {
            cx,
            dev_id,
            renderer,
            scene: Scene::new(),
            target: None,
        })
    }

    fn ensure_target(&mut self, width: u32, height: u32) {
        if self
            .target
            .as_ref()
            .is_some_and(|target| target.width == width && target.height == height)
        {
            return;
        }
        let device = &self.cx.devices[self.dev_id].device;
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("cs-export-frame"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        self.target = Some(Target {
            width,
            height,
            texture,
            view,
        });
    }

    fn draw(&mut self, width: u32, height: u32, clear: Color) -> Option<ImageBuf> {
        self.ensure_target(width, height);
        let device = self.cx.devices[self.dev_id].device.clone();
        let queue = self.cx.devices[self.dev_id].queue.clone();
        let view = self.target.as_ref()?.view.clone();
        let texture = self.target.as_ref()?.texture.clone();
        let params = RenderParams {
            base_color: color_of(clear),
            width,
            height,
            antialiasing_method: AaConfig::Area,
        };
        self.renderer
            .render_to_texture(&device, &queue, &self.scene, &view, &params)
            .ok()?;
        read_texture(&device, &queue, &texture, width, height)
    }
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
    with_gpu(|gpu| {
        if background {
            vello_draw::record(&mut gpu.scene, canvas, palette, width, height, dpr);
            gpu.draw(width, height, palette.canvas)
        } else {
            vello_draw::record_items_at(
                &mut gpu.scene,
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
            gpu.draw(width, height, Color::rgba(0, 0, 0, 0))
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
    with_gpu(|gpu| {
        vello_draw::record_items_view(
            &mut gpu.scene,
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
        gpu.draw(patch_w, patch_h, Color::rgba(0, 0, 0, 0))
    })
}

fn read_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    width: u32,
    height: u32,
) -> Option<ImageBuf> {
    let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
    let row = width.saturating_mul(4);
    let padded = row.div_ceil(align).saturating_mul(align).max(align);
    let size = u64::from(padded) * u64::from(height);
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("cs-export-readback"),
        size,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("cs-export-readback"),
    });
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
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
