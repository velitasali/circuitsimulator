import QtQuick
import QtQuick.Controls
import QtQuick.Templates as T
import cs_app

/* Template CheckBox, not the platform style.
 *
 * Windows Quick Controls paints a native checkbox and binds the caption to
 * palette.windowText from the style's light palette, so the text stays black
 * in dark theme.
 */
T.CheckBox {
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
    palette.button: appTheme.button
    palette.buttonText: appTheme.buttonText

    implicitWidth: Math.max(implicitBackgroundWidth + leftInset + rightInset,
                            implicitContentWidth + leftPadding + rightPadding)
    implicitHeight: Math.max(implicitBackgroundHeight + topInset + bottomInset,
                             implicitContentHeight + topPadding + bottomPadding,
                             implicitIndicatorHeight + topPadding + bottomPadding)

    indicator: Rectangle {
        implicitWidth: 16
        implicitHeight: 16
        x: control.text ? (control.mirrored ? control.width - width - control.rightPadding : control.leftPadding)
                        : control.leftPadding + (control.availableWidth - width) / 2
        y: control.topPadding + (control.availableHeight - height) / 2
        radius: 3
        color: control.checkState !== Qt.Unchecked ? appTheme.highlight
             : (control.enabled ? appTheme.base : appTheme.window)
        border.width: 1
        border.color: control.checkState !== Qt.Unchecked || control.hovered || control.visualFocus
                      ? appTheme.highlight : appTheme.mid

        AppIcon {
            anchors.centerIn: parent
            text: "check"
            font.pixelSize: 14
            color: appTheme.highlightedText
            visible: control.checkState === Qt.Checked
        }
        Rectangle {
            anchors.centerIn: parent
            width: 8
            height: 2
            radius: 1
            color: appTheme.highlightedText
            visible: control.checkState === Qt.PartiallyChecked
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
