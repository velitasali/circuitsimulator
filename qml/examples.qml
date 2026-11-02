import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import cs_app

Rectangle {
    id: root
    function tr(s) { return App.i18nTick >= 0 ? App.translate(s) : s }

    signal exampleChosen(string id)

    color: pal.window
    SystemPalette { id: pal; colorGroup: SystemPalette.Active }

    implicitWidth: 460
    implicitHeight: 420

    property var entries: []

    Component.onCompleted: root.entries = CircuitCanvas.examples()

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 16
        spacing: 12

        AppText {
            text: root.tr("Example circuits")
            color: pal.windowText
            font.pixelSize: App.fontTitle
            font.bold: true
        }

        AppText {
            text: root.tr("Open a built-in circuit. It stays unsaved until you choose Save.")
            color: pal.windowText
            Layout.fillWidth: true
            wrapMode: Text.WordWrap
        }

        ListView {
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
            spacing: 8
            model: root.entries

            delegate: Rectangle {
                id: card
                required property var modelData
                width: ListView.view ? ListView.view.width : 400
                height: cardText.implicitHeight + 20
                radius: 4
                color: pal.base
                border.color: pal.mid
                border.width: 1

                Column {
                    id: cardText
                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.verticalCenter: parent.verticalCenter
                    anchors.leftMargin: 12
                    anchors.rightMargin: 12
                    spacing: 4

                    AppText {
                        text: card.modelData.title
                        color: pal.windowText
                        font.bold: true
                        width: parent.width
                    }
                    AppText {
                        text: card.modelData.summary
                        color: pal.windowText
                        width: parent.width
                        wrapMode: Text.WordWrap
                    }
                }

                MouseArea {
                    anchors.fill: parent
                    cursorShape: Qt.PointingHandCursor
                    onClicked: root.exampleChosen(card.modelData.id)
                }
            }
        }
    }
}
