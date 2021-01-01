#import <AppKit/AppKit.h>
#include <QtGui/QGuiApplication>
#include <QtGui/QPalette>
#include <QtGui/QColor>
#include <QtGui/QStyleHints>
#include <string.h>
#include <dlfcn.h>

#include "cs_macos.h"

typedef short (*CPSSetProcessNameFn)(ProcessSerialNumber* psn, const char* name);
typedef short (*GetCurrentProcessFn)(ProcessSerialNumber* psn);

void cs_macos_init_process(const char* name) {
    [NSApplication sharedApplication];
    [NSApp setActivationPolicy:NSApplicationActivationPolicyRegular];
    // Ensure Qt terminates when the last window closes on macOS.
    QGuiApplication::setQuitOnLastWindowClosed(true);

    if (name && strlen(name) > 0) {
        [[NSProcessInfo processInfo] setProcessName:[NSString stringWithUTF8String:name]];
        CPSSetProcessNameFn setProcessName = (CPSSetProcessNameFn)dlsym(RTLD_DEFAULT, "CPSSetProcessName");
        GetCurrentProcessFn getProcess = (GetCurrentProcessFn)dlsym(RTLD_DEFAULT, "GetCurrentProcess");
        if (setProcessName && getProcess) {
            ProcessSerialNumber psn = { 0, 0 };
            if (getProcess(&psn) == 0) {
                setProcessName(&psn, name);
            }
        }
    }
}

void cs_macos_setup_window(void) {
    [NSApp setActivationPolicy:NSApplicationActivationPolicyRegular];
    [NSApp activateIgnoringOtherApps:YES];
    for (NSWindow* w in NSApp.windows) {
        if ([w respondsToSelector:@selector(setFrameAutosaveName:)]) {
            [w setFrameAutosaveName:@"CircuitSimulatorMainWindow"];
        }
        [w makeKeyAndOrderFront:nil];
    }
}

void cs_macos_set_titlebar_dark(int dark) {
    NSAppearanceName name = dark ? NSAppearanceNameDarkAqua : NSAppearanceNameAqua;
    NSAppearance* appearance = [NSAppearance appearanceNamed:name];
    for (NSWindow* w in NSApp.windows) {
        if (w.appearance != appearance) {
            w.appearance = appearance;
        }
    }
}

void cs_macos_beep(void) {
    NSBeep();
}

static int s_current_theme_dark = -1;
static bool s_in_apply_theme = false;

int cs_macos_apply_theme(const char* theme_name) {
    if (s_in_apply_theme) {
        return s_current_theme_dark >= 0 ? s_current_theme_dark : 0;
    }
    s_in_apply_theme = true;
    bool isSystemDark = false;
    if (QGuiApplication::instance() && QGuiApplication::styleHints()) {
        if (!theme_name || strcmp(theme_name, "System") == 0 || strcmp(theme_name, "") == 0) {
            QGuiApplication::styleHints()->unsetColorScheme();
        }
        isSystemDark = (QGuiApplication::styleHints()->colorScheme() == Qt::ColorScheme::Dark);
    } else if (NSApp) {
        NSAppearance* appAppearance = [NSApp effectiveAppearance];
        NSAppearanceName bestMatch = [appAppearance bestMatchFromAppearancesWithNames:@[NSAppearanceNameAqua, NSAppearanceNameDarkAqua]];
        isSystemDark = [bestMatch isEqualToString:NSAppearanceNameDarkAqua];
    }

    bool useDark = false;
    if (theme_name && strcmp(theme_name, "Dark") == 0) {
        useDark = true;
    } else if (theme_name && strcmp(theme_name, "Light") == 0) {
        useDark = false;
    } else {
        useDark = isSystemDark;
    }

    if (QGuiApplication::instance()) {
        if (QGuiApplication::styleHints()) {
            if (theme_name && strcmp(theme_name, "System") != 0) {
                QGuiApplication::styleHints()->setColorScheme(useDark ? Qt::ColorScheme::Dark : Qt::ColorScheme::Light);
            }
        }

        QPalette palette;
        if (useDark) {
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
            palette.setColor(QPalette::Mid, QColor(220, 220, 220));
            palette.setColor(QPalette::Midlight, QColor(230, 230, 230));
            palette.setColor(QPalette::Shadow, QColor(200, 200, 200));
            palette.setColor(QPalette::Dark, QColor(210, 210, 210));
            palette.setColor(QPalette::Light, QColor(255, 255, 255));
        }
        QGuiApplication::setPalette(palette);
    }

    if (NSApp) {
        cs_macos_set_titlebar_dark(useDark ? 1 : 0);
    }

    s_current_theme_dark = useDark ? 1 : 0;
    s_in_apply_theme = false;
    return useDark ? 1 : 0;
}
