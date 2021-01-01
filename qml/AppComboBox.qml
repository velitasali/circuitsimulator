pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Window
import QtQuick.Controls
import QtQuick.Templates as T
import cs_app

/* Template ComboBox, not the platform style.
 *
 * Windows Quick Controls paints a native combobox (always light) and ignores
 * the app palette / dark theme. The popup delegate also hardcodes label color
 * on macOS, so both the chrome and the row text are drawn here.
 */
T.ComboBox {
    id: control
    SystemPalette { id: appTheme }

    implicitWidth: Math.max(implicitBackgroundWidth + leftInset + rightInset,
                            implicitContentWidth + leftPadding + rightPadding)
    implicitHeight: Math.max(implicitBackgroundHeight + topInset + bottomInset,
                             implicitContentHeight + topPadding + bottomPadding,
                             implicitIndicatorHeight + topPadding + bottomPadding)

    font: Qt.font({ family: App.fontFamily, pixelSize: App.fontSize })
    padding: 4
    spacing: 4
    leftPadding: 6
    rightPadding: 22
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

    displayText: {
        if (control.textRole && control.currentValue && typeof control.currentValue === "object" && control.textRole in control.currentValue) {
            return "" + control.currentValue[control.textRole]
        }
        if (control.currentText && control.currentText.length > 0) return control.currentText
        if (control.model && Array.isArray(control.model) && control.currentIndex >= 0 && control.currentIndex < control.model.length) {
            var val = control.model[control.currentIndex]
            if (typeof val === "string" || typeof val === "number") return "" + val
            if (control.textRole && typeof val === "object" && val !== null && control.textRole in val) {
                return "" + val[control.textRole]
            }
        }
        var t = control.textAt(control.currentIndex)
        return (t !== undefined && t !== null) ? ("" + t) : ""
    }

    delegate: T.ItemDelegate {
        id: delegateItem
        required property var model
        required property int index
        required property var modelData

        width: ListView.view ? ListView.view.width : control.width
        implicitHeight: 24
        function itemText() {
            if (control.textRole && typeof model === "object" && model !== null && control.textRole in model) {
                return "" + model[control.textRole]
            }
            if (typeof modelData === "string" || typeof modelData === "number") {
                return "" + modelData
            }
            if (typeof model === "string" || typeof model === "number") {
                return "" + model
            }
            if (control.model) {
                if (Array.isArray(control.model) && index >= 0 && index < control.model.length) {
                    var el = control.model[index]
                    if (typeof el === "string" || typeof el === "number") return "" + el
                    if (control.textRole && typeof el === "object" && el !== null && control.textRole in el) {
                        return "" + el[control.textRole]
                    }
                }
            }
            var t = control.textAt(index)
            return (t !== undefined && t !== null) ? ("" + t) : ""
        }
        text: itemText()
        highlighted: control.highlightedIndex === index
        padding: 4
        leftPadding: 8
        rightPadding: 8

        contentItem: Text {
            text: delegateItem.text
            font.family: control.font.family
            font.pixelSize: control.font.pixelSize
            font.weight: control.currentIndex === delegateItem.index ? Font.DemiBold : Font.Normal
            color: delegateItem.highlighted ? appTheme.highlightedText : appTheme.windowText
            elide: Text.ElideRight
            verticalAlignment: Text.AlignVCenter
        }

        background: Rectangle {
            radius: 4
            color: delegateItem.highlighted ? appTheme.highlight : "transparent"
        }
    }

    indicator: AppIcon {
        x: control.mirrored ? control.padding : control.width - width - 4
        y: control.topPadding + (control.availableHeight - height) / 2
        text: "arrow_drop_down"
        font.pixelSize: 18
        color: appTheme.windowText
    }

    contentItem: T.TextField {
        implicitWidth: contentWidth
        implicitHeight: contentHeight
        topPadding: 0
        bottomPadding: 0
        leftPadding: 0
        rightPadding: 0

        text: control.editable ? control.editText : control.displayText
        font: control.font
        // Disabled when not editable so clicks fall through to ComboBox and
        // open the popup. An enabled TextField ate the press, which is why
        // only the arrow opened the list.
        enabled: control.editable
        activeFocusOnPress: control.editable
        autoScroll: control.editable
        readOnly: !control.editable || control.down
        inputMethodHints: control.inputMethodHints
        validator: control.validator
        selectByMouse: control.selectTextByMouse

        color: appTheme.text
        selectionColor: appTheme.highlight
        selectedTextColor: appTheme.highlightedText
        verticalAlignment: Text.AlignVCenter

        background: Item {}
        ContextMenu.menu: control.editable ? editMenu : null
        AppTextContextMenu { id: editMenu; editor: parent }

        onEditingFinished: {
            if (control.editable) {
                control.accepted()
            }
        }
        onActiveFocusChanged: {
            if (control.editable && !activeFocus) {
                control.accepted()
            }
        }
    }

    background: Rectangle {
        implicitWidth: 120
        implicitHeight: 24
        radius: 4
        color: control.enabled ? appTheme.base : appTheme.window
        border.width: 1
        border.color: (control.activeFocus || control.down) ? appTheme.highlight
                    : (control.enabled ? appTheme.mid : appTheme.midlight)
    }

    popup: T.Popup {
        y: control.height + 2
        width: control.width
        height: Math.min(contentItem.implicitHeight + 8, control.Window.height - topMargin - bottomMargin)
        topMargin: 6
        bottomMargin: 6
        padding: 4

        palette.window: appTheme.window
        palette.windowText: appTheme.windowText
        palette.base: appTheme.base
        palette.text: appTheme.text
        palette.highlight: appTheme.highlight
        palette.highlightedText: appTheme.highlightedText
        palette.mid: appTheme.mid

        contentItem: ListView {
            clip: true
            implicitHeight: contentHeight
            model: control.delegateModel
            currentIndex: control.highlightedIndex
            highlightMoveDuration: 0
            boundsBehavior: Flickable.StopAtBounds
            ScrollBar.vertical: AppScrollBar { policy: ScrollBar.AsNeeded }
        }

        background: Rectangle {
            color: appTheme.window
            border.color: appTheme.mid
            border.width: 1
            radius: 6
        }
    }
}
