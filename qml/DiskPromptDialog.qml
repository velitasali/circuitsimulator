pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import cs_app

Dialog {
    id: root

    parent: Overlay.overlay
    modal: true
    dim: true
    padding: 0
    header: null
    footer: null
    standardButtons: Dialog.NoButton
    closePolicy: Popup.NoAutoClose
    width: Math.min(Math.max((parent ? parent.width : 420) - 32, 300), 420)
    x: parent ? Math.round((parent.width - width) / 2) : 0
    y: parent ? Math.max(40, Math.round((parent.height - height) / 2)) : 0

    SystemPalette { id: appTheme }

    property string targetPath: ""
    property bool isDeleted: false
    property string titleText: ""
    property string messageText: ""
    property string secondaryText: ""
    property string primaryText: ""

    signal primaryTriggered()
    signal secondaryTriggered()
    signal dismissed()

    readonly property color accentColor: isDeleted
        ? (CircuitCanvas.msgErrorBg || appTheme.highlight)
        : (CircuitCanvas.msgWarnBg || appTheme.highlight)

    function fileNameForPath(path: string): string {
        if (!path || path.length === 0)
            return ""
        var parts = path.replace(/\\/g, "/").split("/")
        return parts[parts.length - 1] || path
    }

    function fileIconForPath(path: string): string {
        var name = fileNameForPath(path)
        if (App.isCircuitPath(name))
            return "memory"
        var ext = name.split(".").pop().toLowerCase()
        if (ext === "c" || ext === "cpp" || ext === "h" || ext === "hpp" || ext === "ino" || ext === "rs" || ext === "py")
            return "code"
        if (ext === "asm" || ext === "s" || ext === "inc")
            return "terminal"
        if (ext === "hex" || ext === "bin")
            return "data_object"
        if (ext === "xml" || ext === "json" || ext === "txt" || ext === "md")
            return "article"
        return "description"
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

    onOpened: primaryBtn.forceActiveFocus()

    Shortcut {
        enabled: root.visible
        sequences: ["Return", "Enter"]
        onActivated: root.primaryTriggered()
    }

    Shortcut {
        enabled: root.visible
        sequences: ["Escape"]
        onActivated: root.dismissed()
    }

    contentItem: Item {
        implicitWidth: root.width
        implicitHeight: dialogCol.implicitHeight + 32

        ColumnLayout {
            id: dialogCol
            anchors.fill: parent
            anchors.margins: 16
            spacing: 12

            RowLayout {
                Layout.fillWidth: true
                spacing: 10

                Rectangle {
                    implicitWidth: 28
                    implicitHeight: 28
                    radius: height / 2
                    color: Qt.alpha(root.accentColor, 0.18)

                    AppIcon {
                        anchors.centerIn: parent
                        icon: "warning"
                        color: root.accentColor
                        size: 16
                    }
                }

                AppText {
                    text: root.titleText
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
                    onClicked: root.dismissed()
                }
            }

            Rectangle {
                Layout.fillWidth: true
                implicitHeight: Math.max(32, targetRow.implicitHeight + 12)
                radius: 8
                color: Qt.rgba(appTheme.mid.r, appTheme.mid.g, appTheme.mid.b, 0.16)
                border.color: Qt.rgba(appTheme.mid.r, appTheme.mid.g, appTheme.mid.b, 0.32)
                border.width: 1

                RowLayout {
                    id: targetRow
                    anchors.fill: parent
                    anchors.leftMargin: 8
                    anchors.rightMargin: 8
                    spacing: 8

                    AppIcon {
                        icon: root.fileIconForPath(root.targetPath)
                        size: 16
                        color: appTheme.windowText
                        opacity: 0.7
                    }

                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 1

                        AppText {
                            Layout.fillWidth: true
                            text: root.fileNameForPath(root.targetPath)
                            font.pixelSize: App.fontSmall
                            font.bold: true
                            color: appTheme.windowText
                            elide: Text.ElideMiddle
                        }

                        AppText {
                            visible: text.length > 0
                            Layout.fillWidth: true
                            text: root.targetPath
                            font.pixelSize: App.fontMicro
                            color: appTheme.windowText
                            opacity: 0.65
                            elide: Text.ElideMiddle
                        }
                    }
                }
            }

            AppText {
                Layout.fillWidth: true
                text: root.messageText
                color: appTheme.windowText
                opacity: 0.88
                wrapMode: Text.WordWrap
                lineHeight: 1.2
            }

            RowLayout {
                Layout.fillWidth: true
                Layout.topMargin: 2
                spacing: 8

                Item { Layout.fillWidth: true }

                AppButton {
                    id: secondaryBtn
                    text: root.secondaryText
                    Layout.minimumHeight: 28
                    leftPadding: 14
                    rightPadding: 14
                    onClicked: root.secondaryTriggered()
                }

                AppButton {
                    id: primaryBtn
                    text: root.primaryText
                    highlighted: true
                    font.bold: true
                    Layout.minimumHeight: 28
                    leftPadding: 16
                    rightPadding: 16
                    onClicked: root.primaryTriggered()
                }
            }
        }
    }
}
