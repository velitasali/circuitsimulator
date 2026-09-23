import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import cs_app

// The expanded oscilloscope: screen on the left, control panel on the right.
// Parity with src/gui/dataplotwidget/oscwidget.qml.
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

    readonly property var voltDivSteps: [
        1e-4, 1.2e-4, 1.5e-4, 2e-4, 2.5e-4, 3e-4, 4e-4, 5e-4, 6e-4, 7e-4, 8e-4, 9e-4,
        1e-3, 1.2e-3, 1.5e-3, 2e-3, 2.5e-3, 3e-3, 4e-3, 5e-3, 6e-3, 7e-3, 8e-3, 9e-3,
        1e-2, 1.2e-2, 1.5e-2, 2e-2, 2.5e-2, 3e-2, 4e-2, 5e-2, 6e-2, 7e-2, 8e-2, 9e-2,
        0.1, 0.12, 0.15, 0.2, 0.25, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9,
        1.0, 1.2, 1.5, 2.0, 2.5, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0,
        10.0, 12.0, 15.0, 20.0, 25.0, 30.0, 40.0, 50.0, 60.0, 70.0, 80.0, 90.0,
        100.0, 120.0, 150.0, 200.0, 250.0, 300.0, 400.0, 500.0, 600.0, 700.0, 800.0, 900.0,
        1e3, 1.2e3, 1.5e3, 2e3, 2.5e3, 3e3, 4e3, 5e3, 6e3, 7e3, 8e3, 9e3,
        1e4, 1.2e4, 1.5e4, 2e4, 2.5e4, 3e4, 4e4, 5e4
    ]

    property var chVoltDiv: Oscilloscope.chVoltDiv ? Oscilloscope.chVoltDiv.slice() : [ 1.0, 1.0, 1.0, 1.0 ]
    property var chVoltPos: Oscilloscope.chVoltPos ? Oscilloscope.chVoltPos.slice() : [ 0.0, 0.0, 0.0, 0.0 ]
    property var chTimePos: Oscilloscope.chTimePos ? Oscilloscope.chTimePos.slice() : [ 0.0, 0.0, 0.0, 0.0 ]
    property var chTrigLevel: Oscilloscope.chTrigLevel ? Oscilloscope.chTrigLevel.slice() : [ 0.0, 0.0, 0.0, 0.0 ]
    property var chTrigRising: Oscilloscope.chTrigRising ? Oscilloscope.chTrigRising.slice() : [ true, true, true, true ]

    Component.onCompleted: {
        if ( Oscilloscope.chVoltDiv && Oscilloscope.chVoltDiv.length === 4 ) {
            root.chVoltDiv = Oscilloscope.chVoltDiv.slice()
            for ( var i = 0; i < 4; ++i ) {
                CircuitCanvas.setScopeVoltDiv( i, root.chVoltDiv[i] )
            }
        }
        if ( Oscilloscope.chVoltPos && Oscilloscope.chVoltPos.length === 4 ) {
            root.chVoltPos = Oscilloscope.chVoltPos.slice()
            for ( var i = 0; i < 4; ++i ) {
                CircuitCanvas.setScopeVoltPos( i, root.chVoltPos[i] )
            }
        }
        if ( Oscilloscope.chTimePos && Oscilloscope.chTimePos.length === 4 ) {
            root.chTimePos = Oscilloscope.chTimePos.slice()
            CircuitCanvas.setScopeTimePos( root.chTimePos[Oscilloscope.currentChannel] || 0.0 )
        }
        if ( Oscilloscope.chTrigLevel && Oscilloscope.chTrigLevel.length === 4 ) {
            root.chTrigLevel = Oscilloscope.chTrigLevel.slice()
            CircuitCanvas.setScopeTrigLevel( root.chTrigLevel[Oscilloscope.currentChannel] || 0.0 )
        }
        if ( Oscilloscope.chTrigRising && Oscilloscope.chTrigRising.length === 4 ) {
            root.chTrigRising = Oscilloscope.chTrigRising.slice()
        }
        Oscilloscope.dark = App.darkTheme
        CircuitCanvas.setScopeTimeDiv( Oscilloscope.timeDiv )
        CircuitCanvas.setScopeTracks( Oscilloscope.tracks )
        CircuitCanvas.setScopeTrigger( Oscilloscope.trigger )
        CircuitCanvas.setScopeFilter( Oscilloscope.filter )
        if ( Oscilloscope.hidden && Oscilloscope.hidden.length === 4 ) {
            for ( var i = 0; i < 4; ++i ) {
                CircuitCanvas.setScopeHidden( i, Oscilloscope.hidden[i] )
            }
        }
    }

    Connections {
        target: App
        function onDarkThemeChanged() {
            Oscilloscope.dark = App.darkTheme
        }
    }

    Connections {
        target: Oscilloscope
        function onCurrentChannelChanged() {
            var ch = Oscilloscope.currentChannel
            CircuitCanvas.setScopeTimePos( root.chTimePos[ch] || 0.0 )
            CircuitCanvas.setScopeTrigLevel( root.chTrigLevel[ch] || 0.0 )
        }
    }

    function stepTimeDiv( dir ) {
        var cur = Oscilloscope.timeDiv
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
        Oscilloscope.setTimeDiv( nextVal )
        CircuitCanvas.setScopeTimeDiv( nextVal )
    }

    function autoScale( ch ) {
        var res = CircuitCanvas.autoScaleScope( ch )
        if ( !res || !res.valid ) return

        var copyVd = root.chVoltDiv.slice()
        copyVd[ch] = res.voltDiv
        root.chVoltDiv = copyVd
        Oscilloscope.setChVoltDiv( ch, res.voltDiv )
        CircuitCanvas.setScopeVoltDiv( ch, res.voltDiv )

        var copyVp = root.chVoltPos.slice()
        copyVp[ch] = res.voltPos
        root.chVoltPos = copyVp
        Oscilloscope.setChVoltPos( ch, res.voltPos )
        CircuitCanvas.setScopeVoltPos( ch, res.voltPos )

        var copyTp = root.chTimePos.slice()
        copyTp[ch] = 0.0
        root.chTimePos = copyTp
        Oscilloscope.setChTimePos( ch, 0.0 )
        CircuitCanvas.setScopeTimePos( 0.0 )

        if ( res.timeDiv && res.timeDiv > 0 ) {
            Oscilloscope.setTimeDiv( res.timeDiv )
            CircuitCanvas.setScopeTimeDiv( res.timeDiv )
        }
    }

    function autoScaleAll() {
        var res = CircuitCanvas.autoScaleScopeAll()
        if ( !res || !res.channels ) return

        var copyVd = root.chVoltDiv.slice()
        var copyVp = root.chVoltPos.slice()
        var copyTp = root.chTimePos.slice()

        for ( var ch = 0; ch < res.channels.length; ++ch ) {
            var cr = res.channels[ch]
            if ( cr && cr.valid ) {
                copyVd[ch] = cr.voltDiv
                copyVp[ch] = cr.voltPos
                copyTp[ch] = 0.0
                Oscilloscope.setChVoltDiv( ch, cr.voltDiv )
                CircuitCanvas.setScopeVoltDiv( ch, cr.voltDiv )
                Oscilloscope.setChVoltPos( ch, cr.voltPos )
                CircuitCanvas.setScopeVoltPos( ch, cr.voltPos )
                Oscilloscope.setChTimePos( ch, 0.0 )
            }
        }
        root.chVoltDiv = copyVd
        root.chVoltPos = copyVp
        root.chTimePos = copyTp
        CircuitCanvas.setScopeTimePos( 0.0 )

        if ( res.timeDiv && res.timeDiv > 0 ) {
            Oscilloscope.setTimeDiv( res.timeDiv )
            CircuitCanvas.setScopeTimeDiv( res.timeDiv )
        }
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

    // Clean grouped card container for control sections
    component ControlCard: Rectangle {
        id: card
        default property alias content: cardLayout.data

        Layout.fillWidth: true
        implicitHeight: cardLayout.implicitHeight + 12
        radius: 5
        color: Qt.rgba( appTheme.base.r, appTheme.base.g, appTheme.base.b, 0.45 )
        border.color: Qt.rgba( appTheme.mid.r, appTheme.mid.g, appTheme.mid.b, 0.55 )
        border.width: 1

        ColumnLayout {
            id: cardLayout
            anchors.fill: parent
            anchors.margins: 6
            spacing: 5
        }
    }

    // A row of one button per channel, tinted with channel color and high text contrast.
    component ChannelRow: RowLayout {
        id: chRow
        property string label: ""
        property bool exclusive: false
        property int activeIndex: -1
        property var checkedList: []
        required property var onPicked

        spacing: 3

        Text {
            Layout.preferredWidth: 48
            text: chRow.label
            color: appTheme.text
            font.pixelSize: 11
            font.bold: true
        }

        Repeater {
            model: Oscilloscope.channels
            delegate: AppButton {
                required property int index
                required property var modelData

                Layout.preferredWidth: 28
                Layout.preferredHeight: 22
                text: index + 1
                font.pixelSize: 11
                font.bold: on

                readonly property bool on: chRow.exclusive ? chRow.activeIndex === index
                                                           : ( chRow.checkedList && chRow.checkedList[index] === true )

                background: Rectangle {
                    radius: 3
                    color: parent.on ? modelData.color
                                     : ( parent.hovered ? Qt.rgba( appTheme.midlight.r, appTheme.midlight.g, appTheme.midlight.b, 0.35 )
                                                        : Qt.rgba( appTheme.base.r, appTheme.base.g, appTheme.base.b, 0.45 ) )
                    border.width: parent.on ? 1.5 : 1
                    border.color: parent.on ? Qt.darker( modelData.color, 1.25 )
                                            : ( root.isLightColor( modelData.color ) && !App.darkTheme ? Qt.darker( modelData.color, 1.35 ) : modelData.color )
                }
                contentItem: Text {
                    text: parent.text
                    horizontalAlignment: Text.AlignHCenter
                    verticalAlignment: Text.AlignVCenter
                    font: parent.font
                    color: parent.on ? ( root.isLightColor( modelData.color ) ? "#000000" : "#ffffff" ) : appTheme.text
                }

                onClicked: chRow.onPicked( index, !on )
            }
        }
    }


    RowLayout {
        anchors.fill: parent
        anchors.margins: 5
        spacing: 5

        // Oscilloscope waveform display area
        Rectangle {
            id: displaySlot
            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.minimumWidth: 240
            radius: 6
            color: "#000000"
            border.color: Qt.rgba( appTheme.mid.r, appTheme.mid.g, appTheme.mid.b, 0.55 )
            border.width: 1
            clip: true

            PlotCanvas {
                id: plotCanvas
                anchors.fill: parent
                radius: 6
                traces: CircuitCanvas.scopeTraces
                hidden: Oscilloscope.hidden
                paper: "#000000"
                tracks: Oscilloscope.tracks
                chVoltDiv: root.chVoltDiv
                chVoltPos: root.chVoltPos
                chTimePos: root.chTimePos
                voltDiv: root.chVoltDiv[Oscilloscope.currentChannel] || 1.0
                voltPos: root.chVoltPos[Oscilloscope.currentChannel] || 0.0
                timePos: root.chTimePos[Oscilloscope.currentChannel] || 0.0
                expanded: true
                timeDiv: Oscilloscope.timeDiv
            }

            MouseArea {
                id: plotMouseArea
                anchors.fill: parent
                acceptedButtons: Qt.LeftButton | Qt.MiddleButton
                hoverEnabled: true
                cursorShape: pressed ? Qt.ClosedHandCursor : Qt.CrossCursor

                property real lastDragX: 0

                onPressed: (mouse) => {
                    plotMouseArea.lastDragX = mouse.x
                    Oscilloscope.plotPressed( mouse.x, mouse.button )
                }
                onPositionChanged: (mouse) => {
                    plotCanvas.cursorX = mouse.x
                    plotCanvas.cursorY = mouse.y
                    if ( pressed && ( mouse.buttons & Qt.LeftButton ) ) {
                        var dx = mouse.x - plotMouseArea.lastDragX
                        plotMouseArea.lastDragX = mouse.x
                        var timeSpan = ( Oscilloscope.timeDiv || 1e-3 ) * 10.0
                        var dt = ( dx / Math.max( width, 1 ) ) * timeSpan
                        var ch = Oscilloscope.currentChannel
                        var copy = root.chTimePos.slice()
                        copy[ch] = ( copy[ch] || 0.0 ) + dt
                        root.chTimePos = copy
                        Oscilloscope.setChTimePos( ch, copy[ch] )
                        CircuitCanvas.setScopeTimePos( copy[ch] )
                        Oscilloscope.plotMoved( mouse.x )
                    }
                }
                onExited: {
                    plotCanvas.cursorX = -1
                    plotCanvas.cursorY = -1
                }
                onReleased: Oscilloscope.plotReleased()

                property real canvasWheelAccumY: 0
                Timer {
                    id: canvasWheelTimer
                    interval: 180
                    onTriggered: plotMouseArea.canvasWheelAccumY = 0
                }
                onWheel: (wheel) => {
                    // 1. Horizontal trackpad scroll or Shift+wheel: pan Time Pos
                    var dx = 0
                    if ( wheel.pixelDelta && wheel.pixelDelta.x !== 0 ) {
                        dx = wheel.pixelDelta.x
                    } else if ( wheel.angleDelta && wheel.angleDelta.x !== 0 ) {
                        dx = wheel.angleDelta.x / 120.0 * 25.0
                    }

                    if ( dx !== 0 ) {
                        var timeSpan = ( Oscilloscope.timeDiv || 1e-3 ) * 10.0
                        var dt = ( dx / Math.max( width, 1 ) ) * timeSpan
                        var ch = Oscilloscope.currentChannel
                        var copy = root.chTimePos.slice()
                        copy[ch] = ( copy[ch] || 0.0 ) + dt
                        root.chTimePos = copy
                        Oscilloscope.setChTimePos( ch, copy[ch] )
                        CircuitCanvas.setScopeTimePos( copy[ch] )
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
                        if ( ( plotMouseArea.canvasWheelAccumY > 0 && dy < 0 ) || ( plotMouseArea.canvasWheelAccumY < 0 && dy > 0 ) ) {
                            plotMouseArea.canvasWheelAccumY = 0
                        }
                        plotMouseArea.canvasWheelAccumY += dy

                        var thresh = isTrackpad ? 20.0 : 1.0
                        if ( plotMouseArea.canvasWheelAccumY >= thresh ) {
                            root.stepTimeDiv( -1 )
                            plotMouseArea.canvasWheelAccumY = 0
                        } else if ( plotMouseArea.canvasWheelAccumY <= -thresh ) {
                            root.stepTimeDiv( 1 )
                            plotMouseArea.canvasWheelAccumY = 0
                        }
                        canvasWheelTimer.restart()
                    }
                }
            }
        }

        // Sidebar controls
        ColumnLayout {
            Layout.alignment: Qt.AlignTop
            Layout.preferredWidth: 236
            Layout.minimumWidth: 236
            Layout.maximumWidth: 236
            spacing: 5

            ControlCard { // Global: Tracks, Time Div, Filter
                RowLayout {
                    spacing: 3
                    Text {
                        Layout.preferredWidth: 48
                        text: root.tr( "Tracks" )
                        color: appTheme.text
                        font.pixelSize: 11
                        font.bold: true
                    }
                    Repeater {
                        model: [ 1, 2, 4 ]
                        delegate: AppButton {
                            required property var modelData
                            Layout.preferredWidth: 28
                            Layout.preferredHeight: 22
                            text: modelData
                            font.pixelSize: 11
                            font.bold: Oscilloscope.tracks === modelData
                            checked: Oscilloscope.tracks === modelData
                            background: Rectangle {
                                radius: 3
                                color: parent.checked ? appTheme.highlight : ( parent.hovered ? appTheme.midlight : "transparent" )
                                border.width: 1
                                border.color: parent.checked ? appTheme.highlight : appTheme.mid
                            }
                            contentItem: Text {
                                text: parent.text
                                horizontalAlignment: Text.AlignHCenter
                                verticalAlignment: Text.AlignVCenter
                                font: parent.font
                                color: parent.checked ? appTheme.highlightedText : appTheme.text
                            }
                            onClicked: {
                                Oscilloscope.tracksClicked( modelData )
                                CircuitCanvas.setScopeTracks( modelData )
                            }
                        }
                    }
                }

                PlotValueControl {
                    label: root.tr( "Time / Div" )
                    value: Oscilloscope.timeDiv
                    unit: "s"
                    isStepList: true
                    stepList: root.timeDivSteps
                    minVal: 1e-12
                    maxVal: 1000.0
                    defaultValue: 1e-3
                    onValueModified: (newVal) => {
                        Oscilloscope.setTimeDiv( newVal )
                        CircuitCanvas.setScopeTimeDiv( newVal )
                    }
                }

                PlotValueControl {
                    label: root.tr( "Filter" )
                    value: Oscilloscope.filter
                    unit: "V"
                    stepDelta: 0.05
                    minVal: 0.0
                    maxVal: 1e6
                    defaultValue: 0.0
                    onValueModified: (newVal) => {
                        Oscilloscope.filter = newVal
                        CircuitCanvas.setScopeFilter( newVal )
                    }
                }
            }

            ControlCard { // Per-channel toggles: Auto, Trigger, Hide
                ChannelRow {
                    label: root.tr( "Auto" )
                    onPicked: function( index ) { root.autoScale( index ) }
                }
                ChannelRow {
                    label: root.tr( "Trigger" )
                    exclusive: true
                    activeIndex: Oscilloscope.trigger
                    onPicked: function( index ) {
                        Oscilloscope.triggerClicked( index )
                        CircuitCanvas.setScopeTrigger( Oscilloscope.trigger )
                    }
                }
                ChannelRow {
                    label: root.tr( "Hide" )
                    checkedList: Oscilloscope.hidden
                    onPicked: function( index, on ) {
                        Oscilloscope.hideClicked( index, on )
                        CircuitCanvas.setScopeHidden( index, on )
                    }
                }
            }

            ControlCard { // Channel-specific controls
                ChannelRow {
                    label: root.tr( "Channel" )
                    exclusive: true
                    activeIndex: Oscilloscope.currentChannel
                    onPicked: function( index ) { Oscilloscope.currentChannel = index }
                }

                PlotValueControl {
                    label: root.tr( "Volt / Div" )
                    value: root.chVoltDiv[Oscilloscope.currentChannel] || 1.0
                    unit: "V"
                    isStepList: true
                    stepList: root.voltDivSteps
                    minVal: 1e-6
                    maxVal: 1e6
                    defaultValue: 1.0
                    onValueModified: (newVal) => {
                        var ch = Oscilloscope.currentChannel
                        var copy = root.chVoltDiv.slice()
                        copy[ch] = newVal
                        root.chVoltDiv = copy
                        Oscilloscope.setChVoltDiv( ch, newVal )
                        CircuitCanvas.setScopeVoltDiv( ch, newVal )
                    }
                }

                PlotValueControl {
                    label: root.tr( "Volt Pos" )
                    value: root.chVoltPos[Oscilloscope.currentChannel] || 0.0
                    unit: "V"
                    stepDelta: ( root.chVoltDiv[Oscilloscope.currentChannel] || 1.0 ) / 10.0
                    minVal: -1e6
                    maxVal: 1e6
                    defaultValue: 0.0
                    onValueModified: (newVal) => {
                        var ch = Oscilloscope.currentChannel
                        var copy = root.chVoltPos.slice()
                        copy[ch] = newVal
                        root.chVoltPos = copy
                        Oscilloscope.setChVoltPos( ch, newVal )
                        CircuitCanvas.setScopeVoltPos( ch, newVal )
                    }
                }

                PlotValueControl {
                    label: root.tr( "Time Pos" )
                    value: root.chTimePos[Oscilloscope.currentChannel] || 0.0
                    unit: "s"
                    stepDelta: ( Oscilloscope.timeDiv / 10.0 )
                    minVal: -1e6
                    maxVal: 1e6
                    defaultValue: 0.0
                    onValueModified: (newVal) => {
                        var ch = Oscilloscope.currentChannel
                        var copy = root.chTimePos.slice()
                        copy[ch] = newVal
                        root.chTimePos = copy
                        Oscilloscope.setChTimePos( ch, newVal )
                        CircuitCanvas.setScopeTimePos( newVal )
                    }
                }

                PlotValueControl {
                    label: root.tr( "Trig Level" )
                    value: root.chTrigLevel[Oscilloscope.currentChannel] || 0.0
                    unit: "V"
                    stepDelta: ( root.chVoltDiv[Oscilloscope.currentChannel] || 1.0 ) / 10.0
                    minVal: -1e6
                    maxVal: 1e6
                    defaultValue: 0.0
                    onValueModified: (newVal) => {
                        var ch = Oscilloscope.currentChannel
                        var copy = root.chTrigLevel.slice()
                        copy[ch] = newVal
                        root.chTrigLevel = copy
                        Oscilloscope.setChTrigLevel( ch, newVal )
                        CircuitCanvas.setScopeTrigLevel( newVal )
                    }
                }

                RowLayout {
                    spacing: 6
                    Text {
                        Layout.preferredWidth: 48
                        text: root.tr( "Edge" )
                        color: appTheme.text
                        font.pixelSize: 11
                        font.bold: true
                    }
                    AppButton {
                        checkable: true
                        checked: root.chTrigRising[Oscilloscope.currentChannel] !== false
                        Layout.preferredHeight: 22
                        Layout.preferredWidth: 80
                        text: checked ? root.tr( "Rising ↑" ) : root.tr( "Falling ↓" )
                        font.pixelSize: 11
                        font.bold: true

                        background: Rectangle {
                            radius: 3
                            color: parent.checked ? Qt.rgba( appTheme.highlight.r, appTheme.highlight.g, appTheme.highlight.b, 0.25 )
                                                  : "transparent"
                            border.width: 1
                            border.color: parent.checked ? appTheme.highlight : appTheme.mid
                        }
                        contentItem: Text {
                            text: parent.text
                            horizontalAlignment: Text.AlignHCenter
                            verticalAlignment: Text.AlignVCenter
                            font: parent.font
                            color: parent.checked ? appTheme.highlight : appTheme.text
                        }

                        onToggled: {
                            var ch = Oscilloscope.currentChannel
                            var copy = root.chTrigRising.slice()
                            copy[ch] = checked
                            root.chTrigRising = copy
                            Oscilloscope.setChTrigRising( ch, checked )
                        }
                    }
                }
            }

            Item { Layout.fillHeight: true }
        }
    }
}
