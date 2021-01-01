import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import cs_app

/* Flattened component tree. C++ used QAbstractItemModel + TreeView; qt-bridge
 * has QListModel only, so rows come from ComponentList.rows JSON. */
Item {
    id: root
    function tr(s) { return App.i18nTick >= 0 ? App.translate(s) : s }
    SystemPalette { id: appTheme }
    anchors.fill: parent

    readonly property int typeComponent: 1
    readonly property int typeCategMain: 2
    readonly property int typeCategChild: 3

    function syncRows() {
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
        function onRowsChanged() { root.syncRows() }
        function onSearchApplied( filter ) {
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
            text: root.tr( "Clear Recently Used" )
            iconLigature: "delete"
            onTriggered: ComponentList.clearRecentlyUsed()
        }
        ContextMenuItem {
            text: root.tr( "Reload Components" )
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
                if ( model.itemType === root.typeCategMain ) return 30
                if ( model.itemType === root.typeCategChild ) return 20
                return 22
            }

            readonly property bool isCategory: model.itemType !== root.typeComponent

            Drag.active: dragHandler.active
            Drag.source: row
            Drag.imageSource: (!row.isCategory && model.iconUri) ? model.iconUri : ""
            Drag.hotSpot.x: 16
            Drag.hotSpot.y: 16
            Drag.mimeData: {
                var cap = model.caption || model.display
                return { "text/plain": (cap === model.compType) ? model.compType : (cap + "," + model.compType) }
            }
            Drag.dragType: Drag.Automatic

            Rectangle {
                anchors.fill: parent
                anchors.leftMargin: 4
                anchors.rightMargin: 4
                radius: 4
                visible: hover.hovered
                color: appTheme.highlight
            }

            Row {
                anchors.fill: parent
                anchors.leftMargin: 6 + model.depth * 14
                anchors.rightMargin: 6
                spacing: 6

                AppIcon {
                    visible: row.isCategory
                    width: 14
                    anchors.verticalCenter: parent.verticalCenter
                    text: model.expanded ? "expand_more" : "chevron_right"
                    font.pixelSize: 16
                    color: hover.hovered ? appTheme.highlightedText : appTheme.windowText
                }

                ItemDelegate {
                    visible: !row.isCategory
                    anchors.verticalCenter: parent.verticalCenter
                    height: parent.height
                    width: parent.width
                    padding: 0
                    spacing: 6
                    icon.source: model.iconUri || ""
                    icon.width: 16
                    icon.height: 16
                    icon.color: hover.hovered ? appTheme.highlightedText : appTheme.windowText
                    text: model.display
                    font.family: App.fontFamily
                    font.pixelSize: 12
                    palette.buttonText: hover.hovered ? appTheme.highlightedText : appTheme.windowText
                    palette.windowText: hover.hovered ? appTheme.highlightedText : appTheme.windowText
                    palette.text: hover.hovered ? appTheme.highlightedText : appTheme.windowText
                    background: null
                    hoverEnabled: false
                }

                Text {
                    visible: row.isCategory
                    anchors.verticalCenter: parent.verticalCenter
                    text: model.display
                    font.family: App.fontFamily
                    font.bold: true
                    font.pixelSize: model.itemType === root.typeCategMain ? 13 : 12
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
                        ComponentList.toggleExpanded( model.id )
                }
                onDoubleTapped: {
                    if ( row.isCategory )
                        return
                    var t = model.compType
                    var cap = model.caption || model.display
                    var spec = (cap === model.compType) ? model.compType : (cap + "," + model.compType)
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
