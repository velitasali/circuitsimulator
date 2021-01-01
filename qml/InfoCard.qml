import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import cs_app

// The floating readout panel. Sized to its content (the QQuickWidget is in
// SizeViewToRootObject mode) so CircuitWidget can position it by its own size,
// and translucent so the canvas shows through behind it.
Rectangle {
    id: root
    function tr(s) { return App.i18nTick >= 0 ? App.translate(s) : s }

    implicitWidth: Math.max( 280, content.implicitWidth + 30 )
    implicitHeight: content.implicitHeight + 30

    color: InfoWidget.panelBackground
    border.color: InfoWidget.panelBorder
    border.width: 1
    radius: 12

    Connections {
        target: InfoWidget
        function onSpeedChanged() {
            CircuitCanvas.setCurrSpeedSlider(InfoWidget.sliderValue)
        }
    }

    component Readout: RowLayout {
        property alias label: nameText.text
        property alias value: valueText.text
        Layout.fillWidth: true
        spacing: 12

        Text {
            id: nameText
            font.family: App.fontFamily
            font.pixelSize: 12
            font.bold: true
            color: InfoWidget.textColor
            Layout.fillWidth: true
        }
        Text {
            id: valueText
            font.family: App.fontFamily
            font.pixelSize: 12
            font.weight: Font.Normal
            color: InfoWidget.textColor
            horizontalAlignment: Text.AlignRight
        }
    }

    ColumnLayout {
        id: content
        anchors.fill: parent
        anchors.margins: 15
        spacing: 10

        ColumnLayout {
            Layout.fillWidth: true
            spacing: 3

            Text {
                text: root.tr( "Simulation Time" )
                font.family: App.fontFamily
                font.pixelSize: 12
                font.bold: true
                color: InfoWidget.textColor
            }
            Text {
                text: InfoWidget.simTime
                font.family: App.fontFamily
                font.pixelSize: 12
                font.weight: Font.Normal
                color: InfoWidget.textColor
            }
        }

        Rectangle {
            Layout.fillWidth: true
            height: 1
            color: InfoWidget.panelBorder
            opacity: 0.4
        }

        ColumnLayout {
            Layout.fillWidth: true
            spacing: 5

            Readout { label: root.tr( "Target Speed:" ); value: InfoWidget.targetSpeed }
            Readout { label: root.tr( "Real Speed:" ); value: InfoWidget.realSpeed }
            Readout { label: root.tr( "Sim Load:" ); value: InfoWidget.simLoad }
            Readout { label: root.tr( "GUI Load:" ); value: InfoWidget.guiLoad }
            Readout {
                label: root.tr( "Over Load:" )
                value: InfoWidget.overLoad
                visible: InfoWidget.overLoaded
            }
            Readout { label: root.tr( "FPS:" ); value: InfoWidget.fps }
            Readout {
                label: root.tr( "MCU:" )
                value: InfoWidget.mcuDevice
                visible: InfoWidget.hasMcu
            }
            Readout {
                label: root.tr( "Name:" )
                value: InfoWidget.mcuName
                visible: InfoWidget.hasMcu
            }
        }

        Rectangle {
            Layout.fillWidth: true
            height: 1
            color: InfoWidget.panelBorder
            opacity: 0.4
        }

        // Was a separate CurrentWidget; the value it drives is still read from
        // Connector on every repaint.
        ColumnLayout {
            Layout.fillWidth: true
            spacing: 4

            Text {
                text: root.tr( "Current Speed" )
                font.family: App.fontFamily
                font.pixelSize: 12
                font.bold: true
                color: InfoWidget.textColor
            }
            AppSlider {
                from: 1
                to: 1000
                value: InfoWidget.sliderValue
                Layout.fillWidth: true
                onMoved: InfoWidget.sliderValue = value
            }
        }
    }
}
