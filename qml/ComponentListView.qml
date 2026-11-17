pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls
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
        function onRowsChanged(): void { root.syncRows() }
        function onSearchApplied(filter: string): void {
            root.syncRows()
            list.positionViewAtBeginning()
        }
    }
    Connections {
        target: App
        function onI18nTickChanged() {
            ComponentList.reloadComponents()
            root.syncRows()
        }
    }
    Component.onCompleted: root.syncRows()

    AppContextMenu {
        id: contextMenu
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

    MouseArea {
        anchors.fill: parent
        acceptedButtons: Qt.RightButton
        onClicked: contextMenu.popup()
    }

    ListView {
        id: list
        anchors.fill: parent
        anchors.margins: 1
        model: rowModel
        boundsBehavior: Flickable.StopAtBounds
        ScrollBar.vertical: AppScrollBar {}
        clip: true

        delegate: Item {
            id: row
            required property var model
            required property int index
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

            Row {
                anchors.fill: parent
                anchors.leftMargin: 6 + row.model.depth * 14
                anchors.rightMargin: 6
                spacing: 6

                AppIcon {
                    visible: row.isCategory
                    width: 14
                    anchors.verticalCenter: parent.verticalCenter
                    icon: row.model.expanded ? "expand_more" : "chevron_right"
                    size: 16
                    color: hover.hovered ? appTheme.highlightedText : appTheme.windowText
                }

                ItemDelegate {
                    visible: !row.isCategory
                    anchors.verticalCenter: parent.verticalCenter
                    height: parent.height
                    width: parent.width
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

                AppText {
                    visible: row.isCategory
                    anchors.verticalCenter: parent.verticalCenter
                    text: row.model.display
                    font.bold: true
                    color: hover.hovered ? appTheme.highlightedText : appTheme.windowText
                    elide: Text.ElideRight
                    width: parent.width - 24
                }
            }

            HoverHandler { id: hover }

            TapHandler {
                acceptedButtons: Qt.LeftButton
                onTapped: {
                    if ( row.isCategory )
                        ComponentList.toggleExpanded( row.model.id )
                }
                onDoubleTapped: {
                    if ( row.isCategory )
                        return
                    var t = row.model.compType
                    var cap = row.model.caption || row.model.display
                    var spec = (cap === row.model.compType) ? row.model.compType : (cap + "," + row.model.compType)
                    ComponentList.addRecent( spec )
                    if ( t === "Oscope" || t === "Oscilloscope" ) {
                        CircuitCanvas.addComponent( t )
                        App.showOsc()
                    } else if ( t === "LAnalizer" ) {
                        CircuitCanvas.addComponent( t )
                        App.showLa()
                    } else if ( t === "MCU" ) {
                        if ( cap === "MCU" )
                            App.showMcu()
                        else
                            CircuitCanvas.addComponent( cap + "," + t )
                    } else if ( t === "QemuDevice" ) {
                        CircuitCanvas.addComponent( cap + "," + t )
                        App.showMcu()
                    } else if ( t === "SerialPort" ) {
                        CircuitCanvas.addComponent( cap + "," + t )
                        App.showSerialMon()
                    } else if ( t === "SerialTerm" ) {
                        CircuitCanvas.addComponent( cap + "," + t )
                        App.showTerminal()
                    } else {
                        CircuitCanvas.addComponent( spec )
                    }
                }
            }

            TapHandler {
                acceptedButtons: Qt.RightButton
                onTapped: contextMenu.popup()
            }

            DragHandler {
                id: dragHandler
                target: null
                enabled: !row.isCategory
            }
        }
    }
}
