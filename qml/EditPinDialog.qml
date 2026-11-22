import QtQuick
import QtQuick.Layouts
import cs_app
import cs_ui

Rectangle {
    id: root
    implicitWidth: Math.max(380, mainCol.implicitWidth + 24)
    implicitHeight: mainCol.implicitHeight + 24
    color: appTheme.window

    SystemPalette { id: appTheme }
    readonly property font uiFont: Qt.font({ family: App.fontFamily, pixelSize: App.fontNormal })

    property string uid: ""
    property string originalPinId: ""
    property alias pinId: idField.text
    property alias pinLabel: labelField.text
    property string pinType: ""
    property int pinAngle: 180
    property int pinLength: 8
    property int pinSpace: 0
    property int pinX: 0
    property int pinY: 0

    signal accepted()
    signal deleted()
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
        idField.forceActiveFocus()
        idField.selectAll()
    }

    function loadPin(itemUid: string, pId: string): void {
        root.uid = itemUid
        root.originalPinId = pId
        const data = CircuitCanvas.getPackagePinData(itemUid, pId)
        if (data && data.id !== undefined) {
            idField.text = data.id || ""
            labelField.text = data.label || ""
            root.pinType = data.type || ""
            typeCombo.currentIndex = typeValues.indexOf(root.pinType) >= 0 ? typeValues.indexOf(root.pinType) : 0
            root.pinAngle = data.angle !== undefined ? data.angle : 0
            angleCombo.currentIndex = angleValues.indexOf(root.pinAngle) >= 0 ? angleValues.indexOf(root.pinAngle) : 0
            root.pinLength = data.length !== undefined ? data.length : 8
            lengthCombo.currentIndex = root.pinLength <= 1 ? 1 : 0
            spaceSpin.value = data.space || 0
            xSpin.value = data.xpos || 0
            ySpin.value = data.ypos || 0
        }
    }

    function saveAndClose(): void {
        if (!root.uid || !root.originalPinId)
            return
        commitSpin(spaceSpin)
        commitSpin(xSpin)
        commitSpin(ySpin)
        var newId = idField.text.trim() || root.originalPinId
        var newLabel = labelField.text.trim() || newId
        var ptype = root.typeValues[typeCombo.currentIndex] || ""
        var angle = root.angleValues[angleCombo.currentIndex]
        var length = lengthCombo.currentIndex === 1 ? 1 : 8
        CircuitCanvas.updatePackagePin(
            root.uid,
            root.originalPinId,
            newId,
            newLabel,
            ptype,
            xSpin.value,
            ySpin.value,
            angle,
            length,
            spaceSpin.value
        )
        root.accepted()
    }

    function deleteAndClose(): void {
        if (root.uid && root.originalPinId) {
            CircuitCanvas.removePackagePin(root.uid, root.originalPinId)
            root.deleted()
        }
    }

    Shortcut {
        sequences: [ "Return", "Enter" ]
        enabled: root.visible
                 && !typeCombo.popup.visible
                 && !angleCombo.popup.visible
                 && !lengthCombo.popup.visible
        onActivated: root.saveAndClose()
    }

    readonly property list<string> typeValues: ["", "inv", "nc", "bus", "nul", "rst"]
    readonly property list<string> typeNames: [
        I18n.tr("Normal"),
        I18n.tr("Inverted (inv)"),
        I18n.tr("Unused (nc)"),
        I18n.tr("Bus"),
        I18n.tr("Null"),
        I18n.tr("Reset")
    ]

    readonly property list<int> angleValues: [0, 90, 180, 270]
    readonly property list<string> angleNames: [
        I18n.tr("Right (0°)"),
        I18n.tr("Top (90°)"),
        I18n.tr("Left (180°)"),
        I18n.tr("Bottom (270°)")
    ]

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

                // Pin ID
                RowLayout {
                    Layout.fillWidth: true
                    spacing: 8
                    AppText {
                        text: I18n.tr("Pin ID:")
                        color: appTheme.windowText
                        Layout.preferredWidth: 100
                    }
                    AppTextField {
                        id: idField
                        font: root.uiFont
                        Layout.fillWidth: true
                        placeholderText: I18n.tr("e.g. 1, VCC, A0")
                        onAccepted: root.saveAndClose()
                    }
                }

                // Pin Label
                RowLayout {
                    Layout.fillWidth: true
                    spacing: 8
                    AppText {
                        text: I18n.tr("Label:")
                        color: appTheme.windowText
                        Layout.preferredWidth: 100
                    }
                    AppTextField {
                        id: labelField
                        font: root.uiFont
                        Layout.fillWidth: true
                        placeholderText: I18n.tr("Display label")
                        onAccepted: root.saveAndClose()
                    }
                }

                // Pin Type
                RowLayout {
                    Layout.fillWidth: true
                    spacing: 8
                    AppText {
                        text: I18n.tr("Type:")
                        color: appTheme.windowText
                        Layout.preferredWidth: 100
                    }
                    AppComboBox {
                        id: typeCombo
                        font: root.uiFont
                        Layout.fillWidth: true
                        model: root.typeNames
                        onActivated: function(index) {
                            root.pinType = root.typeValues[index]
                        }
                    }
                }

                // Pin Angle / Orientation
                RowLayout {
                    Layout.fillWidth: true
                    spacing: 8
                    AppText {
                        text: I18n.tr("Orientation:")
                        color: appTheme.windowText
                        Layout.preferredWidth: 100
                    }
                    AppComboBox {
                        id: angleCombo
                        font: root.uiFont
                        Layout.fillWidth: true
                        model: root.angleNames
                        onActivated: function(index) {
                            root.pinAngle = root.angleValues[index]
                        }
                    }
                }

                // Pin Length
                RowLayout {
                    Layout.fillWidth: true
                    spacing: 8
                    AppText {
                        text: I18n.tr("Style:")
                        color: appTheme.windowText
                        Layout.preferredWidth: 100
                    }
                    AppComboBox {
                        id: lengthCombo
                        font: root.uiFont
                        Layout.fillWidth: true
                        model: [I18n.tr("Standard (8 px)"), I18n.tr("Point (1 px)")]
                        onActivated: function(index) {
                            root.pinLength = index === 1 ? 1 : 8
                        }
                    }
                }

                // Space
                RowLayout {
                    Layout.fillWidth: true
                    spacing: 8
                    AppText {
                        text: I18n.tr("Space:")
                        color: appTheme.windowText
                        Layout.preferredWidth: 100
                    }
                    AppSpinBox {
                        id: spaceSpin
                        editable: true
                        font: root.uiFont
                        from: 0
                        to: 128
                        stepSize: 1
                        Layout.fillWidth: true
                    }
                }

                // Position X & Y
                RowLayout {
                    Layout.fillWidth: true
                    spacing: 8
                    AppText {
                        text: I18n.tr("Position (X, Y):")
                        color: appTheme.windowText
                        Layout.preferredWidth: 100
                    }
                    AppSpinBox {
                        id: xSpin
                        editable: true
                        font: root.uiFont
                        from: -500
                        to: 500
                        stepSize: 8
                        Layout.fillWidth: true
                    }
                    AppSpinBox {
                        id: ySpin
                        editable: true
                        font: root.uiFont
                        from: -500
                        to: 500
                        stepSize: 8
                        Layout.fillWidth: true
                    }
                }
            }
        }

        // Buttons
        RowLayout {
            Layout.fillWidth: true
            spacing: 8

            AppButton {
                id: deleteBtn
                text: I18n.tr("Delete Pin")
                font: root.uiFont
                palette.buttonText: CircuitCanvas.msgErrorBg || appTheme.windowText
                onClicked: root.deleteAndClose()
            }

            Item { Layout.fillWidth: true }

            AppButton {
                id: cancelBtn
                text: I18n.tr("Cancel")
                font: root.uiFont
                onClicked: root.rejected()
            }

            AppButton {
                id: saveBtn
                text: I18n.tr("Save")
                font: root.uiFont
                highlighted: true
                onClicked: root.saveAndClose()
            }
        }
    }
}
