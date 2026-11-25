pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Effects
import cs_app

/* Elevated floating container with a theme-tinted hairline border and an
 * opt-in two-layer drop shadow (`shadowEnabled: true`, used for menus and
 * dropdown popups).
 *
 * Compact floating surfaces (toolbars, menus, popups, tooltips, cards) default
 * to `appTheme.base`. Larger overlay panels that contain inset `appTheme.base`
 * cards can set `surfaceColor: appTheme.window` while keeping the same border
 * treatment.
 */
Rectangle {
    id: surface
    SystemPalette { id: appTheme }

    property color surfaceColor: appTheme.base
    property real elevation: 1.0
    property bool shadowEnabled: false

    color: surfaceColor
    border.color: Qt.tint(
        surfaceColor,
        Qt.alpha( appTheme.windowText, App.darkTheme ? 0.12 : 0.11 )
    )
    border.width: 1
    antialiasing: true

    // Soft ambient shadow for elevation
    RectangularShadow {
        anchors.fill: parent
        z: -2
        visible: surface.shadowEnabled && surface.width > 0 && surface.height > 0
        radius: surface.radius
        blur: Math.round( 22 * surface.elevation )
        spread: -1
        offset: Qt.vector2d( 0, Math.round( 5 * surface.elevation ) )
        color: Qt.alpha( App.windowShadow, App.darkTheme ? 0.45 : 0.14 )
    }

    // Tight contact shadow anchoring the container edge
    RectangularShadow {
        anchors.fill: parent
        z: -1
        visible: surface.shadowEnabled && surface.width > 0 && surface.height > 0
        radius: surface.radius
        blur: Math.max( 4, Math.round( 6 * surface.elevation ) )
        spread: 0
        offset: Qt.vector2d( 0, 1 )
        color: Qt.alpha( App.windowShadow, App.darkTheme ? 0.36 : 0.10 )
    }
}
