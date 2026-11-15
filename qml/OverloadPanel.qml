pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import cs_app
import cs_ui

// Warnings popup dialog matching C++ OverloadPanel (overloadpanel.qml)
Popup {
    id: root

    property double lastDismissTime: 0

    padding: 0
    margins: 0
    modal: false
    focus: true
    closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutside

    width: 380
    height: 260

    SystemPalette { id: appTheme }

    onClosed: {
        lastDismissTime = Date.now()
        CircuitPanel.setOverloadPanelVisible(false)
    }

    onAboutToShow: {
        CircuitPanel.setOverloadPanelVisible(true)
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
        anchors.margins: 8

        ColumnLayout {
            anchors.fill: parent
            spacing: 6

            AppText {
                visible: CircuitCanvas.overloadLog ? CircuitCanvas.overloadLog.length === 0 : true
                text: I18n.tr("No warnings")
                color: appTheme.windowText
                opacity: 0.65
                Layout.fillWidth: true
                Layout.fillHeight: true
                horizontalAlignment: Text.AlignHCenter
                verticalAlignment: Text.AlignVCenter
            }

            ListView {
                id: list
                visible: CircuitCanvas.overloadLog ? CircuitCanvas.overloadLog.length > 0 : false
                Layout.fillWidth: true
                Layout.fillHeight: true
                clip: true
                model: CircuitCanvas.overloadLog
                boundsBehavior: Flickable.StopAtBounds
                ScrollBar.vertical: AppScrollBar {
                    policy: ScrollBar.AsNeeded
                }

                delegate: Rectangle {
                    id: rowRect
                    required property var modelData
                    required property int index

                    width: list.width
                    height: 26
                    radius: 4
                    color: hover.hovered ? appTheme.highlight : "transparent"

                    RowLayout {
                        anchors.fill: parent
                        anchors.leftMargin: 6
                        anchors.rightMargin: 6
                        spacing: 6

                        AppIcon {
                            icon: "warning"
                            size: 16
                            color: rowRect.modelData.crashed ? CircuitCanvas.msgErrorBg : CircuitCanvas.msgWarnBg
                        }

                        AppText {
                            text: rowRect.modelData.text || ""
                            color: hover.hovered ? appTheme.highlightedText : appTheme.windowText
                            elide: Text.ElideRight
                            verticalAlignment: Text.AlignVCenter
                            Layout.fillWidth: true
                        }
                    }

                    HoverHandler { id: hover }
                    TapHandler {
                        onTapped: {
                            CircuitCanvas.activateOverloadEntry(rowRect.index)
                            root.close()
                        }
                    }
                }
            }

            RowLayout {
                Layout.fillWidth: true
                Item { Layout.fillWidth: true }
                AppButton {
                    text: I18n.tr("Clear")
                    onClicked: CircuitCanvas.clearOverloadLog()
                }
            }
        }
    }
}
