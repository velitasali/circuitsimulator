import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import cs_app

/* File Settings — C++ CodeEditor::fileProps() / PropDialog.
 * Compiler-specific fields live in CompilerSettings.qml. */
Rectangle {
    id: root
    SystemPalette { id: appTheme }
    function tr(s) { return App.i18nTick >= 0 ? App.translate(s) : s }
    readonly property font uiFont: Qt.font({ family: App.fontFamily || "Ubuntu", pixelSize: App.fontSize || 13 })
    implicitWidth: Math.max(420, mainCol.implicitWidth + 24)
    implicitHeight: mainCol.implicitHeight + 24
    color: appTheme.window

    component FieldLabel: Text {
        font: root.uiFont
        color: appTheme.windowText
        verticalAlignment: Text.AlignVCenter
        Layout.preferredWidth: 150
        elide: Text.ElideRight
    }

    ColumnLayout {
        id: mainCol
        anchors.fill: parent
        anchors.margins: 12
        spacing: 10

        Text {
            text: root.tr("Type: CodeEditor")
            font.family: root.uiFont.family
            font.pixelSize: 15
            font.bold: true
            color: appTheme.windowText
            Layout.fillWidth: true
            elide: Text.ElideRight
        }

        Rectangle {
            Layout.fillWidth: true
            implicitWidth: groupCol.implicitWidth + 28
            implicitHeight: groupCol.implicitHeight + 28
            radius: 8
            color: appTheme.base
            border.color: appTheme.mid
            border.width: 1

            ColumnLayout {
                id: groupCol
                x: 14
                y: 14
                width: parent.width - 28
                spacing: 10

                RowLayout {
                    Layout.fillWidth: true
                    spacing: 6

                    FieldLabel { text: root.tr("Compiler") }
                    AppComboBox {
                        id: compilerBox
                        font: root.uiFont
                        Layout.fillWidth: true
                        Layout.preferredWidth: 200
                        model: EditorPanel.compilerNames
                        onActivated: EditorPanel.compilerName = currentText
                        function syncIndex() {
                            var names = EditorPanel.compilerNames
                            var want = EditorPanel.compilerName
                            for (var i = 0; i < names.length; i++) {
                                if (names[i] === want) { currentIndex = i; return }
                            }
                            currentIndex = 0
                        }
                        Connections {
                            target: EditorPanel
                            function onFileSettingsChanged() { compilerBox.syncIndex() }
                        }
                        Component.onCompleted: syncIndex()
                    }
                    AppToolButton {
                        enabled: compilerBox.currentText.length > 0 && compilerBox.currentText !== "None"
                        implicitWidth: 26
                        implicitHeight: 26
                        ToolTip.text: enabled
                                      ? root.tr("%1 Settings").arg(compilerBox.currentText)
                                      : root.tr("Compiler Settings")
                        ToolTip.visible: hovered
                        contentItem: AppIcon {
                            text: "settings"
                            color: appTheme.windowText
                            opacity: parent.enabled ? 1.0 : 0.4
                        }
                        onClicked: EditorPanel.compilerProps()
                    }
                }

                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 0
                    Rectangle {
                        Layout.fillWidth: true
                        Layout.topMargin: 8
                        Layout.bottomMargin: 6
                        Layout.preferredHeight: 1
                        height: 1
                        color: appTheme.mid
                    }
                    Text {
                        text: root.tr("Actions after opening this file:")
                        font.family: root.uiFont.family
                        font.pixelSize: 13
                        font.bold: true
                        color: appTheme.windowText
                    }
                }

                AppCheckBox {
                    text: root.tr("Load Compiler")
                    font: root.uiFont
                    checked: EditorPanel.loadCompiler
                    onToggled: EditorPanel.loadCompiler = checked
                }
                AppCheckBox {
                    text: root.tr("Load Breakpoints")
                    font: root.uiFont
                    checked: EditorPanel.loadBreakp
                    onToggled: EditorPanel.loadBreakp = checked
                }
                AppCheckBox {
                    text: root.tr("Restore files")
                    font: root.uiFont
                    checked: EditorPanel.openFiles
                    onToggled: EditorPanel.openFiles = checked
                }
            }
        }
    }
}
