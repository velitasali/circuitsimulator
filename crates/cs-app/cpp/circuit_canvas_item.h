#pragma once

#include <QQuickItem>
#include <QSGSimpleTextureNode>
#include <QQuickWindow>

class CircuitCanvasItem : public QQuickItem {
    Q_OBJECT
public:
    explicit CircuitCanvasItem(QQuickItem *parent = nullptr);
    ~CircuitCanvasItem() override;

signals:
    // The canvas item has a size. QML asks for the first Vello frame.
    void hostReady();

protected:
    QSGNode *updatePaintNode(QSGNode *oldNode, UpdatePaintNodeData *) override;
    void geometryChange(const QRectF &newGeometry, const QRectF &oldGeometry) override;
    void itemChange(ItemChange change, const ItemChangeData &value) override;

private:
    void syncVelloHost();
    uint64_t m_last_generation = 0;
    double m_host_w = -1.0;
    double m_host_h = -1.0;
    double m_host_dpr = 0.0;
};
