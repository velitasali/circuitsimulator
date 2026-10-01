// Shared frame for Qt on Windows and Linux.
// Vello renders into a texture created on wgpu's device. Qt samples that same
// memory on its own device. A fence (Windows) or a waited-out Vulkan queue
// (Linux) publishes the frame. If this process cannot open the share, adopt()
// fails and the canvas copies the image instead.

#include "vello_share.h"

#include <QQuickWindow>
#include <QSGRendererInterface>
#include <QSize>
#include <rhi/qrhi.h>
#include <qsgtexture_platform.h>

#include <atomic>
#include <cstdint>
#include <cstdio>
#include <mutex>

#if defined(_WIN32)
#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif
#ifndef NOMINMAX
#define NOMINMAX
#endif
#include <d3d11_4.h>
#include <d3d12.h>
#endif

#if defined(__linux__)
#include <vulkan/vulkan.h>
#include <QOpenGLContext>
#endif

static constexpr int kD3D12 = 1;
static constexpr int kVulkan = 2;

static std::mutex g_mu;
static int g_kind = 0;
static uint64_t g_device_bits = 0;
static uint64_t g_queue_bits = 0;
static uint64_t g_instance_bits = 0;
static uint64_t g_physical_bits = 0;
static unsigned g_family = 0;
static int g_width = 0;
static int g_height = 0;
static int g_epoch = 0;
static std::atomic<int> g_held{-1};
static std::atomic<int> g_published{-1};
static std::atomic<uint64_t> g_frame{0};
static std::atomic<int> g_blocked{0};
static uint64_t g_value[2] = {0, 0};
static bool g_logged = false;

#if defined(_WIN32)
static ID3D12Device *g_d3d_device = nullptr;
static ID3D12CommandQueue *g_d3d_queue = nullptr;
static ID3D12Resource *g_d3d_tex[2] = {nullptr, nullptr};
static HANDLE g_d3d_share[2] = {nullptr, nullptr};
static ID3D12Fence *g_d3d_fence = nullptr;
static HANDLE g_d3d_fence_share = nullptr;
static ID3D11Texture2D *g_qt_d3d11[2] = {nullptr, nullptr};
static ID3D11Fence *g_qt_fence11 = nullptr;
static ID3D11Device *g_qt_d3d11_device = nullptr;
static ID3D12Resource *g_qt_d3d12[2] = {nullptr, nullptr};
static ID3D12Fence *g_qt_fence12 = nullptr;
static ID3D12Device *g_qt_d3d12_device = nullptr;
#endif

#if defined(__linux__)
static VkDevice g_vk_device = VK_NULL_HANDLE;
static VkQueue g_vk_queue = VK_NULL_HANDLE;
static VkPhysicalDevice g_vk_physical = VK_NULL_HANDLE;
static VkImage g_vk_image[2] = {VK_NULL_HANDLE, VK_NULL_HANDLE};
static VkDeviceMemory g_vk_memory[2] = {VK_NULL_HANDLE, VK_NULL_HANDLE};
static VkDeviceSize g_vk_bytes[2] = {0, 0};
static VkSemaphore g_vk_sem = VK_NULL_HANDLE;
static VkDevice g_qt_vk = VK_NULL_HANDLE;
static VkImage g_qt_vk_image[2] = {VK_NULL_HANDLE, VK_NULL_HANDLE};
static VkDeviceMemory g_qt_vk_memory[2] = {VK_NULL_HANDLE, VK_NULL_HANDLE};
static VkSemaphore g_qt_vk_sem = VK_NULL_HANDLE;
static GLuint g_gl_mem[2] = {0, 0};
static GLuint g_gl_tex[2] = {0, 0};
static int g_gl_epoch[2] = {-1, -1};

static PFN_vkGetMemoryFdKHR g_get_fd = nullptr;
static PFN_vkCreateImage g_create_image = nullptr;
static PFN_vkDestroyImage g_destroy_image = nullptr;
static PFN_vkAllocateMemory g_alloc_mem = nullptr;
static PFN_vkFreeMemory g_free_mem = nullptr;
static PFN_vkBindImageMemory g_bind_mem = nullptr;
static PFN_vkGetImageMemoryRequirements g_mem_req = nullptr;
static PFN_vkCreateSemaphore g_create_sem = nullptr;
static PFN_vkDestroySemaphore g_destroy_sem = nullptr;
static PFN_vkQueueSubmit g_queue_submit = nullptr;
static PFN_vkQueueWaitIdle g_queue_wait = nullptr;
#endif

static void log_fail(const char *msg) {
    if (g_logged) {
        return;
    }
    g_logged = true;
    std::fprintf(stderr, "[vello] %s\n", msg);
}

static void block_share(const char *msg) {
    log_fail(msg);
    g_blocked.store(1, std::memory_order_release);
}

#if defined(_WIN32)
static void release_d3d_qt() {
    for (int i = 0; i < 2; ++i) {
        if (g_qt_d3d11[i]) {
            g_qt_d3d11[i]->Release();
            g_qt_d3d11[i] = nullptr;
        }
        if (g_qt_d3d12[i]) {
            g_qt_d3d12[i]->Release();
            g_qt_d3d12[i] = nullptr;
        }
    }
    if (g_qt_fence11) {
        g_qt_fence11->Release();
        g_qt_fence11 = nullptr;
    }
    if (g_qt_fence12) {
        g_qt_fence12->Release();
        g_qt_fence12 = nullptr;
    }
    g_qt_d3d11_device = nullptr;
    g_qt_d3d12_device = nullptr;
}

static void release_d3d_gpu() {
    release_d3d_qt();
    for (int i = 0; i < 2; ++i) {
        if (g_d3d_share[i]) {
            CloseHandle(g_d3d_share[i]);
            g_d3d_share[i] = nullptr;
        }
        if (g_d3d_tex[i]) {
            g_d3d_tex[i]->Release();
            g_d3d_tex[i] = nullptr;
        }
    }
    if (g_d3d_fence_share) {
        CloseHandle(g_d3d_fence_share);
        g_d3d_fence_share = nullptr;
    }
    if (g_d3d_fence) {
        g_d3d_fence->Release();
        g_d3d_fence = nullptr;
    }
    if (g_d3d_queue) {
        g_d3d_queue->Release();
        g_d3d_queue = nullptr;
    }
    if (g_d3d_device) {
        g_d3d_device->Release();
        g_d3d_device = nullptr;
    }
}

static bool make_d3d_texture(ID3D12Device *device, int index, int width, int height) {
    D3D12_HEAP_PROPERTIES heap = {};
    heap.Type = D3D12_HEAP_TYPE_DEFAULT;
    D3D12_RESOURCE_DESC desc = {};
    desc.Dimension = D3D12_RESOURCE_DIMENSION_TEXTURE2D;
    desc.Width = static_cast<UINT64>(width);
    desc.Height = static_cast<UINT>(height);
    desc.DepthOrArraySize = 1;
    desc.MipLevels = 1;
    desc.Format = DXGI_FORMAT_R8G8B8A8_UNORM;
    desc.SampleDesc.Count = 1;
    desc.Flags = D3D12_RESOURCE_FLAG_ALLOW_UNORDERED_ACCESS | D3D12_RESOURCE_FLAG_ALLOW_SIMULTANEOUS_ACCESS;
    ID3D12Resource *tex = nullptr;
    HRESULT hr = device->CreateCommittedResource(
        &heap, D3D12_HEAP_FLAG_SHARED, &desc, D3D12_RESOURCE_STATE_COMMON, nullptr,
        IID_PPV_ARGS(&tex));
    if (FAILED(hr) || !tex) {
        log_fail("D3D12 shared texture failed");
        return false;
    }
    HANDLE share = nullptr;
    hr = device->CreateSharedHandle(tex, nullptr, GENERIC_ALL, nullptr, &share);
    if (FAILED(hr) || !share) {
        tex->Release();
        log_fail("D3D12 shared handle failed");
        return false;
    }
    g_d3d_tex[index] = tex;
    g_d3d_share[index] = share;
    return true;
}

static bool ensure_d3d(uint64_t device_bits, uint64_t queue_bits, int width, int height) {
    auto *device = reinterpret_cast<ID3D12Device *>(static_cast<uintptr_t>(device_bits));
    auto *queue = reinterpret_cast<ID3D12CommandQueue *>(static_cast<uintptr_t>(queue_bits));
    if (g_kind == kD3D12 && g_device_bits == device_bits && g_width == width && g_height == height &&
        g_d3d_tex[0] && g_d3d_tex[1] && g_d3d_fence) {
        return true;
    }
    release_d3d_gpu();
    device->AddRef();
    queue->AddRef();
    g_d3d_device = device;
    g_d3d_queue = queue;
    if (!make_d3d_texture(device, 0, width, height) || !make_d3d_texture(device, 1, width, height)) {
        release_d3d_gpu();
        return false;
    }
    HRESULT hr = device->CreateFence(0, D3D12_FENCE_FLAG_SHARED, IID_PPV_ARGS(&g_d3d_fence));
    if (FAILED(hr) || !g_d3d_fence) {
        release_d3d_gpu();
        log_fail("D3D12 shared fence failed");
        return false;
    }
    hr = device->CreateSharedHandle(g_d3d_fence, nullptr, GENERIC_ALL, nullptr, &g_d3d_fence_share);
    if (FAILED(hr) || !g_d3d_fence_share) {
        release_d3d_gpu();
        log_fail("D3D12 fence handle failed");
        return false;
    }
    g_kind = kD3D12;
    g_device_bits = device_bits;
    g_queue_bits = queue_bits;
    g_width = width;
    g_height = height;
    g_epoch += 1;
    g_published.store(-1, std::memory_order_relaxed);
    g_held.store(-1, std::memory_order_relaxed);
    g_value[0] = 0;
    g_value[1] = 0;
    return true;
}
#endif

#if defined(__linux__)
static void release_vk_qt() {
    if (g_qt_vk) {
        if (g_qt_vk_sem) {
            vkDestroySemaphore(g_qt_vk, g_qt_vk_sem, nullptr);
            g_qt_vk_sem = VK_NULL_HANDLE;
        }
        for (int i = 0; i < 2; ++i) {
            if (g_qt_vk_image[i]) {
                vkDestroyImage(g_qt_vk, g_qt_vk_image[i], nullptr);
                g_qt_vk_image[i] = VK_NULL_HANDLE;
            }
            if (g_qt_vk_memory[i]) {
                vkFreeMemory(g_qt_vk, g_qt_vk_memory[i], nullptr);
                g_qt_vk_memory[i] = VK_NULL_HANDLE;
            }
        }
    }
    g_qt_vk = VK_NULL_HANDLE;
    QOpenGLContext *gl = QOpenGLContext::currentContext();
    if (gl) {
        auto del_tex = reinterpret_cast<void (*)(GLsizei, const GLuint *)>(
            gl->getProcAddress("glDeleteTextures"));
        auto del_mem = reinterpret_cast<void (*)(GLsizei, const GLuint *)>(
            gl->getProcAddress("glDeleteMemoryObjectsEXT"));
        if (del_tex) {
            GLuint names[2] = {g_gl_tex[0], g_gl_tex[1]};
            del_tex(2, names);
        }
        if (del_mem) {
            GLuint names[2] = {g_gl_mem[0], g_gl_mem[1]};
            del_mem(2, names);
        }
    }
    g_gl_tex[0] = g_gl_tex[1] = 0;
    g_gl_mem[0] = g_gl_mem[1] = 0;
    g_gl_epoch[0] = g_gl_epoch[1] = -1;
}

static void release_vk_gpu() {
    release_vk_qt();
    if (g_vk_device) {
        if (g_vk_sem && g_destroy_sem) {
            g_destroy_sem(g_vk_device, g_vk_sem, nullptr);
        }
        for (int i = 0; i < 2; ++i) {
            if (g_vk_image[i] && g_destroy_image) {
                g_destroy_image(g_vk_device, g_vk_image[i], nullptr);
            }
            if (g_vk_memory[i] && g_free_mem) {
                g_free_mem(g_vk_device, g_vk_memory[i], nullptr);
            }
            g_vk_image[i] = VK_NULL_HANDLE;
            g_vk_memory[i] = VK_NULL_HANDLE;
            g_vk_bytes[i] = 0;
        }
    }
    g_vk_sem = VK_NULL_HANDLE;
    g_vk_device = VK_NULL_HANDLE;
    g_vk_queue = VK_NULL_HANDLE;
    g_vk_physical = VK_NULL_HANDLE;
}

static bool load_vk(VkDevice device) {
    g_get_fd = reinterpret_cast<PFN_vkGetMemoryFdKHR>(vkGetDeviceProcAddr(device, "vkGetMemoryFdKHR"));
    g_create_image = reinterpret_cast<PFN_vkCreateImage>(vkGetDeviceProcAddr(device, "vkCreateImage"));
    g_destroy_image = reinterpret_cast<PFN_vkDestroyImage>(vkGetDeviceProcAddr(device, "vkDestroyImage"));
    g_alloc_mem = reinterpret_cast<PFN_vkAllocateMemory>(vkGetDeviceProcAddr(device, "vkAllocateMemory"));
    g_free_mem = reinterpret_cast<PFN_vkFreeMemory>(vkGetDeviceProcAddr(device, "vkFreeMemory"));
    g_bind_mem = reinterpret_cast<PFN_vkBindImageMemory>(vkGetDeviceProcAddr(device, "vkBindImageMemory"));
    g_mem_req = reinterpret_cast<PFN_vkGetImageMemoryRequirements>(
        vkGetDeviceProcAddr(device, "vkGetImageMemoryRequirements"));
    g_create_sem = reinterpret_cast<PFN_vkCreateSemaphore>(vkGetDeviceProcAddr(device, "vkCreateSemaphore"));
    g_destroy_sem = reinterpret_cast<PFN_vkDestroySemaphore>(vkGetDeviceProcAddr(device, "vkDestroySemaphore"));
    g_queue_submit = reinterpret_cast<PFN_vkQueueSubmit>(vkGetDeviceProcAddr(device, "vkQueueSubmit"));
    g_queue_wait = reinterpret_cast<PFN_vkQueueWaitIdle>(vkGetDeviceProcAddr(device, "vkQueueWaitIdle"));
    if (!g_get_fd || !g_create_image || !g_alloc_mem || !g_bind_mem || !g_mem_req || !g_create_sem ||
        !g_queue_submit || !g_queue_wait) {
        log_fail("Vulkan external-memory entry points are missing");
        return false;
    }
    return true;
}

static uint32_t memory_type(VkPhysicalDevice physical, uint32_t bits) {
    VkPhysicalDeviceMemoryProperties props = {};
    vkGetPhysicalDeviceMemoryProperties(physical, &props);
    for (uint32_t i = 0; i < props.memoryTypeCount; ++i) {
        if ((bits & (1u << i)) &&
            (props.memoryTypes[i].propertyFlags & VK_MEMORY_PROPERTY_DEVICE_LOCAL_BIT)) {
            return i;
        }
    }
    for (uint32_t i = 0; i < props.memoryTypeCount; ++i) {
        if (bits & (1u << i)) {
            return i;
        }
    }
    return UINT32_MAX;
}

static bool make_vk_image(VkDevice device, VkPhysicalDevice physical, int index, int width, int height) {
    VkExternalMemoryImageCreateInfo ext = {};
    ext.sType = VK_STRUCTURE_TYPE_EXTERNAL_MEMORY_IMAGE_CREATE_INFO;
    ext.handleTypes = VK_EXTERNAL_MEMORY_HANDLE_TYPE_OPAQUE_FD_BIT;
    VkImageCreateInfo info = {};
    info.sType = VK_STRUCTURE_TYPE_IMAGE_CREATE_INFO;
    info.pNext = &ext;
    info.imageType = VK_IMAGE_TYPE_2D;
    info.format = VK_FORMAT_R8G8B8A8_UNORM;
    info.extent = {static_cast<uint32_t>(width), static_cast<uint32_t>(height), 1};
    info.mipLevels = 1;
    info.arrayLayers = 1;
    info.samples = VK_SAMPLE_COUNT_1_BIT;
    info.tiling = VK_IMAGE_TILING_OPTIMAL;
    info.usage = VK_IMAGE_USAGE_STORAGE_BIT | VK_IMAGE_USAGE_SAMPLED_BIT | VK_IMAGE_USAGE_TRANSFER_SRC_BIT |
                 VK_IMAGE_USAGE_TRANSFER_DST_BIT;
    info.initialLayout = VK_IMAGE_LAYOUT_UNDEFINED;
    VkImage image = VK_NULL_HANDLE;
    if (g_create_image(device, &info, nullptr, &image) != VK_SUCCESS) {
        log_fail("Vulkan shared image failed");
        return false;
    }
    VkMemoryRequirements req = {};
    g_mem_req(device, image, &req);
    const uint32_t type = memory_type(physical, req.memoryTypeBits);
    if (type == UINT32_MAX) {
        g_destroy_image(device, image, nullptr);
        log_fail("Vulkan shared image has no memory type");
        return false;
    }
    VkExportMemoryAllocateInfo exp = {};
    exp.sType = VK_STRUCTURE_TYPE_EXPORT_MEMORY_ALLOCATE_INFO;
    exp.handleTypes = VK_EXTERNAL_MEMORY_HANDLE_TYPE_OPAQUE_FD_BIT;
    VkMemoryDedicatedAllocateInfo dedicated = {};
    dedicated.sType = VK_STRUCTURE_TYPE_MEMORY_DEDICATED_ALLOCATE_INFO;
    dedicated.pNext = &exp;
    dedicated.image = image;
    VkMemoryAllocateInfo alloc = {};
    alloc.sType = VK_STRUCTURE_TYPE_MEMORY_ALLOCATE_INFO;
    alloc.pNext = &dedicated;
    alloc.allocationSize = req.size;
    alloc.memoryTypeIndex = type;
    VkDeviceMemory memory = VK_NULL_HANDLE;
    if (g_alloc_mem(device, &alloc, nullptr, &memory) != VK_SUCCESS) {
        g_destroy_image(device, image, nullptr);
        log_fail("Vulkan shared memory failed");
        return false;
    }
    if (g_bind_mem(device, image, memory, 0) != VK_SUCCESS) {
        g_free_mem(device, memory, nullptr);
        g_destroy_image(device, image, nullptr);
        log_fail("Vulkan bind shared image failed");
        return false;
    }
    g_vk_image[index] = image;
    g_vk_memory[index] = memory;
    g_vk_bytes[index] = req.size;
    return true;
}

static bool ensure_vk(uint64_t device_bits, uint64_t queue_bits, uint64_t physical_bits, int width,
                      int height) {
    auto device = reinterpret_cast<VkDevice>(static_cast<uintptr_t>(device_bits));
    auto queue = reinterpret_cast<VkQueue>(static_cast<uintptr_t>(queue_bits));
    auto physical = reinterpret_cast<VkPhysicalDevice>(static_cast<uintptr_t>(physical_bits));
    if (g_kind == kVulkan && g_device_bits == device_bits && g_width == width && g_height == height &&
        g_vk_image[0] && g_vk_image[1] && g_vk_sem) {
        return true;
    }
    release_vk_gpu();
    if (!load_vk(device)) {
        return false;
    }
    if (!make_vk_image(device, physical, 0, width, height) ||
        !make_vk_image(device, physical, 1, width, height)) {
        release_vk_gpu();
        return false;
    }
    VkExportSemaphoreCreateInfo exp = {};
    exp.sType = VK_STRUCTURE_TYPE_EXPORT_SEMAPHORE_CREATE_INFO;
    exp.handleTypes = VK_EXTERNAL_SEMAPHORE_HANDLE_TYPE_OPAQUE_FD_BIT;
    VkSemaphoreTypeCreateInfo timeline = {};
    timeline.sType = VK_STRUCTURE_TYPE_SEMAPHORE_TYPE_CREATE_INFO;
    timeline.pNext = &exp;
    timeline.semaphoreType = VK_SEMAPHORE_TYPE_TIMELINE;
    VkSemaphoreCreateInfo sem_info = {};
    sem_info.sType = VK_STRUCTURE_TYPE_SEMAPHORE_CREATE_INFO;
    sem_info.pNext = &timeline;
    if (g_create_sem(device, &sem_info, nullptr, &g_vk_sem) != VK_SUCCESS) {
        release_vk_gpu();
        log_fail("Vulkan timeline semaphore failed");
        return false;
    }
    g_vk_device = device;
    g_vk_queue = queue;
    g_vk_physical = physical;
    g_kind = kVulkan;
    g_device_bits = device_bits;
    g_queue_bits = queue_bits;
    g_physical_bits = physical_bits;
    g_width = width;
    g_height = height;
    g_epoch += 1;
    g_published.store(-1, std::memory_order_relaxed);
    g_held.store(-1, std::memory_order_relaxed);
    g_value[0] = 0;
    g_value[1] = 0;
    return true;
}

static int export_fd(VkDeviceMemory memory) {
    VkMemoryGetFdInfoKHR info = {};
    info.sType = VK_STRUCTURE_TYPE_MEMORY_GET_FD_INFO_KHR;
    info.memory = memory;
    info.handleType = VK_EXTERNAL_MEMORY_HANDLE_TYPE_OPAQUE_FD_BIT;
    int fd = -1;
    if (!g_get_fd || g_get_fd(g_vk_device, &info, &fd) != VK_SUCCESS) {
        return -1;
    }
    return fd;
}
#endif

static bool same_target(int kind, uint64_t device, int width, int height) {
    return g_kind == kind && g_device_bits == device && g_width == width && g_height == height &&
           width > 0 && height > 0;
}

static void release_all() {
#if defined(_WIN32)
    release_d3d_gpu();
#endif
#if defined(__linux__)
    release_vk_gpu();
#endif
    g_kind = 0;
    g_device_bits = 0;
    g_queue_bits = 0;
    g_width = 0;
    g_height = 0;
    g_published.store(-1, std::memory_order_relaxed);
    g_held.store(-1, std::memory_order_relaxed);
}

extern "C" int cs_vello_share_reset_needed(int kind, uint64_t device, int width, int height) {
    std::lock_guard<std::mutex> lock(g_mu);
    if (g_blocked.load(std::memory_order_acquire)) {
        return 1;
    }
    return same_target(kind, device, width, height) ? 0 : 1;
}

extern "C" int cs_vello_share_begin(int kind, uint64_t device, uint64_t queue, uint64_t instance,
                                    uint64_t physical, unsigned family, int width, int height,
                                    int *out_slot, int *out_epoch) {
    if (width < 1 || height < 1 || !device || !queue) {
        return 0;
    }
    std::lock_guard<std::mutex> lock(g_mu);
    if (g_blocked.load(std::memory_order_acquire)) {
        return 0;
    }
    g_instance_bits = instance;
    g_family = family;
    bool ok = false;
#if defined(_WIN32)
    if (kind == kD3D12) {
        ok = ensure_d3d(device, queue, width, height);
    }
#else
    (void)kind;
#endif
#if defined(__linux__)
    if (kind == kVulkan) {
        ok = ensure_vk(device, queue, physical, width, height);
    }
#else
    (void)physical;
#endif
    if (!ok) {
        return 0;
    }
    const int held = g_held.load(std::memory_order_acquire);
    if (out_slot) {
        *out_slot = held == 0 ? 1 : 0;
    }
    if (out_epoch) {
        *out_epoch = g_epoch;
    }
    return 1;
}

extern "C" uint64_t cs_vello_share_resource(int slot) {
    if (slot < 0 || slot > 1) {
        return 0;
    }
    std::lock_guard<std::mutex> lock(g_mu);
#if defined(_WIN32)
    if (g_d3d_tex[slot]) {
        g_d3d_tex[slot]->AddRef();
        return static_cast<uint64_t>(reinterpret_cast<uintptr_t>(g_d3d_tex[slot]));
    }
#endif
#if defined(__linux__)
    if (g_vk_image[slot]) {
        return static_cast<uint64_t>(reinterpret_cast<uintptr_t>(g_vk_image[slot]));
    }
#endif
    return 0;
}

static void signal_frame(int slot) {
#if defined(_WIN32)
    if (g_d3d_queue && g_d3d_fence) {
        const uint64_t value = g_value[slot] + 1;
        if (SUCCEEDED(g_d3d_queue->Signal(g_d3d_fence, value))) {
            g_value[slot] = value;
        }
    }
#endif
#if defined(__linux__)
    if (g_vk_queue && g_vk_sem && g_queue_submit && g_queue_wait) {
        const uint64_t value = g_value[slot] + 1;
        VkTimelineSemaphoreSubmitInfo timeline = {};
        timeline.sType = VK_STRUCTURE_TYPE_TIMELINE_SEMAPHORE_SUBMIT_INFO;
        timeline.signalSemaphoreValueCount = 1;
        timeline.pSignalSemaphoreValues = &value;
        VkSubmitInfo submit = {};
        submit.sType = VK_STRUCTURE_TYPE_SUBMIT_INFO;
        submit.pNext = &timeline;
        submit.signalSemaphoreCount = 1;
        submit.pSignalSemaphores = &g_vk_sem;
        if (g_queue_submit(g_vk_queue, 1, &submit, VK_NULL_HANDLE) == VK_SUCCESS) {
            g_queue_wait(g_vk_queue);
            g_value[slot] = value;
        }
    }
#endif
    (void)slot;
}

extern "C" void cs_vello_share_commit(int slot) {
    if (slot < 0 || slot > 1) {
        return;
    }
    signal_frame(slot);
    std::lock_guard<std::mutex> lock(g_mu);
    g_published.store(slot, std::memory_order_relaxed);
    g_frame.fetch_add(1, std::memory_order_release);
}

extern "C" int cs_vello_share_has_base(int width, int height) {
    std::lock_guard<std::mutex> lock(g_mu);
    const int published = g_published.load(std::memory_order_acquire);
    if (g_blocked.load(std::memory_order_acquire) || published < 0 || g_width != width ||
        g_height != height) {
        return 0;
    }
    return 1;
}

extern "C" int cs_vello_share_patch_dest(int *out_dst, int *out_src, int *out_copy_full) {
    std::lock_guard<std::mutex> lock(g_mu);
    const int src = g_published.load(std::memory_order_acquire);
    if (src < 0 || src > 1) {
        return 0;
    }
    const int held = g_held.load(std::memory_order_acquire);
    const int dst = held == 0 ? 1 : 0;
    if (out_dst) {
        *out_dst = dst;
    }
    if (out_src) {
        *out_src = src;
    }
    if (out_copy_full) {
        *out_copy_full = dst == src ? 0 : 1;
    }
    return 1;
}

extern "C" int cs_vello_share_blocked(void) {
    return g_blocked.load(std::memory_order_acquire);
}

extern "C" void cs_vello_share_block(void) {
    g_blocked.store(1, std::memory_order_release);
}

extern "C" uint64_t cs_vello_share_generation(void) {
    if (g_blocked.load(std::memory_order_acquire)) {
        return 0;
    }
    return g_frame.load(std::memory_order_acquire);
}

extern "C" void cs_vello_share_release(void) {
    std::lock_guard<std::mutex> lock(g_mu);
    release_all();
    g_frame.store(0, std::memory_order_relaxed);
}

#if defined(_WIN32)
static bool wait_d3d11(ID3D11Device *device, ID3D11DeviceContext *context, int slot) {
    if (!g_d3d_fence_share) {
        return false;
    }
    if (g_qt_d3d11_device != device) {
        if (g_qt_fence11) {
            g_qt_fence11->Release();
            g_qt_fence11 = nullptr;
        }
        g_qt_d3d11_device = device;
    }
    if (!g_qt_fence11) {
        ID3D11Device5 *device5 = nullptr;
        if (FAILED(device->QueryInterface(IID_PPV_ARGS(&device5))) || !device5) {
            block_share("Qt's D3D11 device cannot wait on a shared fence");
            return false;
        }
        const HRESULT hr = device5->OpenSharedFence(g_d3d_fence_share, IID_PPV_ARGS(&g_qt_fence11));
        device5->Release();
        if (FAILED(hr) || !g_qt_fence11) {
            block_share("opening the D3D11 fence failed");
            return false;
        }
    }
    ID3D11DeviceContext4 *context4 = nullptr;
    if (FAILED(context->QueryInterface(IID_PPV_ARGS(&context4))) || !context4) {
        block_share("Qt's D3D11 context cannot wait on a fence");
        return false;
    }
    context4->Wait(g_qt_fence11, g_value[slot]);
    context4->Release();
    return true;
}

static void *adopt_d3d11(QQuickWindow *window, int slot, int *out_w, int *out_h) {
    QRhi *rhi = window->rhi();
    const auto *handles =
        rhi ? static_cast<const QRhiD3D11NativeHandles *>(rhi->nativeHandles()) : nullptr;
    auto *device = handles ? static_cast<ID3D11Device *>(handles->dev) : nullptr;
    auto *context = handles ? static_cast<ID3D11DeviceContext *>(handles->context) : nullptr;
    if (!device || !context || !g_d3d_share[slot]) {
        block_share("Qt is not using D3D11, so the shared texture cannot be sampled");
        return nullptr;
    }
    if (g_qt_d3d11_device != device) {
        for (int i = 0; i < 2; ++i) {
            if (g_qt_d3d11[i]) {
                g_qt_d3d11[i]->Release();
                g_qt_d3d11[i] = nullptr;
            }
        }
    }
    if (!g_qt_d3d11[slot]) {
        ID3D11Device1 *device1 = nullptr;
        if (FAILED(device->QueryInterface(IID_PPV_ARGS(&device1))) || !device1) {
            block_share("Qt's D3D11 device cannot open a shared texture");
            return nullptr;
        }
        const HRESULT hr =
            device1->OpenSharedResource1(g_d3d_share[slot], IID_PPV_ARGS(&g_qt_d3d11[slot]));
        device1->Release();
        if (FAILED(hr) || !g_qt_d3d11[slot]) {
            block_share("opening the D3D11 shared texture failed");
            return nullptr;
        }
    }
    if (!wait_d3d11(device, context, slot)) {
        return nullptr;
    }
    QSGTexture *texture = QNativeInterface::QSGD3D11Texture::fromNative(
        g_qt_d3d11[slot], window, QSize(g_width, g_height), QQuickWindow::TextureHasAlphaChannel);
    if (!texture) {
        block_share("wrapping the D3D11 texture failed");
        return nullptr;
    }
    g_held.store(slot, std::memory_order_release);
    if (out_w) {
        *out_w = g_width;
    }
    if (out_h) {
        *out_h = g_height;
    }
    return texture;
}

static void *adopt_d3d12(QQuickWindow *window, int slot, int *out_w, int *out_h) {
    QRhi *rhi = window->rhi();
    const auto *handles =
        rhi ? static_cast<const QRhiD3D12NativeHandles *>(rhi->nativeHandles()) : nullptr;
    auto *device = handles ? static_cast<ID3D12Device *>(handles->dev) : nullptr;
    auto *queue = handles ? static_cast<ID3D12CommandQueue *>(handles->commandQueue) : nullptr;
    if (!device || !queue || !g_d3d_share[slot] || !g_d3d_fence_share) {
        block_share("Qt's D3D12 device did not expose a queue for the shared texture");
        return nullptr;
    }
    if (g_qt_d3d12_device != device) {
        for (int i = 0; i < 2; ++i) {
            if (g_qt_d3d12[i]) {
                g_qt_d3d12[i]->Release();
                g_qt_d3d12[i] = nullptr;
            }
        }
        if (g_qt_fence12) {
            g_qt_fence12->Release();
            g_qt_fence12 = nullptr;
        }
        g_qt_d3d12_device = device;
    }
    if (!g_qt_fence12) {
        const HRESULT hr = device->OpenSharedHandle(g_d3d_fence_share, IID_PPV_ARGS(&g_qt_fence12));
        if (FAILED(hr) || !g_qt_fence12) {
            block_share("opening the D3D12 fence failed");
            return nullptr;
        }
    }
    if (!g_qt_d3d12[slot]) {
        const HRESULT hr = device->OpenSharedHandle(g_d3d_share[slot], IID_PPV_ARGS(&g_qt_d3d12[slot]));
        if (FAILED(hr) || !g_qt_d3d12[slot]) {
            block_share("opening the D3D12 shared texture failed");
            return nullptr;
        }
    }
    queue->Wait(g_qt_fence12, g_value[slot]);
    QSGTexture *texture = QNativeInterface::QSGD3D12Texture::fromNative(
        g_qt_d3d12[slot], static_cast<int>(D3D12_RESOURCE_STATE_COMMON), window,
        QSize(g_width, g_height), QQuickWindow::TextureHasAlphaChannel);
    if (!texture) {
        block_share("wrapping the D3D12 texture failed");
        return nullptr;
    }
    g_held.store(slot, std::memory_order_release);
    if (out_w) {
        *out_w = g_width;
    }
    if (out_h) {
        *out_h = g_height;
    }
    return texture;
}
#endif

#if defined(__linux__)
static bool import_qt_vulkan(VkDevice device, int slot) {
    if (g_qt_vk != device) {
        release_vk_qt();
        g_qt_vk = device;
    }
    if (g_qt_vk_image[slot]) {
        return true;
    }
    auto create_image = reinterpret_cast<PFN_vkCreateImage>(vkGetDeviceProcAddr(device, "vkCreateImage"));
    auto alloc_mem = reinterpret_cast<PFN_vkAllocateMemory>(vkGetDeviceProcAddr(device, "vkAllocateMemory"));
    auto bind_mem = reinterpret_cast<PFN_vkBindImageMemory>(vkGetDeviceProcAddr(device, "vkBindImageMemory"));
    auto get_req = reinterpret_cast<PFN_vkGetImageMemoryRequirements>(
        vkGetDeviceProcAddr(device, "vkGetImageMemoryRequirements"));
    auto import_fd = reinterpret_cast<PFN_vkGetMemoryFdKHR>(nullptr);
    (void)import_fd;
    auto import_mem = reinterpret_cast<PFN_vkAllocateMemory>(alloc_mem);
    if (!create_image || !import_mem || !bind_mem || !get_req) {
        return false;
    }
    const int fd = export_fd(g_vk_memory[slot]);
    if (fd < 0) {
        return false;
    }
    VkExternalMemoryImageCreateInfo ext = {};
    ext.sType = VK_STRUCTURE_TYPE_EXTERNAL_MEMORY_IMAGE_CREATE_INFO;
    ext.handleTypes = VK_EXTERNAL_MEMORY_HANDLE_TYPE_OPAQUE_FD_BIT;
    VkImageCreateInfo info = {};
    info.sType = VK_STRUCTURE_TYPE_IMAGE_CREATE_INFO;
    info.pNext = &ext;
    info.imageType = VK_IMAGE_TYPE_2D;
    info.format = VK_FORMAT_R8G8B8A8_UNORM;
    info.extent = {static_cast<uint32_t>(g_width), static_cast<uint32_t>(g_height), 1};
    info.mipLevels = 1;
    info.arrayLayers = 1;
    info.samples = VK_SAMPLE_COUNT_1_BIT;
    info.tiling = VK_IMAGE_TILING_OPTIMAL;
    info.usage = VK_IMAGE_USAGE_SAMPLED_BIT;
    info.initialLayout = VK_IMAGE_LAYOUT_UNDEFINED;
    VkImage image = VK_NULL_HANDLE;
    if (create_image(device, &info, nullptr, &image) != VK_SUCCESS) {
        return false;
    }
    VkMemoryRequirements req = {};
    get_req(device, image, &req);
    VkImportMemoryFdInfoKHR imp = {};
    imp.sType = VK_STRUCTURE_TYPE_IMPORT_MEMORY_FD_INFO_KHR;
    imp.handleType = VK_EXTERNAL_MEMORY_HANDLE_TYPE_OPAQUE_FD_BIT;
    imp.fd = fd;
    VkMemoryDedicatedAllocateInfo dedicated = {};
    dedicated.sType = VK_STRUCTURE_TYPE_MEMORY_DEDICATED_ALLOCATE_INFO;
    dedicated.pNext = &imp;
    dedicated.image = image;
    VkMemoryAllocateInfo alloc = {};
    alloc.sType = VK_STRUCTURE_TYPE_MEMORY_ALLOCATE_INFO;
    alloc.pNext = &dedicated;
    alloc.allocationSize = req.size > g_vk_bytes[slot] ? req.size : g_vk_bytes[slot];
    alloc.memoryTypeIndex = 0;
    VkPhysicalDevice qt_physical = VK_NULL_HANDLE;
    // The imported fd selects the memory. The type still has to be a device-local
    // type on Qt's device that covers this image.
    (void)qt_physical;
    VkDeviceMemory memory = VK_NULL_HANDLE;
    // Find a type from the requirements of this image. The fd import ignores the
    // heap choice when the driver matches the exported memory.
    uint32_t type = 0;
    for (; type < 32; ++type) {
        if (req.memoryTypeBits & (1u << type)) {
            break;
        }
    }
    alloc.memoryTypeIndex = type;
    if (alloc_mem(device, &alloc, nullptr, &memory) != VK_SUCCESS ||
        bind_mem(device, image, memory, 0) != VK_SUCCESS) {
        if (memory) {
            auto free_mem = reinterpret_cast<PFN_vkFreeMemory>(vkGetDeviceProcAddr(device, "vkFreeMemory"));
            if (free_mem) {
                free_mem(device, memory, nullptr);
            }
        }
        auto destroy = reinterpret_cast<PFN_vkDestroyImage>(vkGetDeviceProcAddr(device, "vkDestroyImage"));
        if (destroy) {
            destroy(device, image, nullptr);
        }
        return false;
    }
    g_qt_vk_image[slot] = image;
    g_qt_vk_memory[slot] = memory;
    return true;
}

static void wait_qt_vulkan(VkDevice device, VkQueue queue, int slot) {
    if (!queue || !g_vk_sem) {
        return;
    }
    if (!g_qt_vk_sem) {
        auto get_sem_fd = reinterpret_cast<PFN_vkGetSemaphoreFdKHR>(
            vkGetDeviceProcAddr(g_vk_device, "vkGetSemaphoreFdKHR"));
        auto create_sem = reinterpret_cast<PFN_vkCreateSemaphore>(vkGetDeviceProcAddr(device, "vkCreateSemaphore"));
        auto import_ok = get_sem_fd && create_sem;
        int fd = -1;
        if (import_ok) {
            VkSemaphoreGetFdInfoKHR info = {};
            info.sType = VK_STRUCTURE_TYPE_SEMAPHORE_GET_FD_INFO_KHR;
            info.semaphore = g_vk_sem;
            info.handleType = VK_EXTERNAL_SEMAPHORE_HANDLE_TYPE_OPAQUE_FD_BIT;
            import_ok = get_sem_fd(g_vk_device, &info, &fd) == VK_SUCCESS;
        }
        if (!import_ok) {
            return;
        }
        VkImportSemaphoreFdInfoKHR imp = {};
        imp.sType = VK_STRUCTURE_TYPE_IMPORT_SEMAPHORE_FD_INFO_KHR;
        imp.handleType = VK_EXTERNAL_SEMAPHORE_HANDLE_TYPE_OPAQUE_FD_BIT;
        imp.fd = fd;
        VkSemaphoreTypeCreateInfo timeline = {};
        timeline.sType = VK_STRUCTURE_TYPE_SEMAPHORE_TYPE_CREATE_INFO;
        timeline.pNext = &imp;
        timeline.semaphoreType = VK_SEMAPHORE_TYPE_TIMELINE;
        VkSemaphoreCreateInfo sem_info = {};
        sem_info.sType = VK_STRUCTURE_TYPE_SEMAPHORE_CREATE_INFO;
        sem_info.pNext = &timeline;
        if (create_sem(device, &sem_info, nullptr, &g_qt_vk_sem) != VK_SUCCESS) {
            g_qt_vk_sem = VK_NULL_HANDLE;
            return;
        }
    }
    auto submit = reinterpret_cast<PFN_vkQueueSubmit>(vkGetDeviceProcAddr(device, "vkQueueSubmit"));
    if (!submit) {
        return;
    }
    const uint64_t value = g_value[slot];
    VkTimelineSemaphoreSubmitInfo timeline = {};
    timeline.sType = VK_STRUCTURE_TYPE_TIMELINE_SEMAPHORE_SUBMIT_INFO;
    timeline.waitSemaphoreValueCount = 1;
    timeline.pWaitSemaphoreValues = &value;
    VkPipelineStageFlags stage = VK_PIPELINE_STAGE_FRAGMENT_SHADER_BIT;
    VkSubmitInfo info = {};
    info.sType = VK_STRUCTURE_TYPE_SUBMIT_INFO;
    info.pNext = &timeline;
    info.waitSemaphoreCount = 1;
    info.pWaitSemaphores = &g_qt_vk_sem;
    info.pWaitDstStageMask = &stage;
    submit(queue, 1, &info, VK_NULL_HANDLE);
}

static void *adopt_vulkan(QQuickWindow *window, int slot, int *out_w, int *out_h) {
    QRhi *rhi = window->rhi();
    const auto *handles =
        rhi ? static_cast<const QRhiVulkanNativeHandles *>(rhi->nativeHandles()) : nullptr;
    if (!handles || !handles->dev) {
        block_share("Qt is not using Vulkan, so the shared image cannot be sampled that way");
        return nullptr;
    }
    if (!import_qt_vulkan(handles->dev, slot)) {
        block_share("importing the Vulkan image into Qt failed");
        return nullptr;
    }
    wait_qt_vulkan(handles->dev, handles->gfxQueue, slot);
    QSGTexture *texture = QNativeInterface::QSGVulkanTexture::fromNative(
        g_qt_vk_image[slot], VK_IMAGE_LAYOUT_GENERAL, window, QSize(g_width, g_height),
        QQuickWindow::TextureHasAlphaChannel);
    if (!texture) {
        block_share("wrapping the Vulkan image failed");
        return nullptr;
    }
    g_held.store(slot, std::memory_order_release);
    if (out_w) {
        *out_w = g_width;
    }
    if (out_h) {
        *out_h = g_height;
    }
    return texture;
}

static void *adopt_gl(QQuickWindow *window, int slot, int *out_w, int *out_h) {
    QOpenGLContext *gl = QOpenGLContext::currentContext();
    if (!gl) {
        block_share("Qt's OpenGL context is not current");
        return nullptr;
    }
    if (g_gl_epoch[slot] != g_epoch || !g_gl_tex[slot]) {
        auto create_mem = reinterpret_cast<void (*)(GLsizei, GLuint *)>(
            gl->getProcAddress("glCreateMemoryObjectsEXT"));
        auto import_fd = reinterpret_cast<void (*)(GLuint, uint64_t, GLenum, int)>(
            gl->getProcAddress("glImportMemoryFdEXT"));
        auto storage = reinterpret_cast<void (*)(GLuint, GLsizei, GLenum, GLsizei, GLsizei, GLuint, uint64_t)>(
            gl->getProcAddress("glTextureStorageMem2DEXT"));
        auto create_tex = reinterpret_cast<void (*)(GLenum, GLsizei, GLuint *)>(
            gl->getProcAddress("glCreateTextures"));
        if (!create_mem || !import_fd || !storage || !create_tex) {
            block_share("OpenGL cannot import a Vulkan image on this driver");
            return nullptr;
        }
        const int fd = export_fd(g_vk_memory[slot]);
        if (fd < 0) {
            block_share("exporting the Vulkan image failed");
            return nullptr;
        }
        GLuint mem = 0;
        GLuint tex = 0;
        create_mem(1, &mem);
        import_fd(mem, static_cast<uint64_t>(g_vk_bytes[slot]), 0x9586 /* GL_HANDLE_TYPE_OPAQUE_FD_EXT */, fd);
        create_tex(0x0DE1 /* GL_TEXTURE_2D */, 1, &tex);
        storage(tex, 1, 0x8058 /* GL_RGBA8 */, g_width, g_height, mem, 0);
        g_gl_mem[slot] = mem;
        g_gl_tex[slot] = tex;
        g_gl_epoch[slot] = g_epoch;
    }
    auto barrier = reinterpret_cast<void (*)(GLbitfield)>(gl->getProcAddress("glMemoryBarrier"));
    if (barrier) {
        barrier(0xFFFFFFFF);
    }
    QSGTexture *texture = QNativeInterface::QSGOpenGLTexture::fromNative(
        g_gl_tex[slot], window, QSize(g_width, g_height), QQuickWindow::TextureHasAlphaChannel);
    if (!texture) {
        block_share("wrapping the OpenGL texture failed");
        return nullptr;
    }
    g_held.store(slot, std::memory_order_release);
    if (out_w) {
        *out_w = g_width;
    }
    if (out_h) {
        *out_h = g_height;
    }
    return texture;
}
#endif

extern "C" void *cs_vello_share_adopt(void *window_ptr, int *out_w, int *out_h) {
    auto *window = static_cast<QQuickWindow *>(window_ptr);
    if (!window) {
        return nullptr;
    }
    std::lock_guard<std::mutex> lock(g_mu);
    if (g_blocked.load(std::memory_order_acquire)) {
        return nullptr;
    }
    const int slot = g_published.load(std::memory_order_acquire);
    if (slot < 0 || slot > 1 || g_width < 1 || g_height < 1) {
        return nullptr;
    }
    QSGRendererInterface *ri = window->rendererInterface();
    if (!ri) {
        return nullptr;
    }
    switch (ri->graphicsApi()) {
#if defined(_WIN32)
    case QSGRendererInterface::Direct3D11:
        return adopt_d3d11(window, slot, out_w, out_h);
    case QSGRendererInterface::Direct3D12:
        return adopt_d3d12(window, slot, out_w, out_h);
#endif
#if defined(__linux__)
    case QSGRendererInterface::Vulkan:
        return adopt_vulkan(window, slot, out_w, out_h);
    case QSGRendererInterface::OpenGL:
        return adopt_gl(window, slot, out_w, out_h);
#endif
    default:
        block_share("this Qt graphics backend cannot sample the shared texture");
        return nullptr;
    }
}
