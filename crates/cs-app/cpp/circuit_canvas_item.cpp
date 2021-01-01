#include "circuit_canvas_item.h"
#include <QImage>
#include <QtQml>
#include <cmath>

extern "C" {
    const uint8_t *cs_canvas_render(uint32_t *out_width, uint32_t *out_height, uint32_t *out_stride);
    uint64_t cs_canvas_generation();
    void cs_canvas_item_request_update();
    void cs_canvas_meta(double *out_cx, double *out_cy, double *out_zoom,
                        double *out_pad_x, double *out_pad_y, double *out_dpr,
                        double *out_view_w, double *out_view_h);
}

static CircuitCanvasItem *s_active_item = nullptr;

extern "C" void cs_canvas_item_request_update() {
    if (s_active_item) {
        s_active_item->update();
    }
}

CircuitCanvasItem::CircuitCanvasItem(QQuickItem *parent)
    : QQuickItem(parent)
{
    setFlag(ItemHasContents, true);
    s_active_item = this;
}

CircuitCanvasItem::~CircuitCanvasItem() {
    if (s_active_item == this) {
        s_active_item = nullptr;
    }
}

void CircuitCanvasItem::setCenterX(double cx) {
    if (std::abs(m_centerX - cx) > 1e-4) {
        m_centerX = cx;
        emit centerChanged();
        update();
    }
}

void CircuitCanvasItem::setCenterY(double cy) {
    if (std::abs(m_centerY - cy) > 1e-4) {
        m_centerY = cy;
        emit centerChanged();
        update();
    }
}

void CircuitCanvasItem::setZoom(double z) {
    if (std::abs(m_zoom - z) > 1e-4) {
        m_zoom = z;
        emit zoomChanged();
        update();
    }
}

QSGNode *CircuitCanvasItem::updatePaintNode(QSGNode *oldNode, UpdatePaintNodeData *) {
    auto *node = static_cast<QSGSimpleTextureNode *>(oldNode);
    if (!node) {
        node = new QSGSimpleTextureNode();
        node->setFiltering(QSGTexture::Linear);
    }

    if (!window() || width() <= 0 || height() <= 0) {
        return node;
    }

    uint64_t gen = cs_canvas_generation();
    if (gen != m_last_generation || node->texture() == nullptr) {
        m_last_generation = gen;

        uint32_t w = 0, h = 0, stride = 0;
        const uint8_t *pixels = cs_canvas_render(&w, &h, &stride);
        if (pixels && w > 0 && h > 0) {
            double dpr = window() ? window()->devicePixelRatio() : 1.0;
            QImage img(pixels, static_cast<int>(w), static_cast<int>(h),
                       static_cast<qsizetype>(stride),
                       QImage::Format_RGBA8888_Premultiplied);
            img.setDevicePixelRatio(dpr);

            QSGTexture *texture = window()->createTextureFromImage(
                img, QQuickWindow::TextureHasAlphaChannel);
            node->setTexture(texture);
            node->setOwnsTexture(true);

            cs_canvas_meta(&m_rendered_cx, &m_rendered_cy, &m_rendered_zoom,
                           &m_pad_x, &m_pad_y, &m_rendered_dpr,
                           &m_rendered_view_w, &m_rendered_view_h);
        }
    }

    if (node->texture() != nullptr) {
        double dpr = m_rendered_dpr > 0.0 ? m_rendered_dpr : (window() ? window()->devicePixelRatio() : 1.0);
        double zoom_ratio = (m_rendered_zoom > 0.0 && m_zoom > 0.0) ? (m_rendered_zoom / m_zoom) : 1.0;

        double dx = (m_centerX - m_rendered_cx) * m_rendered_zoom * dpr;
        double dy = (m_centerY - m_rendered_cy) * m_rendered_zoom * dpr;

        double srcW = width() * dpr * zoom_ratio;
        double srcH = height() * dpr * zoom_ratio;
        double srcX = m_pad_x * dpr + dx - (srcW - width() * dpr) * 0.5;
        double srcY = m_pad_y * dpr + dy - (srcH - height() * dpr) * 0.5;

        if (std::abs(zoom_ratio - 1.0) < 1e-4) {
            srcX = std::round(srcX);
            srcY = std::round(srcY);
            srcW = std::round(srcW);
            srcH = std::round(srcH);
        }

        node->setRect(0, 0, width(), height());
        node->setSourceRect(QRectF(srcX, srcY, srcW, srcH));
    }

    return node;
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
}

extern "C" const char* cs_qt_version() {
    return qVersion();
}


