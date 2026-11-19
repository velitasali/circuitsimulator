//! Zero-copy GPU frame sharing between Vello (`wgpu`) and Qt's `QRhi` on
//! Windows (D3D12 -> D3D11/D3D12) and Linux (Vulkan -> Vulkan/OpenGL).
//!
//! `cpp/vello_share.cpp` only queries Qt's `QRhi` native handles and wraps the
//! imported native texture via `QNativeInterface::QSG*Texture::fromNative`.

#![cfg(any(windows, target_os = "linux"))]

#[cfg(windows)]
use std::ffi::c_void;
use std::ffi::{CStr, c_char};
#[cfg(windows)]
use std::ptr;
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard};
#[cfg(windows)]
use windows::Win32::Foundation::{CloseHandle, GENERIC_ALL, HANDLE};
#[cfg(windows)]
use windows::Win32::Graphics::Direct3D11::{
    ID3D11Device, ID3D11Device1, ID3D11Device5, ID3D11DeviceContext, ID3D11DeviceContext4,
    ID3D11Fence, ID3D11Texture2D,
};
#[cfg(windows)]
use windows::Win32::Graphics::Direct3D12::{
    D3D12_FENCE_FLAG_SHARED, D3D12_HEAP_FLAG_SHARED, D3D12_HEAP_PROPERTIES,
    D3D12_HEAP_TYPE_DEFAULT, D3D12_RESOURCE_DESC, D3D12_RESOURCE_DIMENSION_TEXTURE2D,
    D3D12_RESOURCE_FLAG_ALLOW_SIMULTANEOUS_ACCESS, D3D12_RESOURCE_FLAG_ALLOW_UNORDERED_ACCESS,
    D3D12_RESOURCE_STATE_COMMON, ID3D12CommandQueue, ID3D12Device, ID3D12Fence, ID3D12Resource,
};
#[cfg(windows)]
use windows::Win32::Graphics::Dxgi::Common::{DXGI_FORMAT_R8G8B8A8_UNORM, DXGI_SAMPLE_DESC};
#[cfg(windows)]
use windows::core::{Interface, PCWSTR};

#[cfg(target_os = "linux")]
use ash::khr::{external_memory_fd, external_semaphore_fd};
#[cfg(target_os = "linux")]
use ash::vk::{self, Handle};

#[cfg(target_os = "linux")]
unsafe extern "C" {
    fn cs_vello_share_release_gl();
}

#[cfg(windows)]
pub struct ShareHandles {
    pub device_bits: u64,
    pub device: ID3D12Device,
    pub queue: ID3D12CommandQueue,
}

#[cfg(target_os = "linux")]
pub struct ShareHandles {
    pub device_bits: u64,
    pub instance: ash::Instance,
    pub device: ash::Device,
    pub physical: vk::PhysicalDevice,
    pub queue: vk::Queue,
}

#[cfg(windows)]
struct D3dState {
    device_bits: u64,
    qt_d3d11_dev: usize,
    qt_d3d11: [Option<ID3D11Texture2D>; 2],
    qt_fence11: Option<ID3D11Fence>,
    qt_d3d12_dev: usize,
    qt_d3d12: [Option<ID3D12Resource>; 2],
    qt_fence12: Option<ID3D12Fence>,
    tex_share: [HANDLE; 2],
    tex: [ID3D12Resource; 2],
    fence_share: HANDLE,
    fence: ID3D12Fence,
    queue: ID3D12CommandQueue,
    // Keep the producer device alive until all child resources above have dropped.
    _device: ID3D12Device,
}

#[cfg(windows)]
impl Drop for D3dState {
    fn drop(&mut self) {
        self.qt_d3d11 = [None, None];
        self.qt_fence11 = None;
        self.qt_d3d12 = [None, None];
        self.qt_fence12 = None;
        for h in &mut self.tex_share {
            if !h.is_invalid() {
                unsafe {
                    let _ = CloseHandle(*h);
                }
                *h = HANDLE::default();
            }
        }
        if !self.fence_share.is_invalid() {
            unsafe {
                let _ = CloseHandle(self.fence_share);
            }
            self.fence_share = HANDLE::default();
        }
    }
}

#[cfg(target_os = "linux")]
struct VkState {
    device_bits: u64,
    instance: ash::Instance,
    device: ash::Device,
    queue: vk::Queue,
    ext_mem: external_memory_fd::Device,
    ext_sem: external_semaphore_fd::Device,
    has_sem_fd: bool,
    images: [vk::Image; 2],
    memory: [vk::DeviceMemory; 2],
    bytes: [vk::DeviceSize; 2],
    sem: vk::Semaphore,
    qt_dev_bits: u64,
    qt_device: Option<ash::Device>,
    qt_images: [vk::Image; 2],
    qt_memory: [vk::DeviceMemory; 2],
    qt_sem: vk::Semaphore,
}

#[cfg(target_os = "linux")]
impl VkState {
    fn release_qt(&mut self) {
        if let Some(qt_dev) = self.qt_device.take() {
            unsafe {
                if self.qt_sem != vk::Semaphore::null() {
                    qt_dev.destroy_semaphore(self.qt_sem, None);
                    self.qt_sem = vk::Semaphore::null();
                }
                for i in 0..2 {
                    if self.qt_images[i] != vk::Image::null() {
                        qt_dev.destroy_image(self.qt_images[i], None);
                        self.qt_images[i] = vk::Image::null();
                    }
                    if self.qt_memory[i] != vk::DeviceMemory::null() {
                        qt_dev.free_memory(self.qt_memory[i], None);
                        self.qt_memory[i] = vk::DeviceMemory::null();
                    }
                }
            }
        }
        self.qt_dev_bits = 0;
        unsafe {
            cs_vello_share_release_gl();
        }
    }

    fn export_fd(&self, slot: usize) -> Option<i32> {
        let info = vk::MemoryGetFdInfoKHR::default()
            .memory(self.memory[slot])
            .handle_type(vk::ExternalMemoryHandleTypeFlags::OPAQUE_FD);
        unsafe { self.ext_mem.get_memory_fd(&info) }.ok()
    }
}

#[cfg(target_os = "linux")]
impl Drop for VkState {
    fn drop(&mut self) {
        self.release_qt();
        unsafe {
            if self.sem != vk::Semaphore::null() {
                self.device.destroy_semaphore(self.sem, None);
                self.sem = vk::Semaphore::null();
            }
            for i in 0..2 {
                if self.images[i] != vk::Image::null() {
                    self.device.destroy_image(self.images[i], None);
                    self.images[i] = vk::Image::null();
                }
                if self.memory[i] != vk::DeviceMemory::null() {
                    self.device.free_memory(self.memory[i], None);
                    self.memory[i] = vk::DeviceMemory::null();
                }
            }
        }
    }
}

struct ShareState {
    width: i32,
    height: i32,
    epoch: i32,
    values: [u64; 2],
    #[cfg(windows)]
    d3d: Option<D3dState>,
    #[cfg(target_os = "linux")]
    vk: Option<VkState>,
}

// All access to GPU handles is serialized behind `STATE`.
unsafe impl Send for ShareState {}

impl ShareState {
    const fn new() -> Self {
        Self {
            width: 0,
            height: 0,
            epoch: 0,
            values: [0, 0],
            #[cfg(windows)]
            d3d: None,
            #[cfg(target_os = "linux")]
            vk: None,
        }
    }

    fn device_bits(&self) -> u64 {
        #[cfg(windows)]
        {
            self.d3d.as_ref().map_or(0, |d| d.device_bits)
        }
        #[cfg(target_os = "linux")]
        {
            self.vk.as_ref().map_or(0, |v| v.device_bits)
        }
    }

    fn same_target(&self, device_bits: u64, width: i32, height: i32) -> bool {
        device_bits != 0
            && self.device_bits() == device_bits
            && self.width == width
            && self.height == height
            && width > 0
            && height > 0
    }

    fn release_all(&mut self) {
        #[cfg(windows)]
        {
            self.d3d = None;
        }
        #[cfg(target_os = "linux")]
        {
            self.vk = None;
        }
        self.width = 0;
        self.height = 0;
        PUBLISHED.store(-1, Ordering::Relaxed);
        HELD.store(-1, Ordering::Relaxed);
    }
}

static STATE: Mutex<ShareState> = Mutex::new(ShareState::new());
static HELD: AtomicI32 = AtomicI32::new(-1);
static PUBLISHED: AtomicI32 = AtomicI32::new(-1);
static FRAME: AtomicU64 = AtomicU64::new(0);
static BLOCKED: AtomicBool = AtomicBool::new(false);
static LOGGED: AtomicBool = AtomicBool::new(false);

fn lock_state() -> MutexGuard<'static, ShareState> {
    STATE.lock().unwrap_or_else(|err| err.into_inner())
}

fn log_fail(msg: &str) {
    if !LOGGED.swap(true, Ordering::AcqRel) {
        eprintln!("[vello] {msg}");
    }
}

fn block_share(msg: &str) {
    log_fail(msg);
    BLOCKED.store(true, Ordering::Release);
}

#[cfg(windows)]
fn make_d3d_texture(
    device: &ID3D12Device,
    width: i32,
    height: i32,
) -> Option<(ID3D12Resource, HANDLE)> {
    let heap = D3D12_HEAP_PROPERTIES {
        Type: D3D12_HEAP_TYPE_DEFAULT,
        ..Default::default()
    };
    let desc = D3D12_RESOURCE_DESC {
        Dimension: D3D12_RESOURCE_DIMENSION_TEXTURE2D,
        Width: width as u64,
        Height: height as u32,
        DepthOrArraySize: 1,
        MipLevels: 1,
        Format: DXGI_FORMAT_R8G8B8A8_UNORM,
        SampleDesc: DXGI_SAMPLE_DESC {
            Count: 1,
            Quality: 0,
        },
        Flags: D3D12_RESOURCE_FLAG_ALLOW_UNORDERED_ACCESS
            | D3D12_RESOURCE_FLAG_ALLOW_SIMULTANEOUS_ACCESS,
        ..Default::default()
    };
    let mut tex: Option<ID3D12Resource> = None;
    let res = unsafe {
        device.CreateCommittedResource(
            &heap,
            D3D12_HEAP_FLAG_SHARED,
            &desc,
            D3D12_RESOURCE_STATE_COMMON,
            None,
            &mut tex,
        )
    };
    let Some(tex) = res.ok().and(tex) else {
        log_fail("D3D12 shared texture failed");
        return None;
    };
    let Ok(handle) =
        (unsafe { device.CreateSharedHandle(&tex, None, GENERIC_ALL.0, PCWSTR::null()) })
    else {
        log_fail("D3D12 shared handle failed");
        return None;
    };
    if handle.is_invalid() {
        log_fail("D3D12 shared handle failed");
        return None;
    }
    Some((tex, handle))
}

#[cfg(windows)]
fn ensure_d3d(state: &mut ShareState, handles: &ShareHandles, width: i32, height: i32) -> bool {
    if state.same_target(handles.device_bits, width, height) {
        return true;
    }
    state.release_all();
    let Some((t0, h0)) = make_d3d_texture(&handles.device, width, height) else {
        return false;
    };
    let Some((t1, h1)) = make_d3d_texture(&handles.device, width, height) else {
        unsafe {
            let _ = CloseHandle(h0);
        }
        return false;
    };
    let Ok(fence) = (unsafe {
        handles
            .device
            .CreateFence::<ID3D12Fence>(0, D3D12_FENCE_FLAG_SHARED)
    }) else {
        unsafe {
            let _ = CloseHandle(h0);
            let _ = CloseHandle(h1);
        }
        log_fail("D3D12 shared fence failed");
        return false;
    };
    let Ok(fence_share) = (unsafe {
        handles
            .device
            .CreateSharedHandle(&fence, None, GENERIC_ALL.0, PCWSTR::null())
    }) else {
        unsafe {
            let _ = CloseHandle(h0);
            let _ = CloseHandle(h1);
        }
        log_fail("D3D12 fence handle failed");
        return false;
    };
    if fence_share.is_invalid() {
        unsafe {
            let _ = CloseHandle(h0);
            let _ = CloseHandle(h1);
        }
        log_fail("D3D12 fence handle failed");
        return false;
    }
    state.d3d = Some(D3dState {
        device_bits: handles.device_bits,
        qt_d3d11_dev: 0,
        qt_d3d11: [None, None],
        qt_fence11: None,
        qt_d3d12_dev: 0,
        qt_d3d12: [None, None],
        qt_fence12: None,
        tex_share: [h0, h1],
        tex: [t0, t1],
        fence_share,
        fence,
        queue: handles.queue.clone(),
        _device: handles.device.clone(),
    });
    state.width = width;
    state.height = height;
    state.epoch += 1;
    state.values = [0, 0];
    PUBLISHED.store(-1, Ordering::Relaxed);
    HELD.store(-1, Ordering::Relaxed);
    true
}

#[cfg(target_os = "linux")]
fn memory_type(instance: &ash::Instance, physical: vk::PhysicalDevice, bits: u32) -> Option<u32> {
    let props = unsafe { instance.get_physical_device_memory_properties(physical) };
    for i in 0..props.memory_type_count {
        if (bits & (1u32 << i)) != 0
            && props.memory_types[i as usize]
                .property_flags
                .contains(vk::MemoryPropertyFlags::DEVICE_LOCAL)
        {
            return Some(i);
        }
    }
    (0..props.memory_type_count).find(|&i| (bits & (1u32 << i)) != 0)
}

#[cfg(target_os = "linux")]
fn make_vk_image(
    instance: &ash::Instance,
    device: &ash::Device,
    physical: vk::PhysicalDevice,
    width: i32,
    height: i32,
) -> Option<(vk::Image, vk::DeviceMemory, vk::DeviceSize)> {
    let mut ext = vk::ExternalMemoryImageCreateInfo::default()
        .handle_types(vk::ExternalMemoryHandleTypeFlags::OPAQUE_FD);
    let info = vk::ImageCreateInfo::default()
        .push_next(&mut ext)
        .image_type(vk::ImageType::TYPE_2D)
        .format(vk::Format::R8G8B8A8_UNORM)
        .extent(vk::Extent3D {
            width: width as u32,
            height: height as u32,
            depth: 1,
        })
        .mip_levels(1)
        .array_layers(1)
        .samples(vk::SampleCountFlags::TYPE_1)
        .tiling(vk::ImageTiling::OPTIMAL)
        .usage(
            vk::ImageUsageFlags::STORAGE
                | vk::ImageUsageFlags::SAMPLED
                | vk::ImageUsageFlags::TRANSFER_SRC
                | vk::ImageUsageFlags::TRANSFER_DST,
        )
        .initial_layout(vk::ImageLayout::UNDEFINED);
    let Ok(image) = (unsafe { device.create_image(&info, None) }) else {
        log_fail("Vulkan shared image failed");
        return None;
    };
    let req = unsafe { device.get_image_memory_requirements(image) };
    let Some(mem_type) = memory_type(instance, physical, req.memory_type_bits) else {
        unsafe { device.destroy_image(image, None) };
        log_fail("Vulkan shared image has no memory type");
        return None;
    };
    let mut exp = vk::ExportMemoryAllocateInfo::default()
        .handle_types(vk::ExternalMemoryHandleTypeFlags::OPAQUE_FD);
    let mut dedicated = vk::MemoryDedicatedAllocateInfo::default().image(image);
    let alloc = vk::MemoryAllocateInfo::default()
        .push_next(&mut exp)
        .push_next(&mut dedicated)
        .allocation_size(req.size)
        .memory_type_index(mem_type);
    let Ok(memory) = (unsafe { device.allocate_memory(&alloc, None) }) else {
        unsafe { device.destroy_image(image, None) };
        log_fail("Vulkan shared memory failed");
        return None;
    };
    if unsafe { device.bind_image_memory(image, memory, 0) }.is_err() {
        unsafe {
            device.free_memory(memory, None);
            device.destroy_image(image, None);
        }
        log_fail("Vulkan bind shared image failed");
        return None;
    }
    Some((image, memory, req.size))
}

#[cfg(target_os = "linux")]
fn ensure_vk(state: &mut ShareState, handles: &ShareHandles, width: i32, height: i32) -> bool {
    if state.same_target(handles.device_bits, width, height) {
        return true;
    }
    state.release_all();
    let has_mem_fd = unsafe {
        handles
            .instance
            .get_device_proc_addr(handles.device.handle(), c"vkGetMemoryFdKHR".as_ptr())
            .is_some()
    };
    if !has_mem_fd {
        log_fail("Vulkan external-memory entry points are missing");
        return false;
    }
    let has_sem_fd = unsafe {
        handles
            .instance
            .get_device_proc_addr(handles.device.handle(), c"vkGetSemaphoreFdKHR".as_ptr())
            .is_some()
    };
    let Some((img0, mem0, bytes0)) = make_vk_image(
        &handles.instance,
        &handles.device,
        handles.physical,
        width,
        height,
    ) else {
        return false;
    };
    let Some((img1, mem1, bytes1)) = make_vk_image(
        &handles.instance,
        &handles.device,
        handles.physical,
        width,
        height,
    ) else {
        unsafe {
            handles.device.destroy_image(img0, None);
            handles.device.free_memory(mem0, None);
        }
        return false;
    };
    let mut exp = vk::ExportSemaphoreCreateInfo::default()
        .handle_types(vk::ExternalSemaphoreHandleTypeFlags::OPAQUE_FD);
    let mut timeline =
        vk::SemaphoreTypeCreateInfo::default().semaphore_type(vk::SemaphoreType::TIMELINE);
    let sem_info = vk::SemaphoreCreateInfo::default()
        .push_next(&mut exp)
        .push_next(&mut timeline);
    let Ok(sem) = (unsafe { handles.device.create_semaphore(&sem_info, None) }) else {
        unsafe {
            handles.device.destroy_image(img0, None);
            handles.device.free_memory(mem0, None);
            handles.device.destroy_image(img1, None);
            handles.device.free_memory(mem1, None);
        }
        log_fail("Vulkan timeline semaphore failed");
        return false;
    };
    let ext_mem = external_memory_fd::Device::new(&handles.instance, &handles.device);
    let ext_sem = external_semaphore_fd::Device::new(&handles.instance, &handles.device);
    state.vk = Some(VkState {
        device_bits: handles.device_bits,
        instance: handles.instance.clone(),
        device: handles.device.clone(),
        queue: handles.queue,
        ext_mem,
        ext_sem,
        has_sem_fd,
        images: [img0, img1],
        memory: [mem0, mem1],
        bytes: [bytes0, bytes1],
        sem,
        qt_dev_bits: 0,
        qt_device: None,
        qt_images: [vk::Image::null(), vk::Image::null()],
        qt_memory: [vk::DeviceMemory::null(), vk::DeviceMemory::null()],
        qt_sem: vk::Semaphore::null(),
    });
    state.width = width;
    state.height = height;
    state.epoch += 1;
    state.values = [0, 0];
    PUBLISHED.store(-1, Ordering::Relaxed);
    HELD.store(-1, Ordering::Relaxed);
    true
}

pub fn is_blocked() -> bool {
    BLOCKED.load(Ordering::Acquire)
}

pub fn block() {
    BLOCKED.store(true, Ordering::Release);
}

pub fn reset_needed(device_bits: u64, width: u32, height: u32) -> bool {
    let state = lock_state();
    if BLOCKED.load(Ordering::Acquire) {
        return true;
    }
    !state.same_target(device_bits, width as i32, height as i32)
}

pub fn begin(handles: &ShareHandles, width: u32, height: u32) -> Option<(usize, i32)> {
    let width = width as i32;
    let height = height as i32;
    if width < 1 || height < 1 || handles.device_bits == 0 {
        return None;
    }
    let mut state = lock_state();
    if BLOCKED.load(Ordering::Acquire) {
        return None;
    }
    #[cfg(windows)]
    let ok = ensure_d3d(&mut state, handles, width, height);
    #[cfg(target_os = "linux")]
    let ok = ensure_vk(&mut state, handles, width, height);
    if !ok {
        return None;
    }
    let held = HELD.load(Ordering::Acquire);
    let slot = if held == 0 { 1 } else { 0 };
    Some((slot, state.epoch))
}

#[cfg(windows)]
pub fn d3d12_resource(slot: usize) -> Option<ID3D12Resource> {
    if slot > 1 {
        return None;
    }
    let state = lock_state();
    state.d3d.as_ref().map(|d| d.tex[slot].clone())
}

#[cfg(target_os = "linux")]
pub fn vk_image(slot: usize) -> Option<vk::Image> {
    if slot > 1 {
        return None;
    }
    let state = lock_state();
    state.vk.as_ref().map(|v| v.images[slot])
}

pub fn commit(slot: usize) {
    if slot > 1 {
        return;
    }
    let mut state = lock_state();
    let next_value = state.values[slot] + 1;
    #[cfg(windows)]
    if let Some(d3d) = state.d3d.as_ref() {
        if unsafe { d3d.queue.Signal(&d3d.fence, next_value) }.is_ok() {
            state.values[slot] = next_value;
        }
    }
    #[cfg(target_os = "linux")]
    if let Some(vk_state) = state.vk.as_ref() {
        let values = [next_value];
        let sems = [vk_state.sem];
        let mut timeline =
            vk::TimelineSemaphoreSubmitInfo::default().signal_semaphore_values(&values);
        let submit = vk::SubmitInfo::default()
            .push_next(&mut timeline)
            .signal_semaphores(&sems);
        if unsafe {
            vk_state
                .device
                .queue_submit(vk_state.queue, &[submit], vk::Fence::null())
        }
        .is_ok()
        {
            let _ = unsafe { vk_state.device.queue_wait_idle(vk_state.queue) };
            state.values[slot] = next_value;
        }
    }
    PUBLISHED.store(slot as i32, Ordering::Relaxed);
    FRAME.fetch_add(1, Ordering::Release);
}

pub fn has_base(width: u32, height: u32) -> bool {
    let state = lock_state();
    let published = PUBLISHED.load(Ordering::Acquire);
    !BLOCKED.load(Ordering::Acquire)
        && published >= 0
        && state.width == width as i32
        && state.height == height as i32
}

pub fn patch_dest() -> Option<(usize, usize, bool)> {
    let _state = lock_state();
    let src = PUBLISHED.load(Ordering::Acquire);
    if !(0..=1).contains(&src) {
        return None;
    }
    let held = HELD.load(Ordering::Acquire);
    let dst = if held == 0 { 1usize } else { 0usize };
    let src = src as usize;
    Some((dst, src, dst != src))
}

pub fn release() {
    let mut state = lock_state();
    state.release_all();
    FRAME.store(0, Ordering::Relaxed);
}

#[unsafe(no_mangle)]
pub extern "C" fn cs_vello_share_generation() -> u64 {
    if BLOCKED.load(Ordering::Acquire) {
        return 0;
    }
    FRAME.load(Ordering::Acquire)
}

#[unsafe(no_mangle)]
pub extern "C" fn cs_vello_share_block_with_msg(msg: *const c_char) {
    if msg.is_null() {
        BLOCKED.store(true, Ordering::Release);
        return;
    }
    let text = unsafe { CStr::from_ptr(msg) }.to_string_lossy();
    block_share(&text);
}

#[cfg(windows)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cs_vello_share_adopt_d3d11(
    dev_ptr: *mut c_void,
    ctx_ptr: *mut c_void,
    out_w: *mut i32,
    out_h: *mut i32,
) -> *mut c_void {
    let mut state = lock_state();
    if BLOCKED.load(Ordering::Acquire) {
        return ptr::null_mut();
    }
    let slot_i = PUBLISHED.load(Ordering::Acquire);
    if !(0..=1).contains(&slot_i) || state.width < 1 || state.height < 1 {
        return ptr::null_mut();
    }
    let slot = slot_i as usize;
    let width = state.width;
    let height = state.height;
    let wait_val = state.values[slot];
    let Some(d3d) = state.d3d.as_mut() else {
        return ptr::null_mut();
    };
    if dev_ptr.is_null() || ctx_ptr.is_null() || d3d.tex_share[slot].is_invalid() {
        block_share("Qt is not using D3D11, so the shared texture cannot be sampled");
        return ptr::null_mut();
    }
    let Some(device) = (unsafe { ID3D11Device::from_raw_borrowed(&dev_ptr) }) else {
        return ptr::null_mut();
    };
    let Some(context) = (unsafe { ID3D11DeviceContext::from_raw_borrowed(&ctx_ptr) }) else {
        return ptr::null_mut();
    };

    let dev_key = dev_ptr as usize;
    if d3d.qt_d3d11_dev != dev_key {
        d3d.qt_d3d11 = [None, None];
        d3d.qt_fence11 = None;
        d3d.qt_d3d11_dev = dev_key;
    }

    if d3d.qt_d3d11[slot].is_none() {
        let Ok(device1) = device.cast::<ID3D11Device1>() else {
            block_share("Qt's D3D11 device cannot open a shared texture");
            return ptr::null_mut();
        };
        let Ok(tex) =
            (unsafe { device1.OpenSharedResource1::<_, ID3D11Texture2D>(d3d.tex_share[slot]) })
        else {
            block_share("opening the D3D11 shared texture failed");
            return ptr::null_mut();
        };
        d3d.qt_d3d11[slot] = Some(tex);
    }

    if d3d.fence_share.is_invalid() {
        return ptr::null_mut();
    }
    if d3d.qt_fence11.is_none() {
        let Ok(device5) = device.cast::<ID3D11Device5>() else {
            block_share("Qt's D3D11 device cannot wait on a shared fence");
            return ptr::null_mut();
        };
        let mut fence: Option<ID3D11Fence> = None;
        let res = unsafe { device5.OpenSharedFence(d3d.fence_share, &mut fence) };
        let Some(fence) = res.ok().and(fence) else {
            block_share("opening the D3D11 fence failed");
            return ptr::null_mut();
        };
        d3d.qt_fence11 = Some(fence);
    }
    let Ok(context4) = context.cast::<ID3D11DeviceContext4>() else {
        block_share("Qt's D3D11 context cannot wait on a fence");
        return ptr::null_mut();
    };
    if let Some(fence) = d3d.qt_fence11.as_ref() {
        let _ = unsafe { context4.Wait(fence, wait_val) };
    }

    HELD.store(slot_i, Ordering::Release);
    unsafe {
        if !out_w.is_null() {
            *out_w = width;
        }
        if !out_h.is_null() {
            *out_h = height;
        }
    }
    d3d.qt_d3d11[slot]
        .as_ref()
        .map_or(ptr::null_mut(), Interface::as_raw)
}

#[cfg(windows)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cs_vello_share_adopt_d3d12(
    dev_ptr: *mut c_void,
    queue_ptr: *mut c_void,
    out_w: *mut i32,
    out_h: *mut i32,
) -> *mut c_void {
    let mut state = lock_state();
    if BLOCKED.load(Ordering::Acquire) {
        return ptr::null_mut();
    }
    let slot_i = PUBLISHED.load(Ordering::Acquire);
    if !(0..=1).contains(&slot_i) || state.width < 1 || state.height < 1 {
        return ptr::null_mut();
    }
    let slot = slot_i as usize;
    let width = state.width;
    let height = state.height;
    let wait_val = state.values[slot];
    let Some(d3d) = state.d3d.as_mut() else {
        return ptr::null_mut();
    };
    if dev_ptr.is_null()
        || queue_ptr.is_null()
        || d3d.tex_share[slot].is_invalid()
        || d3d.fence_share.is_invalid()
    {
        block_share("Qt's D3D12 device did not expose a queue for the shared texture");
        return ptr::null_mut();
    }
    let Some(device) = (unsafe { ID3D12Device::from_raw_borrowed(&dev_ptr) }) else {
        return ptr::null_mut();
    };
    let Some(queue) = (unsafe { ID3D12CommandQueue::from_raw_borrowed(&queue_ptr) }) else {
        return ptr::null_mut();
    };

    let dev_key = dev_ptr as usize;
    if d3d.qt_d3d12_dev != dev_key {
        d3d.qt_d3d12 = [None, None];
        d3d.qt_fence12 = None;
        d3d.qt_d3d12_dev = dev_key;
    }

    if d3d.qt_fence12.is_none() {
        let mut fence: Option<ID3D12Fence> = None;
        let res = unsafe { device.OpenSharedHandle(d3d.fence_share, &mut fence) };
        let Some(fence) = res.ok().and(fence) else {
            block_share("opening the D3D12 fence failed");
            return ptr::null_mut();
        };
        d3d.qt_fence12 = Some(fence);
    }
    if d3d.qt_d3d12[slot].is_none() {
        let mut tex: Option<ID3D12Resource> = None;
        let res = unsafe { device.OpenSharedHandle(d3d.tex_share[slot], &mut tex) };
        let Some(tex) = res.ok().and(tex) else {
            block_share("opening the D3D12 shared texture failed");
            return ptr::null_mut();
        };
        d3d.qt_d3d12[slot] = Some(tex);
    }
    if let Some(fence) = d3d.qt_fence12.as_ref() {
        let _ = unsafe { queue.Wait(fence, wait_val) };
    }

    HELD.store(slot_i, Ordering::Release);
    unsafe {
        if !out_w.is_null() {
            *out_w = width;
        }
        if !out_h.is_null() {
            *out_h = height;
        }
    }
    d3d.qt_d3d12[slot]
        .as_ref()
        .map_or(ptr::null_mut(), Interface::as_raw)
}

#[cfg(target_os = "linux")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cs_vello_share_adopt_vulkan(
    dev_bits: u64,
    queue_bits: u64,
    out_w: *mut i32,
    out_h: *mut i32,
) -> u64 {
    let mut state = lock_state();
    if BLOCKED.load(Ordering::Acquire) {
        return 0;
    }
    let slot_i = PUBLISHED.load(Ordering::Acquire);
    if !(0..=1).contains(&slot_i) || state.width < 1 || state.height < 1 {
        return 0;
    }
    let slot = slot_i as usize;
    let width = state.width;
    let height = state.height;
    let wait_val = state.values[slot];
    let Some(vk_state) = state.vk.as_mut() else {
        return 0;
    };
    if dev_bits == 0 {
        block_share("Qt is not using Vulkan, so the shared image cannot be sampled that way");
        return 0;
    }

    if vk_state.qt_dev_bits != dev_bits {
        vk_state.release_qt();
        let qt_raw = vk::Device::from_raw(dev_bits);
        let qt_dev = unsafe { ash::Device::load(vk_state.instance.fp_v1_0(), qt_raw) };
        vk_state.qt_device = Some(qt_dev);
        vk_state.qt_dev_bits = dev_bits;
    }

    if vk_state.qt_images[slot] == vk::Image::null() {
        let Some(fd) = vk_state.export_fd(slot) else {
            block_share("importing the Vulkan image into Qt failed");
            return 0;
        };
        let Some(qt_dev) = vk_state.qt_device.as_ref() else {
            return 0;
        };
        let mut ext = vk::ExternalMemoryImageCreateInfo::default()
            .handle_types(vk::ExternalMemoryHandleTypeFlags::OPAQUE_FD);
        let info = vk::ImageCreateInfo::default()
            .push_next(&mut ext)
            .image_type(vk::ImageType::TYPE_2D)
            .format(vk::Format::R8G8B8A8_UNORM)
            .extent(vk::Extent3D {
                width: width as u32,
                height: height as u32,
                depth: 1,
            })
            .mip_levels(1)
            .array_layers(1)
            .samples(vk::SampleCountFlags::TYPE_1)
            .tiling(vk::ImageTiling::OPTIMAL)
            .usage(vk::ImageUsageFlags::SAMPLED)
            .initial_layout(vk::ImageLayout::UNDEFINED);
        let Ok(image) = (unsafe { qt_dev.create_image(&info, None) }) else {
            block_share("importing the Vulkan image into Qt failed");
            return 0;
        };
        let req = unsafe { qt_dev.get_image_memory_requirements(image) };
        let mem_type = (0u32..32)
            .find(|&i| (req.memory_type_bits & (1u32 << i)) != 0)
            .unwrap_or(0);
        let mut imp = vk::ImportMemoryFdInfoKHR::default()
            .handle_type(vk::ExternalMemoryHandleTypeFlags::OPAQUE_FD)
            .fd(fd);
        let mut dedicated = vk::MemoryDedicatedAllocateInfo::default().image(image);
        let alloc = vk::MemoryAllocateInfo::default()
            .push_next(&mut imp)
            .push_next(&mut dedicated)
            .allocation_size(req.size.max(vk_state.bytes[slot]))
            .memory_type_index(mem_type);
        let Ok(memory) = (unsafe { qt_dev.allocate_memory(&alloc, None) }) else {
            unsafe { qt_dev.destroy_image(image, None) };
            block_share("importing the Vulkan image into Qt failed");
            return 0;
        };
        if unsafe { qt_dev.bind_image_memory(image, memory, 0) }.is_err() {
            unsafe {
                qt_dev.free_memory(memory, None);
                qt_dev.destroy_image(image, None);
            }
            block_share("importing the Vulkan image into Qt failed");
            return 0;
        }
        vk_state.qt_images[slot] = image;
        vk_state.qt_memory[slot] = memory;
    }

    if queue_bits != 0 && vk_state.sem != vk::Semaphore::null() {
        if vk_state.qt_sem == vk::Semaphore::null()
            && vk_state.has_sem_fd
            && let Some(qt_dev) = vk_state.qt_device.as_ref()
        {
            let has_import = unsafe {
                vk_state
                    .instance
                    .get_device_proc_addr(qt_dev.handle(), c"vkImportSemaphoreFdKHR".as_ptr())
                    .is_some()
            };
            if has_import {
                let get_info = vk::SemaphoreGetFdInfoKHR::default()
                    .semaphore(vk_state.sem)
                    .handle_type(vk::ExternalSemaphoreHandleTypeFlags::OPAQUE_FD);
                if let Ok(fd) = unsafe { vk_state.ext_sem.get_semaphore_fd(&get_info) } {
                    let mut timeline = vk::SemaphoreTypeCreateInfo::default()
                        .semaphore_type(vk::SemaphoreType::TIMELINE);
                    let sem_info = vk::SemaphoreCreateInfo::default().push_next(&mut timeline);
                    if let Ok(sem) = unsafe { qt_dev.create_semaphore(&sem_info, None) } {
                        let qt_ext_sem =
                            external_semaphore_fd::Device::new(&vk_state.instance, qt_dev);
                        let imp = vk::ImportSemaphoreFdInfoKHR::default()
                            .semaphore(sem)
                            .handle_type(vk::ExternalSemaphoreHandleTypeFlags::OPAQUE_FD)
                            .fd(fd);
                        if unsafe { qt_ext_sem.import_semaphore_fd(&imp) }.is_ok() {
                            vk_state.qt_sem = sem;
                        } else {
                            unsafe { qt_dev.destroy_semaphore(sem, None) };
                        }
                    }
                }
            }
        }
        if vk_state.qt_sem != vk::Semaphore::null()
            && let Some(qt_dev) = vk_state.qt_device.as_ref()
        {
            let values = [wait_val];
            let sems = [vk_state.qt_sem];
            let stages = [vk::PipelineStageFlags::FRAGMENT_SHADER];
            let mut timeline =
                vk::TimelineSemaphoreSubmitInfo::default().wait_semaphore_values(&values);
            let submit = vk::SubmitInfo::default()
                .push_next(&mut timeline)
                .wait_semaphores(&sems)
                .wait_dst_stage_mask(&stages);
            let queue = vk::Queue::from_raw(queue_bits);
            let _ = unsafe { qt_dev.queue_submit(queue, &[submit], vk::Fence::null()) };
        }
    }

    HELD.store(slot_i, Ordering::Release);
    unsafe {
        if !out_w.is_null() {
            *out_w = width;
        }
        if !out_h.is_null() {
            *out_h = height;
        }
    }
    vk_state.qt_images[slot].as_raw()
}

#[cfg(target_os = "linux")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cs_vello_share_gl_begin(
    out_slot: *mut i32,
    out_epoch: *mut i32,
    out_w: *mut i32,
    out_h: *mut i32,
    out_bytes: *mut u64,
) -> i32 {
    let state = lock_state();
    if BLOCKED.load(Ordering::Acquire) {
        return 0;
    }
    let slot_i = PUBLISHED.load(Ordering::Acquire);
    if !(0..=1).contains(&slot_i) || state.width < 1 || state.height < 1 {
        return 0;
    }
    let Some(vk_state) = state.vk.as_ref() else {
        return 0;
    };
    unsafe {
        if !out_slot.is_null() {
            *out_slot = slot_i;
        }
        if !out_epoch.is_null() {
            *out_epoch = state.epoch;
        }
        if !out_w.is_null() {
            *out_w = state.width;
        }
        if !out_h.is_null() {
            *out_h = state.height;
        }
        if !out_bytes.is_null() {
            *out_bytes = vk_state.bytes[slot_i as usize];
        }
    }
    1
}

#[cfg(target_os = "linux")]
#[unsafe(no_mangle)]
pub extern "C" fn cs_vello_share_gl_export_fd(slot: i32) -> i32 {
    if !(0..=1).contains(&slot) {
        return -1;
    }
    let state = lock_state();
    let Some(vk_state) = state.vk.as_ref() else {
        return -1;
    };
    vk_state.export_fd(slot as usize).unwrap_or(-1)
}

#[cfg(target_os = "linux")]
#[unsafe(no_mangle)]
pub extern "C" fn cs_vello_share_gl_hold(slot: i32) {
    if (0..=1).contains(&slot) {
        HELD.store(slot, Ordering::Release);
    }
}
