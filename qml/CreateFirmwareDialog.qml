import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtQuick.Dialogs
import cs_app
import cs_ui

Dialog {
    id: root
    modal: true
    dim: true
    anchors.centerIn: parent
    padding: 0
    header: null
    footer: null
    closePolicy: Popup.CloseOnEscape
    width: Math.min(Math.max(parent ? parent.width - 32 : 440, 360), 480)

    SystemPalette { id: appTheme }

    property string targetUid: ""
    property string mcuLabel: ""
    property string mcuDevice: ""
    property string compilerName: ""
    property string boardName: ""
    property string deviceName: ""
    property string fileExt: "ino"
    property string submitError: ""

    property var validation: ({ valid: false, error: "", previewPath: "" })
    readonly property bool canSubmit: !!(validation && validation.valid)
    readonly property string displayError: submitError.length > 0
        ? submitError
        : ((validation && validation.error) ? String(validation.error) : "")
    readonly property string previewPath: (validation && validation.previewPath)
        ? String(validation.previewPath)
        : ""

    readonly property string presetSummary: {
        var parts = []
        if (mcuLabel.length > 0)
            parts.push(mcuLabel)
        else if (mcuDevice.length > 0)
            parts.push(mcuDevice)
        if (compilerName.length > 0 && compilerName !== "None")
            parts.push(compilerName)
        if (boardName.length > 0)
            parts.push(boardName)
        else if (deviceName.length > 0 && deviceName !== mcuLabel)
            parts.push(deviceName)
        return parts.join(" · ")
    }

    function refreshValidation(): void {
        submitError = ""
        if (targetUid.length === 0) {
            validation = { valid: false, error: "", previewPath: "" }
            return
        }
        validation = CircuitCanvas.validateNewFirmware(targetUid, dirInput.text, nameInput.text)
    }

    function openFor(uid: string): void {
        submitError = ""
        var info = CircuitCanvas.firmwareCreateInfo(uid)
        if (!info)
            return
        targetUid = info.uid ? String(info.uid) : uid
        mcuLabel = info.mcuLabel ? String(info.mcuLabel) : ""
        mcuDevice = info.mcuDevice ? String(info.mcuDevice) : ""
        compilerName = info.compiler ? String(info.compiler) : ""
        boardName = info.board ? String(info.board) : ""
        deviceName = info.device ? String(info.device) : ""
        fileExt = info.extension ? String(info.extension) : "ino"
        dirInput.text = info.defaultDir ? String(info.defaultDir) : ""
        nameInput.text = info.suggestedName ? String(info.suggestedName) : ""
        refreshValidation()
        open()
    }

    function submit(): void {
        if (!canSubmit || targetUid.length === 0)
            return
        var res = CircuitCanvas.createFirmware(targetUid, dirInput.text, nameInput.text)
        if (res && res.ok) {
            accept()
        } else {
            submitError = (res && res.error) ? String(res.error) : I18n.tr("Failed to create firmware.")
        }
    }

    FolderDialog {
        id: folderDialog
        title: I18n.tr("Select Firmware Directory")
        currentFolder: App.suggestFolderUrl(dirInput.text || CircuitCanvas.filePath)
        onAccepted: {
            dirInput.text = App.pathFromUrl("" + selectedFolder)
            root.refreshValidation()
        }
    }

    background: Rectangle {
        color: appTheme.window
        border.color: appTheme.mid
        border.width: 1
        radius: 14
    }

    Overlay.modal: Rectangle {
        color: Qt.rgba(0, 0, 0, 0.45)
    }

    onOpened: {
        nameInput.forceActiveFocus()
        nameInput.selectAll()
    }

    Shortcut {
        enabled: root.visible
        sequences: ["Return", "Enter"]
        onActivated: root.submit()
    }

    contentItem: Item {
        implicitWidth: root.width
        implicitHeight: dialogCol.implicitHeight + 32

        ColumnLayout {
            id: dialogCol
            anchors.fill: parent
            anchors.margins: 16
            spacing: 12

            // Header
            RowLayout {
                Layout.fillWidth: true
                spacing: 10

                Rectangle {
                    implicitWidth: 28
                    implicitHeight: 28
                    radius: height / 2
                    color: Qt.rgba(appTheme.highlight.r, appTheme.highlight.g, appTheme.highlight.b, 0.18)

                    AppIcon {
                        anchors.centerIn: parent
                        icon: "note_add"
                        color: appTheme.highlight
                        size: 16
                    }
                }

                AppText {
                    text: I18n.tr("Create Firmware")
                    font.pixelSize: App.fontTitle
                    font.bold: true
                    color: appTheme.windowText
                    Layout.fillWidth: true
                    elide: Text.ElideRight
                }

                AppToolButton {
                    implicitWidth: 24
                    implicitHeight: 24
                    padding: 0
                    contentItem: AppIcon {
                        icon: "close"
                        size: 16
                        color: appTheme.windowText
                        opacity: 0.65
                    }
                    onClicked: root.close()
                }
            }

            // Target MCU / preset pill
            Rectangle {
                visible: root.presetSummary.length > 0
                Layout.fillWidth: true
                implicitHeight: Math.max(28, presetRow.implicitHeight + 10)
                radius: 8
                color: Qt.rgba(appTheme.mid.r, appTheme.mid.g, appTheme.mid.b, 0.16)
                border.color: Qt.rgba(appTheme.mid.r, appTheme.mid.g, appTheme.mid.b, 0.32)
                border.width: 1

                RowLayout {
                    id: presetRow
                    anchors.fill: parent
                    anchors.leftMargin: 8
                    anchors.rightMargin: 8
                    spacing: 6

                    AppIcon {
                        icon: "developer_board"
                        size: 16
                        color: appTheme.windowText
                        opacity: 0.7
                    }

                    AppText {
                        Layout.fillWidth: true
                        font.pixelSize: App.fontSmall
                        color: appTheme.windowText
                        opacity: 0.85
                        wrapMode: Text.WordWrap
                        text: root.presetSummary
                    }
                }
            }

            // Firmware name input
            ColumnLayout {
                Layout.fillWidth: true
                spacing: 5

                AppText {
                    text: I18n.tr("Firmware name")
                    font.bold: true
                    color: appTheme.windowText
                    opacity: 0.85
                }

                Rectangle {
                    Layout.fillWidth: true
                    implicitHeight: 34
                    radius: 8
                    color: appTheme.base
                    border.width: nameInput.activeFocus ? 2 : 1
                    border.color: nameInput.activeFocus ? appTheme.highlight : appTheme.mid

                    RowLayout {
                        anchors.fill: parent
                        anchors.leftMargin: 8
                        anchors.rightMargin: 6
                        spacing: 6

                        AppIcon {
                            icon: "code"
                            size: 16
                            color: nameInput.text.trim().length > 0 ? appTheme.highlight : appTheme.windowText
                            opacity: nameInput.text.trim().length > 0 ? 1.0 : 0.45
                        }

                        AppTextField {
                            id: nameInput
                            Layout.fillWidth: true
                            Layout.fillHeight: true
                            placeholderText: I18n.tr("e.g. blink")
                            selectByMouse: true
                            background: null
                            padding: 0
                            leftPadding: 2
                            rightPadding: 2
                            verticalAlignment: Text.AlignVCenter
                            color: appTheme.windowText
                            placeholderTextColor: Qt.rgba(appTheme.windowText.r, appTheme.windowText.g, appTheme.windowText.b, 0.4)
                            onTextChanged: root.refreshValidation()
                            onAccepted: root.submit()
                        }

                        AppText {
                            text: "." + root.fileExt
                            font.pixelSize: App.fontSmall
                            color: appTheme.windowText
                            opacity: 0.55
                        }

                        AppToolButton {
                            visible: nameInput.text.length > 0
                            implicitWidth: 20
                            implicitHeight: 20
                            padding: 0
                            contentItem: AppIcon {
                                icon: "close"
                                size: 16
                                color: appTheme.windowText
                                opacity: 0.6
                            }
                            onClicked: {
                                nameInput.text = ""
                                nameInput.forceActiveFocus()
                            }
                        }
                    }
                }
            }

            // Save directory input + Browse button
            ColumnLayout {
                Layout.fillWidth: true
                spacing: 5

                AppText {
                    text: I18n.tr("Directory")
                    font.bold: true
                    color: appTheme.windowText
                    opacity: 0.85
                }

                RowLayout {
                    Layout.fillWidth: true
                    spacing: 6

                    Rectangle {
                        Layout.fillWidth: true
                        implicitHeight: 34
                        radius: 8
                        color: appTheme.base
                        border.width: dirInput.activeFocus ? 2 : 1
                        border.color: dirInput.activeFocus ? appTheme.highlight : appTheme.mid

                        RowLayout {
                            anchors.fill: parent
                            anchors.leftMargin: 8
                            anchors.rightMargin: 6
                            spacing: 6

                            AppIcon {
                                icon: "folder_open"
                                size: 16
                                color: dirInput.text.trim().length > 0 ? appTheme.highlight : appTheme.windowText
                                opacity: dirInput.text.trim().length > 0 ? 1.0 : 0.45
                            }

                            AppTextField {
                                id: dirInput
                                Layout.fillWidth: true
                                Layout.fillHeight: true
                                placeholderText: I18n.tr("Choose directory")
                                selectByMouse: true
                                background: null
                                padding: 0
                                leftPadding: 2
                                rightPadding: 2
                                verticalAlignment: Text.AlignVCenter
                                color: appTheme.windowText
                                placeholderTextColor: Qt.rgba(appTheme.windowText.r, appTheme.windowText.g, appTheme.windowText.b, 0.4)
                                onTextChanged: root.refreshValidation()
                                onAccepted: root.submit()
                            }
                        }
                    }

                    AppToolButton {
                        implicitWidth: 34
                        implicitHeight: 34
                        ToolTip.text: I18n.tr("Browse")
                        ToolTip.visible: hovered
                        contentItem: AppIcon {
                            icon: "folder_open"
                            size: 16
                            color: appTheme.windowText
                        }
                        onClicked: folderDialog.open()
                    }
                }
            }

            // Live file path preview
            AppText {
                visible: root.canSubmit && root.previewPath.length > 0
                Layout.fillWidth: true
                font.pixelSize: App.fontSmall
                color: appTheme.windowText
                opacity: 0.65
                wrapMode: Text.WrapAnywhere
                text: I18n.tr("Creates: %1").arg(root.previewPath)
            }

            // Validation or creation error notice
            Rectangle {
                visible: root.displayError.length > 0 && (nameInput.text.length > 0 || dirInput.text.length > 0 || root.submitError.length > 0)
                Layout.fillWidth: true
                implicitHeight: Math.max(28, errRow.implicitHeight + 10)
                radius: 8
                color: Qt.alpha(CircuitCanvas.msgErrorBg || appTheme.highlight, 0.16)
                border.color: Qt.alpha(CircuitCanvas.msgErrorBg || appTheme.highlight, 0.45)
                border.width: 1

                RowLayout {
                    id: errRow
                    anchors.fill: parent
                    anchors.leftMargin: 8
                    anchors.rightMargin: 8
                    spacing: 6

                    AppIcon {
                        icon: "warning"
                        size: 16
                        color: CircuitCanvas.msgErrorText || appTheme.windowText
                    }

                    AppText {
                        Layout.fillWidth: true
                        font.pixelSize: App.fontSmall
                        color: appTheme.windowText
                        wrapMode: Text.WordWrap
                        text: root.displayError
                    }
                }
            }

            // Action buttons
            RowLayout {
                Layout.fillWidth: true
                Layout.topMargin: 2
                spacing: 8

                Item { Layout.fillWidth: true }

                AppButton {
                    text: I18n.tr("Cancel")
                    Layout.minimumHeight: 28
                    leftPadding: 14
                    rightPadding: 14
                    onClicked: root.close()
                }

                AppButton {
                    id: createBtn
                    text: I18n.tr("Create")
                    highlighted: true
                    enabled: root.canSubmit
                    font.bold: true
                    Layout.minimumHeight: 28
                    leftPadding: 16
                    rightPadding: 16
                    onClicked: root.submit()
                }
            }
        }
    }
}
