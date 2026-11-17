import QtQuick
import QtQuick.Layouts
import cs_app
import cs_ui

// The floating readout panel. Sized to its content (the QQuickWidget is in
// SizeViewToRootObject mode) so CircuitWidget can position it by its own size,
// and translucent so the canvas shows through behind it.
Rectangle {
    id: root

    implicitWidth: Math.max( 280, content.implicitWidth + 30 )
    implicitHeight: content.implicitHeight + 30
    // The panel rect can be shorter than this implicit height. Clip the readout
    // so it does not paint through the toolbar; the border stays on this item.
    clip: true

    color: InfoWidget.panelBackground
    border.color: InfoWidget.panelBorder
    border.width: 1
    radius: 16

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

        AppText {
            id: nameText
            font.pixelSize: App.fontSmall
            font.bold: true
            color: InfoWidget.textColor
            Layout.fillWidth: true
        }
        AppText {
            id: valueText
            font.pixelSize: App.fontSmall
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

            AppText {
                text: I18n.tr( "Simulation Time" )
                font.pixelSize: App.fontSmall
                font.bold: true
                color: InfoWidget.textColor
            }
            AppText {
                text: InfoWidget.simTime
                font.pixelSize: App.fontSmall
                font.weight: Font.Normal
                color: InfoWidget.textColor
            }
        }

        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 1
            color: InfoWidget.panelBorder
            opacity: 0.4
        }

        ColumnLayout {
            Layout.fillWidth: true
            spacing: 5

            Readout { label: I18n.tr( "Target Speed:" ); value: InfoWidget.targetSpeed }
            Readout { label: I18n.tr( "Real Speed:" ); value: InfoWidget.realSpeed }
            Readout { label: I18n.tr( "Sim Load:" ); value: InfoWidget.simLoad }
            Readout { label: I18n.tr( "GUI Load:" ); value: InfoWidget.guiLoad }
            Readout {
                label: I18n.tr( "Over Load:" )
                value: InfoWidget.overLoad
                visible: InfoWidget.overLoaded
            }
            Readout { label: I18n.tr( "FPS:" ); value: InfoWidget.fps }
            Readout {
                label: I18n.tr( "MCU:" )
                value: InfoWidget.mcuDevice
                visible: InfoWidget.hasMcu
            }
            Readout {
                label: I18n.tr( "Name:" )
                value: InfoWidget.mcuName
                visible: InfoWidget.hasMcu
            }
        }

        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 1
            color: InfoWidget.panelBorder
            opacity: 0.4
        }

        // Was a separate CurrentWidget; the value it drives is still read from
        // Connector on every repaint.
        ColumnLayout {
            Layout.fillWidth: true
            spacing: 4

            AppText {
                text: I18n.tr( "Current Speed" )
                font.pixelSize: App.fontSmall
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
