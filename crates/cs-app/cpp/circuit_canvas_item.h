#pragma once

#include <QQuickItem>
#include <QQuickWindow>
#include <QSGSimpleTextureNode>

constexpr int CS_FRAME_CANVAS = 0;
constexpr int CS_FRAME_SCOPE = 1;
constexpr int CS_FRAME_LOGIC = 2;

class CircuitCanvasItem : public QQuickItem {
    Q_OBJECT
public:
    explicit CircuitCanvasItem(QQuickItem *parent = nullptr);
    ~CircuitCanvasItem() override;

signals:
    // The canvas item has a size. QML asks for the first Vello frame.
    void hostReady();
    // A background Vello frame finished; QML can flush any deferred dirty set.
    void frameReady();

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

// One scene-graph item for CPU-uploaded instrument screens.
class ScreenItem : public QQuickItem {
    Q_OBJECT
public:
    explicit ScreenItem(int slot, QQuickItem *parent = nullptr);
    ~ScreenItem() override;

protected:
    QSGNode *updatePaintNode(QSGNode *oldNode, UpdatePaintNodeData *) override;

private:
    using ClosedFn = void (*)();

    int m_slot = -1;
    ClosedFn m_closed = nullptr;
    uint64_t m_last_generation = 0;
};

class ScopeScreenItem : public ScreenItem {
    Q_OBJECT
public:
    explicit ScopeScreenItem(QQuickItem *parent = nullptr)
        : ScreenItem(CS_FRAME_SCOPE, parent) {}
};

class LogicScreenItem : public ScreenItem {
    Q_OBJECT
public:
    explicit LogicScreenItem(QQuickItem *parent = nullptr)
        : ScreenItem(CS_FRAME_LOGIC, parent) {}
};
