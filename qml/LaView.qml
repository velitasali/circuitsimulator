import QtQuick
import QtQuick.Controls
import QtQuick.Effects
import QtQuick.Layouts
import QtQuick.Window
import cs_app

// The expanded logic analyzer. Parity with src/gui/dataplotwidget/lawidget.qml.
Rectangle {
    id: root
    function tr(s) { return App.i18nTick >= 0 ? App.translate(s) : s }
    SystemPalette { id: appTheme }
    color: appTheme.window

    function isLightColor( c ) {
        var col = Qt.color( c )
        var lum = 0.299 * col.r + 0.587 * col.g + 0.114 * col.b
        return lum > 0.55
    }

    Component.onCompleted: {
        LogicAnalyzer.dark = App.darkTheme
        CircuitCanvas.setLaTimeDiv( LogicAnalyzer.timeDiv )
        CircuitCanvas.setLaTimePos( LogicAnalyzer.timePos )
        CircuitCanvas.setLaTrigger( LogicAnalyzer.trigger )
        CircuitCanvas.setLaThresholds( LogicAnalyzer.thresholdR, LogicAnalyzer.thresholdF )
    }

    Connections {
        target: App
        function onDarkThemeChanged() {
            LogicAnalyzer.dark = App.darkTheme
        }
    }

    Connections {
        target: CircuitCanvas
        function onPlotsChanged() { root.refreshCursor() }
    }

    readonly property var timeDivSteps: [
        1e-9, 1.2e-9, 1.5e-9, 2e-9, 2.5e-9, 3e-9, 4e-9, 5e-9, 6e-9, 7e-9, 8e-9, 9e-9,
        1e-8, 1.2e-8, 1.5e-8, 2e-8, 2.5e-8, 3e-8, 4e-8, 5e-8, 6e-8, 7e-8, 8e-8, 9e-8,
        1e-7, 1.2e-7, 1.5e-7, 2e-7, 2.5e-7, 3e-7, 4e-7, 5e-7, 6e-7, 7e-7, 8e-7, 9e-7,
        1e-6, 1.2e-6, 1.5e-6, 2e-6, 2.5e-6, 3e-6, 4e-6, 5e-6, 6e-6, 7e-6, 8e-6, 9e-6,
        1e-5, 1.2e-5, 1.5e-5, 2e-5, 2.5e-5, 3e-5, 4e-5, 5e-5, 6e-5, 7e-5, 8e-5, 9e-5,
        1e-4, 1.2e-4, 1.5e-4, 2e-4, 2.5e-4, 3e-4, 4e-4, 5e-4, 6e-4, 7e-4, 8e-4, 9e-4,
        1e-3, 1.2e-3, 1.5e-3, 2e-3, 2.5e-3, 3e-3, 4e-3, 5e-3, 6e-3, 7e-3, 8e-3, 9e-3,
        1e-2, 1.2e-2, 1.5e-2, 2e-2, 2.5e-2, 3e-2, 4e-2, 5e-2, 6e-2, 7e-2, 8e-2, 9e-2,
        0.1, 0.12, 0.15, 0.2, 0.25, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9,
        1.0, 1.2, 1.5, 2.0, 2.5, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0,
        10.0, 12.0, 15.0, 20.0, 25.0, 30.0, 40.0, 50.0
    ]

    function laTimeSpan() {
        var w = laScreen ? laScreen.width : 0
        var h = laScreen ? laScreen.height : 0
        return ( LogicAnalyzer.timeDiv || 1e-3 ) * CircuitCanvas.laHDivs( w, h )
    }

    function stepTimeDiv( dir ) {
        var cur = LogicAnalyzer.timeDiv
        var idx = 18 // 1ms default index
        var minDiff = 1e12
        for ( var i = 0; i < timeDivSteps.length; ++i ) {
            var diff = Math.abs( Math.log( Math.max( 1e-18, timeDivSteps[i] ) ) - Math.log( Math.max( 1e-18, cur ) ) )
            if ( diff < minDiff ) {
                minDiff = diff
                idx = i
            }
        }
        if ( dir > 0 ) {
            if ( timeDivSteps[idx] <= cur * 1.0001 && idx + 1 < timeDivSteps.length ) {
                idx = idx + 1
            }
        } else if ( dir < 0 ) {
            if ( timeDivSteps[idx] >= cur * 0.9999 && idx - 1 >= 0 ) {
                idx = idx - 1
            }
        }
        var nextIdx = Math.max( 0, Math.min( timeDivSteps.length - 1, idx ) )
        var nextVal = timeDivSteps[nextIdx]
        LogicAnalyzer.setTimeDiv( nextVal )
        CircuitCanvas.setLaTimeDiv( nextVal )
    }

    function formatEng( val, unit ) {
        if ( val === 0 ) return "0 " + unit
        var abs = Math.abs( val )
        if ( abs < 1e-9 ) return ( val * 1e12 ).toFixed( 1 ) + " p" + unit
        if ( abs < 1e-6 ) return ( val * 1e9 ).toFixed( 1 ) + " n" + unit
        if ( abs < 1e-3 ) return ( val * 1e6 ).toFixed( 1 ) + " µ" + unit
        if ( abs < 1.0 ) return ( val * 1e3 ).toFixed( 1 ) + " m" + unit
        if ( abs >= 1e9 ) return ( val / 1e9 ).toFixed( 2 ) + " G" + unit
        if ( abs >= 1e6 ) return ( val / 1e6 ).toFixed( 2 ) + " M" + unit
        if ( abs >= 1e3 ) return ( val / 1e3 ).toFixed( 2 ) + " k" + unit
        return val.toFixed( 2 ) + " " + unit
    }

    property bool laPaintQueued: false
    property real cursorX: -1
    property real cursorY: -1
    property real markerX: 0
    property real markerFraction: 0.5
    property var markerHits: []
    property var markerRows: []

    function laChannelColor( ch ) {
        var list = LogicAnalyzer.channels
        if ( !list || ch < 0 || ch >= list.length )
            return LogicAnalyzer.cursorText
        return list[ch].color
    }

    function formatEngTime( val ) {
        var sign = val < 0 ? "-" : "+"
        var abs = Math.abs( val )
        if ( abs < 1e-9 ) return "0.00 s"
        if ( abs < 1e-6 ) return sign + ( abs * 1e9 ).toFixed( 1 ) + " ns"
        if ( abs < 1e-3 ) return sign + ( abs * 1e6 ).toFixed( 1 ) + " µs"
        if ( abs < 1.0 ) return sign + ( abs * 1e3 ).toFixed( 2 ) + " ms"
        return sign + abs.toFixed( 2 ) + " s"
    }

    function formatLevel( val ) {
        if ( val >= 0.995 ) return "1"
        if ( val <= 0.005 ) return "0"
        return Number( val ).toFixed( 2 )
    }

    function formatLevels( values ) {
        var nums = values.slice().sort( function( a, b ) { return a - b } )
        var lo = root.formatLevel( nums[0] )
        var hi = root.formatLevel( nums[nums.length - 1] )
        return lo === hi ? lo : lo + "–" + hi
    }

    function rowsFromHits( hits ) {
        var rows = []
        for ( var i = 0; i < hits.length; ++i ) {
            var h = hits[i]
            var found = -1
            for ( var j = 0; j < rows.length; ++j ) {
                if ( rows[j].channel === h.channel ) {
                    found = j
                    break
                }
            }
            if ( found < 0 )
                rows.push( { channel: h.channel, values: [ h.value ] } )
            else
                rows[found].values.push( h.value )
        }
        return rows
    }

    function cursorDeltaTime() {
        return ( root.markerFraction - 0.5 ) * root.laTimeSpan()
    }

    function clearMarker() {
        root.markerHits = []
        root.markerRows = []
    }

    function refreshCursor() {
        if ( !laScreen || root.cursorX < 0 || laScreen.width < 2 || laScreen.height < 2 ) {
            root.clearMarker()
            return
        }
        var res = CircuitCanvas.laSignalAt(
                    root.cursorX, laScreen.width, laScreen.height,
                    Screen.devicePixelRatio || 1 )
        if ( !res || res.fraction === undefined || res.fraction === null ) {
            root.clearMarker()
            return
        }
        root.markerX = res.x
        root.markerFraction = res.fraction
        var hits = res.hits || []
        root.markerHits = hits
        root.markerRows = root.rowsFromHits( hits )
    }

    function repaintLa() {
        if ( !laScreen || laScreen.width < 2 || laScreen.height < 2 )
            return
        CircuitCanvas.paintLaScreen( laScreen.width, laScreen.height, Screen.devicePixelRatio || 1 )
        root.refreshCursor()
    }

    function scheduleLaPaint() {
        if ( root.laPaintQueued )
            return
        root.laPaintQueued = true
        Qt.callLater( function() {
            root.laPaintQueued = false
            root.repaintLa()
        } )
    }


    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 5
        spacing: 5

        // Logic analyzer waveform display
        Rectangle {
            id: displaySlot
            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.minimumHeight: 200
            color: appTheme.window

            // Eight rows fill the height. Each time column is one row wide.
            Item {
                id: laMask
                anchors.fill: laFrame
                visible: false
                layer.enabled: true
                Rectangle {
                    anchors.fill: parent
                    radius: 6
                    color: "white"
                }
            }

            Rectangle {
                id: laFrame
                anchors.fill: parent
                color: LogicAnalyzer.paperColor
                radius: 6
                layer.enabled: true
                layer.effect: MultiEffect {
                    maskEnabled: true
                    maskSource: laMask
                }

            LogicScreen {
                id: laScreen
                anchors.fill: parent
                readonly property real dpr: Screen.devicePixelRatio
                onWidthChanged: root.scheduleLaPaint()
                onHeightChanged: root.scheduleLaPaint()
                onDprChanged: root.scheduleLaPaint()
                Component.onCompleted: root.scheduleLaPaint()
            }

            Item {
                id: cursorOverlay
                anchors.fill: parent
                visible: root.cursorX >= 0 && root.cursorX <= width && root.cursorY >= 0 && root.cursorY <= height
                z: 1

                Rectangle {
                    width: 1
                    height: parent.height
                    x: Math.round( root.markerX )
                    color: LogicAnalyzer.cursorColor
                    opacity: 0.85
                }
                Repeater {
                    model: root.markerHits
                    delegate: Rectangle {
                        required property var modelData
                        width: 7
                        height: 7
                        radius: 3.5
                        x: Math.round( root.markerX ) - 3
                        y: Math.round( modelData.y ) - 3
                        visible: modelData.y >= -1 && modelData.y <= cursorOverlay.height + 1
                        color: root.laChannelColor( modelData.channel )
                        border.width: 1
                        border.color: LogicAnalyzer.cursorColor
                    }
                }

                Rectangle {
                    x: Math.min( parent.width - width - 6, Math.max( 6, root.cursorX + 8 ) )
                    y: Math.min( parent.height - height - 6, Math.max( 6, root.cursorY + 8 ) )
                    width: measureCol.implicitWidth + 12
                    height: measureCol.implicitHeight + 8
                    radius: 3
                    color: LogicAnalyzer.cursorFill
                    border.color: LogicAnalyzer.cursorBorder
                    border.width: 1

                    Column {
                        id: measureCol
                        anchors.centerIn: parent
                        spacing: 1

                        AppText {
                            text: "Δt: " + root.formatEngTime( root.cursorDeltaTime() )
                            color: LogicAnalyzer.cursorText
                            font.bold: true
                            font.pixelSize: App.fontMicro
                        }
                        Repeater {
                            model: root.markerRows
                            delegate: AppText {
                                required property var modelData
                                text: "Ch " + ( modelData.channel + 1 ) + ": " + root.formatLevels( modelData.values )
                                color: root.laChannelColor( modelData.channel )
                                font.bold: true
                                font.pixelSize: App.fontMicro
                            }
                        }
                    }
                }
            }

            MouseArea {
                id: laMouseArea
                anchors.fill: parent
                z: 2
                acceptedButtons: Qt.LeftButton | Qt.MiddleButton
                hoverEnabled: true
                cursorShape: pressed ? Qt.ClosedHandCursor : Qt.CrossCursor

                property real lastDragX: 0

                onPressed: (mouse) => {
                    laMouseArea.lastDragX = mouse.x
                    LogicAnalyzer.plotPressed( mouse.x, mouse.button )
                }
                onPositionChanged: (mouse) => {
                    root.cursorX = mouse.x
                    root.cursorY = mouse.y
                    root.refreshCursor()
                    if ( pressed && ( mouse.buttons & Qt.LeftButton ) ) {
                        var dx = mouse.x - laMouseArea.lastDragX
                        laMouseArea.lastDragX = mouse.x
                        var timeSpan = root.laTimeSpan()
                        var dt = ( dx / Math.max( width, 1 ) ) * timeSpan
                        LogicAnalyzer.timePos = LogicAnalyzer.timePos + dt
                        CircuitCanvas.setLaTimePos( LogicAnalyzer.timePos )
                        LogicAnalyzer.plotMoved( mouse.x )
                    }
                }
                onExited: {
                    root.cursorX = -1
                    root.cursorY = -1
                    root.clearMarker()
                }
                onReleased: LogicAnalyzer.plotReleased()

                property real canvasWheelAccumY: 0
                Timer {
                    id: canvasWheelTimer
                    interval: 180
                    onTriggered: laMouseArea.canvasWheelAccumY = 0
                }
                onWheel: (wheel) => {
                    // 1. Horizontal trackpad scroll: pan Time Pos
                    var dx = 0
                    if ( wheel.pixelDelta && wheel.pixelDelta.x !== 0 ) {
                        dx = wheel.pixelDelta.x
                    } else if ( wheel.angleDelta && wheel.angleDelta.x !== 0 ) {
                        dx = wheel.angleDelta.x / 120.0 * 25.0
                    }

                    if ( dx !== 0 ) {
                        var timeSpan = root.laTimeSpan()
                        var dt = ( dx / Math.max( width, 1 ) ) * timeSpan
                        LogicAnalyzer.timePos = LogicAnalyzer.timePos + dt
                        CircuitCanvas.setLaTimePos( LogicAnalyzer.timePos )
                    }

                    // 2. Vertical scroll: zoom Time / Div
                    var dy = 0
                    var isTrackpad = false
                    if ( wheel.pixelDelta && wheel.pixelDelta.y !== 0 ) {
                        isTrackpad = true
                        dy = wheel.pixelDelta.y
                    } else if ( wheel.angleDelta && wheel.angleDelta.y !== 0 ) {
                        dy = wheel.angleDelta.y / 120.0
                    }

                    if ( dy !== 0 ) {
                        if ( ( laMouseArea.canvasWheelAccumY > 0 && dy < 0 ) || ( laMouseArea.canvasWheelAccumY < 0 && dy > 0 ) ) {
                            laMouseArea.canvasWheelAccumY = 0
                        }
                        laMouseArea.canvasWheelAccumY += dy

                        var thresh = isTrackpad ? 20.0 : 1.0
                        if ( laMouseArea.canvasWheelAccumY >= thresh ) {
                            root.stepTimeDiv( -1 )
                            laMouseArea.canvasWheelAccumY = 0
                        } else if ( laMouseArea.canvasWheelAccumY <= -thresh ) {
                            root.stepTimeDiv( 1 )
                            laMouseArea.canvasWheelAccumY = 0
                        }
                        canvasWheelTimer.restart()
                    }
                }
            }
            }
        }

        // Top control bar: Trigger, conditions, export
        RowLayout {
            spacing: 6

            AppText {
                text: root.tr( "Trigger" )
                color: appTheme.text
                font.pixelSize: App.fontSmall
                font.bold: true
            }
            AppComboBox {
                Layout.preferredWidth: 110
                model: [ root.tr( "None" ), root.tr( "Rising" ), root.tr( "Falling" ), root.tr( "Both" ) ]
                currentIndex: LogicAnalyzer.trigger
                onActivated: {
                    LogicAnalyzer.trigger = currentIndex
                    CircuitCanvas.setLaTrigger( currentIndex )
                }
            }

            AppText {
                text: root.tr( "Conditions" )
                color: appTheme.text
                font.pixelSize: App.fontSmall
                font.bold: true
            }
            AppTextField {
                Layout.fillWidth: true
                text: LogicAnalyzer.conds
                onEditingFinished: LogicAnalyzer.conds = text
            }

            AppButton {
                text: root.tr( "Export" )
                onClicked: LogicAnalyzer.exportData()
            }

            AppComboBox {
                id: decodeProto
                Layout.preferredWidth: 90
                model: [ "UART", "I2C", "SPI" ]
            }
            AppTextField {
                id: decodeBaud
                Layout.preferredWidth: 72
                text: "9600"
            }
            AppButton {
                text: root.tr( "Decode" )
                onClicked: {
                    var baud = Number( decodeBaud.text )
                    if ( !( baud > 0 ) )
                        baud = 9600
                    var res = CircuitCanvas.laDecode( decodeProto.currentText, root.laTimeSpan(), baud )
                    decodeText.text = ( res && res.text ) ? res.text : ""
                }
            }
            AppText {
                id: decodeText
                Layout.fillWidth: true
                color: appTheme.text
                font.pixelSize: App.fontSmall
                elide: Text.ElideRight
            }
        }

        // Bottom control bar: Time / Div, Time Pos, Bus channels, Thresholds
        RowLayout {
            spacing: 12

            PlotValueControl {
                label: root.tr( "Time / Div" )
                value: LogicAnalyzer.timeDiv
                unit: "s"
                isStepList: true
                stepList: root.timeDivSteps
                minVal: 1e-12
                maxVal: 1000.0
                defaultValue: 1e-3
                onValueModified: (newVal) => {
                    LogicAnalyzer.setTimeDiv( newVal )
                    CircuitCanvas.setLaTimeDiv( newVal )
                }
            }

            PlotValueControl {
                label: root.tr( "Time Pos" )
                value: LogicAnalyzer.timePos
                unit: "s"
                stepDelta: ( LogicAnalyzer.timeDiv / 10.0 )
                minVal: -1e6
                maxVal: 1e6
                defaultValue: 0.0
                onValueModified: (newVal) => {
                    LogicAnalyzer.timePos = newVal
                    CircuitCanvas.setLaTimePos( newVal )
                }
            }

            ColumnLayout { // Which channels are read as a bus
                spacing: 2

                AppText {
                    text: root.tr( "Bus" )
                    color: appTheme.text
                    font.pixelSize: App.fontSmall
                    font.bold: true
                }

                RowLayout {
                    spacing: 3
                    Repeater {
                        model: LogicAnalyzer.channels
                        delegate: AppCheckBox {
                            required property int index
                            required property var modelData

                            readonly property color chColor: modelData.color || modelData

                            checked: LogicAnalyzer.buses[index] === true
                            onToggled: LogicAnalyzer.busClicked( index, checked )

                            indicator: Rectangle {
                                implicitWidth: 18
                                implicitHeight: 18
                                radius: 3
                                color: parent.checked ? chColor
                                                      : ( parent.hovered ? Qt.rgba( appTheme.midlight.r, appTheme.midlight.g, appTheme.midlight.b, 0.35 )
                                                                         : Qt.rgba( appTheme.base.r, appTheme.base.g, appTheme.base.b, 0.45 ) )
                                border.width: parent.checked ? 1.5 : 1
                                border.color: parent.checked ? Qt.darker( chColor, 1.25 )
                                                             : ( root.isLightColor( chColor ) && !App.darkTheme ? Qt.darker( chColor, 1.35 ) : chColor )

                                AppText {
                                    anchors.centerIn: parent
                                    text: index + 1
                                    font.pixelSize: App.fontTiny
                                    font.bold: true
                                    color: parent.parent.checked ? ( root.isLightColor( chColor ) ? "#000000" : "#ffffff" ) : appTheme.text
                                }
                            }
                        }
                    }
                }
            }

            ColumnLayout { // Logic thresholds
                spacing: 2

                AppText {
                    text: root.tr( "Threshold" )
                    color: appTheme.text
                    font.pixelSize: App.fontSmall
                    font.bold: true
                }
                RowLayout {
                    spacing: 4
                    AppText { text: root.tr( "Rise" ); color: appTheme.text; font.pixelSize: App.fontSmall }
                    AppSpinBox {
                        editable: true
                        from: 0; to: 100
                        value: Math.round( LogicAnalyzer.thresholdR * 10 )
                        onValueModified: {
                            LogicAnalyzer.thresholdR = value / 10
                            CircuitCanvas.setLaThresholds( LogicAnalyzer.thresholdR, LogicAnalyzer.thresholdF )
                        }
                    }
                    AppText { text: root.tr( "Fall" ); color: appTheme.text; font.pixelSize: App.fontSmall }
                    AppSpinBox {
                        editable: true
                        from: 0; to: 100
                        value: Math.round( LogicAnalyzer.thresholdF * 10 )
                        onValueModified: {
                            LogicAnalyzer.thresholdF = value / 10
                            CircuitCanvas.setLaThresholds( LogicAnalyzer.thresholdR, LogicAnalyzer.thresholdF )
                        }
                    }
                }
            }

            Item { Layout.fillWidth: true }
        }
    }
}
