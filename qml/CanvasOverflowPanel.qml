import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import cs_app
import cs_ui

// Canvas Overflow popup dialog matching app design
Popup {
    id: root

    property double lastDismissTime: 0

    padding: 0
    margins: 0
    modal: false
    focus: true
    closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutside

    width: 380
    readonly property real naturalHeight: 290
    // CircuitPanelView writes this. Assigning height would drop the fixed size.
    property real heightCap: 1000000
    height: Math.min(naturalHeight, heightCap)

    SystemPalette { id: appTheme }

    onClosed: {
        lastDismissTime = Date.now()
        CircuitPanel.setCanvasOverflowPanelVisible(false)
    }

    onAboutToShow: {
        CircuitPanel.setCanvasOverflowPanelVisible(true)
    }

    function toggle(): void {
        if (visible) {
            close()
            return
        }
        var now = Date.now()
        if (now - lastDismissTime < 300) {
            lastDismissTime = 0
            return
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
        clip: true

        ColumnLayout {
            anchors.fill: parent
            spacing: 10

            // Header with warning badge
            RowLayout {
                Layout.fillWidth: true
                spacing: 8

                Rectangle {
                    Layout.preferredWidth: 28
                    Layout.preferredHeight: 28
                    radius: 14
                    color: Qt.alpha(CircuitCanvas.msgWarnBg, 0.25)

                    AppIcon {
                        anchors.centerIn: parent
                        icon: "warning"
                        size: 18
                        color: CircuitCanvas.msgWarnBg
                    }
                }

                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 1

                    AppText {
                        text: I18n.tr("Canvas Overflow")
                        font.pixelSize: App.fontTitle
                        font.bold: true
                        color: appTheme.windowText
                    }

                    AppText {
                        text: I18n.tr("%1 item(s) outside canvas bounds").arg(CircuitCanvas.canvasOverflowCount)
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
                    rowSpacing: 6
                    columnSpacing: 10

                    AppText {
                        text: I18n.tr("Current Canvas Size:")
                        color: appTheme.windowText
                        opacity: 0.8
                    }
                    AppText {
                        text: I18n.tr("%1 × %2 px").arg(CircuitCanvas.canvasCurrentWidth).arg(CircuitCanvas.canvasCurrentHeight)
                        font.bold: true
                        color: appTheme.windowText
                    }

                    AppText {
                        text: I18n.tr("Recommended Size:")
                        color: appTheme.windowText
                        opacity: 0.8
                    }
                    AppText {
                        text: I18n.tr("%1 × %2 px").arg(CircuitCanvas.canvasRequiredWidth).arg(CircuitCanvas.canvasRequiredHeight)
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
                        text: I18n.tr("Auto-Fit Canvas")
                        font.bold: true
                        highlighted: true
                        onClicked: {
                            CircuitCanvas.autoFitCanvas()
                            root.close()
                        }
                    }

                    AppButton {
                        Layout.fillWidth: true
                        text: I18n.tr("Center Circuit")
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
                        text: I18n.tr("Select Offending")
                        onClicked: {
                            CircuitCanvas.selectOverflowing()
                            root.close()
                        }
                    }

                    AppButton {
                        Layout.fillWidth: true
                        text: I18n.tr("Circuit Settings...")
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
