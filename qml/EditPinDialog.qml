import QtQuick
import QtQuick.Layouts
import cs_app
import cs_ui

Rectangle {
    id: root
    implicitWidth: 380
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

    function loadPin(itemUid: string, pId: string): void {
        root.uid = itemUid
        root.originalPinId = pId
        const data = CircuitCanvas.getPackagePinData(itemUid, pId)
        if (data && data.id !== undefined) {
            idField.text = data.id || ""
            labelField.text = data.label || ""
            root.pinType = data.type || ""
            typeCombo.currentIndex = typeValues.indexOf(root.pinType) >= 0 ? typeValues.indexOf(root.pinType) : 0
            root.pinAngle = data.angle || 0
            angleCombo.currentIndex = angleValues.indexOf(root.pinAngle) >= 0 ? angleValues.indexOf(root.pinAngle) : 0
            root.pinLength = data.length || 8
            lengthCombo.currentIndex = root.pinLength <= 1 ? 1 : 0
            spaceSpin.value = data.space || 0
            xSpin.value = data.xpos || 0
            ySpin.value = data.ypos || 0
        }
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

        AppText {
            text: I18n.tr("Edit Package Pin")
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
                font: root.uiFont
                from: -500
                to: 500
                stepSize: 8
                Layout.fillWidth: true
            }
            AppSpinBox {
                id: ySpin
                font: root.uiFont
                from: -500
                to: 500
                stepSize: 8
                Layout.fillWidth: true
            }
        }

        Item { Layout.preferredHeight: 6 }

        // Buttons
        RowLayout {
            Layout.fillWidth: true
            spacing: 8

            AppButton {
                text: I18n.tr("Delete Pin")
                font: root.uiFont
                palette.buttonText: CircuitCanvas.msgErrorBg || "#d9534f"
                onClicked: {
                    if (root.uid && root.originalPinId) {
                        CircuitCanvas.removePackagePin(root.uid, root.originalPinId)
                        root.deleted()
                    }
                }
            }

            Item { Layout.fillWidth: true }

            AppButton {
                text: I18n.tr("Cancel")
                font: root.uiFont
                onClicked: root.rejected()
            }

            AppButton {
                text: I18n.tr("Save")
                font: root.uiFont
                highlighted: true
                onClicked: {
                    if (root.uid && root.originalPinId) {
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
                }
            }
        }
    }
}
