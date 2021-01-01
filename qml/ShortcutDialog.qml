import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import cs_app

Dialog {
    id: root
    function tr(s) { return App.i18nTick >= 0 ? App.translate(s) : s }
    modal: true
    dim: true
    parent: Overlay.overlay
    x: parent ? Math.round((parent.width - width) / 2) : 100
    y: parent ? Math.max(40, Math.round((parent.height - height) / 2)) : 100
    width: 380
    padding: 0
    closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutside

    SystemPalette { id: appTheme }

    property string actionId: ""
    property string actionName: ""
    property string currentShortcut: ""
    property string recordedShortcut: ""

    onOpened: {
        recordedShortcut = currentShortcut
        captureArea.forceActiveFocus()
        focusTimer.restart()
    }
    onVisibleChanged: {
        if (visible) {
            captureArea.forceActiveFocus()
            focusTimer.restart()
        }
    }

    Timer {
        id: focusTimer
        interval: 20
        repeat: false
        onTriggered: {
            if (root.visible) {
                captureArea.forceActiveFocus()
            }
        }
    }

    function keyToString(key, text) {
        if (key >= Qt.Key_F1 && key <= Qt.Key_F12) {
            return "F" + (key - Qt.Key_F1 + 1)
        }
        switch (key) {
            case Qt.Key_Escape: return "Esc"
            case Qt.Key_Tab: return "Tab"
            case Qt.Key_Backtab: return "Backtab"
            case Qt.Key_Backspace: return "Backspace"
            case Qt.Key_Return:
            case Qt.Key_Enter: return "Return"
            case Qt.Key_Insert: return "Insert"
            case Qt.Key_Delete: return "Delete"
            case Qt.Key_Pause: return "Pause"
            case Qt.Key_Print: return "Print"
            case Qt.Key_Home: return "Home"
            case Qt.Key_End: return "End"
            case Qt.Key_Left: return "Left"
            case Qt.Key_Up: return "Up"
            case Qt.Key_Right: return "Right"
            case Qt.Key_Down: return "Down"
            case Qt.Key_PageUp: return "PageUp"
            case Qt.Key_PageDown: return "PageDown"
            case Qt.Key_Space: return "Space"
            case Qt.Key_Comma: return ","
            case Qt.Key_Period:
            case Qt.Key_decimal: return "."
            case Qt.Key_Minus:
            case Qt.Key_subtract: return "-"
            case Qt.Key_Plus:
            case Qt.Key_add: return "+"
            case Qt.Key_Equal: return "="
            case Qt.Key_Slash:
            case Qt.Key_divide: return "/"
            case Qt.Key_Asterisk:
            case Qt.Key_multiply: return "*"
            case Qt.Key_Backslash: return "\\"
            case Qt.Key_Semicolon: return ";"
            case Qt.Key_Apostrophe: return "'"
            case Qt.Key_BracketLeft: return "["
            case Qt.Key_BracketRight: return "]"
            case Qt.Key_QuoteLeft: return "`"
        }
        if (key >= Qt.Key_A && key <= Qt.Key_Z) {
            return String.fromCharCode(key)
        }
        if (key >= Qt.Key_0 && key <= Qt.Key_9) {
            return String.fromCharCode(key)
        }
        if (text && text.length === 1 && text.charCodeAt(0) >= 32) {
            return text.toUpperCase()
        }
        return ""
    }

    background: Rectangle {
        implicitWidth: 380
        color: appTheme.window
        border.color: appTheme.mid
        border.width: 1
        radius: 10
    }

    contentItem: Item {
        implicitWidth: 380
        implicitHeight: layout.implicitHeight + 32

        TapHandler {
            onTapped: captureArea.forceActiveFocus()
        }

        ColumnLayout {
            id: layout
            anchors.fill: parent
            anchors.margins: 16
            spacing: 12

            Text {
                text: root.tr("Enter new shortcut for: ") + root.actionName
                font: Qt.font({ family: App.fontFamily || "Ubuntu", pixelSize: App.fontSize || 13, bold: true })
                color: appTheme.windowText
                wrapMode: Text.WordWrap
                Layout.fillWidth: true
            }

            Rectangle {
                id: captureArea
                Layout.fillWidth: true
                Layout.preferredHeight: 36
                color: appTheme.base
                border.color: captureArea.activeFocus ? appTheme.highlight : appTheme.mid
                border.width: captureArea.activeFocus ? 2 : 1
                radius: 6

                focus: true
                activeFocusOnTab: false

                Text {
                    anchors.fill: parent
                    anchors.leftMargin: 10
                    text: root.recordedShortcut
                    font: Qt.font({ family: App.fontFamily || "Ubuntu", pixelSize: (App.fontSize || 13) + 1, bold: true })
                    color: appTheme.windowText
                    verticalAlignment: Text.AlignVCenter
                    visible: root.recordedShortcut.length > 0
                }

                Text {
                    anchors.fill: parent
                    anchors.leftMargin: 10
                    text: root.tr("Press a key combination...")
                    font: Qt.font({ family: App.fontFamily || "Ubuntu", pixelSize: App.fontSize || 13 })
                    color: appTheme.windowText
                    opacity: 0.5
                    verticalAlignment: Text.AlignVCenter
                    visible: root.recordedShortcut.length === 0
                }

                Keys.priority: Keys.BeforeItem
                Keys.onPressed: (event) => {
                    if (event.key === Qt.Key_Control || event.key === Qt.Key_Shift ||
                        event.key === Qt.Key_Alt || event.key === Qt.Key_Meta ||
                        event.key === Qt.Key_AltGr) {
                        event.accepted = true
                        return
                    }
                    if (event.key === Qt.Key_Escape && event.modifiers === Qt.NoModifier) {
                        root.close()
                        event.accepted = true
                        return
                    }
                    if ((event.key === Qt.Key_Return || event.key === Qt.Key_Enter) && event.modifiers === Qt.NoModifier) {
                        if (root.actionId) {
                            CommandCenter.setShortcut(root.actionId, root.recordedShortcut)
                        }
                        root.close()
                        event.accepted = true
                        return
                    }
                    var keyStr = root.keyToString(event.key, event.text)
                    if (!keyStr) return

                    var parts = []
                    if (event.modifiers & (Qt.ControlModifier | Qt.MetaModifier)) parts.push("Ctrl")
                    if (event.modifiers & Qt.AltModifier) parts.push("Alt")
                    if (event.modifiers & Qt.ShiftModifier) parts.push("Shift")
                    parts.push(keyStr)
                    root.recordedShortcut = parts.join("+")
                    event.accepted = true
                }

                TapHandler {
                    onTapped: captureArea.forceActiveFocus()
                }
            }

            RowLayout {
                Layout.fillWidth: true
                spacing: 8

                AppButton {
                    text: root.tr("Clear")
                    font: Qt.font({ family: App.fontFamily || "Ubuntu", pixelSize: App.fontSize || 13 })
                    onClicked: root.recordedShortcut = ""
                }

                Item { Layout.fillWidth: true }

                AppButton {
                    text: root.tr("Cancel")
                    font: Qt.font({ family: App.fontFamily || "Ubuntu", pixelSize: App.fontSize || 13 })
                    onClicked: root.close()
                }

                AppButton {
                    text: root.tr("OK")
                    font: Qt.font({ family: App.fontFamily || "Ubuntu", pixelSize: App.fontSize || 13, bold: true })
                    highlighted: true
                    onClicked: {
                        if (root.actionId) {
                            CommandCenter.setShortcut(root.actionId, root.recordedShortcut)
                        }
                        root.close()
                    }
                }
            }
        }
    }
}
