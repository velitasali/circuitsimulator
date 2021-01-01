import QtQuick
import QtQuick.Controls
import QtQuick.Templates as T
import cs_app

/* Template Switch, not the platform style.
 *
 * Provides a clean pill-shaped toggle switch with theme-aware styling.
 */
T.Switch {
    id: control
    SystemPalette { id: appTheme }

    font: Qt.font({ family: App.fontFamily, pixelSize: App.fontSize })
    padding: 4
    spacing: 8
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

    implicitWidth: Math.max(implicitBackgroundWidth + leftInset + rightInset,
                            implicitContentWidth + leftPadding + rightPadding)
    implicitHeight: Math.max(implicitBackgroundHeight + topInset + bottomInset,
                             implicitContentHeight + topPadding + bottomPadding,
                             implicitIndicatorHeight + topPadding + bottomPadding)

    indicator: Rectangle {
        implicitWidth: 36
        implicitHeight: 20
        x: control.text ? (control.mirrored ? control.width - width - control.rightPadding : control.leftPadding)
                        : control.leftPadding + (control.availableWidth - width) / 2
        y: control.topPadding + (control.availableHeight - height) / 2
        radius: 10
        color: control.checked ? appTheme.highlight
             : (control.hovered ? appTheme.mid : appTheme.midlight)
        border.width: 1
        border.color: control.visualFocus ? appTheme.highlight : "transparent"

        Behavior on color { ColorAnimation { duration: 120 } }

        Rectangle {
            id: thumb
            x: control.checked ? parent.width - width - 2 : 2
            y: 2
            width: 16
            height: 16
            radius: 8
            color: control.checked ? appTheme.highlightedText : appTheme.base

            Behavior on x { NumberAnimation { duration: 120; easing.type: Easing.OutQuad } }
            Behavior on color { ColorAnimation { duration: 120 } }
        }
    }

    contentItem: Text {
        text: control.text
        font: control.font
        color: appTheme.windowText
        elide: Text.ElideRight
        verticalAlignment: Text.AlignVCenter
        leftPadding: control.indicator && !control.mirrored ? control.indicator.width + control.spacing : 0
        rightPadding: control.indicator && control.mirrored ? control.indicator.width + control.spacing : 0
    }
}
