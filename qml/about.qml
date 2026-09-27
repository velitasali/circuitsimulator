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

            AppText {
                text: root.tr("Circuit Simulator")
                color: pal.windowText
                font.pixelSize: 22
                font.bold: true
            }

            GridLayout {
                columns: 2
                columnSpacing: 8
                rowSpacing: 4
                Layout.fillWidth: true

                AppText {
                    text: root.tr("Version:")
                    color: pal.windowText
                    font.bold: true
                }
                AppText {
                    text: App.version
                    color: pal.windowText
                }

                AppText {
                    text: root.tr("Website:")
                    color: pal.windowText
                    font.bold: true
                }
                AppText {
                    text: "<a href=\"http://velitasali.com/\">http://velitasali.com</a>"
                    textFormat: Text.RichText
                    color: pal.windowText
                    linkColor: pal.highlight
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
