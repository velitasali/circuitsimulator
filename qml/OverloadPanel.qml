import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import cs_app

// Warnings popup dialog matching C++ OverloadPanel (overloadpanel.qml)
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
    height: 260

    SystemPalette { id: appTheme }

    onClosed: {
        lastDismissTime = Date.now()
        if (ctx) ctx.setOverloadPanelVisible(false)
    }

    onAboutToShow: {
        if (ctx) ctx.setOverloadPanelVisible(true)
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
        anchors.margins: 8

        ColumnLayout {
            anchors.fill: parent
            spacing: 6

            Text {
                visible: CircuitCanvas.overloadLog ? CircuitCanvas.overloadLog.length === 0 : true
                text: root.tr("No warnings")
                font.family: App.fontFamily
                font.pixelSize: App.fontSize
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
                            text: "warning"
                            font.pixelSize: 16
                            color: modelData.crashed ? CircuitCanvas.msgErrorBg : CircuitCanvas.msgWarnBg
                        }

                        Text {
                            text: modelData.text || ""
                            font.family: App.fontFamily
                            font.pixelSize: App.fontSize
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
                    text: root.tr("Clear")
                    font.family: App.fontFamily
                    font.pixelSize: App.fontSize
                    onClicked: CircuitCanvas.clearOverloadLog()
                }
            }
        }
    }
}
