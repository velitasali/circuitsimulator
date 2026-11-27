pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import cs_app
import cs_ui

/* Flattened component tree. C++ used QAbstractItemModel + TreeView; qt-bridge
 * has QListModel only, so rows come from ComponentList.rows JSON. */
Item {
    id: root
    SystemPalette { id: appTheme }
    anchors.fill: parent

    readonly property int typeComponent: 1
    readonly property int typeCategMain: 2
    readonly property int typeCategChild: 3

    property var hoveredItemInfo: null
    property real hoveredRowCenterY: 0
    property bool hoverCardVisible: false

    property real pendingHoverId: -1
    property Item pendingHoverRow: null

    function clearHover(): void {
        hoverDelayTimer.stop()
        hoverLeaveTimer.stop()
        root.pendingHoverId = -1
        root.pendingHoverRow = null
        root.hoverCardVisible = false
        root.hoveredItemInfo = null
    }

    function applyHover(itemId: real, rowItem: Item): void {
        if (!rowItem || contextMenu.opened || list.moving) {
            root.clearHover()
            return
        }
        var info = ComponentList.componentInfo(itemId)
        if (!info) {
            root.clearHover()
            return
        }
        root.hoveredItemInfo = info
        root.hoveredRowCenterY = rowItem.mapToItem(root, 0, rowItem.height / 2).y
        root.hoverCardVisible = true
    }

    function beginRowHover(itemId: real, rowItem: Item, isCat: bool): void {
        hoverLeaveTimer.stop()
        if (isCat || contextMenu.opened) {
            root.clearHover()
            return
        }
        root.pendingHoverId = itemId
        root.pendingHoverRow = rowItem
        if (root.hoverCardVisible) {
            hoverDelayTimer.stop()
            root.applyHover(itemId, rowItem)
        } else {
            hoverDelayTimer.restart()
        }
    }

    function endRowHover(itemId: real): void {
        if (root.pendingHoverId === itemId) {
            hoverDelayTimer.stop()
            root.pendingHoverId = -1
            root.pendingHoverRow = null
            if (root.hoverCardVisible)
                hoverLeaveTimer.restart()
        }
    }

    onVisibleChanged: if (!visible) root.clearHover()

    Timer {
        id: hoverDelayTimer
        interval: 350
        repeat: false
        onTriggered: {
            if (root.pendingHoverId >= 0 && root.pendingHoverRow)
                root.applyHover(root.pendingHoverId, root.pendingHoverRow)
        }
    }

    Timer {
        id: hoverLeaveTimer
        interval: 60
        repeat: false
        onTriggered: root.clearHover()
    }

    function syncRows(): void {
        var arr = ComponentList.rows
        if ( !arr )
            return
        if ( rowModel.count === 0 ) {
            for ( var i = 0; i < arr.length; ++i )
                rowModel.append( arr[i] )
            return
        }
        if ( arr.length === 0 ) {
            rowModel.clear()
            return
        }

        var newIndexMap = {}
        for ( var k = 0; k < arr.length; ++k ) {
            newIndexMap[arr[k].id] = k
        }

        var i = 0
        var j = 0
        while ( i < rowModel.count && j < arr.length ) {
            var oldItem = rowModel.get( i )
            var newItem = arr[j]

            if ( oldItem.id === newItem.id ) {
                if ( oldItem.expanded !== newItem.expanded ||
                     oldItem.display !== newItem.display ||
                     oldItem.caption !== newItem.caption ||
                     oldItem.depth !== newItem.depth ||
                     oldItem.hasChildren !== newItem.hasChildren ||
                     oldItem.itemType !== newItem.itemType ||
                     oldItem.compType !== newItem.compType ||
                     oldItem.spec !== newItem.spec ||
                     oldItem.actionId !== newItem.actionId ||
                     oldItem.shortcut !== newItem.shortcut ||
                     oldItem.defaultShortcut !== newItem.defaultShortcut ||
                     oldItem.iconUri !== newItem.iconUri ||
                     oldItem.isCustom !== newItem.isCustom ) {
                    rowModel.set( i, newItem )
                }
                i++
                j++
            } else if ( newIndexMap[oldItem.id] === undefined ) {
                rowModel.remove( i, 1 )
            } else if ( newIndexMap[oldItem.id] > j ) {
                rowModel.insert( i, newItem )
                i++
                j++
            } else {
                rowModel.remove( i, 1 )
            }
        }

        while ( i < rowModel.count ) {
            rowModel.remove( i, 1 )
        }

        while ( j < arr.length ) {
            rowModel.append( arr[j] )
            j++
        }
    }

    ListModel { id: rowModel }

    Connections {
        target: ComponentList
        function onRowsChanged(): void {
            root.clearHover()
            root.syncRows()
        }
        function onSearchApplied(filter: string): void {
            root.clearHover()
            root.syncRows()
            list.positionViewAtBeginning()
        }
    }
    Connections {
        target: App
        function onI18nTickChanged() {
            root.clearHover()
            ComponentList.reloadComponents()
            root.syncRows()
        }
    }
    Component.onCompleted: root.syncRows()

    AppContextMenu {
        id: contextMenu
        property string targetActionId: ""
        property string targetName: ""
        property string targetShortcut: ""
        property string targetDefaultShortcut: ""
        onAboutToShow: root.clearHover()
        ContextMenuItem {
            visible: contextMenu.targetActionId.length > 0
            text: contextMenu.targetShortcut.length > 0
                ? I18n.tr( "Edit Shortcut..." )
                : I18n.tr( "Assign Shortcut..." )
            shortcutText: contextMenu.targetShortcut
            iconLigature: "keyboard"
            onTriggered: {
                CircuitPanel.promoteFloating( "components" )
                compShortcutDialog.actionId = contextMenu.targetActionId
                compShortcutDialog.actionName = contextMenu.targetName
                compShortcutDialog.currentShortcut = contextMenu.targetShortcut
                compShortcutDialog.defaultShortcut = contextMenu.targetDefaultShortcut
                compShortcutDialog.open()
            }
        }
        ContextMenuSeparator { visible: contextMenu.targetActionId.length > 0 }
        ContextMenuItem {
            text: I18n.tr( "Clear Recently Used" )
            iconLigature: "delete"
            onTriggered: ComponentList.clearRecentlyUsed()
        }
        ContextMenuItem {
            text: I18n.tr( "Reload Components" )
            iconLigature: "refresh"
            onTriggered: ComponentList.reloadComponents()
        }
    }

    ShortcutDialog {
        id: compShortcutDialog
        singleKeyOnly: true
    }

    MouseArea {
        anchors.fill: parent
        acceptedButtons: Qt.RightButton
        onClicked: {
            contextMenu.targetActionId = ""
            contextMenu.targetName = ""
            contextMenu.targetShortcut = ""
            contextMenu.targetDefaultShortcut = ""
            contextMenu.popup()
        }
    }

    ListView {
        id: list
        anchors.fill: parent
        anchors.margins: 1
        model: rowModel
        boundsBehavior: Flickable.StopAtBounds
        ScrollBar.vertical: AppScrollBar {}
        clip: true
        onContentYChanged: root.clearHover()
        onMovingChanged: if (list.moving) root.clearHover()

        delegate: Item {
            id: row
            required property var model
            required property int index
            Component.onDestruction: if (root.pendingHoverRow === row) root.clearHover()
            width: list.width
            height: {
                if ( row.model.itemType === root.typeCategMain ) return 30
                if ( row.model.itemType === root.typeCategChild ) return 20
                return 24
            }

            readonly property bool isCategory: model.itemType !== root.typeComponent

            // QQuickPixmap does not apply icon.color. Component SVGs stroke
            // with currentColor, which Qt SVG draws as black. tintedIconUrl
            // rewrites that to the theme color and returns a file URL.
            Drag.active: dragHandler.active
            Drag.source: row
            Drag.imageSource: ( (hover.hovered || dragHandler.active) && !row.isCategory && model.iconUri )
                ? ComponentList.tintedIconUrl( model.iconUri, appTheme.windowText.toString() )
                : ""
            Drag.hotSpot.x: 10
            Drag.hotSpot.y: 10
            Drag.mimeData: {
                var cap = row.model.caption || row.model.display
                return { "text/plain": (cap === row.model.compType) ? row.model.compType : (cap + "," + row.model.compType) }
            }
            Drag.dragType: Drag.Automatic

            Rectangle {
                anchors.fill: parent
                anchors.leftMargin: 4
                anchors.rightMargin: 4
                radius: 6
                visible: hover.hovered
                color: appTheme.highlight
            }

            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: 6 + row.model.depth * 14
                anchors.rightMargin: 8
                spacing: 6

                AppIcon {
                    visible: row.isCategory
                    Layout.preferredWidth: 14
                    Layout.alignment: Qt.AlignVCenter
                    icon: row.model.expanded ? "expand_more" : "chevron_right"
                    size: 16
                    color: hover.hovered ? appTheme.highlightedText : appTheme.windowText
                }

                ItemDelegate {
                    visible: !row.isCategory
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    Layout.alignment: Qt.AlignVCenter
                    padding: 0
                    spacing: 6
                    icon.source: row.model.iconUri || ""
                    icon.width: 20
                    icon.height: 20
                    icon.color: hover.hovered ? appTheme.highlightedText : appTheme.windowText
                    text: row.model.display
                    font.family: App.fontFamily
                    font.pixelSize: App.fontNormal
                    palette.buttonText: hover.hovered ? appTheme.highlightedText : appTheme.windowText
                    palette.windowText: hover.hovered ? appTheme.highlightedText : appTheme.windowText
                    palette.text: hover.hovered ? appTheme.highlightedText : appTheme.windowText
                    background: null
                    hoverEnabled: false
                }

                Rectangle {
                    visible: !row.isCategory && Boolean(row.model.shortcut && row.model.shortcut.length > 0)
                    Layout.alignment: Qt.AlignVCenter
                    implicitWidth: Math.max(18, shortcutBadgeLabel.implicitWidth + 8)
                    implicitHeight: 16
                    radius: 4
                    color: hover.hovered
                        ? Qt.rgba(appTheme.highlightedText.r, appTheme.highlightedText.g, appTheme.highlightedText.b, 0.18)
                        : Qt.rgba(appTheme.windowText.r, appTheme.windowText.g, appTheme.windowText.b, 0.08)
                    border.color: hover.hovered
                        ? Qt.rgba(appTheme.highlightedText.r, appTheme.highlightedText.g, appTheme.highlightedText.b, 0.35)
                        : Qt.rgba(appTheme.windowText.r, appTheme.windowText.g, appTheme.windowText.b, 0.2)
                    border.width: 1

                    AppText {
                        id: shortcutBadgeLabel
                        anchors.centerIn: parent
                        text: row.model.shortcut || ""
                        font.pixelSize: App.fontMicro
                        font.bold: true
                        color: hover.hovered ? appTheme.highlightedText : appTheme.windowText
                        opacity: hover.hovered ? 1.0 : 0.72
                    }
                }

                AppText {
                    visible: row.isCategory
                    Layout.fillWidth: true
                    Layout.alignment: Qt.AlignVCenter
                    text: row.model.display
                    font.bold: true
                    color: hover.hovered ? appTheme.highlightedText : appTheme.windowText
                    elide: Text.ElideRight
                }
            }

            HoverHandler {
                id: hover
                onHoveredChanged: {
                    if (hover.hovered) {
                        if (!dragHandler.active)
                            root.beginRowHover(row.model.id, row, row.isCategory)
                    } else {
                        root.endRowHover(row.model.id)
                    }
                }
            }

            TapHandler {
                acceptedButtons: Qt.LeftButton
                onTapped: {
                    root.clearHover()
                    if ( row.isCategory )
                        ComponentList.toggleExpanded( row.model.id )
                }
                onDoubleTapped: {
                    root.clearHover()
                    if ( row.isCategory )
                        return
                    var cap = row.model.caption || row.model.display
                    var fallbackSpec = (cap === row.model.compType) ? row.model.compType : (cap + "," + row.model.compType)
                    CircuitCanvas.insertComponent( row.model.spec || fallbackSpec )
                }
            }

            TapHandler {
                acceptedButtons: Qt.RightButton
                onTapped: {
                    if ( !row.isCategory && row.model.actionId ) {
                        contextMenu.targetActionId = row.model.actionId
                        contextMenu.targetName = row.model.display || row.model.caption || ""
                        contextMenu.targetShortcut = row.model.shortcut || ""
                        contextMenu.targetDefaultShortcut = row.model.defaultShortcut || ""
                    } else {
                        contextMenu.targetActionId = ""
                        contextMenu.targetName = ""
                        contextMenu.targetShortcut = ""
                        contextMenu.targetDefaultShortcut = ""
                    }
                    contextMenu.popup()
                }
            }

            DragHandler {
                id: dragHandler
                target: null
                enabled: !row.isCategory
                onActiveChanged: if (dragHandler.active) root.clearHover()
            }
        }
    }
}
