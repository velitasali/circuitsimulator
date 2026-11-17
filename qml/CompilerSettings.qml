pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtQuick.Dialogs
import cs_app
import cs_ui

/* Compiler Settings — C++ Compiler::compilerProps() / PropDialog, including
 * the Tool Path combo that offers PATH-discovered folders and rejects a
 * directory that does not contain the compiler's tools. */
Rectangle {
    id: root
    SystemPalette { id: appTheme }
    readonly property font uiFont: Qt.font({ family: App.fontFamily, pixelSize: App.fontNormal })
    implicitWidth: Math.max(420, mainCol.implicitWidth + 24)
    implicitHeight: mainCol.implicitHeight + 24
    color: appTheme.window

    readonly property bool fileSection: EditorPanel.usesBoard || EditorPanel.usesDevice || EditorPanel.usesFamily || EditorPanel.usesExtraArgs

    FolderDialog {
        id: toolPathDialog
        title: I18n.tr("Tool Path")
        currentFolder: App.suggestFolderUrl(EditorPanel.toolPath)
        onAccepted: EditorPanel.commitToolPath("" + selectedFolder)
    }
    FolderDialog {
        id: inclPathDialog
        title: EditorPanel.isArduino ? I18n.tr("Custom Library Path") : I18n.tr("Include Path")
        currentFolder: App.suggestFolderUrl(EditorPanel.inclPath)
        onAccepted: EditorPanel.inclPath = "" + selectedFolder
    }

    component FieldLabel: AppText {
        color: appTheme.windowText
        verticalAlignment: Text.AlignVCenter
        Layout.preferredWidth: 150
        elide: Text.ElideRight
    }

    component SectionLabel: ColumnLayout {
        id: sectionRoot
        property string caption: ""
        property bool isFirst: false
        Layout.fillWidth: true
        spacing: 0
        Rectangle {
            visible: !sectionRoot.isFirst
            Layout.fillWidth: true
            Layout.topMargin: 8
            Layout.bottomMargin: 6
            Layout.preferredHeight: 1
            color: appTheme.mid
        }
        AppText {
            text: sectionRoot.caption
            font.bold: true
            color: appTheme.windowText
        }
    }

    ColumnLayout {
        id: mainCol
        anchors.fill: parent
        anchors.margins: 12
        spacing: 10

        AppText {
            text: I18n.tr("Type: Compiler")
            font.pixelSize: App.fontTitle
            font.bold: true
            color: appTheme.windowText
            Layout.fillWidth: true
            elide: Text.ElideRight
        }

        Rectangle {
            Layout.fillWidth: true
            implicitWidth: groupCol.implicitWidth + 28
            implicitHeight: groupCol.implicitHeight + 28
            radius: 12
            color: appTheme.base
            border.color: appTheme.mid
            border.width: 1

            ColumnLayout {
                id: groupCol
                x: 14
                y: 14
                width: parent.width - 28
                spacing: 10

                SectionLabel {
                    caption: I18n.tr("For this compiler type:")
                    isFirst: true
                }

                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 4

                    RowLayout {
                        Layout.fillWidth: true
                        spacing: 6

                        FieldLabel { text: I18n.tr("Tool Path") }
                        AppComboBox {
                            id: pathBox
                            editable: true
                            model: EditorPanel.toolPathSuggestions
                            font: root.uiFont
                            Layout.fillWidth: true
                            Layout.preferredWidth: 200
                            property bool syncing: false
                            function syncEditText() {
                                pathBox.syncing = true
                                Qt.callLater(function() {
                                    pathBox.editText = EditorPanel.toolPath
                                    pathBox.syncing = false
                                })
                            }
                            Connections {
                                target: EditorPanel
                                function onFileSettingsChanged() { pathBox.syncEditText() }
                            }
                            Component.onCompleted: syncEditText()
                            onAccepted: if (!syncing) EditorPanel.commitToolPath(editText)
                            onActivated: if (!syncing) EditorPanel.commitToolPath(textAt(currentIndex))
                        }
                        AppToolButton {
                            implicitWidth: 26
                            implicitHeight: 26
                            ToolTip.text: I18n.tr("Browse")
                            ToolTip.visible: hovered
                            contentItem: AppIcon { icon: "folder_open"; color: appTheme.windowText }
                            onClicked: toolPathDialog.open()
                        }
                    }

                    Rectangle {
                        visible: EditorPanel.toolPathWarning.length > 0
                        Layout.fillWidth: true
                        Layout.preferredHeight: hintRow.implicitHeight + 8
                        color: Qt.rgba(appTheme.highlight.r, appTheme.highlight.g, appTheme.highlight.b, 0.12)
                        border.color: appTheme.highlight
                        border.width: 1
                        radius: 8

                        RowLayout {
                            id: hintRow
                            anchors.fill: parent
                            anchors.margins: 4
                            spacing: 6
                            AppIcon { icon: "warning"; color: appTheme.windowText }
                            AppText {
                                text: EditorPanel.toolPathWarning
                                color: appTheme.windowText
                                wrapMode: Text.WordWrap
                                Layout.fillWidth: true
                            }
                        }
                    }
                }

                RowLayout {
                    visible: EditorPanel.usesInclPath
                    Layout.fillWidth: true
                    spacing: 6

                    FieldLabel {
                        text: EditorPanel.isArduino ? I18n.tr("Custom Library Path") : I18n.tr("Include Path")
                    }
                    AppTextField {
                        font: root.uiFont
                        Layout.fillWidth: true
                        Layout.preferredWidth: 200
                        text: EditorPanel.inclPath
                        onEditingFinished: EditorPanel.inclPath = text
                    }
                    AppToolButton {
                        implicitWidth: 26
                        implicitHeight: 26
                        ToolTip.text: I18n.tr("Browse")
                        ToolTip.visible: hovered
                        contentItem: AppIcon { icon: "folder_open"; color: appTheme.windowText }
                        onClicked: inclPathDialog.open()
                    }
                }

                SectionLabel {
                    visible: root.fileSection
                    caption: I18n.tr("For this file:")
                    isFirst: false
                }

                RowLayout {
                    visible: EditorPanel.usesBoard
                    Layout.fillWidth: true
                    spacing: 8

                    FieldLabel { text: I18n.tr("Board") }
                    AppComboBox {
                        id: boardBox
                        font: root.uiFont
                        Layout.fillWidth: true
                        Layout.preferredWidth: 200
                        model: EditorPanel.boardList
                        onActivated: EditorPanel.board = currentText
                        function syncIndex() {
                            var list = EditorPanel.boardList
                            var want = EditorPanel.board
                            for (var i = 0; i < list.length; i++) {
                                if (list[i] === want) { currentIndex = i; return }
                            }
                            currentIndex = 0
                        }
                        Connections {
                            target: EditorPanel
                            function onFileSettingsChanged() { boardBox.syncIndex() }
                        }
                        Component.onCompleted: syncIndex()
                    }
                }

                RowLayout {
                    visible: EditorPanel.usesBoard && EditorPanel.isCustomBoard
                    Layout.fillWidth: true
                    spacing: 8

                    FieldLabel { text: I18n.tr("Custom Board") }
                    AppTextField {
                        font: root.uiFont
                        Layout.fillWidth: true
                        Layout.preferredWidth: 200
                        text: EditorPanel.customBoard
                        placeholderText: I18n.tr("e.g. esp32:esp32:esp32da")
                        onEditingFinished: EditorPanel.customBoard = text
                    }
                }

                RowLayout {
                    visible: EditorPanel.usesDevice && !EditorPanel.usesBoard
                    Layout.fillWidth: true
                    spacing: 8

                    FieldLabel { text: I18n.tr("Device") }
                    AppTextField {
                        font: root.uiFont
                        Layout.fillWidth: true
                        Layout.preferredWidth: 200
                        text: EditorPanel.device
                        onEditingFinished: EditorPanel.device = text
                    }
                }
                RowLayout {
                    visible: EditorPanel.usesFamily
                    Layout.fillWidth: true
                    spacing: 8

                    FieldLabel { text: I18n.tr("Family") }
                    AppTextField {
                        font: root.uiFont
                        Layout.fillWidth: true
                        Layout.preferredWidth: 200
                        text: EditorPanel.family
                        onEditingFinished: EditorPanel.family = text
                    }
                }
                RowLayout {
                    visible: EditorPanel.usesExtraArgs
                    Layout.fillWidth: true
                    spacing: 8

                    FieldLabel { text: I18n.tr("Extra build arguments") }
                    AppTextField {
                        font: root.uiFont
                        Layout.fillWidth: true
                        Layout.preferredWidth: 200
                        text: EditorPanel.extraArgs
                        onEditingFinished: EditorPanel.extraArgs = text
                    }
                }
            }
        }
    }
}
