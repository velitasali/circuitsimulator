import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import cs_app

Dialog {
    id: root
    visible: TabSwitcher.visible
    modal: true
    dim: true
    width: 380
    height: Math.min(360, Math.max(120, list.count * 32 + 32))
    x: parent ? Math.round((parent.width - width) / 2) : 100
    y: parent ? Math.max(40, Math.round((parent.height - height) / 3)) : 100
    padding: 0
    closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutside

    SystemPalette { id: appTheme }

    background: Rectangle {
        color: appTheme.window
        border.color: appTheme.mid
        border.width: 1
        radius: 12
    }

    onClosed: TabSwitcher.visible = false
    onOpened: list.forceActiveFocus()

    Connections {
        target: TabSwitcher
        function onVisibleChanged() {
            if (TabSwitcher.visible) {
                root.open()
                list.forceActiveFocus()
            } else {
                root.close()
            }
        }
    }

    contentItem: Item {
        anchors.fill: parent
        anchors.margins: 8

        ListView {
            id: list
            anchors.fill: parent
            clip: true
            focus: true
            model: TabSwitcher.entries
            currentIndex: TabSwitcher.currentRow
            highlightMoveDuration: 0
            boundsBehavior: Flickable.StopAtBounds
            ScrollBar.vertical: AppScrollBar {
                id: vbar
                policy: ScrollBar.AsNeeded
            }

            onCurrentIndexChanged: positionViewAtIndex(currentIndex, ListView.Contain)

            Keys.priority: Keys.BeforeItem
            Keys.onPressed: (event) => {
                if (event.key === Qt.Key_Tab || event.key === Qt.Key_Down) {
                    TabSwitcher.moveSelection(1)
                    event.accepted = true
                } else if (event.key === Qt.Key_Backtab || event.key === Qt.Key_Up) {
                    TabSwitcher.moveSelection(-1)
                    event.accepted = true
                } else if (event.key === Qt.Key_Escape) {
                    TabSwitcher.reject()
                    event.accepted = true
                } else if (event.key === Qt.Key_Return || event.key === Qt.Key_Enter) {
                    TabSwitcher.commitSelection()
                    event.accepted = true
                }
            }

            Keys.onReleased: (event) => {
                if (event.key === Qt.Key_Control || event.key === Qt.Key_Meta) {
                    TabSwitcher.commitSelection()
                    event.accepted = true
                }
            }

            delegate: Rectangle {
                id: delegateItem
                required property var modelData
                required property int index

                width: list.width - (vbar.visible ? vbar.width : 0)
                height: 28
                radius: 4
                color: index === list.currentIndex ? appTheme.highlight : (hover.hovered ? appTheme.alternateBase : "transparent")

                RowLayout {
                    anchors.fill: parent
                    anchors.leftMargin: 8
                    anchors.rightMargin: 8
                    spacing: 8

                    AppIcon {
                        text: delegateItem.modelData.icon || (delegateItem.modelData.kind === "circuit" ? "account_tree" : "code")
                        font.pixelSize: 16
                        color: delegateItem.index === list.currentIndex ? appTheme.highlightedText : appTheme.windowText
                        Layout.preferredWidth: 18
                    }

                    Text {
                        text: delegateItem.modelData.label || ""
                        font: Qt.font({ family: App.fontFamily, pixelSize: App.fontSize })
                        color: delegateItem.index === list.currentIndex ? appTheme.highlightedText : appTheme.windowText
                        elide: Text.ElideRight
                        verticalAlignment: Text.AlignVCenter
                        Layout.fillWidth: true
                    }

                    Text {
                        visible: Boolean(delegateItem.modelData.badge)
                        text: delegateItem.modelData.badge || ""
                        font: Qt.font({ family: App.fontFamily, pixelSize: App.fontSize - 2 })
                        color: delegateItem.index === list.currentIndex ? appTheme.highlightedText : appTheme.mid
                        horizontalAlignment: Text.AlignRight
                        verticalAlignment: Text.AlignVCenter
                    }
                }

                ToolTip.text: delegateItem.modelData.tooltip || ""
                ToolTip.visible: hover.hovered && Boolean(delegateItem.modelData.tooltip)
                HoverHandler { id: hover }

                TapHandler {
                    acceptedButtons: Qt.LeftButton
                    onTapped: {
                        TabSwitcher.currentRow = delegateItem.index
                        TabSwitcher.commitSelection()
                    }
                }
            }
        }
    }
}
