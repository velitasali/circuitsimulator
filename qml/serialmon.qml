pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import cs_app
import cs_ui

// Body of SerialMonitor. The two read-only panes bind to flushed strings
// (C++ SerialLog), so a fast UART doesn't repaint per byte.
Rectangle {
    id: root
    SystemPalette { id: appTheme }
    color: appTheme.window
    readonly property font uiFont: Qt.font({ family: App.fontFamily, pixelSize: App.fontNormal })

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
        function onMonitorUpdated(id: string, inText: string, outText: string): void {
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

    function doSetPrintMode(idx: int): void {
        root.currentPrintMode = idx
        if (monitorId) SerialMonitor.setPrintModeFor(monitorId, idx)
        else SerialMonitor.printMode = idx
    }

    function doSetPaused(p: bool): void {
        root.currentPaused = p
        if (monitorId) SerialMonitor.setPausedFor(monitorId, p)
        else SerialMonitor.paused = p
    }

    function doSetAddCR(a: bool): void {
        root.currentAddCR = a
        if (monitorId) SerialMonitor.setAddCrFor(monitorId, a)
        else SerialMonitor.addCR = a
    }

    function doClearLogs(): void {
        if (monitorId) SerialMonitor.clearLogsFor(monitorId)
        else SerialMonitor.clearLogs()
    }

    function doSendText(txt: string): void {
        if (monitorId) SerialMonitor.sendTextFor(monitorId, txt)
        else SerialMonitor.sendText(txt)
    }

    function doSendValue(val: string): void {
        if (monitorId) SerialMonitor.sendValueFor(monitorId, val)
        else SerialMonitor.sendValue(val)
    }

    component LogPane: ColumnLayout {
        id: pane
        required property string title
        required property string text
        spacing: 2

        AppText {
            text: pane.title
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
            radius: 10
            clip: true

            ScrollableTextEdit {
                id: logArea
                anchors.fill: parent
                anchors.margins: 1
                text: pane.text
                readOnly: true
                selectByMouse: true
                wrapMode: TextEdit.WrapAnywhere
                contentPadding: 6
                font: Qt.font({
                    family: App.fontMonoFamily || "Ubuntu Mono",
                    pixelSize: App.fontNormal,
                    fixedPitch: true
                })
                color: appTheme.windowText
                onTextChanged: scrollToTail()
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

            AppText {
                text: I18n.tr( "Format:" )
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
                leftPadding: 10
                rightPadding: 10
                text: root.currentPaused ? I18n.tr( "Resume" ) : I18n.tr( "Pause" )
                font: root.uiFont
                onToggled: root.doSetPaused(checked)
            }
            AppButton {
                text: I18n.tr( "Clear" )
                font: root.uiFont
                onClicked: root.doClearLogs()
            }
        }

        LogPane {
            title: I18n.tr( "Input" )
            text: root.currentInText
            Layout.fillWidth: true
            Layout.fillHeight: true
        }
        LogPane {
            title: I18n.tr( "Output" )
            text: root.currentOutText
            Layout.fillWidth: true
            Layout.fillHeight: true
        }

        RowLayout {
            visible: root.currentSendEnabled
            Layout.fillWidth: true
            spacing: 6

            AppText {
                text: I18n.tr( "Text:" )
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
                contentItem: AppIcon { icon: "keyboard_return"; color: appTheme.windowText }
                ToolTip.text: I18n.tr( "Append carriage return" )
                ToolTip.visible: hovered
                onToggled: root.doSetAddCR(checked)
            }
            AppText {
                text: I18n.tr( "Value:" )
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
