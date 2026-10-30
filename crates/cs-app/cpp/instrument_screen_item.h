#pragma once

#include "image_frame.h"

#include <QQuickItem>

extern "C" void cs_instrument_screen_request_update(int slot);

// One scene-graph item. The frame slot selects the image and the close hook.
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
