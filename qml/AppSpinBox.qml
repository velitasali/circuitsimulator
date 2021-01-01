import QtQuick
import QtQuick.Controls
import QtQuick.Templates as T
import cs_app

/* Template SpinBox, not the platform style. See AppTextField. */
T.SpinBox {
    id: control
    SystemPalette { id: appTheme }

    implicitWidth: Math.max(implicitBackgroundWidth + leftInset + rightInset,
                            contentItem.implicitWidth + leftPadding + rightPadding)
    implicitHeight: Math.max(implicitBackgroundHeight + topInset + bottomInset,
                             implicitContentHeight + topPadding + bottomPadding)

    font: Qt.font({ family: App.fontFamily, pixelSize: App.fontSize })
    padding: 4
    leftPadding: 6
    rightPadding: 18
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

    textFromValue: function(value, locale) {
        return Number(value).toString()
    }

    valueFromText: function(text, locale) {
        var clean = ("" + text).replace(/[^\d\-+]/g, "")
        var val = parseInt(clean, 10)
        return isNaN(val) ? control.from : val
    }

    validator: IntValidator {
        locale: control.locale.name
        bottom: Math.min(control.from, control.to)
        top: Math.max(control.from, control.to)
    }

    contentItem: TextInput {
        id: textInput
        z: 2
        text: control.displayText
        font: control.font
        color: appTheme.text
        selectionColor: appTheme.highlight
        selectedTextColor: appTheme.highlightedText
        horizontalAlignment: Qt.AlignLeft
        verticalAlignment: Qt.AlignVCenter
        readOnly: !control.editable
        validator: control.validator
        inputMethodHints: control.inputMethodHints
        selectByMouse: true
        clip: true
        ContextMenu.menu: AppTextContextMenu { editor: parent }
    }

    up.indicator: Item {
        x: control.mirrored ? 2 : control.width - width - 2
        y: 0
        width: 14
        height: Math.floor(control.height / 2)

        AppIcon {
            anchors.centerIn: parent
            anchors.verticalCenterOffset: 2
            text: "arrow_drop_up"
            font.pixelSize: 12
            color: appTheme.windowText
            opacity: control.up.pressed ? 0.5 : 1
        }
    }

    down.indicator: Item {
        x: control.mirrored ? 2 : control.width - width - 2
        y: Math.floor(control.height / 2)
        width: 14
        height: Math.ceil(control.height / 2)

        AppIcon {
            anchors.centerIn: parent
            anchors.verticalCenterOffset: -2
            text: "arrow_drop_down"
            font.pixelSize: 12
            color: appTheme.windowText
            opacity: control.down.pressed ? 0.5 : 1
        }
    }

    background: Rectangle {
        implicitWidth: 80
        implicitHeight: 24
        radius: 4
        color: control.enabled ? appTheme.base : appTheme.window
        border.width: 1
        border.color: control.activeFocus ? appTheme.highlight
                    : (control.enabled ? appTheme.mid : appTheme.midlight)
    }
}
