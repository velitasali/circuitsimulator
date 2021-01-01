import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import cs_app

// Body of SerialMonitor. The two read-only panes bind to flushed strings
// (C++ SerialLog), so a fast UART doesn't repaint per byte.
Rectangle {
    id: root
    function tr(s) { return App.i18nTick >= 0 ? App.translate(s) : s }
    SystemPalette { id: appTheme }
    color: appTheme.window
    readonly property font uiFont: Qt.font({ family: App.fontFamily || "Ubuntu", pixelSize: App.fontSize || 13 })

    property string monitorId: ""

    property int currentPrintMode: monitorId ? SerialMonitor.printModeFor(monitorId) : SerialMonitor.printMode
    property bool currentPaused: monitorId ? SerialMonitor.pausedFor(monitorId) : SerialMonitor.paused
    property bool currentAddCR: monitorId ? SerialMonitor.addCrFor(monitorId) : SerialMonitor.addCR
    property bool currentSendEnabled: monitorId ? SerialMonitor.sendEnabledFor(monitorId) : SerialMonitor.sendEnabled
    property string currentInText: monitorId ? SerialMonitor.inTextFor(monitorId) : SerialMonitor.inText
    property string currentOutText: monitorId ? SerialMonitor.outTextFor(monitorId) : SerialMonitor.outText

    onMonitorIdChanged: {
        if (monitorId) {
            currentPrintMode = SerialMonitor.printModeFor(monitorId)
            currentPaused = SerialMonitor.pausedFor(monitorId)
            currentAddCR = SerialMonitor.addCrFor(monitorId)
            currentSendEnabled = SerialMonitor.sendEnabledFor(monitorId)
            currentInText = SerialMonitor.inTextFor(monitorId)
            currentOutText = SerialMonitor.outTextFor(monitorId)
        }
    }

    Connections {
        target: SerialMonitor
        function onMonitorUpdated(id, inText, outText) {
            if (id === root.monitorId) {
                root.currentInText = inText
                root.currentOutText = outText
            }
        }
        function onOptionsChanged() {
            if (!root.monitorId) {
                root.currentPrintMode = SerialMonitor.printMode
                root.currentPaused = SerialMonitor.paused
                root.currentAddCR = SerialMonitor.addCR
            }
        }
        function onLogsChanged() {
            if (!root.monitorId || root.monitorId === "default") {
                root.currentInText = SerialMonitor.inText
                root.currentOutText = SerialMonitor.outText
            }
        }
        function onSendEnabledChanged() {
            if (!root.monitorId) {
                root.currentSendEnabled = SerialMonitor.sendEnabled
            }
        }
    }

    Timer {
        interval: 100
        repeat: true
        running: true
        onTriggered: SerialMonitor.sync()
    }

    function doSetPrintMode(idx) {
        root.currentPrintMode = idx
        if (monitorId) SerialMonitor.setPrintModeFor(monitorId, idx)
        else SerialMonitor.printMode = idx
    }

    function doSetPaused(p) {
        root.currentPaused = p
        if (monitorId) SerialMonitor.setPausedFor(monitorId, p)
        else SerialMonitor.paused = p
    }

    function doSetAddCR(a) {
        root.currentAddCR = a
        if (monitorId) SerialMonitor.setAddCrFor(monitorId, a)
        else SerialMonitor.addCR = a
    }

    function doClearLogs() {
        if (monitorId) SerialMonitor.clearLogsFor(monitorId)
        else SerialMonitor.clearLogs()
    }

    function doSendText(txt) {
        if (monitorId) SerialMonitor.sendTextFor(monitorId, txt)
        else SerialMonitor.sendText(txt)
    }

    function doSendValue(val) {
        if (monitorId) SerialMonitor.sendValueFor(monitorId, val)
        else SerialMonitor.sendValue(val)
    }

    component LogPane: ColumnLayout {
        id: pane
        required property string title
        required property string text
        spacing: 2

        Text {
            text: pane.title
            font.family: root.uiFont.family
            font.pixelSize: root.uiFont.pixelSize
            font.bold: true
            color: appTheme.windowText
            opacity: 0.65
        }

        Rectangle {
            Layout.fillWidth: true
            Layout.fillHeight: true
            color: appTheme.base
            border.color: appTheme.mid
            border.width: 1
            radius: 6
            clip: true

            Flickable {
                anchors.fill: parent
                anchors.margins: 1
                clip: true
                boundsBehavior: Flickable.StopAtBounds
                contentWidth: logArea.implicitWidth
                contentHeight: logArea.implicitHeight

                onWidthChanged: { contentX += 1; contentX -= 1 }
                onHeightChanged: { contentY += 1; contentY -= 1 }

                ScrollBar.vertical: AppScrollBar { policy: ScrollBar.AsNeeded }
                ScrollBar.horizontal: AppScrollBar { policy: ScrollBar.AsNeeded }

                TextArea.flickable: TextArea {
                    id: logArea
                    text: pane.text
                    readOnly: true
                    wrapMode: TextEdit.WrapAnywhere
                    font.family: "Ubuntu Mono"
                    font.pixelSize: 13
                    color: appTheme.windowText
                    background: null
                    onTextChanged: cursorPosition = length
                }
            }
        }
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 8
        spacing: 8

        RowLayout {
            Layout.fillWidth: true
            spacing: 6

            Text {
                text: root.tr( "Format:" )
                font: root.uiFont
                color: appTheme.windowText
            }
            AppComboBox {
                id: printBox
                model: [ "ASCII", "HEX", "DEC", "OCT", "BIN" ]
                currentIndex: root.currentPrintMode
                font: root.uiFont
                Layout.preferredWidth: 100
                onActivated: root.doSetPrintMode(currentIndex)
            }
            Item { Layout.fillWidth: true }
            AppToolButton {
                id: pauseBtn
                checkable: true
                checked: root.currentPaused
                text: root.currentPaused ? root.tr( "Resume" ) : root.tr( "Pause" )
                font: root.uiFont
                onToggled: root.doSetPaused(checked)
            }
            AppButton {
                text: root.tr( "Clear" )
                font: root.uiFont
                onClicked: root.doClearLogs()
            }
        }

        LogPane {
            title: root.tr( "Input" )
            text: root.currentInText
            Layout.fillWidth: true
            Layout.fillHeight: true
        }
        LogPane {
            title: root.tr( "Output" )
            text: root.currentOutText
            Layout.fillWidth: true
            Layout.fillHeight: true
        }

        RowLayout {
            visible: root.currentSendEnabled
            Layout.fillWidth: true
            spacing: 6

            Text {
                text: root.tr( "Text:" )
                font: root.uiFont
                color: appTheme.windowText
            }
            AppTextField {
                id: textField
                font: root.uiFont
                Layout.fillWidth: true
                onAccepted: { root.doSendText( text ); text = ""; }
            }
            AppToolButton {
                checkable: true
                checked: root.currentAddCR
                contentItem: AppIcon { text: "keyboard_return"; color: appTheme.windowText }
                ToolTip.text: root.tr( "Append carriage return" )
                ToolTip.visible: hovered
                onToggled: root.doSetAddCR(checked)
            }
            Text {
                text: root.tr( "Value:" )
                font: root.uiFont
                color: appTheme.windowText
            }
            AppTextField {
                id: valueField
                font: root.uiFont
                Layout.preferredWidth: 90
                validator: IntValidator {}
                onAccepted: { root.doSendValue( text ); text = ""; }
            }
        }
    }
}
