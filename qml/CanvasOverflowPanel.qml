import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import cs_app

// Canvas Overflow popup dialog matching app design
Popup {
    id: root
    function tr(s) { return App.i18nTick >= 0 ? App.translate(s) : s }

    property double lastDismissTime: 0

    padding: 0
    margins: 0
    modal: false
    focus: true
    closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutside

    width: 380
    height: 290

    SystemPalette { id: appTheme }

    onClosed: {
        lastDismissTime = Date.now()
        if (ctx) ctx.setCanvasOverflowPanelVisible(false)
    }

    onAboutToShow: {
        if (ctx) ctx.setCanvasOverflowPanelVisible(true)
    }

    function toggle(pt) {
        if (visible) {
            close()
            return
        }
        var now = Date.now()
        if (now - lastDismissTime < 300) {
            lastDismissTime = 0
            return
        }
        if (root.parent) {
            x = Math.max(8, Math.min(root.parent.width - width - 8, pt.x))
            y = pt.y
        }
        open()
    }

    background: Rectangle {
        color: appTheme.window
        border.color: appTheme.mid
        border.width: 1
        radius: 12
    }

    contentItem: Item {
        anchors.fill: parent
        anchors.margins: 12

        ColumnLayout {
            anchors.fill: parent
            spacing: 10

            // Header with warning badge
            RowLayout {
                Layout.fillWidth: true
                spacing: 8

                Rectangle {
                    width: 28
                    height: 28
                    radius: 14
                    color: Qt.rgba(CircuitCanvas.msgWarnBg.r, CircuitCanvas.msgWarnBg.g, CircuitCanvas.msgWarnBg.b, 0.25)

                    AppIcon {
                        anchors.centerIn: parent
                        text: "warning"
                        font.pixelSize: 18
                        color: CircuitCanvas.msgWarnBg
                    }
                }

                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 1

                    Text {
                        text: root.tr("Canvas Overflow")
                        font.family: App.fontFamily
                        font.pixelSize: App.fontSize + 1
                        font.bold: true
                        color: appTheme.windowText
                    }

                    Text {
                        text: root.tr("%1 item(s) outside canvas bounds").arg(CircuitCanvas.canvasOverflowCount)
                        font.family: App.fontFamily
                        font.pixelSize: App.fontSize - 1
                        color: appTheme.windowText
                        opacity: 0.75
                    }
                }
            }

            // Info Card showing dimensions
            Rectangle {
                Layout.fillWidth: true
                Layout.preferredHeight: 74
                radius: 8
                color: appTheme.base
                border.color: appTheme.mid
                border.width: 1

                GridLayout {
                    anchors.fill: parent
                    anchors.margins: 10
                    columns: 2
                    rowSpacing: 4
                    columnSpacing: 10

                    Text {
                        text: root.tr("Current Canvas Size:")
                        font.family: App.fontFamily
                        font.pixelSize: App.fontSize
                        color: appTheme.windowText
                        opacity: 0.8
                    }
                    Text {
                        text: root.tr("%1 × %2 px").arg(CircuitCanvas.canvasCurrentWidth).arg(CircuitCanvas.canvasCurrentHeight)
                        font.family: App.fontFamily
                        font.pixelSize: App.fontSize
                        font.bold: true
                        color: appTheme.windowText
                    }

                    Text {
                        text: root.tr("Recommended Size:")
                        font.family: App.fontFamily
                        font.pixelSize: App.fontSize
                        color: appTheme.windowText
                        opacity: 0.8
                    }
                    Text {
                        text: root.tr("%1 × %2 px").arg(CircuitCanvas.canvasRequiredWidth).arg(CircuitCanvas.canvasRequiredHeight)
                        font.family: App.fontFamily
                        font.pixelSize: App.fontSize
                        font.bold: true
                        color: CircuitCanvas.msgWarnBg
                    }
                }
            }

            Item { Layout.fillHeight: true }

            // Action Buttons
            ColumnLayout {
                Layout.fillWidth: true
                spacing: 6

                RowLayout {
                    Layout.fillWidth: true
                    spacing: 8

                    AppButton {
                        Layout.fillWidth: true
                        text: root.tr("Auto-Fit Canvas")
                        font.family: App.fontFamily
                        font.pixelSize: App.fontSize
                        font.bold: true
                        highlighted: true
                        onClicked: {
                            CircuitCanvas.autoFitCanvas()
                            root.close()
                        }
                    }

                    AppButton {
                        Layout.fillWidth: true
                        text: root.tr("Center Circuit")
                        font.family: App.fontFamily
                        font.pixelSize: App.fontSize
                        onClicked: {
                            CircuitCanvas.centerCircuit()
                            root.close()
                        }
                    }
                }

                RowLayout {
                    Layout.fillWidth: true
                    spacing: 8

                    AppButton {
                        Layout.fillWidth: true
                        text: root.tr("Select Offending")
                        font.family: App.fontFamily
                        font.pixelSize: App.fontSize
                        onClicked: {
                            CircuitCanvas.selectOverflowing()
                            root.close()
                        }
                    }

                    AppButton {
                        Layout.fillWidth: true
                        text: root.tr("Circuit Settings...")
                        font.family: App.fontFamily
                        font.pixelSize: App.fontSize
                        onClicked: {
                            root.close()
                            App.showCircuitSettings()
                        }
                    }
                }
            }
        }
    }
}
