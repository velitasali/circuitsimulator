#include "instrument_screen_item.h"

#include <QMetaObject>
#include <QPointer>

extern "C" void cs_scope_screen_closed();
extern "C" void cs_logic_screen_closed();

namespace {
constexpr int kSlotCount = 3;
ScreenItem *s_active[kSlotCount] = {};

void (*close_hook(int slot))() {
    if (slot == CS_FRAME_SCOPE) {
        return cs_scope_screen_closed;
    }
    if (slot == CS_FRAME_LOGIC) {
        return cs_logic_screen_closed;
    }
    return nullptr;
}
}

extern "C" void cs_instrument_screen_request_update(int slot) {
    if (slot < 0 || slot >= kSlotCount) {
        return;
    }
    QPointer<ScreenItem> item = s_active[slot];
    if (!item) {
        return;
    }
    QMetaObject::invokeMethod(
        item.data(),
        [item]() {
            if (item) {
                item->update();
            }
        },
        Qt::QueuedConnection);
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
    // A replacement item may already be current. Only the live one owns the frame.
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
    return cs_upload_image_frame(
        m_slot, oldNode, window(), width(), height(), &m_last_generation);
}
