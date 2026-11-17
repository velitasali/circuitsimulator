pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Effects
import QtQuick.Layouts
import QtQuick.Window
import cs_app
import cs_ui

// The expanded logic analyzer. Parity with src/gui/dataplotwidget/lawidget.qml.
Rectangle {
    id: root
    SystemPalette { id: appTheme }
    color: appTheme.window

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

    function laTimeSpan(): real {
        var w = laScreen ? laScreen.width : 0
        var h = laScreen ? laScreen.height : 0
        return ( LogicAnalyzer.timeDiv || 1e-3 ) * CircuitCanvas.laHDivs( w, h )
    }

    function stepTimeDiv( dir: int ): void {
        var nextVal = UiUtils.stepLogList( UiUtils.timeDivSteps, LogicAnalyzer.timeDiv, dir )
        LogicAnalyzer.setTimeDiv( nextVal )
        CircuitCanvas.setLaTimeDiv( nextVal )
    }

    property bool laPaintQueued: false
    property real cursorX: -1
    property real cursorY: -1
    property real markerX: 0
    property real markerFraction: 0.5
    property var markerHits: []
    property var markerRows: []

    function laChannelColor( ch: int ): color {
        var list = LogicAnalyzer.channels
        if ( !list || ch < 0 || ch >= list.length )
            return LogicAnalyzer.cursorText
        return list[ch].color
    }

    function formatLevel( val: real ): string {
        if ( val >= 0.995 ) return "1"
        if ( val <= 0.005 ) return "0"
        return Number( val ).toFixed( 2 )
    }

    function formatLevels( values: var ): string {
        var nums = values.slice().sort( function( a: real, b: real ): real { return a - b } )
        var lo = root.formatLevel( nums[0] )
        var hi = root.formatLevel( nums[nums.length - 1] )
        return lo === hi ? lo : lo + "–" + hi
    }

    function rowsFromHits( hits: var ): var {
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

    function cursorDeltaTime(): real {
        return ( root.markerFraction - 0.5 ) * root.laTimeSpan()
    }

    function clearMarker(): void {
        root.markerHits = []
        root.markerRows = []
    }

    function refreshCursor(): void {
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

    function repaintLa(): void {
        if ( !laScreen || laScreen.width < 2 || laScreen.height < 2 )
            return
        CircuitCanvas.paintLaScreen( laScreen.width, laScreen.height, Screen.devicePixelRatio || 1 )
        root.refreshCursor()
    }

    function _runQueuedLaPaint(): void {
        root.laPaintQueued = false
        root.repaintLa()
    }

    function scheduleLaPaint(): void {
        if ( root.laPaintQueued )
            return
        root.laPaintQueued = true
        Qt.callLater( root._runQueuedLaPaint )
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
                    radius: 10
                    color: "white"
                }
            }

            Rectangle {
                id: laFrame
                anchors.fill: parent
                color: LogicAnalyzer.paperColor
                radius: 10
                layer.enabled: true
                layer.effect: MultiEffect {
                    maskEnabled: true
                    maskSource: laFrame ? laMask : null
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
                        id: hitDot
                        required property var modelData
                        width: 7
                        height: 7
                        radius: 3.5
                        x: Math.round( root.markerX ) - 3
                        y: Math.round( hitDot.modelData.y ) - 3
                        visible: hitDot.modelData.y >= -1 && hitDot.modelData.y <= cursorOverlay.height + 1
                        color: root.laChannelColor( hitDot.modelData.channel )
                        border.width: 1
                        border.color: LogicAnalyzer.cursorColor
                    }
                }

                Rectangle {
                    x: Math.min( parent.width - width - 6, Math.max( 6, root.cursorX + 8 ) )
                    y: Math.min( parent.height - height - 6, Math.max( 6, root.cursorY + 8 ) )
                    width: measureCol.implicitWidth + 12
                    height: measureCol.implicitHeight + 8
                    radius: 6
                    color: LogicAnalyzer.cursorFill
                    border.color: LogicAnalyzer.cursorBorder
                    border.width: 1

                    Column {
                        id: measureCol
                        anchors.centerIn: parent
                        spacing: 1

                        AppText {
                            text: "Δt: " + UiUtils.formatEngTime( root.cursorDeltaTime() )
                            color: LogicAnalyzer.cursorText
                            font.bold: true
                            font.pixelSize: App.fontMicro
                        }
                        Repeater {
                            model: root.markerRows
                            delegate: AppText {
                                id: rowLabel
                                required property var modelData
                                text: "Ch " + ( rowLabel.modelData.channel + 1 ) + ": " + root.formatLevels( rowLabel.modelData.values )
                                color: root.laChannelColor( rowLabel.modelData.channel )
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
                text: I18n.tr( "Trigger" )
                color: appTheme.text
                font.pixelSize: App.fontSmall
                font.bold: true
            }
            AppComboBox {
                Layout.preferredWidth: 110
                model: [ I18n.tr( "None" ), I18n.tr( "Rising" ), I18n.tr( "Falling" ), I18n.tr( "Both" ) ]
                currentIndex: LogicAnalyzer.trigger
                onActivated: {
                    LogicAnalyzer.trigger = currentIndex
                    CircuitCanvas.setLaTrigger( currentIndex )
                }
            }

            AppText {
                text: I18n.tr( "Conditions" )
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
                text: I18n.tr( "Export" )
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
                text: I18n.tr( "Decode" )
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
                label: I18n.tr( "Time / Div" )
                value: LogicAnalyzer.timeDiv
                unit: "s"
                isStepList: true
                stepList: UiUtils.timeDivSteps
                minVal: 1e-12
                maxVal: 1000.0
                defaultValue: 1e-3
                onValueModified: (newVal) => {
                    LogicAnalyzer.setTimeDiv( newVal )
                    CircuitCanvas.setLaTimeDiv( newVal )
                }
            }

            PlotValueControl {
                label: I18n.tr( "Time Pos" )
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
                    text: I18n.tr( "Bus" )
                    color: appTheme.text
                    font.pixelSize: App.fontSmall
                    font.bold: true
                }

                RowLayout {
                    spacing: 3
                    Repeater {
                        model: LogicAnalyzer.channels
                        delegate: AppCheckBox {
                            id: busCheck
                            required property int index
                            required property var modelData

                            readonly property color chColor: busCheck.modelData.color || busCheck.modelData

                            checked: LogicAnalyzer.buses[busCheck.index] === true
                            onToggled: LogicAnalyzer.busClicked( busCheck.index, busCheck.checked )

                            indicator: Rectangle {
                                implicitWidth: 18
                                implicitHeight: 18
                                radius: 5
                                color: busCheck.checked ? busCheck.chColor
                                                        : ( busCheck.hovered ? Qt.rgba( appTheme.midlight.r, appTheme.midlight.g, appTheme.midlight.b, 0.35 )
                                                                             : Qt.rgba( appTheme.base.r, appTheme.base.g, appTheme.base.b, 0.45 ) )
                                border.width: busCheck.checked ? 1.5 : 1
                                border.color: busCheck.checked ? Qt.darker( busCheck.chColor, 1.25 )
                                                               : ( UiUtils.isLightColor( busCheck.chColor ) && !App.darkTheme ? Qt.darker( busCheck.chColor, 1.35 ) : busCheck.chColor )

                                AppText {
                                    anchors.centerIn: parent
                                    text: busCheck.index + 1
                                    font.pixelSize: App.fontTiny
                                    font.bold: true
                                    color: busCheck.checked ? ( UiUtils.isLightColor( busCheck.chColor ) ? "#000000" : "#ffffff" ) : appTheme.text
                                }
                            }
                        }
                    }
                }
            }

            ColumnLayout { // Logic thresholds
                spacing: 2

                AppText {
                    text: I18n.tr( "Threshold" )
                    color: appTheme.text
                    font.pixelSize: App.fontSmall
                    font.bold: true
                }
                RowLayout {
                    spacing: 4
                    AppText { text: I18n.tr( "Rise" ); color: appTheme.text; font.pixelSize: App.fontSmall }
                    AppSpinBox {
                        editable: true
                        from: 0; to: 100
                        value: Math.round( LogicAnalyzer.thresholdR * 10 )
                        onValueModified: {
                            LogicAnalyzer.thresholdR = value / 10
                            CircuitCanvas.setLaThresholds( LogicAnalyzer.thresholdR, LogicAnalyzer.thresholdF )
                        }
                    }
                    AppText { text: I18n.tr( "Fall" ); color: appTheme.text; font.pixelSize: App.fontSmall }
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
