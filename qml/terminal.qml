import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import cs_app
import cs_ui

// Body of Terminal: received bytes on top, send controls below.
Rectangle {
    id: root
    SystemPalette { id: appTheme }
    color: appTheme.window
    readonly property font uiFont: Qt.font({ family: App.fontFamily, pixelSize: App.fontNormal })

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 8
        spacing: 6

        RowLayout {
            Layout.fillWidth: true
            spacing: 6

            AppText {
                text: I18n.tr( "Format:" )
                color: appTheme.windowText
            }
            AppComboBox {
                model: [ "ASCII", "HEX", "DEC", "OCT", "BIN" ]
                currentIndex: model.indexOf( SerialTerminal.printMode )
                font: root.uiFont
                Layout.preferredWidth: 100
                onActivated: SerialTerminal.printMode = currentValue
            }
            Item { Layout.fillWidth: true }
            AppToolButton {
                contentItem: AppIcon { icon: "save"; color: appTheme.windowText }
                ToolTip.text: I18n.tr( "Save Log" )
                ToolTip.visible: hovered
                onClicked: SerialTerminal.requestSave()
            }
            AppButton {
                text: I18n.tr( "Clear" )
                font: root.uiFont
                onClicked: SerialTerminal.clearReceive()
            }
        }

        Rectangle {
            Layout.fillWidth: true
            Layout.fillHeight: true
            color: SerialTerminal.rxBackground
            radius: 4
            clip: true

            Flickable {
                anchors.fill: parent
                clip: true
                boundsBehavior: Flickable.StopAtBounds
                contentWidth: rxArea.implicitWidth
                contentHeight: rxArea.implicitHeight

                onWidthChanged: { contentX += 1; contentX -= 1 }
                onHeightChanged: { contentY += 1; contentY -= 1 }

                ScrollBar.vertical: AppScrollBar { policy: ScrollBar.AsNeeded }
                ScrollBar.horizontal: AppScrollBar { policy: ScrollBar.AsNeeded }

                TextArea.flickable: TextArea {
                    id: rxArea
                    text: SerialTerminal.log
                    textFormat: TextEdit.RichText
                    readOnly: true
                    wrapMode: TextEdit.WrapAnywhere
                    font.family: "Ubuntu Mono"
                    font.pixelSize: App.fontNormal
                    color: SerialTerminal.rxText
                    background: null
                    onTextChanged: cursorPosition = length
                }
            }
        }

        RowLayout {
            Layout.fillWidth: true
            spacing: 6

            AppText {
                text: I18n.tr( "Format:" )
                color: appTheme.windowText
            }
            AppComboBox {
                model: [ "ASCII", "HEX", "DEC", "OCT", "BIN" ]
                currentIndex: model.indexOf( SerialTerminal.sendMode )
                font: root.uiFont
                Layout.preferredWidth: 100
                onActivated: SerialTerminal.sendMode = currentValue
            }
            Item { Layout.fillWidth: true }
            AppToolButton {
                contentItem: AppIcon { icon: "file_open"; color: appTheme.windowText }
                ToolTip.text: I18n.tr( "Load File" )
                ToolTip.visible: hovered
                onClicked: SerialTerminal.requestLoad()
            }
            AppButton {
                text: I18n.tr( "Clear" )
                font: root.uiFont
                onClicked: SerialTerminal.clearSend()
            }
        }

        RowLayout {
            Layout.fillWidth: true
            spacing: 6

            AppTextField {
                id: txField
                Layout.fillWidth: true
                font.family: "Ubuntu Mono"
                text: SerialTerminal.inputText
                onTextEdited: SerialTerminal.inputText = text
                onAccepted: SerialTerminal.send()

                Connections {
                    target: SerialTerminal
                    function onInputTextChanged() {
                        if ( txField.text !== SerialTerminal.inputText )
                            txField.text = SerialTerminal.inputText
                    }
                }
            }
            AppButton {
                text: I18n.tr( "Send" )
                font: root.uiFont
                onClicked: SerialTerminal.send()
            }
        }
    }
}
