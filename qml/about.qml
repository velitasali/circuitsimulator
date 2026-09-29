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

    implicitWidth: 440
    implicitHeight: mainLayout.implicitHeight + 40

    ColumnLayout {
        id: mainLayout
        anchors.fill: parent
        anchors.margins: 20
        spacing: 16

        RowLayout {
            spacing: 20
            Layout.fillWidth: true

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
                            cursorShape: parent.hoveredLink ? Qt.PointingHandCursor : Qt.ArrowCursor
                            acceptedButtons: Qt.NoButton
                        }
                    }
                }
            }
        }

        Rectangle {
            Layout.fillWidth: true
            height: 1
            color: pal.mid
        }

        AppText {
            Layout.fillWidth: true
            Layout.preferredWidth: 400
            wrapMode: Text.WordWrap
            font.pixelSize: App.fontSmall
            color: pal.windowText
            linkColor: pal.highlight
            textFormat: Text.RichText
            lineHeight: 1.25
            text: "<p>" + root.tr("This program uses Qt version %1.").replace("%1", App.qtVersion) + "<br/>"
                + root.tr("Qt is licensed under the GNU LGPL version 3.") + "<br/>"
                + root.tr("Please see <a href=\"https://www.qt.io/licensing/\">qt.io/licensing</a> for an overview of Qt licensing.") + "</p>"
                + "<p>" + root.tr("Copyright (C) 2026 The Qt Company Ltd and other contributors.") + "</p>"
            onLinkActivated: (link) => Qt.openUrlExternally(link)
            MouseArea {
                anchors.fill: parent
                cursorShape: parent.hoveredLink ? Qt.PointingHandCursor : Qt.ArrowCursor
                acceptedButtons: Qt.NoButton
            }
        }
    }
}
