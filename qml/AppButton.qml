import QtQuick
import QtQuick.Controls.impl
import QtQuick.Templates as T
import cs_app

/* Template Button, not the platform style.
 *
 * Windows Quick Controls paints a native push button (always light) and
 * colors the label with palette.buttonText. In dark theme that is nearly
 * white, so the control reads as an all-white rectangle.
 *
 * Borderless tonal fill differentiates push buttons from bordered input
 * controls (AppTextField, AppComboBox, AppSpinBox) and matches AppToolButton,
 * AppTabButton, and AppSwitch.
 */
T.Button {
    id: control
    SystemPalette { id: appTheme }

    font: Qt.font({ family: App.fontFamily, pixelSize: App.fontSize, weight: Font.Medium })
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
        font: control.font.weight === Font.Normal
              ? Qt.font({
                    family: control.font.family,
                    pixelSize: control.font.pixelSize > 0 ? control.font.pixelSize : App.fontSize,
                    weight: Font.Medium,
                    italic: control.font.italic
                })
              : control.font
        color: control.highlighted ? control.palette.highlightedText : control.palette.buttonText
    }

    background: Rectangle {
        implicitWidth: 80
        implicitHeight: 24
        visible: !control.flat || control.down || control.checked || control.highlighted
        radius: 8
        color: {
            if (control.highlighted)
                return control.down ? Qt.darker(appTheme.highlight, 1.15)
                     : (control.hovered ? Qt.lighter(appTheme.highlight, 1.08) : appTheme.highlight)
            if (control.down)
                return appTheme.dark
            if (control.hovered)
                return App.darkTheme ? appTheme.midlight : appTheme.mid
            return appTheme.button
        }
        border.width: control.visualFocus ? 1 : 0
        border.color: control.highlighted
                      ? (App.darkTheme ? Qt.lighter(appTheme.highlight, 1.35) : Qt.darker(appTheme.highlight, 1.35))
                      : appTheme.highlight

        Behavior on color { ColorAnimation { duration: 90 } }
    }
}
