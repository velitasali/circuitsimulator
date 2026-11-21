pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import cs_app
import cs_ui

Dialog {
    id: root
    visible: CommandCenter.visible
    modal: true
    dim: true
    width: parent ? Math.min(520, Math.max(320, parent.width - 32)) : 520
    height: parent ? Math.min(360, Math.max(200, parent.height - 80)) : 360
    x: parent ? Math.round((parent.width - width) / 2) : 100
    y: parent ? Math.max(40, Math.round((parent.height - height) / 3)) : 100
    padding: 0
    closePolicy: shortcutDialog.visible ? Popup.NoAutoClose : (Popup.CloseOnEscape | Popup.CloseOnPressOutside)

    readonly property bool isCommandMode: searchField.text.startsWith(">")
    readonly property bool hasFilterQuery: root.isCommandMode
        ? searchField.text.length > 1
        : searchField.text.length > 0

    function focusSearchEnd(): void {
        if (!shortcutDialog.visible) {
            searchField.forceActiveFocus()
            searchField.cursorPosition = searchField.length
        }
    }

    SystemPalette { id: appTheme }

    background: Rectangle {
        color: appTheme.window
        border.color: appTheme.mid
        border.width: 1
        radius: 16
    }

    onClosed: {
        if (shortcutDialog.visible) {
            shortcutDialog.close()
        }
        CommandCenter.visible = false
    }
    onOpened: root.focusSearchEnd()

    Connections {
        target: CommandCenter
        function onVisibleChanged() {
            if (CommandCenter.visible) {
                root.open()
                root.focusSearchEnd()
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
        spacing: 0

        // Integrated Spotlight-style search header.
        Item {
            id: searchHeader
            Layout.fillWidth: true
            Layout.preferredHeight: 46

            MouseArea {
                anchors.fill: parent
                acceptedButtons: Qt.LeftButton
                onClicked: searchField.forceActiveFocus()
            }

            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: 14
                anchors.rightMargin: 10
                spacing: 8

                AppIcon {
                    icon: "search"
                    size: 18
                    color: searchField.activeFocus ? appTheme.highlight : appTheme.windowText
                    opacity: searchField.activeFocus ? 0.95 : 0.6
                    Layout.preferredWidth: 18
                    Layout.preferredHeight: 18
                    Layout.alignment: Qt.AlignVCenter
                    Behavior on color { ColorAnimation { duration: 120 } }
                    Behavior on opacity { NumberAnimation { duration: 120 } }
                }

                AppTextField {
                    id: searchField
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    focus: !shortcutDialog.visible
                    enabled: !shortcutDialog.visible
                    font.pixelSize: App.fontNormal + 2
                    padding: 0
                    leftPadding: 0
                    rightPadding: 0
                    topPadding: 0
                    bottomPadding: 0
                    background: null
                    placeholderText: I18n.tr( "Jump to component, or type > for commands..." )
                    text: CommandCenter.searchText

                    Component.onCompleted: root.focusSearchEnd()
                    onVisibleChanged: if (visible) root.focusSearchEnd()

                    onTextEdited: CommandCenter.searchText = text

                    TextMetrics {
                        id: prefixMetrics
                        font: searchField.font
                        text: searchField.text
                    }

                    AppText {
                        visible: /^>\s*$/.test(searchField.text)
                            && !searchField.preeditText
                            && searchField.selectedText.length === 0
                        x: searchField.leftPadding + Math.ceil(prefixMetrics.advanceWidth) + 4
                        y: searchField.topPadding
                        width: Math.max(0, searchField.width - x - searchField.rightPadding)
                        height: searchField.height - searchField.topPadding - searchField.bottomPadding
                        text: I18n.tr( "Search commands or components..." )
                        font: searchField.font
                        color: searchField.placeholderTextColor
                        verticalAlignment: Text.AlignVCenter
                        elide: Text.ElideRight
                    }

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
                            searchField.cursorPosition = searchField.length
                            event.accepted = true
                        }
                    }
                }

                AppToolButton {
                    id: clearBtn
                    visible: root.hasFilterQuery
                    Layout.preferredWidth: 22
                    Layout.preferredHeight: 22
                    Layout.alignment: Qt.AlignVCenter
                    padding: 0
                    focusPolicy: Qt.NoFocus
                    ToolTip.visible: hovered
                    ToolTip.text: I18n.tr( "Clear" )
                    onClicked: {
                        CommandCenter.searchText = root.isCommandMode ? ">" : ""
                        root.focusSearchEnd()
                    }

                    contentItem: Item {
                        implicitWidth: 14
                        implicitHeight: 14
                        AppIcon {
                            anchors.centerIn: parent
                            icon: "close"
                            size: 14
                            color: appTheme.windowText
                            opacity: clearBtn.hovered ? 0.95 : 0.6
                        }
                    }
                }

                Rectangle {
                    id: modePill
                    Layout.alignment: Qt.AlignVCenter
                    implicitWidth: Math.ceil(modeLabel.implicitWidth) + 14
                    implicitHeight: 22
                    radius: height / 2
                    color: root.isCommandMode
                        ? (modeMouse.pressed
                            ? Qt.rgba(appTheme.highlight.r, appTheme.highlight.g, appTheme.highlight.b, 0.28)
                            : modeMouse.containsMouse
                                ? Qt.rgba(appTheme.highlight.r, appTheme.highlight.g, appTheme.highlight.b, 0.22)
                                : Qt.rgba(appTheme.highlight.r, appTheme.highlight.g, appTheme.highlight.b, 0.14))
                        : (modeMouse.pressed
                            ? Qt.rgba(appTheme.windowText.r, appTheme.windowText.g, appTheme.windowText.b, 0.18)
                            : modeMouse.containsMouse
                                ? Qt.rgba(appTheme.windowText.r, appTheme.windowText.g, appTheme.windowText.b, 0.14)
                                : Qt.rgba(appTheme.windowText.r, appTheme.windowText.g, appTheme.windowText.b, 0.08))
                    border.width: 1
                    border.color: root.isCommandMode
                        ? Qt.rgba(appTheme.highlight.r, appTheme.highlight.g, appTheme.highlight.b, 0.45)
                        : Qt.rgba(appTheme.windowText.r, appTheme.windowText.g, appTheme.windowText.b, 0.14)
                    Behavior on color { ColorAnimation { duration: 90 } }
                    Behavior on border.color { ColorAnimation { duration: 90 } }

                    AppText {
                        id: modeLabel
                        anchors.centerIn: parent
                        text: root.isCommandMode ? I18n.tr( "Commands" ) : I18n.tr( "Jump" )
                        font.pixelSize: App.fontSmall
                        font.weight: Font.Medium
                        color: root.isCommandMode ? appTheme.highlight : appTheme.windowText
                    }

                    MouseArea {
                        id: modeMouse
                        anchors.fill: parent
                        hoverEnabled: true
                        acceptedButtons: Qt.LeftButton
                        cursorShape: Qt.PointingHandCursor
                        ToolTip.delay: 600
                        ToolTip.visible: containsMouse
                        ToolTip.text: {
                            var actionId = root.isCommandMode ? "view.jumpRef" : "view.cmdPalette"
                            var label = root.isCommandMode
                                ? I18n.tr( "Switch to Jump to Reference" )
                                : I18n.tr( "Switch to Command Center" )
                            var sc = UiUtils.shortcutFor(actionId)
                            return sc !== "" ? (label + " (" + sc + ")") : label
                        }
                        onClicked: {
                            CommandCenter.togglePalettePrefix(!root.isCommandMode)
                            root.focusSearchEnd()
                        }
                    }
                }
            }
        }

        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 1
            color: appTheme.mid
            opacity: 0.45
        }

        Item {
            Layout.fillWidth: true
            Layout.fillHeight: true

            AppText {
                anchors.centerIn: parent
                visible: list.count === 0
                text: root.isCommandMode
                    ? I18n.tr( "No matching commands" )
                    : I18n.tr( "No matching components" )
                color: appTheme.placeholderText
            }

            ListView {
                id: list
                anchors.fill: parent
                anchors.margins: 8
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
                    height: 30
                    radius: 8
                    color: index === list.currentIndex ? appTheme.highlight : (hover.hovered ? appTheme.alternateBase : "transparent")

                    RowLayout {
                        anchors.fill: parent
                        anchors.leftMargin: 8
                        anchors.rightMargin: 8
                        spacing: 8

                        AppIcon {
                            visible: Boolean(delegateItem.modelData.icon)
                            icon: delegateItem.modelData.icon || ""
                            size: 16
                            color: delegateItem.index === list.currentIndex ? appTheme.highlightedText : appTheme.windowText
                            Layout.preferredWidth: 18
                        }

                        AppText {
                            text: delegateItem.modelData.text || ""
                            color: delegateItem.index === list.currentIndex ? appTheme.highlightedText : appTheme.windowText
                            elide: Text.ElideRight
                            verticalAlignment: Text.AlignVCenter
                            Layout.fillWidth: true
                        }

                        AppText {
                            visible: delegateItem.modelData.shortcut && delegateItem.modelData.shortcut.length > 0
                            text: UiUtils.formatShortcut(delegateItem.modelData.shortcut || "")
                            font.pixelSize: App.fontSmall
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
                            if (delegateItem.modelData && (delegateItem.modelData.type === "action" || delegateItem.modelData.type === "component")) {
                                CommandCenter.currentRow = delegateItem.index
                                var isComp = delegateItem.modelData.type === "component"
                                shortcutDialog.actionId = isComp
                                    ? (delegateItem.modelData.actionId || "")
                                    : (delegateItem.modelData.data || "")
                                shortcutDialog.actionName = delegateItem.modelData.text || ""
                                shortcutDialog.currentShortcut = delegateItem.modelData.shortcut || ""
                                shortcutDialog.defaultShortcut = delegateItem.modelData.defaultShortcut || ""
                                shortcutDialog.singleKeyOnly = isComp
                                actionContextMenu.popup()
                            }
                        }
                    }
                }
            }
        }
    }

    AppContextMenu {
        id: actionContextMenu
        ContextMenuItem {
            text: I18n.tr("Edit Shortcut...")
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
