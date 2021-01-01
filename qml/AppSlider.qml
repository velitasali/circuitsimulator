import QtQuick
import QtQuick.Controls
import QtQuick.Templates as T
import cs_app

/* Template Slider, not the platform style.
 *
 * Ensures consistent track and thumb rendering across all operating systems
 * and respects the active application theme.
 */
T.Slider {
    id: control
    SystemPalette { id: appTheme }

    implicitWidth: Math.max(implicitBackgroundWidth + leftInset + rightInset,
                            implicitHandleWidth + leftPadding + rightPadding)
    implicitHeight: Math.max(implicitBackgroundHeight + topInset + bottomInset,
                             implicitHandleHeight + topPadding + bottomPadding)

    padding: 6
    hoverEnabled: true
    opacity: control.enabled ? 1.0 : 0.4

    palette.window: appTheme.window
    palette.windowText: appTheme.windowText
    palette.base: appTheme.base
    palette.text: appTheme.text
    palette.highlight: appTheme.highlight
    palette.highlightedText: appTheme.highlightedText
    palette.mid: appTheme.mid
    palette.midlight: appTheme.midlight
    palette.button: appTheme.button
    palette.buttonText: appTheme.buttonText

    background: Rectangle {
        x: control.leftPadding + (control.horizontal ? 0 : (control.availableWidth - width) / 2)
        y: control.topPadding + (control.horizontal ? (control.availableHeight - height) / 2 : 0)
        implicitWidth: control.horizontal ? 200 : 4
        implicitHeight: control.horizontal ? 4 : 200
        width: control.horizontal ? control.availableWidth : implicitWidth
        height: control.horizontal ? implicitHeight : control.availableHeight
        radius: 2
        color: appTheme.midlight

        Rectangle {
            x: control.horizontal ? (control.mirrored ? parent.width - width : 0) : 0
            y: control.horizontal ? 0 : (control.mirrored ? 0 : parent.height - height)
            width: control.horizontal ? control.position * parent.width : parent.width
            height: control.horizontal ? parent.height : control.position * parent.height
            radius: 2
            color: control.enabled ? appTheme.highlight : appTheme.mid
        }
    }

    handle: Rectangle {
        x: control.leftPadding + (control.horizontal ? control.visualPosition * (control.availableWidth - width) : (control.availableWidth - width) / 2)
        y: control.topPadding + (control.horizontal ? (control.availableHeight - height) / 2 : control.visualPosition * (control.availableHeight - height))
        implicitWidth: 16
        implicitHeight: 16
        radius: 8
        color: control.pressed ? appTheme.highlight
             : (control.hovered ? Qt.lighter(appTheme.highlight, 1.1) : appTheme.base)
        border.width: 2
        border.color: control.enabled ? appTheme.highlight : appTheme.mid

        Behavior on color { ColorAnimation { duration: 90 } }
    }
}
