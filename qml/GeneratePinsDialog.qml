import QtQuick
import QtQuick.Layouts
import cs_app
import cs_ui

Rectangle {
    id: root
    implicitWidth: Math.max(360, mainCol.implicitWidth + 24)
    implicitHeight: mainCol.implicitHeight + 24
    color: appTheme.window

    SystemPalette { id: appTheme }
    readonly property font uiFont: Qt.font({ family: App.fontFamily, pixelSize: App.fontNormal })

    property string uid: ""

    signal accepted()
    signal rejected()

    function commitSpin(spin: AppSpinBox): void {
        if (!spin)
            return
        var input = spin.contentItem as TextInput
        if (!input)
            return
        var parsed = spin.valueFromText(input.text, spin.locale)
        spin.value = Math.max(spin.from, Math.min(spin.to, parsed))
    }

    function focusFirstField(): void {
        var input = leftSpin.contentItem as TextInput
        if (input) {
            input.forceActiveFocus()
            input.selectAll()
        } else {
            leftSpin.forceActiveFocus()
        }
    }

    function generateAndClose(): void {
        if (!root.uid)
            return
        commitSpin(leftSpin)
        commitSpin(rightSpin)
        commitSpin(topSpin)
        commitSpin(bottomSpin)
        commitSpin(startSpin)
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

    Shortcut {
        sequences: [ "Return", "Enter" ]
        enabled: root.visible
        onActivated: root.generateAndClose()
    }

    ColumnLayout {
        id: mainCol
        anchors.fill: parent
        anchors.margins: 12
        spacing: 10

        Rectangle {
            Layout.fillWidth: true
            implicitWidth: formCol.implicitWidth + 24
            implicitHeight: formCol.implicitHeight + 24
            radius: 12
            color: appTheme.base
            border.color: appTheme.mid
            border.width: 1

            ColumnLayout {
                id: formCol
                anchors.fill: parent
                anchors.margins: 12
                spacing: 10

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
                        editable: true
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
                        editable: true
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
                        editable: true
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
                        editable: true
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
                        onAccepted: root.generateAndClose()
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
                        editable: true
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
            }
        }

        // Buttons
        RowLayout {
            Layout.fillWidth: true
            spacing: 8

            Item { Layout.fillWidth: true }

            AppButton {
                id: cancelBtn
                text: I18n.tr("Cancel")
                font: root.uiFont
                onClicked: root.rejected()
            }

            AppButton {
                id: generateBtn
                text: I18n.tr("Generate")
                font: root.uiFont
                highlighted: true
                onClicked: root.generateAndClose()
            }
        }
    }
}
