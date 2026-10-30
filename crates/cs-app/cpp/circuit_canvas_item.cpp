#include "circuit_canvas_item.h"
#include "image_frame.h"
#include "instrument_screen_item.h"
#include <QMetaObject>
#include <QPointer>
#include <QtQml>
#include <cmath>

#ifndef Q_OS_MACOS
#include "vello_share.h"
#endif

#ifdef Q_OS_MACOS
#include <rhi/qrhi.h>
#endif

extern "C" {
    void cs_canvas_item_request_update();
    void cs_vello_preview_release(void);
}

#ifdef Q_OS_MACOS
extern "C" {
    void cs_vello_shared_release(void);
    void *cs_vello_qt_texture(void *qt_device, int *out_width, int *out_height);
    void *cs_vello_wrap_texture(void *mtl_texture, void *window, int width, int height);
    // Pairs with cs_vello_qt_texture. Pass the QSGTexture, or null to drop the retain.
    void cs_vello_attach_keep(void *qsg_texture);
    uint64_t cs_vello_frame_generation(void);
}
#endif

static CircuitCanvasItem *s_active_item = nullptr;

extern "C" void cs_canvas_item_request_update() {
    // The Vello frame is finished on a background thread. Queue the repaint
    // onto the item's thread; update() itself is not safe to call from there.
    QPointer<CircuitCanvasItem> item = s_active_item;
    if (!item) {
        return;
    }
    QMetaObject::invokeMethod(
        item.data(),
        [item]() {
            if (item) {
                item->update();
                emit item->frameReady();
            }
        },
        Qt::QueuedConnection);
}

CircuitCanvasItem::CircuitCanvasItem(QQuickItem *parent)
    : QQuickItem(parent)
{
    setFlag(ItemHasContents, true);
    s_active_item = this;
}

CircuitCanvasItem::~CircuitCanvasItem() {
    cs_vello_preview_release();
#ifdef Q_OS_MACOS
    cs_vello_shared_release();
#endif
    if (s_active_item == this) {
        s_active_item = nullptr;
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
        return cs_node_with_texture(node, width(), height());
    }
#ifdef Q_OS_MACOS
    QRhi *rhi = window()->rhi();
    const auto *handles = rhi ? static_cast<const QRhiMetalNativeHandles *>(rhi->nativeHandles()) : nullptr;
    if (handles && handles->dev) {
        const uint64_t gen = cs_vello_frame_generation();
        if (!node || node->texture() == nullptr || gen != m_last_generation) {
            int tw = 0;
            int th = 0;
            void *native = cs_vello_qt_texture(reinterpret_cast<void *>(handles->dev), &tw, &th);
            if (native && tw > 0 && th > 0) {
                QSGTexture *texture = static_cast<QSGTexture *>(
                    cs_vello_wrap_texture(native, window(), tw, th));
                if (texture) {
                    cs_vello_attach_keep(texture);
                    node = cs_texture_node(node);
                    node->setTexture(texture);
                    node->setSourceRect(QRectF(0, 0, tw, th));
                    m_last_generation = gen;
                } else {
                    cs_vello_attach_keep(nullptr);
                }
            }
        }
    }
    return cs_node_with_texture(node, width(), height());
#else
    const uint64_t share_gen = cs_vello_share_generation();
    if (share_gen != 0 && (!node || node->texture() == nullptr || share_gen != m_last_generation)) {
        int tw = 0;
        int th = 0;
        void *native = cs_vello_share_adopt(window(), &tw, &th);
        if (native && tw > 0 && th > 0) {
            node = cs_texture_node(node);
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
    // No shared texture yet. Upload the read-back image through the same
    // frame slot the oscilloscope window uses.
    if (share_gen == 0) {
        return cs_upload_image_frame(
            CS_FRAME_CANVAS, node, window(), width(), height(), &m_last_generation);
    }
    return cs_node_with_texture(node, width(), height());
#endif
}

extern "C" void cs_set_native_text_rendering() {
#ifdef Q_OS_MACOS
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

extern "C" const char* cs_qt_version() {
    return qVersion();
}
