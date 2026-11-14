#include "theme.h"

#include <cstring>

#include <QtCore/QEvent>
#include <QtCore/QMetaObject>
#include <QtCore/QtGlobal>
#include <QtGui/QColor>
#include <QtGui/QGuiApplication>
#include <QtGui/QIcon>
#include <QtGui/QKeyEvent>
#include <QtGui/QPalette>
#include <QtGui/QStyleHints>
#include <QtGui/QWindow>

#ifdef Q_OS_WIN
#include <QtCore/QSettings>
#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif
#include <windows.h>
#include <dwmapi.h>
#endif

static int s_current_theme_dark = -1;
static bool s_in_apply_theme = false;
static int (*s_on_alt)(int phase) = nullptr;

#ifdef Q_OS_WIN
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
        if (type == QEvent::Show || type == QEvent::WinIdChange) {
            if (s_current_theme_dark >= 0)
                apply_dwm(w, s_current_theme_dark != 0);
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
#endif

    s_in_apply_theme = false;
    return useDark ? 1 : 0;
}

}
