import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import cs_app

// Interactive PlotValueControl: Infinite Rotary Knob + Probe-style SI Text Field + Unit Label
RowLayout {
    id: pvc
    SystemPalette { id: appTheme }

    property string label: ""
    property real value: 1.0
    property string unit: ""
    property bool isStepList: false
    property var stepList: []
    property real stepDelta: 0.1
    property real minVal: -1e9
    property real maxVal: 1e9
    property real defaultValue: 0.0
    signal valueModified( real newValue )

    spacing: 6

    Item {
        id: dialItem
        Layout.preferredWidth: 32
        Layout.preferredHeight: 32
        Layout.alignment: Qt.AlignVCenter

        property real knobAngle: 0
        property real dragAccum: 0
        property real wheelAccum: 0

        Timer {
            id: wheelResetTimer
            interval: 180
            onTriggered: {
                dialItem.wheelAccum = 0
            }
        }

        Rectangle {
            id: dialBg
            anchors.centerIn: parent
            width: 28
            height: 28
            radius: 14
            color: mouseArea.pressed ? Qt.darker( appTheme.button, 1.2 ) : appTheme.button
            border.color: mouseArea.containsMouse ? appTheme.highlight : appTheme.mid
            border.width: 1.5

            Rectangle {
                id: notch
                x: parent.width / 2 - width / 2
                y: 2
                width: 2.5
                height: 6
                radius: 1.25
                color: mouseArea.pressed ? appTheme.highlight : appTheme.text
                transform: Rotation {
                    origin.x: notch.width / 2
                    origin.y: dialBg.height / 2 - notch.y
                    angle: dialItem.knobAngle
                }
            }
        }

        MouseArea {
            id: mouseArea
            anchors.fill: parent
            hoverEnabled: true
            cursorShape: Qt.PointingHandCursor

            property real lastDragX: 0
            property real lastDragY: 0

            onPressed: (mouse) => {
                valInput.focus = false
                lastDragX = mouse.x
                lastDragY = mouse.y
                dialItem.dragAccum = 0
            }

            onPositionChanged: (mouse) => {
                if ( !pressed ) return
                var dy = lastDragY - mouse.y
                var dx = mouse.x - lastDragX
                lastDragX = mouse.x
                lastDragY = mouse.y

                // Dragging up or right increases value, dragging down or left decreases value
                var dragAmount = dy + dx * 0.5
                dialItem.knobAngle += dragAmount * 1.5

                // Reset accumulator if reversing direction mid-drag
                if ( ( dialItem.dragAccum > 0 && dragAmount < 0 ) || ( dialItem.dragAccum < 0 && dragAmount > 0 ) ) {
                    dialItem.dragAccum = 0
                }
                dialItem.dragAccum += dragAmount

                var dragStepThreshold = 18
                if ( dialItem.dragAccum >= dragStepThreshold ) {
                    pvc.step( 1 )
                    dialItem.dragAccum = 0
                } else if ( dialItem.dragAccum <= -dragStepThreshold ) {
                    pvc.step( -1 )
                    dialItem.dragAccum = 0
                }
            }

            onReleased: {
                dialItem.dragAccum = 0
            }

            onDoubleClicked: {
                valInput.focus = false
                pvc.applyNewValue( pvc.defaultValue )
            }
        }

        WheelHandler {
            target: dialItem
            acceptedDevices: PointerDevice.Mouse | PointerDevice.TouchPad
            onWheel: (event) => {
                valInput.focus = false
                var dy = 0
                var isTrackpad = false

                if ( event.pixelDelta && event.pixelDelta.y !== 0 ) {
                    isTrackpad = true
                    dy = event.pixelDelta.y
                } else if ( event.angleDelta && event.angleDelta.y !== 0 ) {
                    dy = event.angleDelta.y / 120.0
                }

                if ( dy === 0 ) return

                // Smooth real-time visual rotation
                dialItem.knobAngle += ( isTrackpad ? dy * 3.0 : dy * 30.0 )

                // Direction reversal reset
                if ( ( dialItem.wheelAccum > 0 && dy < 0 ) || ( dialItem.wheelAccum < 0 && dy > 0 ) ) {
                    dialItem.wheelAccum = 0
                }
                dialItem.wheelAccum += dy

                var threshold = isTrackpad ? 20.0 : 1.0
                if ( dialItem.wheelAccum >= threshold ) {
                    pvc.step( 1 )
                    dialItem.wheelAccum = 0
                } else if ( dialItem.wheelAccum <= -threshold ) {
                    pvc.step( -1 )
                    dialItem.wheelAccum = 0
                }

                wheelResetTimer.restart()
            }
        }
    }

    function step( dir ) {
        if ( isStepList && stepList.length > 0 ) {
            var cur = pvc.value
            var idx = 0
            var minDiff = 1e12
            for ( var i = 0; i < stepList.length; ++i ) {
                var diff = Math.abs( Math.log( Math.max( 1e-18, stepList[i] ) ) - Math.log( Math.max( 1e-18, cur ) ) )
                if ( diff < minDiff ) {
                    minDiff = diff
                    idx = i
                }
            }
            if ( dir > 0 ) {
                if ( stepList[idx] <= cur * 1.0001 && idx + 1 < stepList.length ) {
                    pvc.applyNewValue( stepList[idx + 1] )
                } else {
                    pvc.applyNewValue( stepList[idx] )
                }
            } else if ( dir < 0 ) {
                if ( stepList[idx] >= cur * 0.9999 && idx - 1 >= 0 ) {
                    pvc.applyNewValue( stepList[idx - 1] )
                } else {
                    pvc.applyNewValue( stepList[idx] )
                }
            }
        } else {
            var delta = pvc.stepDelta || 0.1
            var nv = pvc.value + dir * delta
            pvc.applyNewValue( nv )
        }
    }

    function applyNewValue( nv ) {
        nv = Math.max( pvc.minVal, Math.min( pvc.maxVal, nv ) )
        pvc.valueModified( nv )
    }

    function formatVal( v, u ) {
        if ( typeof App !== "undefined" && typeof App.formatSiValue === "function" ) {
            return App.formatSiValue( v, u )
        }
        return "" + v
    }

    function parseVal( s, u ) {
        if ( typeof App !== "undefined" && typeof App.parseSi === "function" ) {
            return App.parseSi( s, u )
        }
        var f = parseFloat( s )
        return isNaN( f ) ? NaN : f
    }

    function syncText() {
        var t = formatVal( pvc.value, pvc.unit )
        if ( valInput.text !== t ) {
            valInput.text = t
        }
    }

    onValueChanged: syncText()
    onUnitChanged: syncText()
    Component.onCompleted: syncText()

    ColumnLayout {
        spacing: 2
        Layout.fillWidth: true

        Text {
            text: pvc.label
            color: appTheme.text
            font.pixelSize: 10
            font.bold: true
            opacity: 0.85
        }

        RowLayout {
            spacing: 4
            Layout.fillWidth: true

            AppTextField {
                id: valInput
                Layout.fillWidth: true
                Layout.preferredHeight: 24
                font.pixelSize: 11
                onActiveFocusChanged: {
                    if ( !activeFocus ) {
                        text = pvc.formatVal( pvc.value, pvc.unit )
                    }
                }
                onEditingFinished: {
                    var raw = text.trim()
                    var parsed = pvc.parseVal( raw, pvc.unit )
                    if ( !isNaN( parsed ) && isFinite( parsed ) ) {
                        pvc.applyNewValue( parsed )
                    }
                    text = pvc.formatVal( pvc.value, pvc.unit )
                }
            }

            Text {
                id: unitLabel
                visible: pvc.unit.length > 0
                text: pvc.unit
                font.family: App.fontFamily
                font.pixelSize: 11
                color: appTheme.text
                opacity: 0.85
                Layout.preferredWidth: visible ? 16 : 0
                verticalAlignment: Text.AlignVCenter
                elide: Text.ElideRight
            }
        }
    }
}
