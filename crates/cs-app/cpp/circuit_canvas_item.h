#pragma once

#include <QQuickItem>
#include <QSGSimpleTextureNode>
#include <QQuickWindow>

class CircuitCanvasItem : public QQuickItem {
    Q_OBJECT
    Q_PROPERTY(double centerX READ centerX WRITE setCenterX NOTIFY centerChanged)
    Q_PROPERTY(double centerY READ centerY WRITE setCenterY NOTIFY centerChanged)
    Q_PROPERTY(double zoom READ zoom WRITE setZoom NOTIFY zoomChanged)
public:
    explicit CircuitCanvasItem(QQuickItem *parent = nullptr);
    ~CircuitCanvasItem() override;

    double centerX() const { return m_centerX; }
    void setCenterX(double cx);

    double centerY() const { return m_centerY; }
    void setCenterY(double cy);

    double zoom() const { return m_zoom; }
    void setZoom(double z);

signals:
    void centerChanged();
    void zoomChanged();

protected:
    QSGNode *updatePaintNode(QSGNode *oldNode, UpdatePaintNodeData *) override;

private:
    uint64_t m_last_generation = 0;
    double m_centerX = 0.0;
    double m_centerY = 0.0;
    double m_zoom = 1.0;

    double m_rendered_cx = 0.0;
    double m_rendered_cy = 0.0;
    double m_rendered_zoom = 1.0;
    double m_pad_x = 0.0;
    double m_pad_y = 0.0;
    double m_rendered_dpr = 1.0;
    double m_rendered_view_w = 0.0;
    double m_rendered_view_h = 0.0;
};

