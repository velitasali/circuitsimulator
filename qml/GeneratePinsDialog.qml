import QtQuick
import QtQuick.Layouts
import cs_app
import cs_ui

Rectangle {
    id: root
    implicitWidth: 360
    implicitHeight: mainCol.implicitHeight + 24
    color: appTheme.window

    SystemPalette { id: appTheme }
    readonly property font uiFont: Qt.font({ family: App.fontFamily, pixelSize: App.fontNormal })

    property string uid: ""

    signal accepted()
    signal rejected()

    ColumnLayout {
        id: mainCol
        anchors.fill: parent
        anchors.margins: 12
        spacing: 10

        AppText {
            text: I18n.tr("Generate Package Pins")
            font.pixelSize: App.fontTitle
            font.bold: true
            color: appTheme.windowText
            Layout.fillWidth: true
        }

        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 1
            color: appTheme.mid
        }

        // Left pins
        RowLayout {
            Layout.fillWidth: true
            spacing: 8
            AppText {
                text: I18n.tr("Left Pins:")
                color: appTheme.windowText
                Layout.preferredWidth: 110
            }
            AppSpinBox {
                id: leftSpin
                font: root.uiFont
                from: 0
                to: 128
                value: 4
                Layout.fillWidth: true
            }
        }

        // Right pins
        RowLayout {
            Layout.fillWidth: true
            spacing: 8
            AppText {
                text: I18n.tr("Right Pins:")
                color: appTheme.windowText
                Layout.preferredWidth: 110
            }
            AppSpinBox {
                id: rightSpin
                font: root.uiFont
                from: 0
                to: 128
                value: 4
                Layout.fillWidth: true
            }
        }

        // Top pins
        RowLayout {
            Layout.fillWidth: true
            spacing: 8
            AppText {
                text: I18n.tr("Top Pins:")
                color: appTheme.windowText
                Layout.preferredWidth: 110
            }
            AppSpinBox {
                id: topSpin
                font: root.uiFont
                from: 0
                to: 128
                value: 0
                Layout.fillWidth: true
            }
        }

        // Bottom pins
        RowLayout {
            Layout.fillWidth: true
            spacing: 8
            AppText {
                text: I18n.tr("Bottom Pins:")
                color: appTheme.windowText
                Layout.preferredWidth: 110
            }
            AppSpinBox {
                id: bottomSpin
                font: root.uiFont
                from: 0
                to: 128
                value: 0
                Layout.fillWidth: true
            }
        }

        // Prefix
        RowLayout {
            Layout.fillWidth: true
            spacing: 8
            AppText {
                text: I18n.tr("Pin Prefix:")
                color: appTheme.windowText
                Layout.preferredWidth: 110
            }
            AppTextField {
                id: prefixField
                font: root.uiFont
                text: "pin"
                Layout.fillWidth: true
            }
        }

        // Start index
        RowLayout {
            Layout.fillWidth: true
            spacing: 8
            AppText {
                text: I18n.tr("Start Index:")
                color: appTheme.windowText
                Layout.preferredWidth: 110
            }
            AppSpinBox {
                id: startSpin
                font: root.uiFont
                from: 0
                to: 1000
                value: 1
                Layout.fillWidth: true
            }
        }

        // Clear existing pins
        AppCheckBox {
            id: clearCheck
            text: I18n.tr("Clear existing pins")
            checked: true
            font: root.uiFont
            Layout.fillWidth: true
        }

        Item { Layout.preferredHeight: 6 }

        // Buttons
        RowLayout {
            Layout.fillWidth: true
            spacing: 8

            Item { Layout.fillWidth: true }

            AppButton {
                text: I18n.tr("Cancel")
                font: root.uiFont
                onClicked: root.rejected()
            }

            AppButton {
                text: I18n.tr("Generate")
                font: root.uiFont
                highlighted: true
                onClicked: {
                    if (root.uid) {
                        CircuitCanvas.generatePackagePins(
                            root.uid,
                            leftSpin.value,
                            rightSpin.value,
                            topSpin.value,
                            bottomSpin.value,
                            prefixField.text,
                            startSpin.value,
                            clearCheck.checked
                        )
                        root.accepted()
                    }
                }
            }
        }
    }
}
