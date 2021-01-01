import QtQuick
import QtQuick.Controls
import QtQuick.Controls.impl
import QtQuick.Templates as T
import cs_app

/* Template TextField, not the platform style.
 *
 * Windows Quick Controls paints a native EDIT (always light) and ignores
 * QGuiApplication::setPalette / dark theme. macOS native fields do follow
 * the palette, which is why this only looked wrong on Windows.
 */
T.TextField {
    id: control
    SystemPalette { id: appTheme }

    font: Qt.font({ family: App.fontFamily, pixelSize: App.fontSize })
    color: appTheme.text
    selectionColor: appTheme.highlight
    selectedTextColor: appTheme.highlightedText
    placeholderTextColor: appTheme.placeholderText
    verticalAlignment: TextInput.AlignVCenter
    padding: 4
    leftPadding: 6
    rightPadding: 6
    opacity: control.enabled ? 1.0 : 0.4

    palette.window: appTheme.window
    palette.windowText: appTheme.windowText
    palette.base: appTheme.base
    palette.text: appTheme.text
    palette.button: appTheme.button
    palette.buttonText: appTheme.buttonText
    palette.highlight: appTheme.highlight
    palette.highlightedText: appTheme.highlightedText
    palette.mid: appTheme.mid
    palette.midlight: appTheme.midlight

    ContextMenu.menu: AppTextContextMenu { editor: control }

    implicitWidth: implicitBackgroundWidth + leftInset + rightInset
                   || Math.max(contentWidth, placeholder.implicitWidth) + leftPadding + rightPadding
    implicitHeight: Math.max(implicitBackgroundHeight + topInset + bottomInset,
                             contentHeight + topPadding + bottomPadding,
                             placeholder.implicitHeight + topPadding + bottomPadding)

    PlaceholderText {
        id: placeholder
        x: control.leftPadding
        y: control.topPadding
        width: control.width - (control.leftPadding + control.rightPadding)
        height: control.height - (control.topPadding + control.bottomPadding)
        text: control.placeholderText
        font: control.font
        color: control.placeholderTextColor
        verticalAlignment: control.verticalAlignment
        visible: !control.length && !control.preeditText && (!control.activeFocus || control.horizontalAlignment !== Qt.AlignHCenter)
        elide: Text.ElideRight
        renderType: control.renderType
    }

    background: Rectangle {
        implicitWidth: 120
        implicitHeight: 24
        radius: 4
        color: control.enabled ? appTheme.base : appTheme.window
        border.width: 1
        border.color: control.activeFocus ? appTheme.highlight
                    : (control.enabled ? appTheme.mid : appTheme.midlight)
    }
}
