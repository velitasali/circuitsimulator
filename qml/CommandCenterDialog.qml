import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import cs_app

Dialog {
    id: root
    function tr(s) { return App.i18nTick >= 0 ? App.translate(s) : s }
    visible: CommandCenter.visible
    modal: true
    dim: true
    width: 480
    height: 320
    x: parent ? Math.round((parent.width - width) / 2) : 100
    y: parent ? Math.max(40, Math.round((parent.height - height) / 3)) : 100
    padding: 0
    closePolicy: shortcutDialog.visible ? Popup.NoAutoClose : (Popup.CloseOnEscape | Popup.CloseOnPressOutside)

    SystemPalette { id: appTheme }

    background: Rectangle {
        color: appTheme.window
        border.color: appTheme.mid
        border.width: 1
        radius: 12
    }

    onClosed: {
        if (shortcutDialog.visible) {
            shortcutDialog.close()
        }
        CommandCenter.visible = false
    }
    onOpened: {
        if (!shortcutDialog.visible) {
            searchField.forceActiveFocus()
        }
    }

    Connections {
        target: CommandCenter
        function onVisibleChanged() {
            if (CommandCenter.visible) {
                root.open()
                if (!shortcutDialog.visible) {
                    searchField.forceActiveFocus()
                }
            } else {
                if (shortcutDialog.visible) {
                    shortcutDialog.close()
                }
                root.close()
            }
        }
    }

    contentItem: ColumnLayout {
        anchors.fill: parent
        anchors.margins: 8
        spacing: 6

        AppTextField {
            id: searchField
            Layout.fillWidth: true
            focus: !shortcutDialog.visible
            enabled: !shortcutDialog.visible
            font: Qt.font({ family: App.fontFamily, pixelSize: App.fontSize })
            placeholderText: root.tr( "Type > for commands, or type to jump to component..." )
            text: CommandCenter.searchText

            Component.onCompleted: if (!shortcutDialog.visible) searchField.forceActiveFocus()
            onVisibleChanged: if (visible && !shortcutDialog.visible) searchField.forceActiveFocus()

            onTextEdited: CommandCenter.searchText = text

            Keys.onPressed: ( event ) => {
                if (shortcutDialog.visible) {
                    return
                }
                if ( event.key === Qt.Key_Escape ) {
                    CommandCenter.reject()
                    event.accepted = true
                } else if ( event.key === Qt.Key_Down ) {
                    CommandCenter.moveSelection( 1 )
                    event.accepted = true
                } else if ( event.key === Qt.Key_Up ) {
                    CommandCenter.moveSelection( -1 )
                    event.accepted = true
                } else if ( event.key === Qt.Key_Return || event.key === Qt.Key_Enter ) {
                    CommandCenter.activateCurrent()
                    event.accepted = true
                } else if ( ( event.modifiers & ( Qt.ControlModifier | Qt.MetaModifier ) ) && event.key === Qt.Key_P ) {
                    CommandCenter.togglePalettePrefix( ( event.modifiers & Qt.ShiftModifier ) !== 0 )
                    event.accepted = true
                }
            }
        }

        ListView {
            id: list
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
            model: CommandCenter.items
            currentIndex: CommandCenter.currentRow
            highlightMoveDuration: 0
            boundsBehavior: Flickable.StopAtBounds
            ScrollBar.vertical: AppScrollBar {
                id: vbar
                policy: ScrollBar.AsNeeded
            }

            onCurrentIndexChanged: positionViewAtIndex( currentIndex, ListView.Contain )

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
                        visible: Boolean(delegateItem.modelData.icon)
                        text: delegateItem.modelData.icon || ""
                        font.pixelSize: 16
                        color: delegateItem.index === list.currentIndex ? appTheme.highlightedText : appTheme.windowText
                        Layout.preferredWidth: 18
                    }

                    Text {
                        text: delegateItem.modelData.text || ""
                        font: Qt.font({ family: App.fontFamily, pixelSize: App.fontSize })
                        color: delegateItem.index === list.currentIndex ? appTheme.highlightedText : appTheme.windowText
                        elide: Text.ElideRight
                        verticalAlignment: Text.AlignVCenter
                        Layout.fillWidth: true
                    }

                    Text {
                        visible: delegateItem.modelData.shortcut && delegateItem.modelData.shortcut.length > 0
                        text: delegateItem.modelData.shortcut || ""
                        font: Qt.font({ family: App.fontFamily, pixelSize: App.fontSize - 1 })
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
                        CommandCenter.currentRow = delegateItem.index
                        CommandCenter.activateCurrent()
                    }
                }

                TapHandler {
                    acceptedButtons: Qt.RightButton
                    onTapped: {
                        if (delegateItem.modelData && delegateItem.modelData.type === "action") {
                            CommandCenter.currentRow = delegateItem.index
                            shortcutDialog.actionId = delegateItem.modelData.data || ""
                            shortcutDialog.actionName = delegateItem.modelData.text || ""
                            shortcutDialog.currentShortcut = delegateItem.modelData.shortcut || ""
                            actionContextMenu.popup()
                        }
                    }
                }
            }
        }
    }

    AppContextMenu {
        id: actionContextMenu
        ContextMenuItem {
            text: root.tr("Edit Shortcut...")
            iconLigature: "keyboard"
            onTriggered: shortcutDialog.open()
        }
    }

    ShortcutDialog {
        id: shortcutDialog
        onClosed: {
            if (root.visible) {
                searchField.forceActiveFocus()
            }
        }
    }
}
