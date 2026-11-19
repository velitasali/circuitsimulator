// Thin Qt RHI / QSGTexture wrapper for Vello shared frames on Windows and Linux.
// Producer/consumer GPU sharing and slot state live in `src/vello_share.rs`.

#if defined(__linux__)
#undef VK_NO_PROTOTYPES
#include <vulkan/vulkan.h>
#endif

#include "vello_share.h"

#include <QQuickWindow>
#include <QSGRendererInterface>
#include <QSize>
#include <qsgtexture_platform.h>
#include <rhi/qrhi.h>

#include <cstdint>
#include <type_traits>

#if defined(__linux__)
#include <QOpenGLContext>
#endif

extern "C" {
void cs_vello_share_block_with_msg(const char *msg);
#if defined(_WIN32)
void *cs_vello_share_adopt_d3d11(void *dev, void *ctx, int *out_w, int *out_h);
void *cs_vello_share_adopt_d3d12(void *dev, void *queue, int *out_w, int *out_h);
#endif
#if defined(__linux__)
uint64_t cs_vello_share_adopt_vulkan(uint64_t dev_bits, uint64_t queue_bits, int *out_w,
                                     int *out_h);
int cs_vello_share_gl_begin(int *out_slot, int *out_epoch, int *out_w, int *out_h,
                            uint64_t *out_bytes);
int cs_vello_share_gl_export_fd(int slot);
void cs_vello_share_gl_hold(int slot);
#endif
}

#if defined(__linux__)
template <typename Handle>
static Handle vk_handle_from_bits(uint64_t bits) {
    if constexpr (std::is_pointer_v<Handle>) {
        return reinterpret_cast<Handle>(static_cast<uintptr_t>(bits));
    } else {
        return static_cast<Handle>(bits);
    }
}

static GLuint g_gl_mem[2] = {0, 0};
static GLuint g_gl_tex[2] = {0, 0};
static int g_gl_epoch[2] = {-1, -1};

extern "C" void cs_vello_share_release_gl(void) {
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

static void *adopt_gl(QQuickWindow *window, int *out_w, int *out_h) {
    QOpenGLContext *gl = QOpenGLContext::currentContext();
    if (!gl) {
        cs_vello_share_block_with_msg("Qt's OpenGL context is not current");
        return nullptr;
    }
    int slot = -1;
    int epoch = 0;
    int width = 0;
    int height = 0;
    uint64_t bytes = 0;
    if (!cs_vello_share_gl_begin(&slot, &epoch, &width, &height, &bytes) || slot < 0 || slot > 1) {
        return nullptr;
    }
    if (g_gl_epoch[slot] != epoch || !g_gl_tex[slot]) {
        auto create_mem = reinterpret_cast<void (*)(GLsizei, GLuint *)>(
            gl->getProcAddress("glCreateMemoryObjectsEXT"));
        auto import_fd = reinterpret_cast<void (*)(GLuint, uint64_t, GLenum, int)>(
            gl->getProcAddress("glImportMemoryFdEXT"));
        auto storage = reinterpret_cast<void (*)(GLuint, GLsizei, GLenum, GLsizei, GLsizei, GLuint,
                                                 uint64_t)>(
            gl->getProcAddress("glTextureStorageMem2DEXT"));
        auto create_tex = reinterpret_cast<void (*)(GLenum, GLsizei, GLuint *)>(
            gl->getProcAddress("glCreateTextures"));
        if (!create_mem || !import_fd || !storage || !create_tex) {
            cs_vello_share_block_with_msg("OpenGL cannot import a Vulkan image on this driver");
            return nullptr;
        }
        const int fd = cs_vello_share_gl_export_fd(slot);
        if (fd < 0) {
            cs_vello_share_block_with_msg("exporting the Vulkan image failed");
            return nullptr;
        }
        GLuint mem = 0;
        GLuint tex = 0;
        create_mem(1, &mem);
        import_fd(mem, bytes, 0x9586 /* GL_HANDLE_TYPE_OPAQUE_FD_EXT */, fd);
        create_tex(0x0DE1 /* GL_TEXTURE_2D */, 1, &tex);
        storage(tex, 1, 0x8058 /* GL_RGBA8 */, width, height, mem, 0);
        g_gl_mem[slot] = mem;
        g_gl_tex[slot] = tex;
        g_gl_epoch[slot] = epoch;
    }
    auto barrier = reinterpret_cast<void (*)(GLbitfield)>(gl->getProcAddress("glMemoryBarrier"));
    if (barrier) {
        barrier(0xFFFFFFFF);
    }
    QSGTexture *texture = QNativeInterface::QSGOpenGLTexture::fromNative(
        g_gl_tex[slot], window, QSize(width, height), QQuickWindow::TextureHasAlphaChannel);
    if (!texture) {
        cs_vello_share_block_with_msg("wrapping the OpenGL texture failed");
        return nullptr;
    }
    cs_vello_share_gl_hold(slot);
    if (out_w)
        *out_w = width;
    if (out_h)
        *out_h = height;
    return texture;
}
#endif

extern "C" void *cs_vello_share_adopt(void *window_ptr, int *out_w, int *out_h) {
    auto *window = static_cast<QQuickWindow *>(window_ptr);
    if (!window) {
        return nullptr;
    }
    QSGRendererInterface *ri = window->rendererInterface();
    if (!ri) {
        return nullptr;
    }
    QRhi *rhi = window->rhi();
    int w = 0;
    int h = 0;
    switch (ri->graphicsApi()) {
#if defined(_WIN32)
    case QSGRendererInterface::Direct3D11: {
        const auto *handles =
            rhi ? static_cast<const QRhiD3D11NativeHandles *>(rhi->nativeHandles()) : nullptr;
        void *native = cs_vello_share_adopt_d3d11(
            handles ? handles->dev : nullptr, handles ? handles->context : nullptr, &w, &h);
        if (!native || w < 1 || h < 1) {
            return nullptr;
        }
        QSGTexture *texture = QNativeInterface::QSGD3D11Texture::fromNative(
            native, window, QSize(w, h), QQuickWindow::TextureHasAlphaChannel);
        if (!texture) {
            cs_vello_share_block_with_msg("wrapping the D3D11 texture failed");
            return nullptr;
        }
        if (out_w)
            *out_w = w;
        if (out_h)
            *out_h = h;
        return texture;
    }
    case QSGRendererInterface::Direct3D12: {
        const auto *handles =
            rhi ? static_cast<const QRhiD3D12NativeHandles *>(rhi->nativeHandles()) : nullptr;
        void *native = cs_vello_share_adopt_d3d12(
            handles ? handles->dev : nullptr, handles ? handles->commandQueue : nullptr, &w, &h);
        if (!native || w < 1 || h < 1) {
            return nullptr;
        }
        QSGTexture *texture = QNativeInterface::QSGD3D12Texture::fromNative(
            native, 0 /* D3D12_RESOURCE_STATE_COMMON */, window, QSize(w, h),
            QQuickWindow::TextureHasAlphaChannel);
        if (!texture) {
            cs_vello_share_block_with_msg("wrapping the D3D12 texture failed");
            return nullptr;
        }
        if (out_w)
            *out_w = w;
        if (out_h)
            *out_h = h;
        return texture;
    }
#endif
#if defined(__linux__)
    case QSGRendererInterface::Vulkan: {
        const auto *handles =
            rhi ? static_cast<const QRhiVulkanNativeHandles *>(rhi->nativeHandles()) : nullptr;
        const uint64_t dev_bits =
            handles ? static_cast<uint64_t>(reinterpret_cast<uintptr_t>(handles->dev)) : 0;
        const uint64_t queue_bits =
            handles ? static_cast<uint64_t>(reinterpret_cast<uintptr_t>(handles->gfxQueue)) : 0;
        const uint64_t img_bits = cs_vello_share_adopt_vulkan(dev_bits, queue_bits, &w, &h);
        if (!img_bits || w < 1 || h < 1) {
            return nullptr;
        }
        const VkImage img = vk_handle_from_bits<VkImage>(img_bits);
        QSGTexture *texture = QNativeInterface::QSGVulkanTexture::fromNative(
            img, VK_IMAGE_LAYOUT_GENERAL, window, QSize(w, h),
            QQuickWindow::TextureHasAlphaChannel);
        if (!texture) {
            cs_vello_share_block_with_msg("wrapping the Vulkan image failed");
            return nullptr;
        }
        if (out_w)
            *out_w = w;
        if (out_h)
            *out_h = h;
        return texture;
    }
    case QSGRendererInterface::OpenGL:
        return adopt_gl(window, out_w, out_h);
#endif
    default:
        cs_vello_share_block_with_msg("this Qt graphics backend cannot sample the shared texture");
        return nullptr;
    }
}
