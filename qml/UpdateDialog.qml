pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import cs_app
import cs_ui

Rectangle {
    id: root
    color: appTheme.window

    SystemPalette { id: appTheme; colorGroup: SystemPalette.Active }
    readonly property font uiFont: Qt.font({ family: App.fontFamily, pixelSize: App.fontNormal })

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 16
        spacing: 12

        // Header: App icon + Version summary + Status
        RowLayout {
            Layout.fillWidth: true
            spacing: 14

            Image {
                source: "qrc:/icons/circuitsimulator.png"
                Layout.preferredWidth: 48
                Layout.preferredHeight: 48
                sourceSize.width: 96
                sourceSize.height: 96
                fillMode: Image.PreserveAspectFit
                mipmap: true
                Layout.alignment: Qt.AlignVCenter
            }

            ColumnLayout {
                Layout.fillWidth: true
                spacing: 4

                AppText {
                    text: Updater.updateAvailable && Updater.releaseTitle !== ""
                        ? Updater.releaseTitle
                        : I18n.tr("Circuit Simulator Updates")
                    font.pixelSize: App.fontTitle
                    font.bold: true
                    color: appTheme.windowText
                    elide: Text.ElideRight
                    Layout.fillWidth: true
                }

                RowLayout {
                    spacing: 12
                    Layout.fillWidth: true

                    AppText {
                        text: I18n.tr("Current: %1").replace("%1", Updater.currentVersion)
                        font.pixelSize: App.fontSmall
                        color: appTheme.windowText
                        opacity: 0.75
                    }

                    AppText {
                        visible: Updater.hasChecked && Updater.errorText === ""
                        text: Updater.publishedAt !== ""
                            ? I18n.tr("Latest: %1 (%2)").replace("%1", Updater.latestVersion).replace("%2", Updater.publishedAt)
                            : I18n.tr("Latest: %1").replace("%1", Updater.latestVersion)
                        font.pixelSize: App.fontSmall
                        font.bold: Updater.updateAvailable
                        color: Updater.updateAvailable ? appTheme.highlight : appTheme.windowText
                        opacity: Updater.updateAvailable ? 1.0 : 0.75
                    }
                }
            }
        }

        // Status banner
        Rectangle {
            Layout.fillWidth: true
            implicitHeight: statusRow.implicitHeight + 16
            radius: 10
            color: appTheme.base
            border.color: appTheme.mid
            border.width: 1

            RowLayout {
                id: statusRow
                anchors.fill: parent
                anchors.leftMargin: 12
                anchors.rightMargin: 12
                spacing: 10

                AppIcon {
                    id: busySpinner
                    visible: Updater.checking || Updater.downloading
                    icon: "refresh"
                    size: 18
                    color: appTheme.highlight
                    Layout.preferredWidth: 18
                    Layout.preferredHeight: 18

                    NumberAnimation on rotation {
                        from: 0
                        to: 360
                        duration: 900
                        loops: Animation.Infinite
                        running: busySpinner.visible
                    }
                }

                AppIcon {
                    visible: !Updater.checking && !Updater.downloading && Updater.statusText !== ""
                    icon: Updater.errorText !== ""
                        ? "warning"
                        : Updater.updateAvailable
                            ? "file_download"
                            : "check"
                    size: 18
                    color: Updater.errorText !== ""
                        ? CircuitCanvas.msgWarnBg
                        : Updater.updateAvailable
                            ? appTheme.highlight
                            : CircuitCanvas.msgOkBg
                }

                AppText {
                    text: Updater.statusText !== ""
                        ? Updater.statusText
                        : I18n.tr("Click 'Check for Updates' to query the latest GitHub release.")
                    color: appTheme.windowText
                    wrapMode: Text.WordWrap
                    Layout.fillWidth: true
                }
            }
        }

        // Release notes box
        ColumnLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            spacing: 6

            AppText {
                text: I18n.tr("Release Notes")
                font.pixelSize: App.fontSmall
                font.bold: true
                color: appTheme.windowText
                opacity: 0.8
            }

            Rectangle {
                Layout.fillWidth: true
                Layout.fillHeight: true
                radius: 10
                color: appTheme.base
                border.color: appTheme.mid
                border.width: 1
                clip: true

                ScrollableTextEdit {
                    id: notesArea
                    anchors.fill: parent
                    anchors.margins: 1
                    readOnly: true
                    selectByMouse: true
                    wordWrap: true
                    contentPadding: 10
                    textFormat: Updater.releaseNotes !== ""
                        ? TextEdit.MarkdownText
                        : TextEdit.PlainText
                    text: Updater.releaseNotes !== ""
                        ? Updater.releaseNotes
                        : (Updater.checking
                            ? I18n.tr("Fetching release notes from GitHub...")
                            : I18n.tr("No release notes available."))
                    font: root.uiFont
                    color: appTheme.windowText
                    onTextChanged: flickable.contentY = 0
                    onLinkActivated: (link) => {
                        if (/^https?:\/\//i.test(link))
                            Qt.openUrlExternally(link)
                    }

                    HoverHandler {
                        cursorShape: notesArea.hoveredLink !== ""
                            ? Qt.PointingHandCursor
                            : Qt.IBeamCursor
                    }
                }
            }
        }

        // Build / Package selector (auto-detects current build type at index 0, allows choosing alternative)
        ColumnLayout {
            visible: Updater.updateAvailable && Updater.assets.length > 0
            Layout.fillWidth: true
            spacing: 6

            RowLayout {
                Layout.fillWidth: true
                spacing: 8

                AppText {
                    text: I18n.tr("Package:")
                    font.pixelSize: App.fontSmall
                    font.bold: true
                    color: appTheme.windowText
                }

                AppText {
                    text: I18n.tr("(Auto-detected: %1)").replace("%1", Updater.detectedBuildLabel)
                    font.pixelSize: App.fontSmall
                    color: appTheme.windowText
                    opacity: 0.65
                    elide: Text.ElideRight
                    Layout.fillWidth: true
                }
            }

            AppComboBox {
                id: assetCombo
                Layout.fillWidth: true
                font: root.uiFont
                enabled: !Updater.downloading && !Updater.checking
                model: Updater.assets
                currentIndex: Updater.selectedAssetIndex
                onActivated: Updater.selectedAssetIndex = currentIndex
                Binding on currentIndex { value: Updater.selectedAssetIndex }
            }
        }

        // Downloaded file actions bar
        Rectangle {
            visible: Updater.downloadedPath !== ""
            Layout.fillWidth: true
            implicitHeight: downloadedRow.implicitHeight + 14
            radius: 10
            color: appTheme.base
            border.color: appTheme.highlight
            border.width: 1

            RowLayout {
                id: downloadedRow
                anchors.fill: parent
                anchors.leftMargin: 10
                anchors.rightMargin: 10
                spacing: 8

                AppIcon {
                    icon: "check"
                    size: 16
                    color: appTheme.highlight
                }

                AppText {
                    text: Updater.downloadedPath
                    font.pixelSize: App.fontSmall
                    color: appTheme.windowText
                    elide: Text.ElideMiddle
                    Layout.fillWidth: true
                }

                AppButton {
                    text: I18n.tr("Show in Folder")
                    font: root.uiFont
                    onClicked: Updater.openDownloadFolder()
                }

                AppButton {
                    text: Updater.canLaunchInstaller ? I18n.tr("Run Installer") : I18n.tr("Open")
                    font: root.uiFont
                    highlighted: true
                    onClicked: Updater.openDownloadedFile()
                }
            }
        }

        // Footer buttons
        RowLayout {
            Layout.fillWidth: true
            spacing: 8

            AppButton {
                text: I18n.tr("View on GitHub")
                font: root.uiFont
                onClicked: Qt.openUrlExternally(Updater.releaseUrl)
            }

            AppButton {
                text: I18n.tr("Check Again")
                font: root.uiFont
                enabled: !Updater.checking && !Updater.downloading
                onClicked: Updater.checkForUpdates()
            }

            Item { Layout.fillWidth: true }

            AppButton {
                visible: Updater.updateAvailable && Updater.assets.length > 0
                text: Updater.downloading ? I18n.tr("Downloading...") : I18n.tr("Download Update")
                font: root.uiFont
                highlighted: true
                enabled: !Updater.downloading && !Updater.checking
                onClicked: Updater.downloadSelectedAsset()
            }

            AppButton {
                text: I18n.tr("Close")
                font: root.uiFont
                onClicked: Updater.dialogVisible = false
            }
        }
    }
}
