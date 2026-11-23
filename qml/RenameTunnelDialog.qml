import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import cs_app
import cs_ui

Dialog {
    id: root
    modal: true
    dim: true
    parent: Overlay.overlay
    anchors.centerIn: parent
    padding: 0
    header: null
    footer: null
    closePolicy: Popup.CloseOnEscape
    width: Math.min(Math.max(parent ? parent.width - 32 : 360, 300), 380)

    SystemPalette { id: appTheme }

    property string targetUid: ""
    property string currentName: ""
    property int groupCount: 0
    property var existingGroups: ({})

    readonly property bool hasCurrentName: currentName.length > 0
    readonly property string trimmedName: nameInput.text.trim()
    readonly property bool canSubmit: trimmedName.length > 0 && trimmedName !== currentName
    readonly property int mergeTargetCount: {
        if (!canSubmit || !existingGroups)
            return 0
        var n = existingGroups[trimmedName]
        return n ? Number(n) : 0
    }

    function openFor(uid: string): void {
        targetUid = uid
        const info = CircuitCanvas.tunnelGroupInfo(uid)
        currentName = (info && info.name) ? String(info.name) : ""
        groupCount = (info && info.groupCount !== undefined) ? Number(info.groupCount) : 1
        existingGroups = (info && info.existingGroups) ? info.existingGroups : ({})
        nameInput.text = currentName
        open()
    }

    function submit(): void {
        if (canSubmit)
            accept()
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

    onAccepted: {
        if (targetUid.length > 0 && canSubmit)
            CircuitCanvas.renameTunnelGroup(targetUid, trimmedName)
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
                        icon: "edit"
                        color: appTheme.highlight
                        size: 16
                    }
                }

                AppText {
                    text: root.hasCurrentName ? I18n.tr("Rename Tunnel Group") : I18n.tr("Name Tunnel")
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

            // Current group context pill
            Rectangle {
                Layout.fillWidth: true
                implicitHeight: Math.max(28, contextRow.implicitHeight + 10)
                radius: 8
                color: Qt.rgba(appTheme.mid.r, appTheme.mid.g, appTheme.mid.b, 0.16)
                border.color: Qt.rgba(appTheme.mid.r, appTheme.mid.g, appTheme.mid.b, 0.32)
                border.width: 1

                RowLayout {
                    id: contextRow
                    anchors.fill: parent
                    anchors.leftMargin: 8
                    anchors.rightMargin: 8
                    spacing: 6

                    AppIcon {
                        icon: "link"
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
                        text: {
                            if (!root.hasCurrentName)
                                return I18n.tr("Unnamed tunnel — only this tunnel will be named.")
                            if (root.groupCount === 1)
                                return I18n.tr("Current group: %1 (1 tunnel)").arg(root.currentName)
                            return I18n.tr("Current group: %1 (%2 tunnels)")
                                .arg(root.currentName)
                                .arg(root.groupCount)
                        }
                    }
                }
            }

            // Input field
            ColumnLayout {
                Layout.fillWidth: true
                spacing: 5

                AppText {
                    text: root.hasCurrentName ? I18n.tr("New name") : I18n.tr("Net name")
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
                            icon: "link"
                            size: 16
                            color: root.trimmedName.length > 0 ? appTheme.highlight : appTheme.windowText
                            opacity: root.trimmedName.length > 0 ? 1.0 : 0.45
                        }

                        AppTextField {
                            id: nameInput
                            Layout.fillWidth: true
                            Layout.fillHeight: true
                            placeholderText: I18n.tr("e.g. VCC, CLK, DATA0")
                            selectByMouse: true
                            background: null
                            padding: 0
                            leftPadding: 2
                            rightPadding: 2
                            verticalAlignment: Text.AlignVCenter
                            color: appTheme.windowText
                            placeholderTextColor: Qt.rgba(appTheme.windowText.r, appTheme.windowText.g, appTheme.windowText.b, 0.4)
                            onAccepted: root.submit()
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

            // Live merge notice when target name already exists
            Rectangle {
                visible: root.mergeTargetCount > 0
                Layout.fillWidth: true
                implicitHeight: Math.max(28, mergeRow.implicitHeight + 10)
                radius: 8
                color: Qt.alpha(CircuitCanvas.msgWarnBg || appTheme.highlight, 0.16)
                border.color: Qt.alpha(CircuitCanvas.msgWarnBg || appTheme.highlight, 0.45)
                border.width: 1

                RowLayout {
                    id: mergeRow
                    anchors.fill: parent
                    anchors.leftMargin: 8
                    anchors.rightMargin: 8
                    spacing: 6

                    AppIcon {
                        icon: "info"
                        size: 16
                        color: CircuitCanvas.msgWarnBg || appTheme.highlight
                    }

                    AppText {
                        Layout.fillWidth: true
                        font.pixelSize: App.fontSmall
                        color: appTheme.windowText
                        wrapMode: Text.WordWrap
                        text: root.mergeTargetCount === 1
                            ? I18n.tr("Will merge with 1 existing tunnel in \"%1\".").arg(root.trimmedName)
                            : I18n.tr("Will merge with %1 existing tunnels in \"%2\".")
                                .arg(root.mergeTargetCount)
                                .arg(root.trimmedName)
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
                    id: confirmBtn
                    text: root.hasCurrentName ? I18n.tr("Rename") : I18n.tr("Set Name")
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
