import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import cs_app

Rectangle {
    id: root
    function tr(s) { return App.i18nTick >= 0 ? App.translate(s) : s }
    implicitWidth: 380
    implicitHeight: mainCol.implicitHeight + 24
    color: appTheme.window

    SystemPalette { id: appTheme }
    readonly property font uiFont: Qt.font({ family: App.fontFamily || "Ubuntu", pixelSize: App.fontSize || 13 })

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

    function loadPin(itemUid, pId) {
        root.uid = itemUid
        root.originalPinId = pId
        var data = CircuitCanvas.getPackagePinData(itemUid, pId)
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

    readonly property var typeValues: ["", "inv", "nc", "bus", "nul", "rst"]
    readonly property var typeNames: [
        root.tr("Normal"),
        root.tr("Inverted (inv)"),
        root.tr("Unused (nc)"),
        root.tr("Bus"),
        root.tr("Null"),
        root.tr("Reset")
    ]

    readonly property var angleValues: [0, 90, 180, 270]
    readonly property var angleNames: [
        root.tr("Right (0°)"),
        root.tr("Top (90°)"),
        root.tr("Left (180°)"),
        root.tr("Bottom (270°)")
    ]

    ColumnLayout {
        id: mainCol
        anchors.fill: parent
        anchors.margins: 12
        spacing: 10

        Text {
            text: root.tr("Edit Package Pin")
            font.family: root.uiFont.family
            font.pixelSize: 15
            font.bold: true
            color: appTheme.windowText
            Layout.fillWidth: true
        }

        Rectangle {
            Layout.fillWidth: true
            height: 1
            color: appTheme.mid
        }

        // Pin ID
        RowLayout {
            Layout.fillWidth: true
            spacing: 8
            Text {
                text: root.tr("Pin ID:")
                font: root.uiFont
                color: appTheme.windowText
                Layout.preferredWidth: 100
            }
            AppTextField {
                id: idField
                font: root.uiFont
                Layout.fillWidth: true
                placeholderText: root.tr("e.g. 1, VCC, A0")
            }
        }

        // Pin Label
        RowLayout {
            Layout.fillWidth: true
            spacing: 8
            Text {
                text: root.tr("Label:")
                font: root.uiFont
                color: appTheme.windowText
                Layout.preferredWidth: 100
            }
            AppTextField {
                id: labelField
                font: root.uiFont
                Layout.fillWidth: true
                placeholderText: root.tr("Display label")
            }
        }

        // Pin Type
        RowLayout {
            Layout.fillWidth: true
            spacing: 8
            Text {
                text: root.tr("Type:")
                font: root.uiFont
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
            Text {
                text: root.tr("Orientation:")
                font: root.uiFont
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
            Text {
                text: root.tr("Style:")
                font: root.uiFont
                color: appTheme.windowText
                Layout.preferredWidth: 100
            }
            AppComboBox {
                id: lengthCombo
                font: root.uiFont
                Layout.fillWidth: true
                model: [root.tr("Standard (8 px)"), root.tr("Point (1 px)")]
                onActivated: function(index) {
                    root.pinLength = index === 1 ? 1 : 8
                }
            }
        }

        // Space
        RowLayout {
            Layout.fillWidth: true
            spacing: 8
            Text {
                text: root.tr("Space:")
                font: root.uiFont
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
            Text {
                text: root.tr("Position (X, Y):")
                font: root.uiFont
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
                text: root.tr("Delete Pin")
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
                text: root.tr("Cancel")
                font: root.uiFont
                onClicked: root.rejected()
            }

            AppButton {
                text: root.tr("Save")
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
