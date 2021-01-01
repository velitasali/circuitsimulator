import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import cs_app

Rectangle {
    id: root
    function tr(s) { return App.i18nTick >= 0 ? App.translate(s) : s }
    SystemPalette { id: appTheme }
    readonly property font uiFont: Qt.font({ family: App.fontFamily || "Ubuntu", pixelSize: App.fontSize || 13 })
    color: appTheme.window
    anchors.fill: parent

    readonly property var emptyRow: ({})

    function tabSource() {
        if ( McuMonitor.tabs && tabs.currentIndex >= 0 && tabs.currentIndex < McuMonitor.tabs.length )
            return McuMonitor.tabs[tabs.currentIndex].source
        return "watch"
    }

    Timer {
        interval: 150
        running: App.mcuVisible
        repeat: true
        onTriggered: {
            CircuitCanvas.syncMcu()
            McuMonitor.refresh()
        }
    }

    Connections {
        target: App
        function onMcuVisibleChanged() {
            if ( App.mcuVisible ) {
                CircuitCanvas.syncMcu()
                McuMonitor.refresh()
            }
        }
    }

    component TableFrame: Rectangle {
        id: frame
        color: appTheme.base
        border.color: appTheme.mid
        border.width: 1
        radius: 3
        clip: true
        Layout.fillWidth: true
        Layout.fillHeight: true
        Layout.minimumHeight: 0

        property alias header: headerData.data
        property int headerHeight: 22
        default property alias listData: listHost.data

        ColumnLayout {
            anchors.fill: parent
            anchors.margins: 1
            spacing: 0

            Rectangle {
                Layout.fillWidth: true
                height: frame.headerHeight
                color: appTheme.window
                Item {
                    id: headerData
                    anchors.fill: parent
                    anchors.leftMargin: 6
                    anchors.rightMargin: 14
                }
                Rectangle {
                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.bottom: parent.bottom
                    height: 1
                    color: appTheme.mid
                }
            }

            Item {
                id: listHost
                Layout.fillWidth: true
                Layout.fillHeight: true
                Layout.minimumHeight: 0
            }
        }
    }

    component HexRow: Rectangle {
        id: hexRow
        required property int index
        property var rowData: root.emptyRow
        property int columns: 16
        property int cellW: 22
        property int spacingW: 2
        property int memType: 0
        property bool wordMode: false
        property int pcCol: -1
        property int hoverCol: -1

        width: ListView.view ? ListView.view.width : 100
        height: 20
        color: index % 2 ? appTheme.alternateBase : "transparent"

        readonly property int hexWidth: columns * cellW + Math.max( 0, columns - 1 ) * spacingW

        Row {
            anchors.fill: parent
            anchors.leftMargin: 6
            anchors.rightMargin: 14
            spacing: 0

            Text {
                width: 46
                height: parent.height
                text: ( hexRow.rowData && hexRow.rowData.address ) ? hexRow.rowData.address : "0000"
                font.family: "Menlo"
                font.pixelSize: 11
                font.bold: true
                color: appTheme.mid
                verticalAlignment: Text.AlignVCenter
            }

            Item {
                width: hexRow.hexWidth
                height: parent.height

                Repeater {
                    model: hexRow.columns
                    Rectangle {
                        required property int index
                        x: index * ( hexRow.cellW + hexRow.spacingW )
                        width: hexRow.cellW
                        height: parent.height - 2
                        y: 1
                        radius: 2
                        readonly property string hexVal: {
                            if ( !hexRow.rowData )
                                return hexRow.wordMode ? "----" : "--"
                            if ( hexRow.wordMode )
                                return ( hexRow.rowData.words && index < hexRow.rowData.words.length ) ? hexRow.rowData.words[index] : "----"
                            return ( hexRow.rowData.bytes && index < hexRow.rowData.bytes.length ) ? hexRow.rowData.bytes[index] : "--"
                        }
                        readonly property bool isPc: index === hexRow.pcCol
                        readonly property bool isHover: index === hexRow.hoverCol
                        color: isPc ? appTheme.highlight : ( isHover ? appTheme.midlight : "transparent" )

                        Text {
                            anchors.centerIn: parent
                            text: parent.hexVal
                            font.family: "Menlo"
                            font.pixelSize: 11
                            font.bold: parent.isPc || ( parent.hexVal !== "00" && parent.hexVal !== "0000" && parent.hexVal !== "--" && parent.hexVal !== "----" && parent.hexVal !== "FFFF" && parent.hexVal !== "FF" )
                            color: parent.isPc ? appTheme.highlightedText
                                  : ( parent.isHover ? appTheme.highlight
                                    : ( ( parent.hexVal === "00" || parent.hexVal === "FF" || parent.hexVal === "0000" || parent.hexVal === "FFFF" ) ? appTheme.mid : appTheme.windowText ) )
                        }
                    }
                }

                MouseArea {
                    anchors.fill: parent
                    hoverEnabled: true
                    cursorShape: Qt.PointingHandCursor
                    onPositionChanged: function( mouse ) {
                        var stride = hexRow.cellW + hexRow.spacingW
                        hexRow.hoverCol = Math.min( hexRow.columns - 1, Math.max( 0, Math.floor( mouse.x / stride ) ) )
                    }
                    onExited: hexRow.hoverCol = -1
                    onClicked: function( mouse ) {
                        var stride = hexRow.cellW + hexRow.spacingW
                        var col = Math.min( hexRow.columns - 1, Math.max( 0, Math.floor( mouse.x / stride ) ) )
                        var addr = ( hexRow.rowData && hexRow.rowData.addrs && col < hexRow.rowData.addrs.length ) ? hexRow.rowData.addrs[col] : -1
                        var val = ( hexRow.rowData && hexRow.rowData.vals && col < hexRow.rowData.vals.length ) ? hexRow.rowData.vals[col] : -1
                        if ( addr >= 0 )
                            editPopup.openForAddr( addr, val, hexRow.memType )
                    }
                    ToolTip.visible: containsMouse && hexRow.hoverCol >= 0
                    ToolTip.delay: 350
                    ToolTip.text: {
                        var col = hexRow.hoverCol
                        if ( col < 0 || !hexRow.rowData || !hexRow.rowData.addrs || col >= hexRow.rowData.addrs.length )
                            return ""
                        var addr = hexRow.rowData.addrs[col]
                        var val = ( hexRow.rowData.vals && col < hexRow.rowData.vals.length ) ? hexRow.rowData.vals[col] : -1
                        if ( addr < 0 )
                            return ""
                        if ( hexRow.wordMode ) {
                            var hexStr = "0x" + val.toString( 16 ).toUpperCase().padStart( 4, "0" )
                            var isPc = ( McuMonitor.pcValue === addr )
                            return root.tr( "Flash Addr: 0x%1 (%2)%3\nWord: %4 (Dec: %5)\n(Click to edit)" )
                                .arg( addr.toString( 16 ).toUpperCase().padStart( 4, "0" ) )
                                .arg( addr )
                                .arg( isPc ? " [PC]" : "" )
                                .arg( hexStr )
                                .arg( val )
                        }
                        return root.tr( "Address: 0x%1 (%2)\nDec: %3\nHex: 0x%4\nBin: 0b%5\n(Click to edit)" )
                            .arg( addr.toString( 16 ).toUpperCase().padStart( 4, "0" ) )
                            .arg( addr )
                            .arg( val.toString( 10 ) )
                            .arg( val.toString( 16 ).toUpperCase().padStart( 2, "0" ) )
                            .arg( val.toString( 2 ).padStart( 8, "0" ) )
                    }
                }
            }

            Text {
                width: Math.max( 40, parent.width - 46 - hexRow.hexWidth )
                height: parent.height
                leftPadding: 8
                text: ( hexRow.rowData && hexRow.rowData.ascii ) ? hexRow.rowData.ascii : ""
                font.family: "Menlo"
                font.pixelSize: 11
                color: appTheme.highlight
                verticalAlignment: Text.AlignVCenter
            }
        }

        ListView.onPooled: hoverCol = -1
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 8
        spacing: 8

        Label {
            visible: McuMonitor.mcuId.length === 0
            text: root.tr( "No MCU in this circuit. Open or create a circuit with an MCU component." )
            font: root.uiFont
            color: appTheme.windowText
            wrapMode: Text.WordWrap
            Layout.fillWidth: true
        }

        RowLayout {
            visible: McuMonitor.hasStatus
            spacing: 8
            Layout.fillWidth: true

            Rectangle {
                Layout.preferredHeight: 28
                implicitWidth: statusRow.implicitWidth + 8
                color: appTheme.base
                radius: 4
                border.color: appTheme.mid
                border.width: 1

                RowLayout {
                    id: statusRow
                    anchors.fill: parent
                    anchors.margins: 3
                    spacing: 4

                    Label {
                        text: root.tr( "STATUS" )
                        font.family: root.uiFont.family
                        font.bold: true
                        font.pixelSize: 10
                        color: appTheme.windowText
                        Layout.leftMargin: 2
                        Layout.rightMargin: 2
                    }

                    Row {
                        spacing: 2
                        Repeater {
                            model: 8
                            delegate: Rectangle {
                                required property int index
                                width: 24; height: 22
                                radius: 3
                                readonly property bool bitSet: ( McuMonitor.statusValue >> ( 7 - index ) ) & 1
                                readonly property string bitName: ( index < McuMonitor.statusBits.length && McuMonitor.statusBits[index].length > 0 ) ? McuMonitor.statusBits[index] : ( "b" + ( 7 - index ) )
                                color: bitSet ? ( CircuitCanvas.pinInputHighColor || appTheme.highlight ) : appTheme.alternateBase
                                border.color: mouseArea.containsMouse ? appTheme.highlight : appTheme.mid
                                border.width: 1

                                Text {
                                    anchors.centerIn: parent
                                    text: bitName
                                    font.family: root.uiFont.family
                                    font.pixelSize: 9
                                    font.bold: true
                                    color: bitSet ? appTheme.highlightedText : appTheme.text
                                }

                                MouseArea {
                                    id: mouseArea
                                    anchors.fill: parent
                                    hoverEnabled: true
                                    cursorShape: Qt.PointingHandCursor
                                    onClicked: McuMonitor.toggleStatusBit( 7 - index )

                                    ToolTip.visible: containsMouse
                                    ToolTip.delay: 400
                                    ToolTip.text: root.tr( "Bit %1 (%2): %3\nClick to toggle" ).arg( 7 - index ).arg( bitName ).arg( bitSet ? "1" : "0" )
                                }
                            }
                        }
                    }
                }
            }

            Rectangle {
                Layout.preferredHeight: 28
                implicitWidth: pcRow.implicitWidth + 12
                color: appTheme.base
                radius: 4
                border.color: appTheme.mid
                border.width: 1

                RowLayout {
                    id: pcRow
                    anchors.fill: parent
                    anchors.margins: 4
                    spacing: 6

                    Label {
                        text: root.tr( "PC" )
                        font.family: root.uiFont.family
                        font.bold: true
                        font.pixelSize: 10
                        color: appTheme.windowText
                    }
                    Text {
                        text: McuMonitor.pcValue
                        font.family: "Menlo"
                        font.bold: true
                        font.pixelSize: 12
                        color: appTheme.highlight
                    }
                    Text {
                        text: "(" + McuMonitor.pcHex + ")"
                        font.family: "Menlo"
                        font.pixelSize: 11
                        color: appTheme.windowText
                        opacity: 0.75
                    }
                }
            }

            Item { Layout.fillWidth: true }

            AppCheckBox {
                text: root.tr( "Byte Mode" )
                font: root.uiFont
                checked: McuMonitor.byteMode
                onToggled: McuMonitor.byteMode = checked
                ToolTip.visible: hovered
                ToolTip.delay: 400
                ToolTip.text: root.tr( "Show Program Counter & addresses in byte units" )
            }

            AppCheckBox {
                text: root.tr( "Jump to active address" )
                font: root.uiFont
                checked: McuMonitor.jumpToAddress
                onToggled: McuMonitor.jumpToAddress = checked
                ToolTip.visible: hovered
                ToolTip.delay: 400
                ToolTip.text: root.tr( "Automatically track and scroll to active Program Counter in memory views" )
            }

            AppButton {
                text: root.tr( "↻ Refresh" )
                font: root.uiFont
                onClicked: {
                    CircuitCanvas.syncMcu()
                    McuMonitor.refresh()
                }
            }
        }

        AppTabBar {
            id: tabs
            Layout.fillWidth: true
            Repeater {
                model: McuMonitor.tabs
                AppTabButton {
                    required property var modelData
                    required property int index
                    text: modelData.title
                }
            }
        }
        Connections {
            target: tabs
            function onCurrentIndexChanged() { McuMonitor.setCurrentTab( tabs.currentIndex ) }
        }

        StackLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            currentIndex: {
                var src = root.tabSource()
                if ( src === "ram_matrix" ) return 1
                if ( src === "ram" ) return 2
                if ( src === "flash" ) return 3
                if ( src === "eeprom" ) return 4
                return 0
            }

            // Tab 0: Watch
            SplitView {
                orientation: Qt.Horizontal
                Layout.fillWidth: true
                Layout.fillHeight: true

                SplitView {
                    SplitView.preferredWidth: 240
                    SplitView.minimumWidth: 160
                    SplitView.maximumWidth: 450
                    orientation: Qt.Vertical

                    Rectangle {
                        SplitView.preferredHeight: 220
                        SplitView.minimumHeight: 100
                        color: appTheme.base
                        border.color: appTheme.mid
                        border.width: 1
                        radius: 4

                        ColumnLayout {
                            anchors.fill: parent
                            anchors.margins: 1
                            spacing: 0

                            RowLayout {
                                Layout.fillWidth: true
                                Layout.leftMargin: 6
                                Layout.rightMargin: 6
                                Layout.topMargin: 4
                                Layout.bottomMargin: 4
                                spacing: 4
                                Label {
                                    text: root.tr( "Registers (%1)" ).arg( McuMonitor.registers ? McuMonitor.registers.length : 0 )
                                    font.family: root.uiFont.family
                                    font.bold: true
                                    font.pixelSize: 11
                                    color: appTheme.windowText
                                }
                                Item { Layout.fillWidth: true }
                                AppButton {
                                    text: root.tr( "+ All" )
                                    font.pixelSize: 10
                                    Layout.preferredHeight: 20
                                    padding: 2
                                    onClicked: McuMonitor.addAllRegistersWatch()
                                    ToolTip.visible: hovered
                                    ToolTip.text: root.tr( "Add all SFR registers to Watch list" )
                                }
                            }

                            AppTextField {
                                id: regFilter
                                placeholderText: root.tr( "Filter registers..." )
                                Layout.fillWidth: true
                                Layout.leftMargin: 4
                                Layout.rightMargin: 4
                                Layout.bottomMargin: 2
                                font.family: root.uiFont.family
                                font.pixelSize: 11
                                Layout.preferredHeight: 24
                            }

                            Rectangle {
                                Layout.fillWidth: true
                                height: 1
                                color: appTheme.mid
                            }

                            Item {
                                Layout.fillWidth: true
                                Layout.fillHeight: true
                                Layout.minimumHeight: 0

                                ListView {
                                    id: regListView
                                    anchors.fill: parent
                                    clip: true
                                    boundsBehavior: Flickable.StopAtBounds
                                    flickableDirection: Flickable.VerticalFlick
                                    ScrollBar.vertical: AppScrollBar { policy: ScrollBar.AlwaysOn }
                                    reuseItems: true
                                    readonly property int itemH: 22

                                    model: {
                                        var list = McuMonitor.registers || []
                                        var q = regFilter.text.trim().toLowerCase()
                                        if ( !q ) return list
                                        return list.filter( function( r ) {
                                            return ( r.name && r.name.toLowerCase().indexOf( q ) >= 0 ) ||
                                                   ( r.address && r.address.toLowerCase().indexOf( q ) >= 0 )
                                        } )
                                    }

                                    delegate: Rectangle {
                                        required property var modelData
                                        required property int index
                                        width: ListView.view.width
                                        height: regListView.itemH
                                        radius: 2
                                        color: regMouse.containsMouse ? appTheme.midlight : ( index % 2 ? appTheme.alternateBase : "transparent" )

                                        RowLayout {
                                            anchors.fill: parent
                                            anchors.leftMargin: 6
                                            anchors.rightMargin: 4
                                            spacing: 4

                                            Text {
                                                text: modelData.name || ""
                                                font.family: root.uiFont.family
                                                font.bold: true
                                                font.pixelSize: 11
                                                color: appTheme.windowText
                                                Layout.fillWidth: true
                                                elide: Text.ElideRight
                                            }
                                            Text {
                                                text: modelData.address || ""
                                                font.family: "Menlo"
                                                font.pixelSize: 10
                                                color: appTheme.mid
                                            }
                                            Text {
                                                text: "+"
                                                font.family: root.uiFont.family
                                                font.bold: true
                                                font.pixelSize: 13
                                                color: appTheme.highlight
                                                opacity: regMouse.containsMouse ? 1.0 : 0.0
                                            }
                                        }

                                        MouseArea {
                                            id: regMouse
                                            anchors.fill: parent
                                            hoverEnabled: true
                                            cursorShape: Qt.PointingHandCursor
                                            onClicked: McuMonitor.addWatch( modelData.name, modelData.name, "uint8" )
                                            ToolTip.visible: containsMouse
                                            ToolTip.delay: 400
                                            ToolTip.text: root.tr( "Click to watch %1 (%2)" ).arg( modelData.name ).arg( modelData.address )
                                        }
                                    }
                                }
                            }
                        }
                    }

                    Rectangle {
                        SplitView.fillHeight: true
                        SplitView.minimumHeight: 100
                        color: appTheme.base
                        border.color: appTheme.mid
                        border.width: 1
                        radius: 4

                        ColumnLayout {
                            anchors.fill: parent
                            anchors.margins: 1
                            spacing: 0

                            RowLayout {
                                Layout.fillWidth: true
                                Layout.leftMargin: 6
                                Layout.rightMargin: 6
                                Layout.topMargin: 4
                                Layout.bottomMargin: 4
                                spacing: 4
                                Label {
                                    text: root.tr( "Variables (%1)" ).arg( McuMonitor.variables ? McuMonitor.variables.length : 0 )
                                    font.family: root.uiFont.family
                                    font.bold: true
                                    font.pixelSize: 11
                                    color: appTheme.windowText
                                }
                                Item { Layout.fillWidth: true }
                            }

                            AppTextField {
                                id: varFilter
                                placeholderText: root.tr( "Filter variables..." )
                                Layout.fillWidth: true
                                Layout.leftMargin: 4
                                Layout.rightMargin: 4
                                Layout.bottomMargin: 2
                                font.family: root.uiFont.family
                                font.pixelSize: 11
                                Layout.preferredHeight: 24
                            }

                            Rectangle {
                                Layout.fillWidth: true
                                height: 1
                                color: appTheme.mid
                            }

                            Item {
                                Layout.fillWidth: true
                                Layout.fillHeight: true
                                Layout.minimumHeight: 0

                                Text {
                                    anchors.centerIn: parent
                                    width: parent.width - 16
                                    visible: !McuMonitor.variables || McuMonitor.variables.length === 0
                                    text: root.tr( "No debug variables loaded.\nCompile the sketch (an ELF next to the .hex) to list program variables." )
                                    horizontalAlignment: Text.AlignHCenter
                                    font.family: root.uiFont.family
                                    font.pixelSize: 10
                                    color: appTheme.mid
                                    wrapMode: Text.WordWrap
                                }

                                ListView {
                                    id: varListView
                                    anchors.fill: parent
                                    visible: McuMonitor.variables && McuMonitor.variables.length > 0
                                    clip: true
                                    boundsBehavior: Flickable.StopAtBounds
                                    flickableDirection: Flickable.VerticalFlick
                                    ScrollBar.vertical: AppScrollBar { policy: ScrollBar.AlwaysOn }
                                    reuseItems: true
                                    readonly property int itemH: 22

                                    model: {
                                        var list = McuMonitor.variables || []
                                        var q = varFilter.text.trim().toLowerCase()
                                        if ( !q ) return list
                                        return list.filter( function( v ) {
                                            return ( v.name && v.name.toLowerCase().indexOf( q ) >= 0 ) ||
                                                   ( v.address && v.address.toLowerCase().indexOf( q ) >= 0 )
                                        } )
                                    }

                                    delegate: Rectangle {
                                        required property var modelData
                                        required property int index
                                        width: ListView.view.width
                                        height: varListView.itemH
                                        radius: 2
                                        color: varMouse.containsMouse ? appTheme.midlight : ( index % 2 ? appTheme.alternateBase : "transparent" )

                                        RowLayout {
                                            anchors.fill: parent
                                            anchors.leftMargin: 6
                                            anchors.rightMargin: 4
                                            spacing: 4

                                            Text {
                                                text: modelData.name || ""
                                                font.family: root.uiFont.family
                                                font.bold: true
                                                font.pixelSize: 11
                                                color: appTheme.windowText
                                                Layout.fillWidth: true
                                                elide: Text.ElideRight
                                            }
                                            Text {
                                                text: modelData.type || "uint8"
                                                font.family: root.uiFont.family
                                                font.pixelSize: 9
                                                color: appTheme.mid
                                            }
                                            Text {
                                                text: modelData.address || ""
                                                font.family: "Menlo"
                                                font.pixelSize: 10
                                                color: appTheme.mid
                                            }
                                            Text {
                                                text: "+"
                                                font.family: root.uiFont.family
                                                font.bold: true
                                                font.pixelSize: 13
                                                color: appTheme.highlight
                                                opacity: varMouse.containsMouse ? 1.0 : 0.0
                                            }
                                        }

                                        MouseArea {
                                            id: varMouse
                                            anchors.fill: parent
                                            hoverEnabled: true
                                            cursorShape: Qt.PointingHandCursor
                                            onClicked: McuMonitor.addWatch( modelData.name, modelData.name, modelData.type || "uint8" )
                                            ToolTip.visible: containsMouse
                                            ToolTip.delay: 400
                                            ToolTip.text: root.tr( "Click to watch %1 (%2, %3)" ).arg( modelData.name ).arg( modelData.type || "uint8" ).arg( modelData.address )
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                Rectangle {
                    SplitView.fillWidth: true
                    SplitView.fillHeight: true
                    color: appTheme.base
                    border.color: appTheme.mid
                    border.width: 1
                    radius: 4

                    ColumnLayout {
                        anchors.fill: parent
                        anchors.margins: 1
                        spacing: 0

                        RowLayout {
                            Layout.fillWidth: true
                            Layout.leftMargin: 6
                            Layout.rightMargin: 6
                            Layout.topMargin: 6
                            Layout.bottomMargin: 6
                            spacing: 6

                            AppTextField {
                                id: watchExpr
                                placeholderText: root.tr( "Variable, SFR name, or address (e.g. PORTB, counter, 0x20)..." )
                                Layout.fillWidth: true
                                font.family: "Menlo"
                                font.pixelSize: 12
                                Layout.preferredHeight: 26
                                onAccepted: addWatchBtn.clicked()
                            }

                            AppComboBox {
                                id: watchType
                                model: [ "uint8", "int8", "uint16", "int16", "uint32", "int32", "float32", "bool", "char", "string" ]
                                font: root.uiFont
                                Layout.preferredWidth: 95
                                Layout.preferredHeight: 26
                            }

                            AppButton {
                                id: addWatchBtn
                                text: root.tr( "+ Add Watch" )
                                font: root.uiFont
                                Layout.preferredHeight: 26
                                onClicked: {
                                    if ( watchExpr.text.trim().length > 0 ) {
                                        McuMonitor.addWatch( watchExpr.text.trim(), watchExpr.text.trim(), watchType.currentText )
                                        watchExpr.text = ""
                                    }
                                }
                            }

                            AppButton {
                                text: root.tr( "Clear All" )
                                font: root.uiFont
                                Layout.preferredHeight: 26
                                enabled: McuMonitor.watchCount > 0
                                onClicked: McuMonitor.clearWatches()
                            }
                        }

                        Rectangle {
                            Layout.fillWidth: true
                            height: 1
                            color: appTheme.mid
                        }

                        Rectangle {
                            Layout.fillWidth: true
                            height: 22
                            color: appTheme.window

                            RowLayout {
                                anchors.fill: parent
                                anchors.leftMargin: 8
                                anchors.rightMargin: 14
                                spacing: 6

                                Text { text: root.tr( "Name" ); font.family: root.uiFont.family; font.bold: true; font.pixelSize: 11; color: appTheme.windowText; Layout.preferredWidth: 120 }
                                Text { text: root.tr( "Address" ); font.family: root.uiFont.family; font.bold: true; font.pixelSize: 11; color: appTheme.windowText; Layout.preferredWidth: 75 }
                                Text { text: root.tr( "Value (Dec  0xHex  Bin)" ); font.family: root.uiFont.family; font.bold: true; font.pixelSize: 11; color: appTheme.windowText; Layout.fillWidth: true }
                                Text { text: root.tr( "Type" ); font.family: root.uiFont.family; font.bold: true; font.pixelSize: 11; color: appTheme.windowText; Layout.preferredWidth: 65 }
                                Item { Layout.preferredWidth: 26 }
                            }

                            Rectangle {
                                anchors.left: parent.left
                                anchors.right: parent.right
                                anchors.bottom: parent.bottom
                                height: 1
                                color: appTheme.mid
                            }
                        }

                        Item {
                            Layout.fillWidth: true
                            Layout.fillHeight: true
                            Layout.minimumHeight: 0

                            ColumnLayout {
                                anchors.centerIn: parent
                                spacing: 8
                                visible: McuMonitor.watchCount === 0

                                Text {
                                    text: root.tr( "No Watched Variables" )
                                    font.family: root.uiFont.family
                                    font.bold: true
                                    font.pixelSize: 13
                                    color: appTheme.windowText
                                    Layout.alignment: Qt.AlignHCenter
                                }
                                Text {
                                    text: root.tr( "Click any register or variable on the left to watch it, or enter a name / address above." )
                                    font.family: root.uiFont.family
                                    font.pixelSize: 11
                                    color: appTheme.mid
                                    Layout.alignment: Qt.AlignHCenter
                                }
                            }

                            ListView {
                                id: watchListView
                                anchors.fill: parent
                                visible: McuMonitor.watchCount > 0
                                clip: true
                                boundsBehavior: Flickable.StopAtBounds
                                flickableDirection: Flickable.VerticalFlick
                                ScrollBar.vertical: AppScrollBar { policy: ScrollBar.AlwaysOn }
                                reuseItems: true
                                readonly property int itemH: 24
                                model: McuMonitor.watchCount

                                delegate: Rectangle {
                                    required property int index
                                    readonly property var rowData: watchListView.visible ? McuMonitor.watchRowData( index, McuMonitor.watchRevision ) : root.emptyRow
                                    width: ListView.view.width
                                    height: watchListView.itemH
                                    color: watchRowMouse.containsMouse ? appTheme.midlight : ( index % 2 ? appTheme.alternateBase : "transparent" )
                                    radius: 2

                                    MouseArea {
                                        id: watchRowMouse
                                        anchors.fill: parent
                                        hoverEnabled: true
                                        acceptedButtons: Qt.LeftButton
                                        z: 0
                                        onDoubleClicked: {
                                            if ( rowData && rowData.addr >= 0 ) {
                                                var curVal = ( rowData.dec !== undefined && rowData.dec !== "---" ) ? parseInt( rowData.dec ) : 0
                                                editPopup.openForAddr( rowData.addr, curVal, 0 )
                                            }
                                        }
                                        ToolTip.visible: containsMouse && rowData && rowData.addr >= 0
                                        ToolTip.delay: 500
                                        ToolTip.text: root.tr( "%1 (Addr: 0x%2)\nDouble-click to edit value" )
                                            .arg( ( rowData && rowData.name ) ? rowData.name : "" )
                                            .arg( ( rowData && rowData.addr >= 0 ) ? rowData.addr.toString( 16 ).toUpperCase().padStart( 4, "0" ) : "----" )
                                    }

                                    RowLayout {
                                        anchors.fill: parent
                                        anchors.leftMargin: 8
                                        anchors.rightMargin: 6
                                        spacing: 6
                                        z: 1

                                        Text {
                                            text: ( rowData && rowData.name ) ? rowData.name : ""
                                            font.family: root.uiFont.family
                                            font.bold: true
                                            font.pixelSize: 11
                                            color: appTheme.windowText
                                            Layout.preferredWidth: 120
                                            elide: Text.ElideRight
                                        }
                                        Text {
                                            text: ( rowData && rowData.address ) ? rowData.address : "---"
                                            font.family: "Menlo"
                                            font.pixelSize: 11
                                            color: appTheme.mid
                                            Layout.preferredWidth: 75
                                        }
                                        Text {
                                            text: ( rowData && rowData.value ) ? rowData.value : "---"
                                            font.family: "Menlo"
                                            font.pixelSize: 11
                                            font.bold: true
                                            color: appTheme.highlight
                                            Layout.fillWidth: true
                                            elide: Text.ElideRight
                                        }
                                        Rectangle {
                                            Layout.preferredWidth: 65
                                            Layout.preferredHeight: 18
                                            color: appTheme.window
                                            radius: 3
                                            border.color: appTheme.mid
                                            border.width: 1
                                            Text {
                                                anchors.centerIn: parent
                                                text: ( rowData && rowData.type ) ? rowData.type : "uint8"
                                                font.family: root.uiFont.family
                                                font.pixelSize: 10
                                                color: appTheme.windowText
                                            }
                                        }
                                        AppButton {
                                            text: "×"
                                            font.bold: true
                                            Layout.preferredWidth: 24
                                            Layout.preferredHeight: 20
                                            padding: 0
                                            onClicked: if ( rowData && rowData.name ) McuMonitor.removeWatch( rowData.name )
                                            ToolTip.visible: hovered
                                            ToolTip.text: root.tr( "Remove %1 from Watch" ).arg( ( rowData && rowData.name ) ? rowData.name : "" )
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Tab 1: RAM hex
            ColumnLayout {
                spacing: 6
                Layout.fillWidth: true
                Layout.fillHeight: true

                RowLayout {
                    Layout.fillWidth: true
                    spacing: 6

                    Label {
                        text: root.tr( "Total RAM: %1 bytes (%2 rows)" )
                            .arg( McuMonitor.ramTotalRows )
                            .arg( McuMonitor.ramRowCount )
                        font.family: root.uiFont.family
                        font.bold: true
                        color: appTheme.windowText
                    }
                    Item { Layout.fillWidth: true }
                    Label { text: root.tr( "Jump to Addr: " ); font: root.uiFont; color: appTheme.windowText }
                    AppTextField {
                        id: ramJumpField
                        placeholderText: "0x0000"
                        Layout.preferredWidth: 80
                        font.family: "Menlo"
                        font.pixelSize: 11
                        onAccepted: {
                            var clean = text.trim().replace( /^0x/i, "" )
                            var addr = parseInt( clean, 16 )
                            if ( !isNaN( addr ) && addr >= 0 && McuMonitor.ramRowCount > 0 ) {
                                var row = Math.floor( addr / 16 )
                                ramTableListView.positionViewAtIndex( Math.min( row, McuMonitor.ramRowCount - 1 ), ListView.Beginning )
                            }
                        }
                    }
                    AppButton {
                        text: root.tr( "Go" )
                        font: root.uiFont
                        onClicked: ramJumpField.accepted()
                    }
                }

                TableFrame {
                    header: RowLayout {
                        anchors.fill: parent
                        spacing: 2
                        Text { text: root.tr( "Addr" ); font.bold: true; font.family: "Menlo"; font.pixelSize: 11; color: appTheme.mid; Layout.preferredWidth: 46 }
                        Repeater {
                            model: 16
                            delegate: Text {
                                required property int index
                                text: ( index < 10 ? "0" : "" ) + index.toString( 16 ).toUpperCase()
                                font.bold: true; font.family: "Menlo"; font.pixelSize: 11; color: appTheme.mid
                                Layout.preferredWidth: 22
                                horizontalAlignment: Text.AlignHCenter
                            }
                        }
                        Text { text: root.tr( "ASCII" ); font.bold: true; font.family: "Menlo"; font.pixelSize: 11; color: appTheme.mid; Layout.fillWidth: true; Layout.leftMargin: 8 }
                    }

                    ListView {
                        id: ramTableListView
                        anchors.fill: parent
                        clip: true
                        boundsBehavior: Flickable.StopAtBounds
                        flickableDirection: Flickable.VerticalFlick
                        ScrollBar.vertical: AppScrollBar { policy: ScrollBar.AlwaysOn }
                        reuseItems: true
                        pixelAligned: true
                        readonly property int itemH: 20
                        model: McuMonitor.ramRowCount

                        delegate: HexRow {
                            rowData: ramTableListView.visible ? McuMonitor.ramRowData( index, McuMonitor.ramRevision ) : root.emptyRow
                            columns: 16
                            cellW: 22
                            spacingW: 2
                            memType: 0
                            wordMode: false
                        }
                    }
                }
            }

            // Tab 2: RAM list
            ColumnLayout {
                spacing: 6
                Layout.fillWidth: true
                Layout.fillHeight: true

                RowLayout {
                    Layout.fillWidth: true
                    spacing: 6

                    Label { text: root.tr( "Total RAM: %1 bytes" ).arg( McuMonitor.ramTotalRows ); font.family: root.uiFont.family; font.pixelSize: root.uiFont.pixelSize; font.bold: true; color: appTheme.windowText }
                    Item { Layout.fillWidth: true }
                    Label { text: root.tr( "Jump to Addr: " ); font: root.uiFont; color: appTheme.windowText }
                    AppTextField {
                        id: ramListJumpField
                        placeholderText: "0x0000"
                        Layout.preferredWidth: 80
                        font.family: "Menlo"
                        font.pixelSize: 11
                        onAccepted: {
                            var clean = text.trim().replace( /^0x/i, "" )
                            var addr = parseInt( clean, 16 )
                            if ( !isNaN( addr ) && addr >= 0 && addr < McuMonitor.ramTotalRows ) {
                                ramListView.positionViewAtIndex( addr, ListView.Beginning )
                            }
                        }
                    }
                    AppButton {
                        text: root.tr( "Go" )
                        font: root.uiFont
                        onClicked: ramListJumpField.accepted()
                    }
                }

                TableFrame {
                    header: RowLayout {
                        anchors.fill: parent
                        spacing: 6
                        Text { text: root.tr( "Address" ); font.family: root.uiFont.family; font.bold: true; font.pixelSize: 11; color: appTheme.windowText; Layout.preferredWidth: 65 }
                        Text { text: root.tr( "Name (SFR / Variable)" ); font.family: root.uiFont.family; font.bold: true; font.pixelSize: 11; color: appTheme.windowText; Layout.preferredWidth: 150 }
                        Text { text: root.tr( "Value (Dec  0xHex  Bin)" ); font.family: root.uiFont.family; font.bold: true; font.pixelSize: 11; color: appTheme.windowText; Layout.fillWidth: true }
                        Text { text: root.tr( "Type" ); font.family: root.uiFont.family; font.bold: true; font.pixelSize: 11; color: appTheme.windowText; Layout.preferredWidth: 65 }
                        Item { Layout.preferredWidth: 26 }
                    }

                    ListView {
                        id: ramListView
                        anchors.fill: parent
                        clip: true
                        boundsBehavior: Flickable.StopAtBounds
                        flickableDirection: Flickable.VerticalFlick
                        ScrollBar.vertical: AppScrollBar { policy: ScrollBar.AlwaysOn }
                        reuseItems: true
                        pixelAligned: true
                        readonly property int itemH: 22
                        model: McuMonitor.ramTotalRows

                        delegate: Rectangle {
                            required property int index
                            readonly property var rowData: ramListView.visible ? McuMonitor.ramListRowData( index, McuMonitor.ramRevision ) : root.emptyRow
                            width: ListView.view.width
                            height: ramListView.itemH
                            color: ramListMouse.containsMouse ? appTheme.midlight : ( index % 2 ? appTheme.alternateBase : "transparent" )
                            radius: 2

                            RowLayout {
                                anchors.fill: parent
                                anchors.leftMargin: 8
                                anchors.rightMargin: 8
                                spacing: 6

                                Text {
                                    text: ( rowData && rowData.address ) ? ( "0x" + rowData.address ) : "0x0000"
                                    font.family: "Menlo"
                                    font.pixelSize: 11
                                    color: appTheme.mid
                                    Layout.preferredWidth: 65
                                }
                                Text {
                                    text: ( rowData && rowData.name ) ? rowData.name : ""
                                    font.family: root.uiFont.family
                                    font.bold: rowData && rowData.name && rowData.name.length > 0
                                    font.pixelSize: 11
                                    color: appTheme.windowText
                                    Layout.preferredWidth: 150
                                    elide: Text.ElideRight
                                }
                                Text {
                                    text: ( rowData && rowData.value ) ? rowData.value : ""
                                    font.family: "Menlo"
                                    font.pixelSize: 11
                                    font.bold: true
                                    color: appTheme.highlight
                                    Layout.fillWidth: true
                                    elide: Text.ElideRight
                                }
                                Text {
                                    text: ( rowData && rowData.type ) ? rowData.type : "uint8"
                                    font.family: root.uiFont.family
                                    font.pixelSize: 10
                                    color: appTheme.mid
                                    Layout.preferredWidth: 65
                                }
                                AppButton {
                                    text: "+"
                                    font.bold: true
                                    Layout.preferredWidth: 24
                                    Layout.preferredHeight: 18
                                    padding: 0
                                    onClicked: {
                                        var watchName = ( rowData && rowData.name && rowData.name.length > 0 ) ? rowData.name : ( "0x" + rowData.address )
                                        McuMonitor.addWatch( watchName, watchName, "uint8" )
                                    }
                                    ToolTip.visible: hovered
                                    ToolTip.text: root.tr( "Add to Watch" )
                                }
                            }

                            MouseArea {
                                id: ramListMouse
                                anchors.fill: parent
                                hoverEnabled: true
                                acceptedButtons: Qt.LeftButton
                                onDoubleClicked: {
                                    if ( rowData && rowData.addr >= 0 ) {
                                        editPopup.openForAddr( rowData.addr, rowData.val, 0 )
                                    }
                                }
                                ToolTip.visible: containsMouse
                                ToolTip.delay: 450
                                ToolTip.text: root.tr( "Address: 0x%1 (%2)\nName: %3\nValue: %4\n(Double-click to edit)" )
                                    .arg( rowData ? rowData.address : "" )
                                    .arg( rowData ? rowData.addr : "" )
                                    .arg( ( rowData && rowData.name ) ? rowData.name : "---" )
                                    .arg( rowData ? rowData.value : "" )
                            }
                        }
                    }
                }
            }

            // Tab 3: Flash
            ColumnLayout {
                spacing: 6
                Layout.fillWidth: true
                Layout.fillHeight: true

                RowLayout {
                    Layout.fillWidth: true
                    spacing: 6

                    Label {
                        text: root.tr( "Total Flash: %1 words (%2 KB)" )
                            .arg( McuMonitor.flashTotalWords )
                            .arg( ( McuMonitor.flashTotalWords * 2 / 1024 ).toFixed( 1 ) )
                        font.family: root.uiFont.family
                        font.bold: true
                        color: appTheme.windowText
                    }
                    Item { Layout.fillWidth: true }
                    Label { text: root.tr( "Jump to Addr: " ); font: root.uiFont; color: appTheme.windowText }
                    AppTextField {
                        id: flashJumpField
                        placeholderText: "0x0000"
                        Layout.preferredWidth: 80
                        font.family: "Menlo"
                        font.pixelSize: 11
                        onAccepted: {
                            var clean = text.trim().replace( /^0x/i, "" )
                            var addr = parseInt( clean, 16 )
                            if ( !isNaN( addr ) && addr >= 0 && McuMonitor.flashRowCount > 0 ) {
                                var divisor = McuMonitor.byteMode ? 16 : 8
                                var row = Math.floor( addr / divisor )
                                flashTableListView.positionViewAtIndex( Math.min( row, McuMonitor.flashRowCount - 1 ), ListView.Beginning )
                            }
                        }
                    }
                    AppButton {
                        text: root.tr( "Go" )
                        font: root.uiFont
                        onClicked: flashJumpField.accepted()
                    }
                    AppButton {
                        text: root.tr( "Jump to PC" )
                        font: root.uiFont
                        onClicked: flashTableListView.jumpToActivePc( true )
                    }
                }

                TableFrame {
                    header: RowLayout {
                        anchors.fill: parent
                        spacing: 4
                        Text { text: root.tr( "Addr" ); font.bold: true; font.family: "Menlo"; font.pixelSize: 11; color: appTheme.mid; Layout.preferredWidth: 46 }
                        Repeater {
                            model: 8
                            delegate: Text {
                                required property int index
                                text: "+" + ( index * ( McuMonitor.byteMode ? 2 : 1 ) ).toString( 16 ).toUpperCase()
                                font.bold: true; font.family: "Menlo"; font.pixelSize: 11; color: appTheme.mid
                                Layout.preferredWidth: 40
                                horizontalAlignment: Text.AlignHCenter
                            }
                        }
                        Text { text: root.tr( "ASCII" ); font.bold: true; font.family: "Menlo"; font.pixelSize: 11; color: appTheme.mid; Layout.fillWidth: true; Layout.leftMargin: 8 }
                    }

                    ListView {
                        id: flashTableListView
                        anchors.fill: parent
                        clip: true
                        boundsBehavior: Flickable.StopAtBounds
                        flickableDirection: Flickable.VerticalFlick
                        ScrollBar.vertical: AppScrollBar { policy: ScrollBar.AlwaysOn }
                        reuseItems: true
                        pixelAligned: true
                        readonly property int itemH: 20
                        model: McuMonitor.flashRowCount

                        function jumpToActivePc( force ) {
                            if ( !( force || McuMonitor.jumpToAddress ) )
                                return
                            if ( !flashTableListView.visible || McuMonitor.flashRowCount <= 0 )
                                return
                            var row = McuMonitor.pcFlashRow
                            if ( row < 0 || row >= McuMonitor.flashRowCount )
                                return
                            if ( !force ) {
                                var first = flashTableListView.indexAt( 1, flashTableListView.contentY + 1 )
                                var last = flashTableListView.indexAt( 1, flashTableListView.contentY + flashTableListView.height - 1 )
                                if ( first >= 0 && last >= first && row >= first && row <= last )
                                    return
                            }
                            flashTableListView.positionViewAtIndex( row, ListView.Center )
                        }

                        onVisibleChanged: if ( visible ) jumpToActivePc( false )

                        Connections {
                            target: McuMonitor
                            function onPcChanged() { flashTableListView.jumpToActivePc( false ) }
                            function onJumpToAddressChanged() { flashTableListView.jumpToActivePc( false ) }
                        }

                        delegate: HexRow {
                            rowData: flashTableListView.visible ? McuMonitor.flashRowData( index, McuMonitor.flashRevision ) : root.emptyRow
                            columns: 8
                            cellW: 40
                            spacingW: 4
                            memType: 1
                            wordMode: true
                            pcCol: {
                                if ( !rowData || !rowData.addrs )
                                    return -1
                                var pc = McuMonitor.pcValue
                                for ( var i = 0; i < rowData.addrs.length; i++ ) {
                                    if ( rowData.addrs[i] === pc )
                                        return i
                                }
                                return -1
                            }
                        }
                    }
                }
            }

            // Tab 4: EEPROM
            ColumnLayout {
                spacing: 6
                Layout.fillWidth: true
                Layout.fillHeight: true

                RowLayout {
                    Layout.fillWidth: true
                    spacing: 6

                    Label {
                        text: root.tr( "Total EEPROM: %1 bytes (%2 rows)" )
                            .arg( McuMonitor.eepromTotalBytes )
                            .arg( McuMonitor.eepromRowCount )
                        font.family: root.uiFont.family
                        font.bold: true
                        color: appTheme.windowText
                    }
                    Item { Layout.fillWidth: true }
                    Label { text: root.tr( "Jump to Addr: " ); font: root.uiFont; color: appTheme.windowText }
                    AppTextField {
                        id: eepromJumpField
                        placeholderText: "0x0000"
                        Layout.preferredWidth: 80
                        font.family: "Menlo"
                        font.pixelSize: 11
                        onAccepted: {
                            var clean = text.trim().replace( /^0x/i, "" )
                            var addr = parseInt( clean, 16 )
                            if ( !isNaN( addr ) && addr >= 0 && McuMonitor.eepromRowCount > 0 ) {
                                var row = Math.floor( addr / 16 )
                                eepromTableListView.positionViewAtIndex( Math.min( row, McuMonitor.eepromRowCount - 1 ), ListView.Beginning )
                            }
                        }
                    }
                    AppButton {
                        text: root.tr( "Go" )
                        font: root.uiFont
                        onClicked: eepromJumpField.accepted()
                    }
                }

                TableFrame {
                    header: RowLayout {
                        anchors.fill: parent
                        spacing: 2
                        Text { text: root.tr( "Addr" ); font.bold: true; font.family: "Menlo"; font.pixelSize: 11; color: appTheme.mid; Layout.preferredWidth: 46 }
                        Repeater {
                            model: 16
                            delegate: Text {
                                required property int index
                                text: ( index < 10 ? "0" : "" ) + index.toString( 16 ).toUpperCase()
                                font.bold: true; font.family: "Menlo"; font.pixelSize: 11; color: appTheme.mid
                                Layout.preferredWidth: 22
                                horizontalAlignment: Text.AlignHCenter
                            }
                        }
                        Text { text: root.tr( "ASCII" ); font.bold: true; font.family: "Menlo"; font.pixelSize: 11; color: appTheme.mid; Layout.fillWidth: true; Layout.leftMargin: 8 }
                    }

                    ListView {
                        id: eepromTableListView
                        anchors.fill: parent
                        clip: true
                        boundsBehavior: Flickable.StopAtBounds
                        flickableDirection: Flickable.VerticalFlick
                        ScrollBar.vertical: AppScrollBar { policy: ScrollBar.AlwaysOn }
                        reuseItems: true
                        pixelAligned: true
                        readonly property int itemH: 20
                        model: McuMonitor.eepromRowCount

                        delegate: HexRow {
                            rowData: eepromTableListView.visible ? McuMonitor.eepromRowData( index, McuMonitor.eepromRevision ) : root.emptyRow
                            columns: 16
                            cellW: 22
                            spacingW: 2
                            memType: 2
                            wordMode: false
                        }
                    }
                }
            }
        }
    }

    Popup {
        id: editPopup
        width: 240
        height: 120
        modal: true
        focus: true
        closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutside
        anchors.centerIn: parent

        property int targetAddr: -1
        property int memType: 0

        function openForAddr( addr, currentVal, type ) {
            targetAddr = addr
            memType = type || 0
            editInput.text = ( currentVal >= 0 ? currentVal.toString( 16 ).toUpperCase() : "00" )
            open()
            editInput.selectAll()
            editInput.forceActiveFocus()
        }

        background: Rectangle {
            color: appTheme.window
            border.color: appTheme.highlight
            border.width: 2
            radius: 6
        }

        ColumnLayout {
            anchors.fill: parent
            anchors.margins: 10
            spacing: 6

            Label {
                text: root.tr( "%1 Addr: 0x%2" )
                    .arg( editPopup.memType === 0 ? "RAM" : ( editPopup.memType === 1 ? "Flash" : "EEPROM" ) )
                    .arg( editPopup.targetAddr.toString( 16 ).toUpperCase().padStart( 4, "0" ) )
                font.bold: true
                font.pixelSize: 11
                color: appTheme.windowText
            }

            RowLayout {
                spacing: 6
                AppTextField {
                    id: editInput
                    Layout.fillWidth: true
                    font.family: "Menlo"
                    font.pixelSize: 12
                    placeholderText: root.tr( "Hex value..." )
                    onAccepted: saveByteBtn.clicked()
                }
                AppButton {
                    id: saveByteBtn
                    text: root.tr( "Set" )
                    onClicked: {
                        var clean = editInput.text.trim().replace( /^0x/i, "" )
                        var val = parseInt( clean, 16 )
                        if ( !isNaN( val ) && editPopup.targetAddr >= 0 ) {
                            if ( editPopup.memType === 0 ) McuMonitor.pokeRam( editPopup.targetAddr, val & 0xFF )
                            else if ( editPopup.memType === 1 ) McuMonitor.pokeFlash( editPopup.targetAddr, val & 0xFFFF )
                            else if ( editPopup.memType === 2 ) McuMonitor.pokeEeprom( editPopup.targetAddr, val & 0xFF )
                        }
                        editPopup.close()
                    }
                }
                AppButton {
                    text: root.tr( "Cancel" )
                    onClicked: editPopup.close()
                }
            }
        }
    }
}
