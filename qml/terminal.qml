import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import cs_app

// Body of Terminal: received bytes on top, send controls below.
Rectangle {
    id: root
    function tr(s) { return App.i18nTick >= 0 ? App.translate(s) : s }
    SystemPalette { id: appTheme }
    color: appTheme.window
    readonly property font uiFont: Qt.font({ family: App.fontFamily || "Ubuntu", pixelSize: App.fontSize || 13 })

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 8
        spacing: 6

        RowLayout {
            Layout.fillWidth: true
            spacing: 6

            Text {
                text: root.tr( "Format:" )
                font: root.uiFont
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
                contentItem: AppIcon { text: "save"; color: appTheme.windowText }
                ToolTip.text: root.tr( "Save Log" )
                ToolTip.visible: hovered
                onClicked: SerialTerminal.requestSave()
            }
            AppButton {
                text: root.tr( "Clear" )
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
                    font.pixelSize: 13
                    color: SerialTerminal.rxText
                    background: null
                    onTextChanged: cursorPosition = length
                }
            }
        }

        RowLayout {
            Layout.fillWidth: true
            spacing: 6

            Text {
                text: root.tr( "Format:" )
                font: root.uiFont
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
                contentItem: AppIcon { text: "file_open"; color: appTheme.windowText }
                ToolTip.text: root.tr( "Load File" )
                ToolTip.visible: hovered
                onClicked: SerialTerminal.requestLoad()
            }
            AppButton {
                text: root.tr( "Clear" )
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
                font.pixelSize: 13
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
                text: root.tr( "Send" )
                font: root.uiFont
                onClicked: SerialTerminal.send()
            }
        }
    }
}
