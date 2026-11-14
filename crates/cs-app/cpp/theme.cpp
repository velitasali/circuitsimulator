#include "theme.h"

#include <cstring>

#include <QtCore/QAbstractNativeEventFilter>
#include <QtCore/QCoreApplication>
#include <QtCore/QEvent>
#include <QtCore/QHash>
#include <QtCore/QMetaObject>
#include <QtCore/QPointer>
#include <QtCore/QSet>
#include <QtCore/QtGlobal>
#include <QtGui/QColor>
#include <QtGui/QGuiApplication>
#include <QtGui/QIcon>
#include <QtGui/QKeyEvent>
#include <QtGui/QPalette>
#include <QtGui/QPlatformSurfaceEvent>
#include <QtGui/QStyleHints>
#include <QtGui/QWindow>

#ifdef Q_OS_LINUX
#include <QtCore/QMargins>
#include <QtCore/QPointer>
#include <QtGui/QRegion>
#if QT_CONFIG(wayland)
#include <qpa/qplatformwindow_p.h>
#endif
#endif

#ifdef Q_OS_WIN
#include <QtQuick/QQuickItem>
#include <QtQuick/QQuickWindow>
#include <QtCore/QSettings>
#ifndef WINVER
#define WINVER 0x0A00
#endif
#ifndef _WIN32_WINNT
#define _WIN32_WINNT 0x0A00
#endif
#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif
#include <windows.h>
#include <windowsx.h>
#include <dwmapi.h>
#include <shellapi.h>
#endif

static int s_current_theme_dark = -1;
static bool s_in_apply_theme = false;
static int (*s_on_alt)(int phase) = nullptr;

#ifdef Q_OS_WIN
static QHash<HWND, QPointer<QWindow>> s_main_windows;
static QSet<HWND> s_chrome_applied;

static bool is_main_window(QWindow* w)
{
    return w && w->transientParent() == nullptr
        && w->objectName() == QStringLiteral("mainWindow");
}

static void prune_main_windows_for(QWindow* w)
{
    for (auto it = s_main_windows.begin(); it != s_main_windows.end();) {
        if (it.value().isNull() || it.value().data() == w) {
            s_chrome_applied.remove(it.key());
            it = s_main_windows.erase(it);
        } else {
            ++it;
        }
    }
}

static void refresh_autohide_clearance();

static void register_main_window_chrome(QWindow* w)
{
    if (!is_main_window(w))
        return;
    HWND hwnd = reinterpret_cast<HWND>(w->winId());
    if (!hwnd)
        return;

    auto existing = s_main_windows.constFind(hwnd);
    if (existing == s_main_windows.constEnd() || existing.value().data() != w) {
        prune_main_windows_for(w);
        s_main_windows.insert(hwnd, w);
    }

    if (!s_chrome_applied.contains(hwnd)) {
        s_chrome_applied.insert(hwnd);
        // Keep WS_CAPTION + WS_THICKFRAME so DWM preserves rounded corners,
        // drop shadow, Aero Snap, and Alt+Space system menu; WM_NCCALCSIZE
        // removes only the top native titlebar strip.
        const LONG_PTR style = GetWindowLongPtrW(hwnd, GWL_STYLE);
        const LONG_PTR desired = style | WS_CAPTION | WS_THICKFRAME | WS_MINIMIZEBOX | WS_MAXIMIZEBOX | WS_SYSMENU;
        if (desired != style)
            SetWindowLongPtrW(hwnd, GWL_STYLE, desired);

        MARGINS margins = { 0, 0, 0, 1 };
        DwmExtendFrameIntoClientArea(hwnd, &margins);
        // Before SWP_FRAMECHANGED, which sends WM_NCCALCSIZE synchronously.
        refresh_autohide_clearance();
        SetWindowPos(hwnd, nullptr, 0, 0, 0, 0,
                     SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED);
    }
}

static QQuickItem* find_quick_item(QQuickItem* item, const QString& objectName)
{
    if (!item)
        return nullptr;
    if (item->objectName() == objectName)
        return item;
    const auto children = item->childItems();
    for (QQuickItem* child : children) {
        if (QQuickItem* found = find_quick_item(child, objectName))
            return found;
    }
    return nullptr;
}

// QQuickText accepts the left button even when it ignores the press, so a title
// label would steal HTCAPTION. Caption controls are mouse areas, or an item that
// opts in with titleBarInteractive.
static bool is_caption_control(const QQuickItem* child)
{
    if (!child->isEnabled())
        return false;
    if (child->property("titleBarInteractive").toBool())
        return true;
    if (!child->inherits("QQuickMouseArea"))
        return false;
    const QVariant accepted = child->property("acceptedButtons");
    return !accepted.isValid() || accepted.toInt() != static_cast<int>(Qt::NoButton);
}

static bool hit_interactive_descendant(QQuickItem* item, const QPointF& scenePt)
{
    const auto children = item->childItems();
    for (auto it = children.crbegin(); it != children.crend(); ++it) {
        QQuickItem* child = *it;
        if (!child->isVisible())
            continue;
        const QPointF local = child->mapFromScene(scenePt);
        if (child->contains(local)) {
            if (hit_interactive_descendant(child, scenePt))
                return true;
            if (is_caption_control(child))
                return true;
        }
    }
    return false;
}

// ABM_GETTASKBARPOS is the primary bar only. Secondary monitors use
// Shell_SecondaryTrayWnd. Cached so WM_NCCALCSIZE does not re-enter the shell.
enum { kMaxAutoHideMonitors = 8 };

struct AutoHideClearance {
    HMONITOR monitor;
    UINT edge;
};

static AutoHideClearance s_autohide[kMaxAutoHideMonitors];
static int s_autohide_count = 0;
static bool s_refreshing_autohide = false;

static UINT tray_edge(const RECT& tray, const RECT& monitor)
{
    const LONG width = tray.right - tray.left;
    const LONG height = tray.bottom - tray.top;
    const LONG leftGap = tray.left - monitor.left;
    const LONG rightGap = monitor.right - tray.right;
    const LONG topGap = tray.top - monitor.top;
    const LONG bottomGap = monitor.bottom - tray.bottom;
    // A hidden bar is a sliver on the screen edge, or parked just off it.
    if (width < height)
        return leftGap <= rightGap ? ABE_LEFT : ABE_RIGHT;
    return topGap <= bottomGap ? ABE_TOP : ABE_BOTTOM;
}

static void remember_autohide(AutoHideClearance* list, int* count, HMONITOR monitor, UINT edge)
{
    if (!monitor || *count >= kMaxAutoHideMonitors)
        return;
    switch (edge) {
    case ABE_LEFT:
    case ABE_TOP:
    case ABE_RIGHT:
    case ABE_BOTTOM:
        break;
    default:
        return;
    }
    for (int i = 0; i < *count; ++i) {
        if (list[i].monitor == monitor)
            return;
    }
    list[*count] = AutoHideClearance{monitor, edge};
    ++*count;
}

static void refresh_autohide_clearance()
{
    if (s_refreshing_autohide)
        return;
    s_refreshing_autohide = true;

    AutoHideClearance next[kMaxAutoHideMonitors];
    int count = 0;

    APPBARDATA abd{};
    abd.cbSize = sizeof(abd);
    if (SHAppBarMessage(ABM_GETSTATE, &abd) & ABS_AUTOHIDE) {
        APPBARDATA pos{};
        pos.cbSize = sizeof(pos);
        if (SHAppBarMessage(ABM_GETTASKBARPOS, &pos)) {
            HMONITOR mon = MonitorFromRect(&pos.rc, MONITOR_DEFAULTTONEAREST);
            if (!mon) {
                if (HWND primary = FindWindowW(L"Shell_TrayWnd", nullptr))
                    mon = MonitorFromWindow(primary, MONITOR_DEFAULTTONEAREST);
            }
            remember_autohide(next, &count, mon, pos.uEdge);
        }

        HWND tray = nullptr;
        while ((tray = FindWindowExW(nullptr, tray, L"Shell_SecondaryTrayWnd", nullptr)) != nullptr) {
            RECT trayRect{};
            if (!GetWindowRect(tray, &trayRect))
                continue;
            HMONITOR mon = MonitorFromRect(&trayRect, MONITOR_DEFAULTTONEAREST);
            if (!mon)
                continue;
            MONITORINFO mi{};
            mi.cbSize = sizeof(mi);
            if (!GetMonitorInfoW(mon, &mi))
                continue;
            remember_autohide(next, &count, mon, tray_edge(trayRect, mi.rcMonitor));
        }
    }

    s_autohide_count = count;
    for (int i = 0; i < count; ++i)
        s_autohide[i] = next[i];
    s_refreshing_autohide = false;
}

static void inset_autohide_edge(HWND hwnd, RECT* rc)
{
    HMONITOR mon = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
    if (!mon)
        return;
    for (int i = 0; i < s_autohide_count; ++i) {
        if (s_autohide[i].monitor != mon)
            continue;
        switch (s_autohide[i].edge) {
        case ABE_LEFT:
            rc->left += 1;
            break;
        case ABE_TOP:
            rc->top += 1;
            break;
        case ABE_RIGHT:
            rc->right -= 1;
            break;
        case ABE_BOTTOM:
            rc->bottom -= 1;
            break;
        default:
            break;
        }
        return;
    }
}

class WinChromeNativeFilter : public QAbstractNativeEventFilter
{
public:
    bool nativeEventFilter(const QByteArray& eventType, void* message, qintptr* result) override
    {
        if (eventType != "windows_generic_MSG" || !message || !result)
            return false;

        MSG* msg = static_cast<MSG*>(message);
        auto it = s_main_windows.find(msg->hwnd);
        if (it == s_main_windows.end())
            return false;
        QWindow* win = it.value().data();
        if (!win) {
            s_chrome_applied.remove(msg->hwnd);
            s_main_windows.erase(it);
            return false;
        }

        if (msg->message == WM_SETTINGCHANGE || msg->message == WM_DISPLAYCHANGE) {
            refresh_autohide_clearance();
            return false;
        }

        if (msg->message == WM_NCCALCSIZE && msg->wParam == TRUE) {
            auto* params = reinterpret_cast<NCCALCSIZE_PARAMS*>(msg->lParam);
            UINT dpi = GetDpiForWindow(msg->hwnd);
            if (dpi == 0)
                dpi = 96;
            const int frameX = GetSystemMetricsForDpi(SM_CXSIZEFRAME, dpi)
                               + GetSystemMetricsForDpi(SM_CXPADDEDBORDER, dpi);
            const int frameY = GetSystemMetricsForDpi(SM_CYSIZEFRAME, dpi)
                               + GetSystemMetricsForDpi(SM_CXPADDEDBORDER, dpi);

            if (IsZoomed(msg->hwnd)) {
                params->rgrc[0].left += frameX;
                params->rgrc[0].top += frameY;
                params->rgrc[0].right -= frameX;
                params->rgrc[0].bottom -= frameY;

                // One pixel on the auto-hide edge lets that edge reveal the bar.
                // A monitor with no taskbar is absent from the cache, so it stays flush.
                inset_autohide_edge(msg->hwnd, &params->rgrc[0]);
            } else {
                // Eliminate top titlebar while preserving invisible left/right/bottom resize borders.
                params->rgrc[0].left += frameX;
                params->rgrc[0].right -= frameX;
                params->rgrc[0].bottom -= frameY;
            }
            *result = 0;
            return true;
        }

        if (msg->message == WM_NCHITTEST) {
            POINT pt = { GET_X_LPARAM(msg->lParam), GET_Y_LPARAM(msg->lParam) };
            ScreenToClient(msg->hwnd, &pt);
            RECT cr{};
            GetClientRect(msg->hwnd, &cr);

            // Let DefWindowProc handle left/right/bottom non-client resize borders.
            if (pt.x < 0 || pt.x >= cr.right || pt.y < 0 || pt.y >= cr.bottom)
                return false;

            bool overTitle = false;
            bool overControl = false;
            if (auto* qw = qobject_cast<QQuickWindow*>(win)) {
                QQuickItem* titleBar = find_quick_item(qw->contentItem(), QStringLiteral("titleBar"));
                if (!titleBar)
                    titleBar = qw->findChild<QQuickItem*>(QStringLiteral("titleBar"));
                if (titleBar && titleBar->isVisible()) {
                    const qreal dpr = win->devicePixelRatio() > 0.0 ? win->devicePixelRatio() : 1.0;
                    const QPointF scenePt(pt.x / dpr, pt.y / dpr);
                    const QPointF local = titleBar->mapFromScene(scenePt);
                    if (local.x() >= 0.0 && local.y() >= 0.0
                        && local.x() < titleBar->width() && local.y() < titleBar->height()) {
                        overTitle = true;
                        overControl = hit_interactive_descendant(titleBar, scenePt);
                    }
                }
            }

            if (!IsZoomed(msg->hwnd)) {
                UINT dpi = GetDpiForWindow(msg->hwnd);
                if (dpi == 0)
                    dpi = 96;
                const int frameX = GetSystemMetricsForDpi(SM_CXSIZEFRAME, dpi)
                                   + GetSystemMetricsForDpi(SM_CXPADDEDBORDER, dpi);
                const int frameY = GetSystemMetricsForDpi(SM_CYSIZEFRAME, dpi)
                                   + GetSystemMetricsForDpi(SM_CXPADDEDBORDER, dpi);
                // Restored top frame is client area because NCCALCSIZE drops the
                // native caption. Same thickness as the other edges; buttons stay clickable.
                if (pt.y < frameY && !overControl) {
                    if (pt.x < frameX)
                        *result = HTTOPLEFT;
                    else if (pt.x >= cr.right - frameX)
                        *result = HTTOPRIGHT;
                    else
                        *result = HTTOP;
                    return true;
                }
            }

            if (overTitle) {
                *result = overControl ? HTCLIENT : HTCAPTION;
                return true;
            }
        }

        return false;
    }
};

static void apply_dwm(QWindow* w, bool dark)
{
    if (!w)
        return;
    HWND hwnd = reinterpret_cast<HWND>(w->winId());
    if (!hwnd)
        return;
    BOOL value = dark ? TRUE : FALSE;
    // 20 = DWMWA_USE_IMMERSIVE_DARK_MODE (Win10 1903+); 19 is the older constant.
    DwmSetWindowAttribute(hwnd, 20, &value, sizeof(value));
    DwmSetWindowAttribute(hwnd, 19, &value, sizeof(value));
    SetWindowPos(hwnd, nullptr, 0, 0, 0, 0,
                 SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_FRAMECHANGED);
}

static void apply_dwm_all(bool dark)
{
    const auto windows = QGuiApplication::topLevelWindows();
    for (QWindow* w : windows)
        apply_dwm(w, dark);
}

static bool windows_apps_use_dark()
{
    QSettings registry(
        QStringLiteral("HKEY_CURRENT_USER\\Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize"),
        QSettings::NativeFormat);
    if (registry.contains(QStringLiteral("AppsUseLightTheme")))
        return registry.value(QStringLiteral("AppsUseLightTheme")).toInt() == 0;
    return false;
}
#endif

#ifdef Q_OS_LINUX
static int linux_shadow_margin(QWindow* w)
{
    const QVariant prop = w->property("linuxBaseShadowMargin");
    if (!prop.isValid())
        return -1;
    const int base = prop.toInt();
    const bool flat = (w->windowStates() & (Qt::WindowMaximized | Qt::WindowFullScreen)) != 0;
    return flat ? 0 : base;
}

#if QT_CONFIG(wayland)
static int linux_resize_overhang(QWindow* w)
{
    const QVariant prop = w->property("linuxResizeOverhang");
    return prop.isValid() ? qMax(0, prop.toInt()) : 3;
}

static void apply_linux_shadow_mask(QWindow* w, int m)
{
    if (m > 0 && w->width() > 2 * m && w->height() > 2 * m) {
        // Handles hang `overhang` px outside the frame. One more pixel stays
        // hittable so the outer edge of the grab is inside the input region.
        const int pad = qMax(0, m - linux_resize_overhang(w) - 1);
        w->setMask(QRegion(pad, pad, w->width() - 2 * pad, w->height() - 2 * pad));
    } else {
        w->setMask(QRegion());
    }
}

static void update_linux_shadow_mask(QWindow* w)
{
    if (!w || !w->nativeInterface<QNativeInterface::Private::QWaylandWindow>())
        return;
    const int m = linux_shadow_margin(w);
    if (m < 0)
        return;
    apply_linux_shadow_mask(w, m);
}
#endif

static void apply_linux_shadow_margins(QWindow* w, bool defer)
{
    if (!w)
        return;
    const int m = linux_shadow_margin(w);
    if (m < 0)
        return;
#if QT_CONFIG(wayland)
    auto* waylandWin = w->nativeInterface<QNativeInterface::Private::QWaylandWindow>();
    if (!waylandWin)
        return;
    const QVariant prev = w->property("_csLastShadowMargin");
    const bool changed = !prev.isValid() || prev.toInt() != m;
    // xdg applyConfigure emits WindowStateChange before resizeFromApplyConfigure,
    // and setCustomMargins sends geometry straight to the shell. A margin change
    // from that handler would publish the pre-configure size.
    if (changed && defer) {
        if (!w->property("_csShadowMarginQueued").toBool()) {
            w->setProperty("_csShadowMarginQueued", true);
            QPointer<QWindow> window(w);
            QMetaObject::invokeMethod(
                w,
                [window]() {
                    if (!window)
                        return;
                    window->setProperty("_csShadowMarginQueued", false);
                    apply_linux_shadow_margins(window.data(), false);
                },
                Qt::QueuedConnection);
        }
        return;
    }
    if (changed) {
        w->setProperty("_csLastShadowMargin", m);
        waylandWin->setCustomMargins(QMargins(m, m, m, m));
    }
    apply_linux_shadow_mask(w, m);
#else
    (void)defer;
    (void)m;
#endif
}
#endif

class ThemeEventFilter : public QObject
{
public:
    explicit ThemeEventFilter(QObject* parent = nullptr)
        : QObject(parent)
    {
    }

    bool eventFilter(QObject* obj, QEvent* event) override
    {
        const QEvent::Type type = event->type();
        if (type == QEvent::ApplicationDeactivate) {
            disarmAlt();
            return QObject::eventFilter(obj, event);
        }

        auto* w = qobject_cast<QWindow*>(obj);
        if (!w)
            return QObject::eventFilter(obj, event);

#ifdef Q_OS_WIN
        if (type == QEvent::PlatformSurface) {
            auto* se = static_cast<QPlatformSurfaceEvent*>(event);
            if (se->surfaceEventType() == QPlatformSurfaceEvent::SurfaceCreated) {
                register_main_window_chrome(w);
            } else if (se->surfaceEventType() == QPlatformSurfaceEvent::SurfaceAboutToBeDestroyed) {
                prune_main_windows_for(w);
            }
        } else if (type == QEvent::Show || type == QEvent::WinIdChange) {
            register_main_window_chrome(w);
            if (s_current_theme_dark >= 0)
                apply_dwm(w, s_current_theme_dark != 0);
        }
#elif defined(Q_OS_LINUX)
        if (type == QEvent::PlatformSurface) {
            auto* se = static_cast<QPlatformSurfaceEvent*>(event);
            if (se->surfaceEventType() == QPlatformSurfaceEvent::SurfaceCreated)
                apply_linux_shadow_margins(w, false);
        } else if (type == QEvent::Show || type == QEvent::WindowStateChange) {
            apply_linux_shadow_margins(w, true);
        } else if (type == QEvent::Resize) {
#if QT_CONFIG(wayland)
            update_linux_shadow_mask(w);
#endif
        }
#endif

        if (w->transientParent() != nullptr) {
            switch (type) {
            case QEvent::KeyPress:
            case QEvent::KeyRelease:
            case QEvent::MouseButtonPress:
            case QEvent::MouseButtonRelease:
            case QEvent::MouseButtonDblClick:
            case QEvent::Wheel:
            case QEvent::WindowDeactivate:
            case QEvent::FocusOut:
            case QEvent::Hide:
                disarmAlt();
                break;
            default:
                break;
            }
            return QObject::eventFilter(obj, event);
        }

        switch (type) {
        case QEvent::KeyPress: {
            auto* ke = static_cast<QKeyEvent*>(event);
            if (!ke->isAutoRepeat()) {
                const Qt::KeyboardModifiers otherMods =
                    ke->modifiers() & (Qt::ControlModifier | Qt::ShiftModifier | Qt::MetaModifier);
                const bool bareAlt = ke->key() == Qt::Key_Alt && otherMods == Qt::NoModifier;
                m_altArmed = bareAlt;
                m_closeOnRelease = false;
                // The open menu would close itself on this press, and the release
                // would then open it again. Eat the press only while it is open
                // so a press on the canvas still reaches Alt-click.
                if (bareAlt && s_on_alt && s_on_alt(0)) {
                    m_closeOnRelease = true;
                    return true;
                }
            }
            break;
        }
        case QEvent::KeyRelease: {
            auto* ke = static_cast<QKeyEvent*>(event);
            if (!ke->isAutoRepeat()) {
                if (ke->key() == Qt::Key_Alt) {
                    const Qt::KeyboardModifiers otherMods =
                        ke->modifiers() & (Qt::ControlModifier | Qt::ShiftModifier | Qt::MetaModifier);
                    const bool trigger = m_altArmed && (otherMods == Qt::NoModifier);
                    const bool closeOnRelease = m_closeOnRelease;
                    disarmAlt();
                    if (trigger && s_on_alt) {
                        const int toggled = s_on_alt(1);
                        if (closeOnRelease || toggled)
                            return true;
                    }
                } else {
                    disarmAlt();
                }
            }
            break;
        }
        case QEvent::MouseButtonPress:
        case QEvent::MouseButtonRelease:
        case QEvent::MouseButtonDblClick:
        case QEvent::Wheel:
        case QEvent::WindowDeactivate:
        case QEvent::FocusOut:
        case QEvent::Hide:
            disarmAlt();
            break;
        default:
            break;
        }

        return QObject::eventFilter(obj, event);
    }

private:
    void disarmAlt()
    {
        m_altArmed = false;
        m_closeOnRelease = false;
    }

    bool m_altArmed = false;
    bool m_closeOnRelease = false;
};

static void ensure_filter()
{
    static ThemeEventFilter* filter = nullptr;
    if (filter)
        return;
    QGuiApplication* app = qobject_cast<QGuiApplication*>(QGuiApplication::instance());
    if (!app)
        return;
    filter = new ThemeEventFilter(app);
    app->installEventFilter(filter);
#ifdef Q_OS_WIN
    static WinChromeNativeFilter* nativeFilter = nullptr;
    if (!nativeFilter) {
        nativeFilter = new WinChromeNativeFilter();
        QCoreApplication::instance()->installNativeEventFilter(nativeFilter);
    }
#endif
}

static QPalette make_palette(bool dark)
{
    QPalette palette;
    if (dark) {
        palette.setColor(QPalette::Window, QColor(30, 30, 30));
        palette.setColor(QPalette::WindowText, QColor(230, 230, 230));
        palette.setColor(QPalette::Base, QColor(42, 42, 42));
        palette.setColor(QPalette::AlternateBase, QColor(50, 50, 50));
        palette.setColor(QPalette::ToolTipBase, QColor(50, 50, 50));
        palette.setColor(QPalette::ToolTipText, QColor(230, 230, 230));
        palette.setColor(QPalette::Text, QColor(230, 230, 230));
        palette.setColor(QPalette::PlaceholderText, QColor(140, 140, 140));
        palette.setColor(QPalette::Disabled, QPalette::Text, Qt::gray);
        palette.setColor(QPalette::Disabled, QPalette::WindowText, Qt::gray);
        palette.setColor(QPalette::Button, QColor(45, 45, 45));
        palette.setColor(QPalette::ButtonText, QColor(230, 230, 230));
        palette.setColor(QPalette::Disabled, QPalette::ButtonText, Qt::gray);
        palette.setColor(QPalette::BrightText, Qt::red);
        palette.setColor(QPalette::Link, QColor(64, 169, 255));
        palette.setColor(QPalette::Highlight, QColor(64, 169, 255));
        palette.setColor(QPalette::HighlightedText, Qt::white);
#if QT_VERSION >= QT_VERSION_CHECK(6, 6, 0)
        palette.setColor(QPalette::Accent, QColor(64, 169, 255));
#endif
        palette.setColor(QPalette::Mid, QColor(60, 60, 60));
        palette.setColor(QPalette::Midlight, QColor(75, 75, 75));
        palette.setColor(QPalette::Shadow, QColor(40, 40, 40));
        palette.setColor(QPalette::Dark, QColor(35, 35, 35));
        palette.setColor(QPalette::Light, QColor(80, 80, 80));
    } else {
        palette.setColor(QPalette::Window, QColor(250, 250, 250));
        palette.setColor(QPalette::WindowText, QColor(30, 30, 30));
        palette.setColor(QPalette::Base, Qt::white);
        palette.setColor(QPalette::AlternateBase, QColor(240, 240, 240));
        palette.setColor(QPalette::ToolTipBase, Qt::white);
        palette.setColor(QPalette::ToolTipText, QColor(30, 30, 30));
        palette.setColor(QPalette::Text, QColor(30, 30, 30));
        palette.setColor(QPalette::PlaceholderText, QColor(130, 130, 130));
        palette.setColor(QPalette::Disabled, QPalette::Text, Qt::gray);
        palette.setColor(QPalette::Disabled, QPalette::WindowText, Qt::gray);
        palette.setColor(QPalette::Button, QColor(240, 240, 240));
        palette.setColor(QPalette::ButtonText, QColor(30, 30, 30));
        palette.setColor(QPalette::Disabled, QPalette::ButtonText, Qt::gray);
        palette.setColor(QPalette::BrightText, Qt::red);
        palette.setColor(QPalette::Link, QColor(24, 144, 255));
        palette.setColor(QPalette::Highlight, QColor(24, 144, 255));
        palette.setColor(QPalette::HighlightedText, Qt::white);
#if QT_VERSION >= QT_VERSION_CHECK(6, 6, 0)
        palette.setColor(QPalette::Accent, QColor(24, 144, 255));
#endif
        palette.setColor(QPalette::Mid, QColor(220, 220, 220));
        palette.setColor(QPalette::Midlight, QColor(230, 230, 230));
        palette.setColor(QPalette::Shadow, QColor(200, 200, 200));
        palette.setColor(QPalette::Dark, QColor(210, 210, 210));
        palette.setColor(QPalette::Light, QColor(255, 255, 255));
    }
    return palette;
}

extern "C" {

void cs_apply_app_icon()
{
    if (QGuiApplication::instance()) {
        QIcon icon(QStringLiteral(":/icons/circuitsimulator.png"));
        if (icon.isNull())
            icon = QIcon(QStringLiteral(":/icons/circuitsimulator.ico"));
        QGuiApplication::setWindowIcon(icon);
    }
}

void cs_set_titlebar_dark(int dark)
{
#ifdef Q_OS_WIN
    s_current_theme_dark = dark ? 1 : 0;
    if (QGuiApplication::instance()) {
        ensure_filter();
        apply_dwm_all(dark != 0);
    }
#else
    (void)dark;
#endif
}

void cs_install_alt_menu_filter(int (*on_alt)(int phase))
{
    s_on_alt = on_alt;
    if (QGuiApplication::instance())
        ensure_filter();
}

int cs_invoke_direct(void* obj, const char* method)
{
    auto* qobj = static_cast<QObject*>(obj);
    if (!qobj || !method)
        return 0;
    return QMetaObject::invokeMethod(qobj, method, Qt::DirectConnection) ? 1 : 0;
}

int cs_apply_theme(const char* theme_name)
{
    if (s_in_apply_theme)
        return s_current_theme_dark >= 0 ? s_current_theme_dark : 0;

    const bool namedDark = theme_name && strcmp(theme_name, "Dark") == 0;
    const bool namedLight = theme_name && strcmp(theme_name, "Light") == 0;
    const bool namedSystem = !theme_name || strcmp(theme_name, "System") == 0
        || strcmp(theme_name, "") == 0;

    if (!QGuiApplication::instance())
        return namedDark ? 1 : 0;

    s_in_apply_theme = true;

    bool isSystemDark = false;
    if (QStyleHints* hints = QGuiApplication::styleHints()) {
        if (namedSystem)
            hints->unsetColorScheme();
        Qt::ColorScheme scheme = hints->colorScheme();
        if (scheme == Qt::ColorScheme::Dark)
            isSystemDark = true;
        else if (scheme == Qt::ColorScheme::Light)
            isSystemDark = false;
        else {
#ifdef Q_OS_WIN
            isSystemDark = windows_apps_use_dark();
#endif
        }
    } else {
#ifdef Q_OS_WIN
        isSystemDark = windows_apps_use_dark();
#endif
    }

    bool useDark = false;
    if (namedDark)
        useDark = true;
    else if (namedLight)
        useDark = false;
    else
        useDark = isSystemDark;

    // Publish before setColorScheme: that call can re-enter via QML's
    // colorSchemeChanged -> App.reloadTheme().
    s_current_theme_dark = useDark ? 1 : 0;

    if (QStyleHints* hints = QGuiApplication::styleHints()) {
        if (!namedSystem)
            hints->setColorScheme(useDark ? Qt::ColorScheme::Dark : Qt::ColorScheme::Light);
    }

    QGuiApplication::setPalette(make_palette(useDark));

#ifdef Q_OS_WIN
    ensure_filter();
    apply_dwm_all(useDark);
#elif defined(Q_OS_LINUX)
    ensure_filter();
#endif

    s_in_apply_theme = false;
    return useDark ? 1 : 0;
}

}
