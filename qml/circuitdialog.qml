import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import cs_app

// Per-circuit settings. Each setting is a control plus an optional "Use app
// default" button, which shows only while the value differs from the app default
// and is labelled with the value it would restore - both driven by this setting's
// entry in CircuitCanvas.resets.
Rectangle {
    id: root
    SystemPalette { id: appTheme }
    readonly property font uiFont: Qt.font({ family: App.fontFamily || "Ubuntu", pixelSize: App.fontSize || 13 })
    function tr(s) { return App.i18nTick >= 0 ? App.translate(s) : s }
    implicitWidth: Math.max(360, contentCol.implicitWidth + 68)
    implicitHeight: mainCol.implicitHeight + 24
    color: appTheme.window

    component Header: ColumnLayout {
        id: headerRoot
        property alias text: headerText.text
        property bool first: false
        Layout.fillWidth: true
        spacing: 0

        Rectangle {
            visible: !headerRoot.first
            Layout.fillWidth: true
            Layout.topMargin: 12
            Layout.bottomMargin: 8
            Layout.preferredHeight: 1
            color: appTheme.mid
        }
        Text {
            id: headerText
            font.family: root.uiFont.family
            font.pixelSize: 14
            font.bold: true
            color: appTheme.windowText
        }
    }

    // Label hard left, editors hard right, slack in between - so every row's
    // inputs line up down the right edge.
    component Row_: RowLayout {
        default property alias content: holder.data
        property alias label: rowLabel.text
        Layout.fillWidth: true
        spacing: 8
        Text {
            id: rowLabel
            font: root.uiFont
            color: appTheme.windowText
            verticalAlignment: Text.AlignVCenter
        }
        Item { Layout.fillWidth: true }
        RowLayout { id: holder; spacing: 6 }
    }

    component ResetButton: AppButton {
        id: resetBtn
        required property string settingKey
        readonly property var info: CircuitCanvas.resets[settingKey]
        visible: info !== undefined && !info.isDefault
        flat: true
        font: root.uiFont
        text: "↩  " + root.tr( "Use app default" ) + ( info !== undefined ? " (" + info.text + ")" : "" )
        Layout.fillWidth: true
        Layout.alignment: Qt.AlignLeft
        horizontalPadding: 0
        contentItem: Text {
            text: resetBtn.text
            font: root.uiFont
            color: appTheme.highlight
            horizontalAlignment: Text.AlignLeft
            verticalAlignment: Text.AlignVCenter
        }
        background: Item {}
        onClicked: CircuitCanvas.restoreDefault( settingKey )
    }

    component Spin: AppSpinBox {
        editable: true
        font: root.uiFont
        Layout.preferredWidth: 140
    }

    ScrollView {
        id: rootScroll
        anchors.fill: parent
        clip: true
        contentWidth: availableWidth
        contentHeight: mainCol.implicitHeight + 24
        ScrollBar.vertical.policy: ScrollBar.AsNeeded

        ColumnLayout {
            id: mainCol
            width: rootScroll.availableWidth - 24
            x: 12
            y: 12
            spacing: 8

            Text {
                text: root.tr( "These settings are saved inside the circuit file." )
                font: root.uiFont
                color: Qt.rgba( appTheme.windowText.r, appTheme.windowText.g,
                                appTheme.windowText.b, 0.6 )
                wrapMode: Text.WordWrap
                Layout.fillWidth: true
            }

            // Rounded container card holding circuit settings
            Rectangle {
                id: container
                Layout.fillWidth: true
                implicitWidth: contentCol.implicitWidth + 28
                implicitHeight: contentCol.implicitHeight + 28
                radius: 8
                color: "transparent"
                border.color: appTheme.mid
                border.width: 1

                ColumnLayout {
                    id: contentCol
                    anchors.fill: parent
                    anchors.margins: 14
                    spacing: 8

                    Header { text: root.tr( "Canvas" ); first: true }

                    Row_ {
                        label: root.tr( "Width in pixels" )
                        Spin { from: 1; to: 10000; value: CircuitCanvas.sceneWidth
                               onValueModified: CircuitCanvas.sceneWidth = value
                               Binding on value { value: CircuitCanvas.sceneWidth } }
                    }
                    ResetButton { settingKey: "width" }

                    Row_ {
                        label: root.tr( "Height in pixels" )
                        Spin { from: 1; to: 10000; value: CircuitCanvas.sceneHeight
                               onValueModified: CircuitCanvas.sceneHeight = value
                               Binding on value { value: CircuitCanvas.sceneHeight } }
                    }
                    ResetButton { settingKey: "height" }

                    Header { text: root.tr( "Animations" ) }

                    AppCheckBox {
                        text: root.tr( "Animate Logic" )
                        font: root.uiFont
                        checked: CircuitCanvas.animateLogic
                        onToggled: CircuitCanvas.animateLogic = checked
                        Binding on checked { value: CircuitCanvas.animateLogic }
                    }
                    ResetButton { settingKey: "animLogic" }

                    AppCheckBox {
                        text: root.tr( "Animate Current Flow" )
                        font: root.uiFont
                        checked: CircuitCanvas.animateCurr
                        onToggled: CircuitCanvas.animateCurr = checked
                        Binding on checked { value: CircuitCanvas.animateCurr }
                    }
                    ResetButton { settingKey: "animCurr" }

                    Header { text: root.tr( "Components" ) }

                    AppCheckBox {
                        text: root.tr( "ANSI Symbols" )
                        font: root.uiFont
                        checked: CircuitCanvas.ansiSymbols
                        onToggled: CircuitCanvas.ansiSymbols = checked
                        Binding on checked { value: CircuitCanvas.ansiSymbols }
                    }
                    ResetButton { settingKey: "ansi" }

                    Header { text: root.tr( "Speed" ) }

                    RowLayout {
                        Layout.fillWidth: true
                        spacing: 6
                        AppSlider {
                            from: 0; to: 100
                            value: CircuitCanvas.speedPercent
                            Layout.fillWidth: true
                            onPressedChanged: {
                                if (pressed) {
                                    CircuitCanvas.beginCircSettingsEdit()
                                } else {
                                    CircuitCanvas.commitCircSettingsEdit()
                                }
                            }
                            onMoved: CircuitCanvas.speedPercent = value
                            Binding on value { value: CircuitCanvas.speedPercent }
                        }
                        Text {
                            text: CircuitCanvas.speedLabel
                            font: root.uiFont
                            color: appTheme.windowText
                            horizontalAlignment: Text.AlignRight
                            Layout.preferredWidth: 50
                        }
                    }
                    Row_ {
                        label: root.tr( "Steps per Second" )
                        Spin { from: 1; to: 999999999; value: CircuitCanvas.step
                               onValueModified: CircuitCanvas.step = value
                               Binding on value { value: CircuitCanvas.step } }
                        AppComboBox {
                            model: CircuitCanvas.timeUnits
                            currentIndex: CircuitCanvas.stepUnit
                            font: root.uiFont
                            Layout.preferredWidth: 80
                            onActivated: CircuitCanvas.stepUnit = currentIndex
                            Binding on currentIndex { value: CircuitCanvas.stepUnit }
                        }
                    }
                    ResetButton { settingKey: "speed" }

                    Header { text: root.tr( "Reactive" ) }

                    Row_ {
                        label: root.tr( "Target Step" )
                        Spin { from: 1; to: 999999999; value: CircuitCanvas.reactStep
                               onValueModified: CircuitCanvas.reactStep = value
                               Binding on value { value: CircuitCanvas.reactStep } }
                        AppComboBox {
                            model: CircuitCanvas.timeUnits
                            currentIndex: CircuitCanvas.reactStepUnit
                            font: root.uiFont
                            Layout.preferredWidth: 80
                            onActivated: CircuitCanvas.reactStepUnit = currentIndex
                            Binding on currentIndex { value: CircuitCanvas.reactStepUnit }
                        }
                    }
                    Row_ {
                        label: root.tr( "Real Step" )
                        Spin { from: 1; to: 999999999; value: CircuitCanvas.realStep; enabled: false
                               Binding on value { value: CircuitCanvas.realStep } }
                        AppComboBox {
                            model: CircuitCanvas.timeUnits
                            currentIndex: CircuitCanvas.realStepUnit
                            font: root.uiFont
                            enabled: false
                            Layout.preferredWidth: 80
                            Binding on currentIndex { value: CircuitCanvas.realStepUnit }
                        }
                    }
                    ResetButton { settingKey: "reactStep" }

                    Header { text: root.tr( "NonLinear" ) }

                    Row_ {
                        label: root.tr( "Max. Iterations" )
                        Spin { from: 0; to: 100000; value: CircuitCanvas.nlSteps
                               onValueModified: CircuitCanvas.nlSteps = value
                               Binding on value { value: CircuitCanvas.nlSteps } }
                    }
                    ResetButton { settingKey: "nlSteps" }
                }
            }
        }
    }
}
