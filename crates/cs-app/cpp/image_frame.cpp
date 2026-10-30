#include "image_frame.h"

#include <QImage>

extern "C" {
    void cs_canvas_item_request_update();
    void cs_instrument_screen_request_update(int slot);
    int cs_image_frame_copy(
        int slot,
        uint8_t *dst,
        size_t dst_len,
        uint32_t dst_stride,
        uint32_t *out_w,
        uint32_t *out_h,
        uint32_t *out_stride,
        uint64_t *out_gen,
        double *out_dpr);
}

extern "C" void cs_image_frame_request_update(int slot) {
    if (slot == CS_FRAME_CANVAS) {
        cs_canvas_item_request_update();
    } else if (slot == CS_FRAME_SCOPE || slot == CS_FRAME_LOGIC) {
        cs_instrument_screen_request_update(slot);
    }
}

QSGNode *cs_upload_image_frame(
    int slot,
    QSGNode *oldNode,
    QQuickWindow *window,
    qreal itemW,
    qreal itemH,
    uint64_t *lastGen)
{
    auto *node = static_cast<QSGSimpleTextureNode *>(oldNode);
    if (!window || !lastGen || itemW <= 0 || itemH <= 0) {
        return cs_node_with_texture(node, itemW, itemH);
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
            slot,
            img.bits(),
            static_cast<size_t>(img.sizeInBytes()),
            static_cast<uint32_t>(bpl),
            &got_w,
            &got_h,
            &stride,
            &gen,
            &dpr);
        if (copied == 0 || got_w != w || got_h != h || stride != got_w * 4u) {
            continue;
        }
        const double upload_dpr = dpr > 0.1 ? dpr : window->devicePixelRatio();
        img.setDevicePixelRatio(upload_dpr > 0.1 ? upload_dpr : 1.0);
        QSGTexture *texture = window->createTextureFromImage(img, QQuickWindow::TextureHasAlphaChannel);
        if (!texture) {
            break;
        }
        node = cs_texture_node(node);
        node->setTexture(texture);
        node->setSourceRect(QRectF(0, 0, w, h));
        *lastGen = gen;
        return cs_node_with_texture(node, itemW, itemH);
    }
    return cs_node_with_texture(node, itemW, itemH);
}
