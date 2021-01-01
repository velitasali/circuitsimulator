import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import cs_app

Rectangle {
    id: root

    color: pal.window

    SystemPalette { id: pal; colorGroup: SystemPalette.Active }
    readonly property font uiFont: Qt.font({ family: App.fontFamily || "Ubuntu", pixelSize: App.fontSize || 13 })
    function tr(s) { return App.i18nTick >= 0 ? App.translate(s) : s }

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 20
        spacing: 16

        RowLayout {
            spacing: 20

            Image {
                source: "qrc:/icons/qtlogo.svg"
                Layout.preferredWidth: 64
                Layout.preferredHeight: 64
                fillMode: Image.PreserveAspectFit
                mipmap: true
            }

            GridLayout {
                columns: 2
                columnSpacing: 12
                rowSpacing: 4
                Layout.fillWidth: true

                Text {
                    text: "Qt"
                    color: pal.windowText
                    font.family: root.uiFont.family
                    font.pixelSize: 26
                    font.bold: true
                    Layout.columnSpan: 2
                }
                Text {
                    text: root.tr("Version:")
                    color: pal.windowText
                    font.family: root.uiFont.family
                    font.pixelSize: root.uiFont.pixelSize
                    font.bold: true
                }
                Text {
                    text: App.qtVersion
                    color: pal.windowText
                    font: root.uiFont
                }
            }
        }

        Rectangle {
            Layout.fillWidth: true
            Layout.fillHeight: true
            color: pal.base
            radius: 8
            border.color: pal.mid
            border.width: 1
            clip: true

            Flickable {
                id: flickable
                anchors.fill: parent
                anchors.margins: 14
                contentWidth: width
                contentHeight: aboutText.implicitHeight
                boundsBehavior: Flickable.StopAtBounds
                clip: true

                ScrollBar.vertical: AppScrollBar {
                    policy: ScrollBar.AsNeeded
                }

                Text {
                    id: aboutText
                    width: flickable.width
                    textFormat: Text.RichText
                    wrapMode: Text.Wrap
                    color: pal.windowText
                    linkColor: pal.highlight
                    font: root.uiFont
                    lineHeight: 1.25
                    text: "<p><b>" + root.tr("This program uses Qt version %1.").replace("%1", App.qtVersion) + "</b></p>"
                        + "<p>" + root.tr("Qt is a C++ toolkit for cross-platform application development.") + "</p>"
                        + "<p>" + root.tr("Qt provides single-source portability across MS Windows, macOS, Linux, and all major commercial Unix variants. Qt is also available for embedded devices.") + "</p>"
                        + "<p>" + root.tr("Qt is available under multiple licensing options designed to accommodate the needs of our various users.") + "</p>"
                        + "<p>" + root.tr("Qt licensed under our commercial license agreement is appropriate for development of proprietary/commercial software where you do not want to share any source code with third parties or otherwise cannot comply with the terms of the GNU LGPL version 3.") + "</p>"
                        + "<p>" + root.tr("Qt licensed under the GNU LGPL version 3 is appropriate for the development of Qt applications (proprietary or open source) provided you can comply with the terms and conditions of the GNU LGPL version 3.") + "</p>"
                        + "<p>" + root.tr("Please see <a href=\"https://www.qt.io/licensing/\">qt.io/licensing</a> for an overview of Qt licensing.") + "</p>"
                        + "<p>" + root.tr("Copyright (C) 2026 The Qt Company Ltd and other contributors.") + "<br/>"
                        + root.tr("Qt and the Qt logo are trademarks of The Qt Company Ltd.") + "<br/>"
                        + root.tr("Qt is The Qt Company Ltd product developed as an open source project. See <a href=\"https://www.qt.io/\">qt.io</a> for more information.") + "</p>"
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
}
