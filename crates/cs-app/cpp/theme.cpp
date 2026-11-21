#include "theme.h"

#include <QtCore/QCoreApplication>
#include <QtCore/QMetaObject>
#include <QtCore/QtGlobal>
#include <QtGui/QColor>
#include <QtGui/QGuiApplication>
#include <QtGui/QPalette>
#include <QtGui/QStyleHints>

#ifndef Q_OS_MACOS
#include <QtCore/QEvent>
#include <QtCore/QPointer>
#include <QtGui/QIcon>
#include <QtGui/QKeyEvent>
#include <QtGui/QPlatformSurfaceEvent>
#include <QtGui/QWindow>
#endif

#ifdef Q_OS_LINUX
#include <QtCore/QMargins>
#include <QtGui/QRegion>
#if QT_CONFIG(wayland)
#include <qpa/qplatformwindow_p.h>
#endif
#endif

#ifdef Q_OS_WIN
#include <QtCore/QAbstractNativeEventFilter>
#include <QtCore/QHash>
#include <QtQuick/QQuickItem>
#include <QtQuick/QQuickWindow>
#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif
#include <windows.h>

extern "C" {
void cs_win_register_chrome(uintptr_t hwnd);
void cs_win_unregister_chrome(uintptr_t hwnd);
void cs_win_apply_dwm(uintptr_t hwnd, int dark, int refresh_frame);
void cs_win_forget_window(uintptr_t hwnd);
int cs_win_native_event(uintptr_t hwnd, uint32_t message, uintptr_t w_param, intptr_t l_param,
                        intptr_t* out_result);
}
#endif

static QColor from_rgb(uint32_t rgb)
{
    return QColor((rgb >> 16) & 0xFF, (rgb >> 8) & 0xFF, rgb & 0xFF);
}

#ifndef Q_OS_MACOS
static int (*s_on_alt)(int phase) = nullptr;
#endif

#ifdef Q_OS_WIN
static int s_current_theme_dark = -1;
static QHash<uintptr_t, QPointer<QWindow>> s_main_windows;
static QHash<uintptr_t, QPointer<QWindow>> s_themed_windows;

static bool is_main_window(QWindow* w)
{
    return w && w->transientParent() == nullptr
        && w->objectName() == QStringLiteral("mainWindow");
}

static bool is_themed_quick_window(QWindow* w)
{
    if (!w)
        return false;
    const Qt::WindowType wt = w->type();
    if (wt != Qt::Window && wt != Qt::Dialog)
        return false;
    return qobject_cast<QQuickWindow*>(w) != nullptr;
}

static void prune_themed_windows_for(QWindow* w)
{
    for (auto it = s_themed_windows.begin(); it != s_themed_windows.end();) {
        if (it.value().isNull() || it.value().data() == w) {
            it = s_themed_windows.erase(it);
        } else {
            ++it;
        }
    }
}

static void register_themed_window(QWindow* w)
{
    if (!is_themed_quick_window(w) || !w->handle())
        return;
    const uintptr_t hwnd = static_cast<uintptr_t>(w->winId());
    if (!hwnd)
        return;
    auto existing = s_themed_windows.constFind(hwnd);
    if (existing == s_themed_windows.constEnd() || existing.value().data() != w) {
        prune_themed_windows_for(w);
        s_themed_windows.insert(hwnd, w);
    }
}

static void prune_main_windows_for(QWindow* w)
{
    for (auto it = s_main_windows.begin(); it != s_main_windows.end();) {
        if (it.value().isNull() || it.value().data() == w) {
            cs_win_unregister_chrome(it.key());
            it = s_main_windows.erase(it);
        } else {
            ++it;
        }
    }
}

static void register_main_window(QWindow* w)
{
    if (!is_main_window(w))
        return;
    const uintptr_t hwnd = static_cast<uintptr_t>(w->winId());
    if (!hwnd)
        return;
    auto existing = s_main_windows.constFind(hwnd);
    if (existing == s_main_windows.constEnd() || existing.value().data() != w) {
        prune_main_windows_for(w);
        s_main_windows.insert(hwnd, w);
    }
    cs_win_register_chrome(hwnd);
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

extern "C" int cs_win_hit_test_titlebar(uintptr_t hwnd, int client_x, int client_y,
                                        int* out_over_title, int* out_over_control)
{
    if (out_over_title)
        *out_over_title = 0;
    if (out_over_control)
        *out_over_control = 0;

    auto it = s_main_windows.constFind(hwnd);
    if (it == s_main_windows.constEnd() || it.value().isNull())
        return 0;

    QWindow* win = it.value().data();
    auto* qw = qobject_cast<QQuickWindow*>(win);
    if (!qw)
        return 0;

    QQuickItem* titleBar = find_quick_item(qw->contentItem(), QStringLiteral("titleBar"));
    if (!titleBar)
        titleBar = qw->findChild<QQuickItem*>(QStringLiteral("titleBar"));
    if (!titleBar || !titleBar->isVisible())
        return 0;

    const qreal dpr = win->devicePixelRatio() > 0.0 ? win->devicePixelRatio() : 1.0;
    const QPointF scenePt(client_x / dpr, client_y / dpr);
    const QPointF local = titleBar->mapFromScene(scenePt);
    if (local.x() >= 0.0 && local.y() >= 0.0 && local.x() < titleBar->width()
        && local.y() < titleBar->height()) {
        if (out_over_title)
            *out_over_title = 1;
        if (out_over_control)
            *out_over_control = hit_interactive_descendant(titleBar, scenePt) ? 1 : 0;
        return 1;
    }
    return 0;
}

static void fill_window_background(HWND hwnd, HDC hdc)
{
    if (!hwnd || !hdc)
        return;
    RECT rc;
    if (!GetClientRect(hwnd, &rc) || rc.right <= rc.left || rc.bottom <= rc.top)
        return;
    const QColor bg = QGuiApplication::palette().color(QPalette::Window);
    HBRUSH brush = CreateSolidBrush(RGB(bg.red(), bg.green(), bg.blue()));
    if (!brush)
        return;
    FillRect(hdc, &rc, brush);
    DeleteObject(brush);
}

static void paint_window_background(HWND hwnd)
{
    if (HDC hdc = GetDC(hwnd)) {
        fill_window_background(hwnd, hdc);
        ReleaseDC(hwnd, hdc);
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
        const uintptr_t hwnd = reinterpret_cast<uintptr_t>(msg->hwnd);

        if (msg->message == WM_ERASEBKGND || (msg->message == WM_SHOWWINDOW && msg->wParam)) {
            auto themedIt = s_themed_windows.find(hwnd);
            if (themedIt != s_themed_windows.end()) {
                if (themedIt.value().isNull()) {
                    s_themed_windows.erase(themedIt);
                } else if (msg->message == WM_ERASEBKGND) {
                    fill_window_background(msg->hwnd, reinterpret_cast<HDC>(msg->wParam));
                    *result = 1;
                    return true;
                } else {
                    paint_window_background(msg->hwnd);
                }
            }
        }

        auto it = s_main_windows.find(hwnd);
        if (it == s_main_windows.end())
            return false;
        if (it.value().isNull()) {
            cs_win_unregister_chrome(hwnd);
            s_main_windows.erase(it);
            return false;
        }

        intptr_t out = 0;
        if (cs_win_native_event(hwnd, msg->message, static_cast<uintptr_t>(msg->wParam),
                                static_cast<intptr_t>(msg->lParam), &out)) {
            *result = static_cast<qintptr>(out);
            return true;
        }
        return false;
    }
};

static void apply_dwm_all(bool dark)
{
    const auto windows = QGuiApplication::topLevelWindows();
    for (QWindow* w : windows) {
        if (w && w->handle()) {
            cs_win_apply_dwm(static_cast<uintptr_t>(w->winId()), dark ? 1 : 0,
                             w->isVisible() ? 1 : 0);
        }
    }
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

#ifndef Q_OS_MACOS
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
        if (type == QEvent::ApplicationDeactivate || type == QEvent::Shortcut) {
            disarmAlt();
            return QObject::eventFilter(obj, event);
        }
        if (type == QEvent::ShortcutOverride) {
            auto* ke = static_cast<QKeyEvent*>(event);
            if (ke->key() != Qt::Key_Alt && ke->key() != Qt::Key_Meta) {
                disarmAlt();
            } else if (s_on_alt && s_on_alt(0)) {
                return true;
            }
            return QObject::eventFilter(obj, event);
        }

        auto* w = qobject_cast<QWindow*>(obj);
        if (!w)
            return QObject::eventFilter(obj, event);

#ifdef Q_OS_WIN
        if (type == QEvent::PlatformSurface) {
            auto* se = static_cast<QPlatformSurfaceEvent*>(event);
            if (se->surfaceEventType() == QPlatformSurfaceEvent::SurfaceCreated) {
                register_main_window(w);
                register_themed_window(w);
                if (s_current_theme_dark >= 0 && w->handle()) {
                    cs_win_apply_dwm(static_cast<uintptr_t>(w->winId()), s_current_theme_dark, 0);
                }
            } else if (se->surfaceEventType() == QPlatformSurfaceEvent::SurfaceAboutToBeDestroyed) {
                if (w->handle())
                    cs_win_forget_window(static_cast<uintptr_t>(w->winId()));
                prune_themed_windows_for(w);
                prune_main_windows_for(w);
            }
        } else if (type == QEvent::Show) {
            register_main_window(w);
            register_themed_window(w);
            if (s_current_theme_dark >= 0 && w->handle()) {
                const uintptr_t hwnd = static_cast<uintptr_t>(w->winId());
                cs_win_apply_dwm(hwnd, s_current_theme_dark, 0);
                if (is_themed_quick_window(w))
                    paint_window_background(reinterpret_cast<HWND>(hwnd));
            }
        } else if (type == QEvent::WinIdChange) {
            register_main_window(w);
            register_themed_window(w);
            if (s_current_theme_dark >= 0 && w->handle()) {
                cs_win_apply_dwm(static_cast<uintptr_t>(w->winId()), s_current_theme_dark,
                                 w->isVisible() ? 1 : 0);
            }
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
            const Qt::KeyboardModifiers otherMods =
                ke->modifiers() & (Qt::ControlModifier | Qt::ShiftModifier | Qt::MetaModifier);
            const bool bareAlt = ke->key() == Qt::Key_Alt && otherMods == Qt::NoModifier;
            if (!ke->isAutoRepeat()) {
                m_altArmed = bareAlt;
                m_closeOnRelease = false;
                // Eat the press only while the menu is open so a press on the
                // canvas still reaches Alt-click.
                if (bareAlt && s_on_alt && s_on_alt(0)) {
                    m_closeOnRelease = true;
                    return true;
                }
            } else if (bareAlt && s_on_alt && s_on_alt(0)) {
                return true;
            } else if (!bareAlt) {
                disarmAlt();
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
#endif

extern "C" {

int cs_qt_query_system_dark(int unset_scheme)
{
    if (!QGuiApplication::instance())
        return -2;
    QStyleHints* hints = QGuiApplication::styleHints();
    if (!hints)
        return -1;
    if (unset_scheme)
        hints->unsetColorScheme();
    const Qt::ColorScheme scheme = hints->colorScheme();
    if (scheme == Qt::ColorScheme::Dark)
        return 1;
    if (scheme == Qt::ColorScheme::Light)
        return 0;
    return -1;
}

void cs_qt_apply_palette(int scheme_mode, const CsQtPalette* pal)
{
    if (!QGuiApplication::instance() || !pal)
        return;

    if (QStyleHints* hints = QGuiApplication::styleHints()) {
        if (scheme_mode == 1)
            hints->setColorScheme(Qt::ColorScheme::Light);
        else if (scheme_mode == 2)
            hints->setColorScheme(Qt::ColorScheme::Dark);
    }

    QPalette palette;
    palette.setColor(QPalette::Window, from_rgb(pal->window));
    palette.setColor(QPalette::WindowText, from_rgb(pal->window_text));
    palette.setColor(QPalette::Base, from_rgb(pal->base));
    palette.setColor(QPalette::AlternateBase, from_rgb(pal->alternate_base));
    palette.setColor(QPalette::ToolTipBase, from_rgb(pal->tooltip_base));
    palette.setColor(QPalette::ToolTipText, from_rgb(pal->tooltip_text));
    palette.setColor(QPalette::Text, from_rgb(pal->text));
    palette.setColor(QPalette::PlaceholderText, from_rgb(pal->placeholder_text));
    palette.setColor(QPalette::Disabled, QPalette::Text, from_rgb(pal->disabled_text));
    palette.setColor(QPalette::Disabled, QPalette::WindowText, from_rgb(pal->disabled_window_text));
    palette.setColor(QPalette::Button, from_rgb(pal->button));
    palette.setColor(QPalette::ButtonText, from_rgb(pal->button_text));
    palette.setColor(QPalette::Disabled, QPalette::ButtonText, from_rgb(pal->disabled_button_text));
    palette.setColor(QPalette::BrightText, from_rgb(pal->bright_text));
    palette.setColor(QPalette::Link, from_rgb(pal->link));
    palette.setColor(QPalette::Highlight, from_rgb(pal->highlight));
    palette.setColor(QPalette::HighlightedText, from_rgb(pal->highlighted_text));
#if QT_VERSION >= QT_VERSION_CHECK(6, 6, 0)
    palette.setColor(QPalette::Accent, from_rgb(pal->accent));
#endif
    palette.setColor(QPalette::Mid, from_rgb(pal->mid));
    palette.setColor(QPalette::Midlight, from_rgb(pal->midlight));
    palette.setColor(QPalette::Shadow, from_rgb(pal->shadow));
    palette.setColor(QPalette::Dark, from_rgb(pal->dark));
    palette.setColor(QPalette::Light, from_rgb(pal->light));
    QGuiApplication::setPalette(palette);
}

void cs_qt_post_on_main(void (*cb)())
{
    if (!cb)
        return;
    QCoreApplication* app = QCoreApplication::instance();
    if (!app) {
        cb();
        return;
    }
    QMetaObject::invokeMethod(
        app,
        [cb]() { cb(); },
        Qt::QueuedConnection);
}

#ifndef Q_OS_MACOS
void cs_apply_app_icon()
{
    if (QGuiApplication::instance()) {
        QIcon icon(QStringLiteral(":/icons/circuitsimulator.png"));
        if (icon.isNull())
            icon = QIcon(QStringLiteral(":/icons/circuitsimulator.ico"));
        QGuiApplication::setWindowIcon(icon);
    }
}

void cs_ensure_theme_filter()
{
    if (QGuiApplication::instance())
        ensure_filter();
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
#endif

} // extern "C"
