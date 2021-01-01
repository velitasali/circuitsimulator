import QtQuick
import QtQuick.Controls
import QtQuick.Controls.impl
import QtQuick.Templates as T
import cs_app

/* Template Button, not the platform style.
 *
 * Windows Quick Controls paints a native push button (always light) and
 * colors the label with palette.buttonText. In dark theme that is nearly
 * white, so the control reads as an all-white rectangle.
 */
T.Button {
    id: control
    SystemPalette { id: appTheme }

    font: Qt.font({ family: App.fontFamily, pixelSize: App.fontSize })
    padding: 6
    leftPadding: 12
    rightPadding: 12
    spacing: 6
    hoverEnabled: true
    opacity: control.enabled ? 1.0 : 0.4

    palette.window: appTheme.window
    palette.windowText: appTheme.windowText
    palette.button: appTheme.button
    palette.buttonText: appTheme.buttonText
    palette.highlight: appTheme.highlight
    palette.highlightedText: appTheme.highlightedText
    palette.mid: appTheme.mid
    palette.midlight: appTheme.midlight
    palette.dark: appTheme.dark

    implicitWidth: Math.max(implicitBackgroundWidth + leftInset + rightInset,
                            implicitContentWidth + leftPadding + rightPadding)
    implicitHeight: Math.max(implicitBackgroundHeight + topInset + bottomInset,
                             implicitContentHeight + topPadding + bottomPadding)

    contentItem: IconLabel {
        spacing: control.spacing
        mirrored: control.mirrored
        display: control.display
        icon: control.icon
        text: control.text
        font: control.font
        color: control.highlighted ? appTheme.highlightedText : appTheme.buttonText
    }

    background: Rectangle {
        implicitWidth: 80
        implicitHeight: 24
        visible: !control.flat || control.down || control.checked || control.highlighted
        radius: 4
        color: {
            if (control.highlighted)
                return control.down ? Qt.darker(appTheme.highlight, 1.15)
                     : (control.hovered ? Qt.lighter(appTheme.highlight, 1.08) : appTheme.highlight)
            if (control.down)
                return appTheme.dark
            if (control.hovered)
                return appTheme.midlight
            return appTheme.button
        }
        border.width: 1
        border.color: control.highlighted ? Qt.darker(appTheme.highlight, 1.2)
                    : (control.visualFocus ? appTheme.highlight : appTheme.mid)
    }
}
