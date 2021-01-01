import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import cs_app

// Component Library Manager / Installer dialog.
Rectangle {
    id: root
    function tr(s) { return App.i18nTick >= 0 ? App.translate(s) : s }
    color: appTheme.window

    SystemPalette { id: appTheme; colorGroup: SystemPalette.Active }
    readonly property font uiFont: Qt.font({ family: App.fontFamily || "Ubuntu", pixelSize: App.fontSize || 13 })

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 12
        spacing: 10

        RowLayout {
            Layout.fillWidth: true
            spacing: 10

            AppButton {
                text: root.tr("Check for Updates")
                font: root.uiFont
                enabled: !Installer.busy
                onClicked: Installer.checkForUpdatesClicked()
            }

            BusyIndicator {
                running: Installer.busy
                visible: running
                implicitWidth: 20
                implicitHeight: 20
            }

            Text {
                text: Installer.statusText
                color: appTheme.windowText
                font: root.uiFont
                opacity: 0.75
                elide: Text.ElideRight
                Layout.fillWidth: true
            }
        }

        Rectangle {
            Layout.fillWidth: true
            Layout.fillHeight: true
            radius: 8
            color: appTheme.base
            border.color: appTheme.mid
            border.width: 1
            clip: true

            ListView {
                id: list
                anchors.fill: parent
                clip: true
                model: Installer.items
                boundsBehavior: Flickable.StopAtBounds
                ScrollBar.vertical: AppScrollBar {}

                delegate: Rectangle {
                    id: rowItem
                    required property var modelData
                    required property int index

                    width: list.width
                    height: modelData.isGroupHeader ? 28 : 54
                    color: modelData.isGroupHeader ? appTheme.alternateBase
                                                   : (index % 2 ? Qt.rgba(appTheme.window.r, appTheme.window.g, appTheme.window.b, 0.35) : "transparent")

                    RowLayout {
                        anchors.fill: parent
                        anchors.leftMargin: 10
                        anchors.rightMargin: 10
                        spacing: 8

                        ColumnLayout {
                            Layout.fillWidth: true
                            spacing: 2

                            Text {
                                text: rowItem.modelData.name
                                textFormat: Text.MarkdownText
                                font.family: root.uiFont.family
                                font.pixelSize: rowItem.modelData.isGroupHeader ? (root.uiFont.pixelSize + 1) : root.uiFont.pixelSize
                                font.bold: rowItem.modelData.isGroupHeader
                                color: appTheme.windowText
                                elide: Text.ElideRight
                                Layout.fillWidth: true
                            }
                            Text {
                                visible: !rowItem.modelData.isGroupHeader
                                text: rowItem.modelData.description || ""
                                textFormat: Text.MarkdownText
                                font.family: root.uiFont.family
                                font.pixelSize: Math.max(10, root.uiFont.pixelSize - 2)
                                color: appTheme.windowText
                                opacity: 0.65
                                elide: Text.ElideRight
                                Layout.fillWidth: true
                            }
                        }

                        BusyIndicator {
                            visible: rowItem.modelData.busy === true
                            running: visible
                            implicitWidth: 24
                            implicitHeight: 24
                        }

                        AppToolButton {
                            id: infoBtn
                            visible: !rowItem.modelData.isGroupHeader
                            implicitWidth: 28; implicitHeight: 28
                            contentItem: AppIcon { text: "help"; color: appTheme.windowText }
                            ToolTip.text: root.tr("Information")
                            ToolTip.visible: infoBtn.hovered
                            onClicked: Installer.showInfo(rowItem.modelData.name)
                        }

                        AppToolButton {
                            id: updtBtn
                            visible: !rowItem.modelData.isGroupHeader && rowItem.modelData.canUpdate === true
                            enabled: !Installer.busy
                            implicitWidth: 28; implicitHeight: 28
                            contentItem: AppIcon { text: "restart_alt"; color: appTheme.windowText }
                            ToolTip.text: root.tr("Update")
                            ToolTip.visible: updtBtn.hovered
                            onClicked: Installer.toggleInstall(rowItem.modelData.name)
                        }

                        AppToolButton {
                            id: instBtn
                            visible: !rowItem.modelData.isGroupHeader
                            enabled: !Installer.busy
                            implicitWidth: 28; implicitHeight: 28
                            contentItem: AppIcon {
                                text: rowItem.modelData.installed ? "delete" : "file_download"
                                color: appTheme.windowText
                            }
                            ToolTip.text: rowItem.modelData.installed ? root.tr("Uninstall") : root.tr("Install")
                            ToolTip.visible: instBtn.hovered
                            onClicked: Installer.toggleInstall(rowItem.modelData.name)
                        }
                    }
                }
            }
        }
    }

    Popup {
        id: infoPopup
        anchors.centerIn: parent
        width: Math.min(root.width - 40, 480)
        height: Math.min(root.height - 40, 400)
        modal: true
        visible: Installer.info.visible === true
        onClosed: Installer.clearInfo()

        background: Rectangle {
            color: appTheme.window
            border.color: appTheme.mid
            border.width: 1
            radius: 8
        }

        ColumnLayout {
            anchors.fill: parent
            anchors.margins: 16
            spacing: 8

            Text {
                text: "### " + (Installer.info.title || "")
                textFormat: Text.MarkdownText
                font: root.uiFont
                color: appTheme.windowText
            }

            Text {
                visible: (Installer.info.author || "").length > 0
                text: "**" + root.tr("Author:") + "** " + (Installer.info.author || "")
                textFormat: Text.MarkdownText
                font: root.uiFont
                color: appTheme.windowText
            }

            Text {
                visible: (Installer.info.description || "").length > 0
                text: Installer.info.description || ""
                wrapMode: Text.WordWrap
                font: root.uiFont
                color: appTheme.windowText
                Layout.fillWidth: true
            }

            Text {
                text: "**" + root.tr("Included Components:") + "**"
                textFormat: Text.MarkdownText
                font: root.uiFont
                color: appTheme.windowText
            }

            ScrollView {
                Layout.fillWidth: true
                Layout.fillHeight: true
                clip: true

                AppTextArea {
                    readOnly: true
                    text: (Installer.info.items && Installer.info.items.length > 0)
                          ? Installer.info.items.join("\n")
                          : root.tr("No installed components found in package.")
                    font: root.uiFont
                    color: appTheme.windowText
                }
            }

            RowLayout {
                Layout.fillWidth: true
                Item { Layout.fillWidth: true }
                AppButton {
                    text: root.tr("Close")
                    font: root.uiFont
                    onClicked: infoPopup.close()
                }
            }
        }
    }
}
