import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import cs_app
import cs_ui

Dialog {
    id: root
    modal: true
    dim: true
    parent: Overlay.overlay
    x: parent ? Math.round((parent.width - width) / 2) : 100
    y: parent ? Math.max(40, Math.round((parent.height - height) / 2)) : 100
    width: 400
    padding: 0
    closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutside

    SystemPalette { id: appTheme }

    property string actionId: ""
    property string actionName: ""
    property string currentShortcut: ""
    property string defaultShortcut: ""
    property string recordedShortcut: ""
    property bool singleKeyOnly: false

    readonly property var conflictInfo: (root.visible && root.actionId)
        ? CommandCenter.shortcutConflict(root.actionId, root.recordedShortcut)
        : ({ kind: "none", label: "" })
    readonly property string conflictKind: (conflictInfo && conflictInfo.kind) ? String(conflictInfo.kind) : "none"
    readonly property string conflictLabel: (conflictInfo && conflictInfo.label) ? String(conflictInfo.label) : ""
    readonly property bool isReserved: conflictKind === "reserved"
    readonly property bool isInvalid: conflictKind === "invalid"
    readonly property bool hasError: isReserved || isInvalid
    readonly property bool isConflict: conflictKind === "conflict"

    function commitShortcut(): void {
        if (root.hasError)
            return
        if (root.actionId) {
            if (root.recordedShortcut === root.defaultShortcut)
                CommandCenter.resetShortcut(root.actionId)
            else
                CommandCenter.setShortcut(root.actionId, root.recordedShortcut)
        }
        root.close()
    }

    onOpened: {
        recordedShortcut = currentShortcut
        CommandCenter.recordingShortcut = true
        captureArea.forceActiveFocus()
        focusTimer.restart()
    }
    onVisibleChanged: {
        CommandCenter.recordingShortcut = visible
        if (visible) {
            captureArea.forceActiveFocus()
            focusTimer.restart()
        }
    }
    Component.onDestruction: CommandCenter.recordingShortcut = false

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

    function keyToString(key: int, text: string): string {
        if (key >= Qt.Key_F1 && key <= Qt.Key_F24) {
            return "F" + (key - Qt.Key_F1 + 1)
        }
        switch (key) {
            case Qt.Key_Escape: return "Esc"
            case Qt.Key_Tab:
            case Qt.Key_Backtab: return "Tab"
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
            case Qt.Key_Period: return "."
            case Qt.Key_Minus: return "-"
            case Qt.Key_Plus: return "+"
            case Qt.Key_Equal: return "="
            case Qt.Key_Slash: return "/"
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
        implicitWidth: 400
        color: appTheme.window
        border.color: appTheme.mid
        border.width: 1
        radius: 16
    }

    contentItem: Item {
        implicitWidth: 400
        implicitHeight: layout.implicitHeight + 32

        TapHandler {
            onTapped: captureArea.forceActiveFocus()
        }

        ColumnLayout {
            id: layout
            anchors.fill: parent
            anchors.margins: 16
            spacing: 12

            AppText {
                text: I18n.tr("Enter new shortcut for: ") + root.actionName
                font.bold: true
                color: appTheme.windowText
                wrapMode: Text.WordWrap
                Layout.fillWidth: true
            }

            Rectangle {
                id: captureArea
                Layout.fillWidth: true
                Layout.preferredHeight: 36
                color: appTheme.base
                border.color: root.hasError
                    ? CircuitCanvas.msgErrorBg
                    : (captureArea.activeFocus ? appTheme.highlight : appTheme.mid)
                border.width: (captureArea.activeFocus || root.hasError) ? 2 : 1
                radius: 10

                focus: true
                activeFocusOnTab: false

                AppText {
                    anchors.fill: parent
                    anchors.leftMargin: 10
                    text: UiUtils.formatShortcut(root.recordedShortcut)
                    font.pixelSize: App.fontTitle
                    font.bold: true
                    color: appTheme.windowText
                    verticalAlignment: Text.AlignVCenter
                    visible: root.recordedShortcut.length > 0
                }

                AppText {
                    anchors.fill: parent
                    anchors.leftMargin: 10
                    text: root.singleKeyOnly
                        ? I18n.tr("Press a key (A–Z, 0–9)...")
                        : I18n.tr("Press a key combination...")
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
                        root.commitShortcut()
                        event.accepted = true
                        return
                    }
                    var keyStr = root.keyToString(event.key, event.text)
                    if (!keyStr) return

                    var parts = []
                    if (event.modifiers & Qt.ControlModifier) parts.push("Ctrl")
                    if (event.modifiers & Qt.AltModifier) parts.push("Alt")
                    if ((event.modifiers & Qt.ShiftModifier) || event.key === Qt.Key_Backtab) parts.push("Shift")
                    if (event.modifiers & Qt.MetaModifier) parts.push("Meta")
                    parts.push(keyStr)
                    root.recordedShortcut = parts.join("+")
                    event.accepted = true
                }

                TapHandler {
                    onTapped: captureArea.forceActiveFocus()
                }
            }

            AppText {
                visible: root.hasError || root.isConflict
                text: root.isInvalid
                    ? root.conflictLabel
                    : (root.isReserved
                        ? I18n.tr("Reserved shortcut (%1). Choose another key combination.").arg(root.conflictLabel)
                        : I18n.tr("Currently assigned to %1. Saving will clear that binding.").arg(root.conflictLabel))
                font.pixelSize: App.fontSmall
                color: root.hasError ? CircuitCanvas.msgErrorBg : CircuitCanvas.msgWarnBg
                wrapMode: Text.WordWrap
                Layout.fillWidth: true
            }

            RowLayout {
                Layout.fillWidth: true
                spacing: 8

                AppButton {
                    text: I18n.tr("Clear")
                    enabled: root.recordedShortcut.length > 0
                    onClicked: {
                        root.recordedShortcut = ""
                        captureArea.forceActiveFocus()
                    }
                }

                AppButton {
                    text: I18n.tr("Reset")
                    visible: root.defaultShortcut.length > 0
                    enabled: root.recordedShortcut !== root.defaultShortcut
                    onClicked: {
                        root.recordedShortcut = root.defaultShortcut
                        captureArea.forceActiveFocus()
                    }
                }

                Item { Layout.fillWidth: true }

                AppButton {
                    text: I18n.tr("Cancel")
                    onClicked: root.close()
                }

                AppButton {
                    text: I18n.tr("OK")
                    font.bold: true
                    highlighted: true
                    enabled: !root.hasError
                    onClicked: root.commitShortcut()
                }
            }
        }
    }
}
