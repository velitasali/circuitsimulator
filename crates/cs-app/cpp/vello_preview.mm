#import <Metal/Metal.h>
#import <IOSurface/IOSurface.h>

#include <QQuickWindow>
#include <QSize>
#include <qsgtexture_platform.h>

#include <atomic>
#include <cstdio>
#include <mutex>

#include "vello_preview.h"

// Two IOSurface frames. Vello writes the one Qt is not sampling, on wgpu's
// device. Qt opens the published surface on its own device, so QML overlays
// composite above the circuit.

static const uint32_t kPixelRGBA = 0x52474241; // 'RGBA'

static std::mutex g_mu;
static id<MTLDevice> g_gpu_device = nil;
static id<MTLCommandQueue> g_queue = nil;
static id<MTLTexture> g_gpu_tex[2] = {nil, nil};
static id<MTLTexture> g_qt_tex[2] = {nil, nil};
static id<MTLDevice> g_qt_device = nil;
static IOSurfaceRef g_surf[2] = {nullptr, nullptr};
static int g_width = 0;
static int g_height = 0;
static int g_qt_epoch[2] = {-1, -1};
static std::atomic<int> g_epoch{0};
static std::atomic<int> g_held{-1};
static std::atomic<int> g_published{-1};
static std::atomic<uint64_t> g_frame{0};
static bool g_logged_fail = false;

static void log_fail(const char *msg) {
    if (g_logged_fail) {
        return;
    }
    g_logged_fail = true;
    std::fprintf(stderr, "[vello] %s\n", msg);
}

static IOSurfaceRef make_surface(int width, int height) {
    const size_t row = IOSurfaceAlignProperty(kIOSurfaceBytesPerRow, (size_t)width * 4);
    NSDictionary *props = @{
        (__bridge id)kIOSurfaceWidth : @(width),
        (__bridge id)kIOSurfaceHeight : @(height),
        (__bridge id)kIOSurfaceBytesPerElement : @4,
        (__bridge id)kIOSurfaceBytesPerRow : @(row),
        (__bridge id)kIOSurfacePixelFormat : @(kPixelRGBA),
    };
    return IOSurfaceCreate((__bridge CFDictionaryRef)props);
}

static id<MTLTexture> make_texture(id<MTLDevice> device, IOSurfaceRef surface, int width, int height,
                                   MTLTextureUsage usage) {
    MTLTextureDescriptor *desc =
        [MTLTextureDescriptor texture2DDescriptorWithPixelFormat:MTLPixelFormatRGBA8Unorm
                                                           width:(NSUInteger)width
                                                          height:(NSUInteger)height
                                                       mipmapped:NO];
    desc.usage = usage;
    desc.storageMode = MTLStorageModeShared;
    return [device newTextureWithDescriptor:desc iosurface:surface plane:0];
}

static void drop_surfaces_locked() {
    g_gpu_tex[0] = nil;
    g_gpu_tex[1] = nil;
    g_qt_tex[0] = nil;
    g_qt_tex[1] = nil;
    for (int i = 0; i < 2; ++i) {
        if (g_surf[i]) {
            CFRelease(g_surf[i]);
            g_surf[i] = nullptr;
        }
        g_qt_epoch[i] = -1;
    }
    g_width = 0;
    g_height = 0;
    g_published.store(-1, std::memory_order_relaxed);
    g_held.store(-1, std::memory_order_relaxed);
}

static bool ensure_surfaces_locked(id<MTLDevice> device, int width, int height) {
    if (g_gpu_device == device && g_width == width && g_height == height && g_surf[0] && g_surf[1] &&
        g_gpu_tex[0] && g_gpu_tex[1]) {
        return true;
    }
    drop_surfaces_locked();
    g_gpu_device = device;
    if (g_queue == nil || g_queue.device != device) {
        g_queue = [device newCommandQueue];
    }
    g_surf[0] = make_surface(width, height);
    g_surf[1] = make_surface(width, height);
    if (!g_surf[0] || !g_surf[1]) {
        log_fail("IOSurfaceCreate failed");
        drop_surfaces_locked();
        return false;
    }
    const MTLTextureUsage usage = MTLTextureUsageShaderRead | MTLTextureUsageShaderWrite;
    g_gpu_tex[0] = make_texture(device, g_surf[0], width, height, usage);
    g_gpu_tex[1] = make_texture(device, g_surf[1], width, height, usage);
    if (!g_gpu_tex[0] || !g_gpu_tex[1]) {
        log_fail("IOSurface texture failed (RGBA8)");
        drop_surfaces_locked();
        return false;
    }
    g_width = width;
    g_height = height;
    g_epoch.fetch_add(1, std::memory_order_acq_rel);
    return true;
}

extern "C" int cs_vello_shared_reset_needed(void *mtl_device, int width, int height) {
    if (!mtl_device || width < 1 || height < 1) {
        return 1;
    }
    id<MTLDevice> device = (__bridge id<MTLDevice>)mtl_device;
    std::lock_guard<std::mutex> lock(g_mu);
    if (g_gpu_device == device && g_width == width && g_height == height && g_surf[0] && g_surf[1] &&
        g_gpu_tex[0] && g_gpu_tex[1]) {
        return 0;
    }
    return 1;
}

extern "C" void *cs_vello_shared_begin(void *mtl_device, int width, int height, int *out_slot,
                                       int *out_epoch) {
    if (!mtl_device || width < 1 || height < 1) {
        return nullptr;
    }
    id<MTLDevice> device = (__bridge id<MTLDevice>)mtl_device;
    std::lock_guard<std::mutex> lock(g_mu);
    if (!ensure_surfaces_locked(device, width, height)) {
        return nullptr;
    }
    const int held = g_held.load(std::memory_order_acquire);
    const int slot = held == 0 ? 1 : 0;
    if (out_slot) {
        *out_slot = slot;
    }
    if (out_epoch) {
        *out_epoch = g_epoch.load(std::memory_order_acquire);
    }
    return (__bridge void *)g_gpu_tex[slot];
}

extern "C" void cs_vello_shared_commit(int slot) {
    if (slot < 0 || slot > 1) {
        return;
    }
    std::lock_guard<std::mutex> lock(g_mu);
    if (!g_surf[slot]) {
        return;
    }
    g_published.store(slot, std::memory_order_relaxed);
    g_frame.fetch_add(1, std::memory_order_release);
}

extern "C" int cs_vello_shared_has_base(int width, int height) {
    std::lock_guard<std::mutex> lock(g_mu);
    const int published = g_published.load(std::memory_order_acquire);
    if (published < 0 || published > 1 || g_width != width || g_height != height) {
        return 0;
    }
    return g_gpu_tex[0] && g_gpu_tex[1] ? 1 : 0;
}

extern "C" int cs_vello_shared_patch(void *patch_tex, int x, int y, int w, int h) {
    if (!patch_tex || w < 1 || h < 1) {
        return 0;
    }
    std::lock_guard<std::mutex> lock(g_mu);
    const int src = g_published.load(std::memory_order_acquire);
    if (src < 0 || src > 1 || !g_gpu_tex[src] || !g_queue) {
        return 0;
    }
    if (x < 0 || y < 0 || x + w > g_width || y + h > g_height) {
        return 0;
    }
    const int held = g_held.load(std::memory_order_acquire);
    // Qt is still sampling the previous buffer, so the published one can be patched.
    const bool in_place = held != src;
    const int dst = in_place ? src : (src == 0 ? 1 : 0);
    if (!g_gpu_tex[dst]) {
        return 0;
    }
    id<MTLTexture> patch = (__bridge id<MTLTexture>)patch_tex;
    id<MTLCommandBuffer> buffer = [g_queue commandBuffer];
    id<MTLBlitCommandEncoder> blit = [buffer blitCommandEncoder];
    if (!in_place) {
        [blit copyFromTexture:g_gpu_tex[src]
                   sourceSlice:0
                   sourceLevel:0
                  sourceOrigin:MTLOriginMake(0, 0, 0)
                    sourceSize:MTLSizeMake((NSUInteger)g_width, (NSUInteger)g_height, 1)
                     toTexture:g_gpu_tex[dst]
              destinationSlice:0
              destinationLevel:0
             destinationOrigin:MTLOriginMake(0, 0, 0)];
    }
    [blit copyFromTexture:patch
               sourceSlice:0
               sourceLevel:0
              sourceOrigin:MTLOriginMake(0, 0, 0)
                sourceSize:MTLSizeMake((NSUInteger)w, (NSUInteger)h, 1)
                 toTexture:g_gpu_tex[dst]
          destinationSlice:0
          destinationLevel:0
         destinationOrigin:MTLOriginMake((NSUInteger)x, (NSUInteger)y, 0)];
    [blit endEncoding];
    [buffer commit];
    [buffer waitUntilCompleted];
    if (buffer.status != MTLCommandBufferStatusCompleted) {
        log_fail("Metal patch blit failed");
        return 0;
    }
    g_published.store(dst, std::memory_order_relaxed);
    g_frame.fetch_add(1, std::memory_order_release);
    return 1;
}

extern "C" void *cs_vello_qt_texture(void *qt_device, int *out_width, int *out_height) {
    if (!qt_device) {
        return nullptr;
    }
    id<MTLDevice> device = (__bridge id<MTLDevice>)qt_device;
    std::lock_guard<std::mutex> lock(g_mu);
    const int slot = g_published.load(std::memory_order_acquire);
    if (slot < 0 || slot > 1 || !g_surf[slot] || g_width < 1 || g_height < 1) {
        return nullptr;
    }
    const int epoch = g_epoch.load(std::memory_order_acquire);
    if (g_qt_device != device) {
        g_qt_tex[0] = nil;
        g_qt_tex[1] = nil;
        g_qt_epoch[0] = -1;
        g_qt_epoch[1] = -1;
        g_qt_device = device;
    }
    if (!g_qt_tex[slot] || g_qt_epoch[slot] != epoch) {
        g_qt_tex[slot] = make_texture(device, g_surf[slot], g_width, g_height, MTLTextureUsageShaderRead);
        g_qt_epoch[slot] = epoch;
    }
    if (!g_qt_tex[slot]) {
        log_fail("Qt IOSurface texture failed");
        return nullptr;
    }
    g_held.store(slot, std::memory_order_release);
    if (out_width) {
        *out_width = g_width;
    }
    if (out_height) {
        *out_height = g_height;
    }
    return (__bridge void *)g_qt_tex[slot];
}

extern "C" void *cs_vello_wrap_texture(void *mtl_texture, void *window, int width, int height) {
    if (!mtl_texture || !window || width < 1 || height < 1) {
        return nullptr;
    }
    QSGTexture *texture = QNativeInterface::QSGMetalTexture::fromNative(
        (__bridge id<MTLTexture>)mtl_texture,
        static_cast<QQuickWindow *>(window),
        QSize(width, height),
        QQuickWindow::TextureHasAlphaChannel);
    return texture;
}

extern "C" uint64_t cs_vello_frame_generation(void) {
    return g_frame.load(std::memory_order_acquire);
}

extern "C" void cs_vello_shared_release(void) {
    std::lock_guard<std::mutex> lock(g_mu);
    drop_surfaces_locked();
    g_queue = nil;
    g_gpu_device = nil;
    g_qt_device = nil;
    g_frame.store(0, std::memory_order_relaxed);
}
