#pragma once

#include <QQuickWindow>
#include <QSGSimpleTextureNode>

constexpr int CS_FRAME_CANVAS = 0;
constexpr int CS_FRAME_SCOPE = 1;
constexpr int CS_FRAME_LOGIC = 2;

extern "C" void cs_image_frame_clear(int slot);

inline QSGSimpleTextureNode *cs_texture_node(QSGSimpleTextureNode *node) {
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
inline QSGNode *cs_node_with_texture(QSGSimpleTextureNode *node, qreal width, qreal height) {
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

QSGNode *cs_upload_image_frame(
    int slot,
    QSGNode *oldNode,
    QQuickWindow *window,
    qreal itemW,
    qreal itemH,
    uint64_t *lastGen);
