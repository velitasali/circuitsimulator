import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
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
    }

    Connections {
        target: App
        function onDarkThemeChanged() {
            LogicAnalyzer.dark = App.darkTheme
        }
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
            radius: 6
            color: "#000000"
            border.color: Qt.rgba( appTheme.mid.r, appTheme.mid.g, appTheme.mid.b, 0.55 )
            border.width: 1
            clip: true

            PlotCanvas {
                id: plotCanvas
                anchors.fill: parent
                radius: 6
                traces: CircuitCanvas.laTraces
                digital: true
                paper: "#000000"
                timeDiv: LogicAnalyzer.timeDiv
                timePos: LogicAnalyzer.timePos
            }

            MouseArea {
                id: laMouseArea
                anchors.fill: parent
                acceptedButtons: Qt.LeftButton | Qt.MiddleButton
                hoverEnabled: false
                cursorShape: pressed ? Qt.ClosedHandCursor : Qt.ArrowCursor

                property real lastDragX: 0

                onPressed: (mouse) => {
                    laMouseArea.lastDragX = mouse.x
                    LogicAnalyzer.plotPressed( mouse.x, mouse.button )
                }
                onPositionChanged: (mouse) => {
                    if ( pressed && ( mouse.buttons & Qt.LeftButton ) ) {
                        var dx = mouse.x - laMouseArea.lastDragX
                        laMouseArea.lastDragX = mouse.x
                        var timeSpan = ( LogicAnalyzer.timeDiv || 1e-3 ) * 10.0
                        var dt = ( dx / Math.max( width, 1 ) ) * timeSpan
                        LogicAnalyzer.timePos = LogicAnalyzer.timePos + dt
                        CircuitCanvas.setLaTimePos( LogicAnalyzer.timePos )
                        LogicAnalyzer.plotMoved( mouse.x )
                    }
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
                        var timeSpan = ( LogicAnalyzer.timeDiv || 1e-3 ) * 10.0
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

        // Top control bar: Trigger, conditions, export
        RowLayout {
            spacing: 6

            Text {
                text: root.tr( "Trigger" )
                color: appTheme.text
                font.pixelSize: 11
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

            Text {
                text: root.tr( "Conditions" )
                color: appTheme.text
                font.pixelSize: 11
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

                Text {
                    text: root.tr( "Bus" )
                    color: appTheme.text
                    font.pixelSize: 11
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

                                Text {
                                    anchors.centerIn: parent
                                    text: index + 1
                                    font.pixelSize: 10
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

                Text {
                    text: root.tr( "Threshold" )
                    color: appTheme.text
                    font.pixelSize: 11
                    font.bold: true
                }
                RowLayout {
                    spacing: 4
                    Text { text: root.tr( "Rise" ); color: appTheme.text; font.pixelSize: 11 }
                    AppSpinBox {
                        editable: true
                        from: 0; to: 100
                        value: Math.round( LogicAnalyzer.thresholdR * 10 )
                        onValueModified: {
                            LogicAnalyzer.thresholdR = value / 10
                            CircuitCanvas.setLaThresholds( LogicAnalyzer.thresholdR, LogicAnalyzer.thresholdF )
                        }
                    }
                    Text { text: root.tr( "Fall" ); color: appTheme.text; font.pixelSize: 11 }
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
