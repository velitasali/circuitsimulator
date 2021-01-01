import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import cs_app

Dialog {
    id: root
    function tr(s) { return App.i18nTick >= 0 ? App.translate(s) : s }
    modal: true
    dim: true
    anchors.centerIn: parent
    padding: 0
    closePolicy: Popup.CloseOnEscape

    SystemPalette { id: appTheme }

    property alias dialogTitle: titleLabel.text
    property alias dialogText: messageLabel.text
    property string acceptText: root.tr( "Save" )
    property string discardText: root.tr( "Discard" )
    property string cancelText: root.tr( "Cancel" )
    property bool showDiscard: true
    property bool showCancel: true

    signal cancelled()

    background: Rectangle {
        implicitWidth: 420
        color: appTheme.window
        border.color: appTheme.mid
        border.width: 1
        radius: 10
    }

    header: null
    footer: null

    onRejected: {
        root.cancelled()
    }

    contentItem: Item {
        implicitWidth: 420
        implicitHeight: mainLayout.implicitHeight + 36

        ColumnLayout {
            id: mainLayout
            anchors.fill: parent
            anchors.margins: 18
            spacing: 14

            Text {
                id: titleLabel
                Layout.fillWidth: true
                font: Qt.font({ family: App.fontFamily || "Ubuntu", pixelSize: (App.fontSize || 13) + 2, bold: true })
                color: appTheme.windowText
                wrapMode: Text.Wrap
            }

            Text {
                id: messageLabel
                Layout.fillWidth: true
                font: Qt.font({ family: App.fontFamily || "Ubuntu", pixelSize: App.fontSize || 13 })
                color: appTheme.windowText
                wrapMode: Text.Wrap
                lineHeight: 1.2
            }

            Item { Layout.preferredHeight: 4 }

            RowLayout {
                Layout.fillWidth: true
                spacing: 8

                // Discard / Don't Save button (left side)
                AppButton {
                    id: discardBtn
                    visible: root.showDiscard
                    text: root.discardText
                    font: Qt.font({ family: App.fontFamily || "Ubuntu", pixelSize: App.fontSize || 13 })
                    padding: 6
                    leftPadding: 14
                    rightPadding: 14

                    background: Rectangle {
                        implicitHeight: 28
                        radius: 5
                        color: discardBtn.down ? appTheme.dark : (discardBtn.hovered ? appTheme.midlight : appTheme.button)
                        border.color: appTheme.mid
                        border.width: 1
                    }

                    contentItem: Text {
                        text: discardBtn.text
                        font: discardBtn.font
                        color: appTheme.buttonText
                        horizontalAlignment: Text.AlignHCenter
                        verticalAlignment: Text.AlignVCenter
                    }

                    onClicked: {
                        root.close()
                        root.discarded()
                    }
                }

                Item { Layout.fillWidth: true }

                // Cancel button
                AppButton {
                    id: cancelBtn
                    visible: root.showCancel
                    text: root.cancelText
                    font: Qt.font({ family: App.fontFamily || "Ubuntu", pixelSize: App.fontSize || 13 })
                    padding: 6
                    leftPadding: 14
                    rightPadding: 14

                    background: Rectangle {
                        implicitHeight: 28
                        radius: 5
                        color: cancelBtn.down ? appTheme.dark : (cancelBtn.hovered ? appTheme.midlight : appTheme.button)
                        border.color: appTheme.mid
                        border.width: 1
                    }

                    contentItem: Text {
                        text: cancelBtn.text
                        font: cancelBtn.font
                        color: appTheme.buttonText
                        horizontalAlignment: Text.AlignHCenter
                        verticalAlignment: Text.AlignVCenter
                    }

                    onClicked: {
                        root.close()
                        root.cancelled()
                    }
                }

                // Accept / Save button (primary)
                AppButton {
                    id: acceptBtn
                    text: root.acceptText
                    font: Qt.font({ family: App.fontFamily || "Ubuntu", pixelSize: App.fontSize || 13, bold: true })
                    padding: 6
                    leftPadding: 16
                    rightPadding: 16
                    focus: true

                    background: Rectangle {
                        implicitHeight: 28
                        radius: 5
                        color: acceptBtn.down ? Qt.darker(appTheme.highlight, 1.2) : (acceptBtn.hovered ? Qt.lighter(appTheme.highlight, 1.1) : appTheme.highlight)
                        border.color: Qt.darker(appTheme.highlight, 1.3)
                        border.width: 1
                    }

                    contentItem: Text {
                        text: acceptBtn.text
                        font: acceptBtn.font
                        color: appTheme.highlightedText
                        horizontalAlignment: Text.AlignHCenter
                        verticalAlignment: Text.AlignVCenter
                    }

                    onClicked: {
                        root.close()
                        root.accepted()
                    }
                }
            }
        }
    }

    Shortcut {
        enabled: root.visible
        sequences: ["Return", "Enter"]
        onActivated: {
            root.close()
            root.accepted()
        }
    }

    Shortcut {
        enabled: root.visible && root.showDiscard
        sequences: ["Ctrl+D", "Meta+D"]
        onActivated: {
            root.close()
            root.discarded()
        }
    }
}
