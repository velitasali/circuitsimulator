#include "circuit_canvas_item.h"

#include <QImage>
#include <QMetaObject>
#include <QPointer>
#include <QtQml>
#include <cmath>

#ifndef Q_OS_MACOS
#include "vello_share.h"
#else
#include <rhi/qrhi.h>
#endif

extern "C" {
void cs_vello_preview_release(void);
void cs_scope_screen_closed();
void cs_logic_screen_closed();
void cs_image_frame_clear(int slot);
int cs_image_frame_copy(int slot, uint8_t *dst, size_t dst_len, uint32_t dst_stride,
                        uint32_t *out_w, uint32_t *out_h, uint32_t *out_stride, uint64_t *out_gen,
                        double *out_dpr);
}

#ifdef Q_OS_MACOS
extern "C" {
void cs_vello_shared_release(void);
void *cs_vello_qt_texture(void *qt_device, int *out_width, int *out_height);
void *cs_vello_wrap_texture(void *mtl_texture, void *window, int width, int height);
void cs_vello_attach_keep(void *qsg_texture);
uint64_t cs_vello_frame_generation(void);
}
#endif

namespace {

constexpr int kSlotCount = 3;
QQuickItem *s_active[kSlotCount] = {};

QSGSimpleTextureNode *texture_node(QSGSimpleTextureNode *node) {
    if (node) {
        return node;
    }
    node = new QSGSimpleTextureNode();
    node->setFiltering(QSGTexture::Linear);
    node->setOwnsTexture(true);
    return node;
}

// A texture-less node still has geometry. Qt batches it and Metal reads a
// null texture. Delete it and return null so the scene graph drops the node.
QSGNode *node_with_texture(QSGSimpleTextureNode *node, qreal width, qreal height) {
    if (!node) {
        return nullptr;
    }
    if (node->texture() == nullptr) {
        delete node;
        return nullptr;
    }
    node->setRect(0, 0, width, height);
    return node;
}

QSGNode *upload_image_frame(int slot, QSGNode *oldNode, QQuickWindow *window, qreal itemW,
                            qreal itemH, uint64_t *lastGen) {
    auto *node = static_cast<QSGSimpleTextureNode *>(oldNode);
    if (!window || !lastGen || itemW <= 0 || itemH <= 0) {
        return node_with_texture(node, itemW, itemH);
    }

    for (int attempt = 0; attempt < 2; ++attempt) {
        uint32_t w = 0;
        uint32_t h = 0;
        uint32_t stride = 0;
        uint64_t gen = 0;
        double dpr = 0.0;
        const int need = cs_image_frame_copy(slot, nullptr, 0, 0, &w, &h, &stride, &gen, &dpr);
        if (need <= 0 || w == 0 || h == 0 || stride != w * 4u) {
            break;
        }
        if (node && node->texture() != nullptr && gen == *lastGen) {
            node->setRect(0, 0, itemW, itemH);
            return node;
        }

        QImage img(static_cast<int>(w), static_cast<int>(h), QImage::Format_RGBA8888_Premultiplied);
        const int bpl = img.bytesPerLine();
        if (img.isNull() || img.bits() == nullptr || bpl < static_cast<int>(stride)) {
            break;
        }
        uint32_t got_w = 0;
        uint32_t got_h = 0;
        const int copied = cs_image_frame_copy(
            slot, img.bits(), static_cast<size_t>(img.sizeInBytes()), static_cast<uint32_t>(bpl),
            &got_w, &got_h, &stride, &gen, &dpr);
        if (copied == 0 || got_w != w || got_h != h || stride != got_w * 4u) {
            continue;
        }
        const double upload_dpr = dpr > 0.1 ? dpr : window->devicePixelRatio();
        img.setDevicePixelRatio(upload_dpr > 0.1 ? upload_dpr : 1.0);
        QSGTexture *texture = window->createTextureFromImage(img, QQuickWindow::TextureHasAlphaChannel);
        if (!texture) {
            break;
        }
        node = texture_node(node);
        node->setTexture(texture);
        node->setSourceRect(QRectF(0, 0, w, h));
        *lastGen = gen;
        return node_with_texture(node, itemW, itemH);
    }
    return node_with_texture(node, itemW, itemH);
}

void (*close_hook(int slot))() {
    if (slot == CS_FRAME_SCOPE)
        return cs_scope_screen_closed;
    if (slot == CS_FRAME_LOGIC)
        return cs_logic_screen_closed;
    return nullptr;
}

} // namespace

extern "C" void cs_image_frame_request_update(int slot) {
    if (slot < 0 || slot >= kSlotCount) {
        return;
    }
    QPointer<QQuickItem> item = s_active[slot];
    if (!item) {
        return;
    }
    QMetaObject::invokeMethod(
        item.data(),
        [item, slot]() {
            if (!item) {
                return;
            }
            item->update();
            if (slot == CS_FRAME_CANVAS) {
                if (auto *canvas = qobject_cast<CircuitCanvasItem *>(item.data())) {
                    emit canvas->frameReady();
                }
            }
        },
        Qt::QueuedConnection);
}

CircuitCanvasItem::CircuitCanvasItem(QQuickItem *parent)
    : QQuickItem(parent)
{
    setFlag(ItemHasContents, true);
    s_active[CS_FRAME_CANVAS] = this;
}

CircuitCanvasItem::~CircuitCanvasItem() {
    cs_vello_preview_release();
#ifdef Q_OS_MACOS
    cs_vello_shared_release();
#endif
    if (s_active[CS_FRAME_CANVAS] == this) {
        s_active[CS_FRAME_CANVAS] = nullptr;
    }
}

void CircuitCanvasItem::geometryChange(const QRectF &newGeometry, const QRectF &oldGeometry) {
    QQuickItem::geometryChange(newGeometry, oldGeometry);
    syncVelloHost();
}

void CircuitCanvasItem::itemChange(ItemChange change, const ItemChangeData &value) {
    QQuickItem::itemChange(change, value);
    if (change == ItemSceneChange) {
        if (!value.window) {
            m_host_w = -1.0;
        } else {
            syncVelloHost();
        }
    }
}

void CircuitCanvasItem::syncVelloHost() {
    QQuickWindow *win = window();
    if (!win || width() < 1.0 || height() < 1.0) {
        return;
    }
    const double dpr = win->devicePixelRatio();
    const double w = width();
    const double h = height();
    if (std::abs(m_host_w - w) < 0.5 && std::abs(m_host_h - h) < 0.5 &&
        std::abs(m_host_dpr - dpr) < 1e-4) {
        return;
    }
    m_host_w = w;
    m_host_h = h;
    m_host_dpr = dpr;
    emit hostReady();
}

QSGNode *CircuitCanvasItem::updatePaintNode(QSGNode *oldNode, UpdatePaintNodeData *) {
    auto *node = static_cast<QSGSimpleTextureNode *>(oldNode);
    if (!window() || width() <= 0 || height() <= 0) {
        return node_with_texture(node, width(), height());
    }
#ifdef Q_OS_MACOS
    QRhi *rhi = window()->rhi();
    const auto *handles =
        rhi ? static_cast<const QRhiMetalNativeHandles *>(rhi->nativeHandles()) : nullptr;
    if (handles && handles->dev) {
        const uint64_t gen = cs_vello_frame_generation();
        if (!node || node->texture() == nullptr || gen != m_last_generation) {
            int tw = 0;
            int th = 0;
            void *native = cs_vello_qt_texture(reinterpret_cast<void *>(handles->dev), &tw, &th);
            if (native && tw > 0 && th > 0) {
                QSGTexture *texture =
                    static_cast<QSGTexture *>(cs_vello_wrap_texture(native, window(), tw, th));
                if (texture) {
                    cs_vello_attach_keep(texture);
                    node = texture_node(node);
                    node->setTexture(texture);
                    node->setSourceRect(QRectF(0, 0, tw, th));
                    m_last_generation = gen;
                } else {
                    cs_vello_attach_keep(nullptr);
                }
            }
        }
    }
    return node_with_texture(node, width(), height());
#else
    const uint64_t share_gen = cs_vello_share_generation();
    if (share_gen != 0 && (!node || node->texture() == nullptr || share_gen != m_last_generation)) {
        int tw = 0;
        int th = 0;
        void *native = cs_vello_share_adopt(window(), &tw, &th);
        if (native && tw > 0 && th > 0) {
            node = texture_node(node);
            node->setTexture(static_cast<QSGTexture *>(native));
            node->setSourceRect(QRectF(0, 0, tw, th));
            m_last_generation = share_gen;
        } else if (cs_vello_share_generation() == 0) {
            QPointer<CircuitCanvasItem> self = this;
            QMetaObject::invokeMethod(
                this,
                [self]() {
                    if (self) {
                        emit self->hostReady();
                    }
                },
                Qt::QueuedConnection);
        }
    }
    if (cs_vello_share_generation() == 0 || !node || node->texture() == nullptr) {
        return upload_image_frame(CS_FRAME_CANVAS, node, window(), width(), height(),
                                  &m_last_generation);
    }
    return node_with_texture(node, width(), height());
#endif
}

ScreenItem::ScreenItem(int slot, QQuickItem *parent)
    : QQuickItem(parent)
    , m_slot(slot)
    , m_closed(close_hook(slot))
{
    setFlag(ItemHasContents, true);
    if (slot >= 0 && slot < kSlotCount) {
        s_active[slot] = this;
    }
}

ScreenItem::~ScreenItem() {
    if (m_slot < 0 || m_slot >= kSlotCount || s_active[m_slot] != this) {
        return;
    }
    if (m_closed) {
        m_closed();
    }
    cs_image_frame_clear(m_slot);
    s_active[m_slot] = nullptr;
}

QSGNode *ScreenItem::updatePaintNode(QSGNode *oldNode, UpdatePaintNodeData *) {
    return upload_image_frame(m_slot, oldNode, window(), width(), height(), &m_last_generation);
}

extern "C" void cs_set_native_text_rendering() {
#if defined(Q_OS_MACOS) || defined(Q_OS_LINUX) || defined(Q_OS_WIN)
    QQuickWindow::setTextRenderType(QQuickWindow::NativeTextRendering);
#else
    QQuickWindow::setTextRenderType(QQuickWindow::QtTextRendering);
#endif
}

extern "C" void cs_register_canvas_item() {
    qmlRegisterType<CircuitCanvasItem>("cs_app", 1, 0, "CircuitCanvasItem");
    qmlRegisterType<ScopeScreenItem>("cs_app", 1, 0, "ScopeScreen");
    qmlRegisterType<LogicScreenItem>("cs_app", 1, 0, "LogicScreen");
}

extern "C" const char *cs_qt_version() {
    return qVersion();
}
