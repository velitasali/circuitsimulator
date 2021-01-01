import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import cs_app

Rectangle {
    id: root
    function tr(s) { return App.i18nTick >= 0 ? App.translate(s) : s }

    // Transparent where AppKit draws the window material behind the dialog,
    // the plain palette background everywhere else.
    color: pal.window

    SystemPalette { id: pal; colorGroup: SystemPalette.Active }
    readonly property font uiFont: Qt.font({ family: App.fontFamily || "Ubuntu", pixelSize: App.fontSize || 13 })

    implicitWidth: Math.max(380, mainLayout.implicitWidth + 40)
    implicitHeight: mainLayout.implicitHeight + 40

    RowLayout {
        id: mainLayout
        anchors.fill: parent
        anchors.margins: 20
        spacing: 20

        Image {
            source: "qrc:/icons/circuitsimulator.png"
            Layout.preferredWidth: 80
            Layout.preferredHeight: 80
            fillMode: Image.PreserveAspectFit
            mipmap: true
            Layout.alignment: Qt.AlignVCenter
        }

        ColumnLayout {
            Layout.fillWidth: true
            Layout.alignment: Qt.AlignVCenter
            spacing: 8

            Text {
                text: root.tr("Circuit Simulator")
                color: pal.windowText
                font.family: root.uiFont.family
                font.pixelSize: 22
                font.bold: true
            }

            GridLayout {
                columns: 2
                columnSpacing: 8
                rowSpacing: 4
                Layout.fillWidth: true

                Text {
                    text: root.tr("Version:")
                    color: pal.windowText
                    font.family: root.uiFont.family
                    font.pixelSize: root.uiFont.pixelSize
                    font.bold: true
                }
                Text {
                    text: App.version
                    color: pal.windowText
                    font: root.uiFont
                }

                Text {
                    text: root.tr("Website:")
                    color: pal.windowText
                    font.family: root.uiFont.family
                    font.pixelSize: root.uiFont.pixelSize
                    font.bold: true
                }
                Text {
                    text: "<a href=\"http://velitasali.com/\">http://velitasali.com</a>"
                    textFormat: Text.RichText
                    color: pal.windowText
                    linkColor: pal.highlight
                    font: root.uiFont
                    onLinkActivated: (link) => Qt.openUrlExternally(link)
                    MouseArea {
                        anchors.fill: parent
                        cursorShape: Qt.PointingHandCursor
                        acceptedButtons: Qt.NoButton
                    }
                }
            }
        }
    }
}
