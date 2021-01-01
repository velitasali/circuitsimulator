import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtQuick.Dialogs
import cs_app

// The four settings tabs. Every control binds to one AppDialog property; the
// setter applies the setting immediately.
Rectangle {
    id: root
    SystemPalette { id: appTheme }
    readonly property font uiFont: Qt.font({ family: App.fontFamily || "Ubuntu", pixelSize: App.fontSize || 13 })
    function tr(s) { return App.i18nTick >= 0 ? App.translate(s) : s }

    FolderDialog {
        id: userFolderDialog
        title: root.tr("User Folder")
        currentFolder: App.suggestFolderUrl(AppDialog.userPath)
        onAccepted: AppDialog.userPath = "" + selectedFolder
    }
    AppConfirmDialog {
        id: resetDialog
        dialogTitle: root.tr("Reset Settings")
        dialogText: root.tr("Reset all settings to their default values?\n\nShortcuts, panel sizes, recent files and component list customizations will be lost.")
        acceptText: root.tr("Reset")
        cancelText: root.tr("Cancel")
        showDiscard: false
        onAccepted: AppDialog.confirmReset()
    }
    Connections {
        target: AppDialog
        function onRequestBrowseUserPath() { userFolderDialog.open() }
        function onRequestReset() { resetDialog.open() }
    }

    implicitWidth: mainCol.implicitWidth + 24
    implicitHeight: mainCol.implicitHeight + 24
    color: appTheme.window

    // A section title with a subtle divider above it, so the groups inside a tab read as
    // separate blocks. The first header of a tab drops the rule.
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

    component FieldLabel: Text {
        font: root.uiFont
        color: appTheme.windowText
        verticalAlignment: Text.AlignVCenter
        Layout.preferredWidth: 175
    }

    // label + arbitrary control, so every row lines up on the same column.
    // holder absorbs the row's leftover width itself, rather than a trailing
    // spacer: that lets one field (e.g. the user folder path) opt into
    // Layout.fillWidth on its own control and actually grow into that space,
    // while every other field just leaves it empty as before.
    component Field: RowLayout {
        default property alias content: holder.data
        property alias label: fieldLabel.text
        spacing: 8
        Layout.fillWidth: true
        FieldLabel { id: fieldLabel }
        RowLayout { id: holder; spacing: 6; Layout.fillWidth: true }
    }

    component Check: AppCheckBox {
        font: root.uiFont
        Layout.fillWidth: true
    }

    component Spin: AppSpinBox {
        editable: true
        font: root.uiFont
        Layout.preferredWidth: 140
    }

    // No ScrollView: the window is sized to fit the largest tab (see stack's
    // implicitWidth/Height below), so nothing here ever needs to scroll.
    component Tab: Item {
        id: tabRoot
        default property alias body: col.data
        implicitWidth: col.implicitWidth + 28
        implicitHeight: col.implicitHeight + 28

        ColumnLayout {
            id: col
            x: 14
            y: 14
            width: tabRoot.width - 28
            spacing: 8
        }
    }

    ColumnLayout {
        id: mainCol
        anchors.fill: parent
        anchors.margins: 12
        spacing: 10

        AppTabBar {
            id: bar
            Layout.fillWidth: true
            AppTabButton { text: root.tr( "App" ) }
            AppTabButton { text: root.tr( "Circuit" ) }
            AppTabButton { text: root.tr( "Simulation" ) }
            AppTabButton { text: root.tr( "Editor" ) }
            AppTabButton { text: root.tr( "Debug" ) }
        }

        // Rounded container card with the tabs' contents
        Rectangle {
            id: container
            Layout.fillWidth: true
            Layout.fillHeight: true
            implicitWidth: stack.implicitWidth
            implicitHeight: stack.implicitHeight
            radius: 8
            color: "transparent"
            border.color: appTheme.mid
            border.width: 1
            clip: true

            StackLayout {
                id: stack
                anchors.fill: parent
                currentIndex: bar.currentIndex
                // Sized to the largest tab, not just the current one, so the
                // window never has to resize -- or scroll -- when switching tabs.
                implicitWidth: Math.max( tabApp.implicitWidth, tabCircuit.implicitWidth,
                                          tabSim.implicitWidth, tabEditor.implicitWidth, tabDebug.implicitWidth )
                implicitHeight: Math.max( tabApp.implicitHeight, tabCircuit.implicitHeight,
                                           tabSim.implicitHeight, tabEditor.implicitHeight, tabDebug.implicitHeight )

                // ------------------------------------------------------------ App
                Tab {
                    id: tabApp
                    Header { text: root.tr( "App Settings" ); first: true }

                    Field {
                        label: root.tr( "Language" )
                        AppComboBox {
                            model: AppDialog.languages
                            currentIndex: AppDialog.language
                            font: root.uiFont
                            Layout.preferredWidth: 140
                            onActivated: AppDialog.language = currentIndex
                            Binding on currentIndex { value: AppDialog.language }
                        }
                    }
                    Field {
                        label: root.tr( "Theme" )
                        AppComboBox {
                            id: themeCombo
                            model: AppDialog.themes
                            currentIndex: themeCombo.model ? themeCombo.model.indexOf( AppDialog.theme ) : -1
                            font: root.uiFont
                            Layout.preferredWidth: 140
                            onActivated: AppDialog.theme = currentValue
                            Binding on currentIndex { value: themeCombo.model ? themeCombo.model.indexOf( AppDialog.theme ) : -1 }
                        }
                    }
                    Field {
                        label: root.tr( "Font" )
                        AppComboBox {
                            id: fontCombo
                            model: Qt.fontFamilies()
                            currentIndex: fontCombo.model ? fontCombo.model.indexOf( AppDialog.fontName ) : -1
                            font: root.uiFont
                            Layout.preferredWidth: 140
                            onActivated: AppDialog.fontName = currentValue
                            Binding on currentIndex { value: fontCombo.model ? fontCombo.model.indexOf( AppDialog.fontName ) : -1 }
                        }
                    }
                    Field {
                        label: root.tr( "User Folder" )
                        AppTextField {
                            text: AppDialog.userPath
                            placeholderText: AppDialog.userPathPlaceholder
                            font: root.uiFont
                            Layout.preferredWidth: 100
                            Layout.fillWidth: true
                            onEditingFinished: AppDialog.userPath = text
                            Binding on text { value: AppDialog.userPath }
                        }
                        AppButton {
                            text: root.tr( "Browse..." )
                            font: root.uiFont
                            onClicked: AppDialog.browseUserPath()
                        }
                    }

                    Header { text: root.tr( "Updates" ) }

                    Check {
                        text: root.tr( "Check for updates on start" )
                        checked: AppDialog.autoUpdate
                        onToggled: AppDialog.autoUpdate = checked
                        Binding on checked { value: AppDialog.autoUpdate }
                    }
                }

                // -------------------------------------------------------- Circuit
                Tab {
                    id: tabCircuit
                    Header { text: root.tr( "Canvas" ); first: true }

                    Check {
                        text: root.tr( "Draw Grid" )
                        checked: AppDialog.drawGrid
                        onToggled: AppDialog.drawGrid = checked
                        Binding on checked { value: AppDialog.drawGrid }
                    }
                    Check {
                        text: root.tr( "Show Scrollbars" )
                        checked: AppDialog.showScroll
                        onToggled: AppDialog.showScroll = checked
                        Binding on checked { value: AppDialog.showScroll }
                    }
                    Field {
                        label: root.tr( "Canvas Width" )
                        Spin { from: 1; to: 10000; value: AppDialog.canvasWidth
                               onValueModified: AppDialog.canvasWidth = value
                               Binding on value { value: AppDialog.canvasWidth } }
                    }
                    Field {
                        label: root.tr( "Canvas Height" )
                        Spin { from: 1; to: 10000; value: AppDialog.canvasHeight
                               onValueModified: AppDialog.canvasHeight = value
                               Binding on value { value: AppDialog.canvasHeight } }
                    }

                    Header { text: root.tr( "Defaults for new circuits" ) }

                    Check {
                        text: root.tr( "Animate Logic" )
                        checked: AppDialog.animateLogic
                        onToggled: AppDialog.animateLogic = checked
                        Binding on checked { value: AppDialog.animateLogic }
                    }
                    Check {
                        text: root.tr( "Animate Current Flow" )
                        checked: AppDialog.animateCurr
                        onToggled: AppDialog.animateCurr = checked
                        Binding on checked { value: AppDialog.animateCurr }
                    }
                    Check {
                        text: root.tr( "ANSI Symbols" )
                        checked: AppDialog.ansiSymbols
                        onToggled: AppDialog.ansiSymbols = checked
                        Binding on checked { value: AppDialog.ansiSymbols }
                    }
                    Field {
                        label: root.tr( "Refresh Rate (fps)" )
                        Spin { from: 1; to: 100; value: AppDialog.fps
                               onValueModified: AppDialog.fps = value
                               Binding on value { value: AppDialog.fps } }
                    }
                    Field {
                        label: root.tr( "Undo Steps" )
                        Spin { from: 0; to: 999; value: AppDialog.undoSteps
                               onValueModified: AppDialog.undoSteps = value
                               Binding on value { value: AppDialog.undoSteps } }
                    }
                }

                // ----------------------------------------------------- Simulation
                Tab {
                    id: tabSim
                    Header { text: root.tr( "Speed" ); first: true }

                    Field {
                        label: root.tr( "Real Time Speed" )
                        AppSlider {
                            from: 0; to: 100
                            value: AppDialog.speedPercent
                            Layout.preferredWidth: 180
                            onMoved: AppDialog.speedPercent = value
                            Binding on value { value: AppDialog.speedPercent }
                        }
                        Text {
                            text: AppDialog.speedLabel
                            font: root.uiFont
                            color: appTheme.windowText
                            Layout.preferredWidth: 60
                        }
                    }
                    Field {
                        label: root.tr( "Steps per Second" )
                        Spin { from: 1; to: 999999999; value: AppDialog.simStep
                               onValueModified: AppDialog.simStep = value
                               Binding on value { value: AppDialog.simStep } }
                        AppComboBox {
                            model: AppDialog.timeUnits
                            currentIndex: AppDialog.simStepUnit
                            font: root.uiFont
                            Layout.preferredWidth: 80
                            onActivated: AppDialog.simStepUnit = currentIndex
                            Binding on currentIndex { value: AppDialog.simStepUnit }
                        }
                    }

                    Header { text: root.tr( "Accuracy" ) }

                    Field {
                        label: root.tr( "Reactive Step" )
                        Spin { from: 1; to: 999999999; value: AppDialog.reactStep
                               onValueModified: AppDialog.reactStep = value
                               Binding on value { value: AppDialog.reactStep } }
                        AppComboBox {
                            model: AppDialog.timeUnits
                            currentIndex: AppDialog.reactStepUnit
                            font: root.uiFont
                            Layout.preferredWidth: 80
                            onActivated: AppDialog.reactStepUnit = currentIndex
                            Binding on currentIndex { value: AppDialog.reactStepUnit }
                        }
                    }
                    Field {
                        label: root.tr( "Max Non-Linear Steps" )
                        Spin { from: 0; to: 100000; value: AppDialog.nlSteps
                               onValueModified: AppDialog.nlSteps = value
                               Binding on value { value: AppDialog.nlSteps } }
                    }
                    Field {
                        label: root.tr( "Slope Steps" )
                        Spin { from: 0; to: 100; value: AppDialog.slopeSteps
                               onValueModified: AppDialog.slopeSteps = value
                               Binding on value { value: AppDialog.slopeSteps } }
                    }
                    Text {
                        text: root.tr( "These are defaults for new circuits; the open circuit is edited from Circuit Settings." )
                        font: root.uiFont
                        color: appTheme.windowText
                        opacity: 0.65
                        wrapMode: Text.WordWrap
                        Layout.fillWidth: true
                        Layout.topMargin: 4
                    }
                }

                // --------------------------------------------------------- Editor
                Tab {
                    id: tabEditor
                    Header { text: root.tr( "Editor Settings" ); first: true }

                    Text {
                        visible: !AppDialog.hasEditor
                        text: root.tr( "The editor is not open, so these settings are unavailable." )
                        font: root.uiFont
                        color: appTheme.windowText
                        opacity: 0.65
                        wrapMode: Text.WordWrap
                        Layout.fillWidth: true
                    }

                    ColumnLayout {
                        visible: AppDialog.hasEditor
                        spacing: 8
                        Layout.fillWidth: true

                        Field {
                            label: root.tr( "Font" )
                            AppComboBox {
                                id: editorFontCombo
                                model: Qt.fontFamilies()
                                currentIndex: editorFontCombo.model ? editorFontCombo.model.indexOf( AppDialog.editorFontFamily ) : -1
                                font: root.uiFont
                                Layout.preferredWidth: 140
                                onActivated: AppDialog.editorFontFamily = currentValue
                                Binding on currentIndex { value: editorFontCombo.model ? editorFontCombo.model.indexOf( AppDialog.editorFontFamily ) : -1 }
                            }
                        }
                        Field {
                            label: root.tr( "Font Size" )
                            Spin { from: 6; to: 72; value: AppDialog.editorFontSize
                                   onValueModified: AppDialog.editorFontSize = value
                                   Binding on value { value: AppDialog.editorFontSize } }
                        }
                        Field {
                            label: root.tr( "Tab Size" )
                            Spin { from: 1; to: 16; value: AppDialog.editorTabSize
                                   onValueModified: AppDialog.editorTabSize = value
                                   Binding on value { value: AppDialog.editorTabSize } }
                        }
                        Check {
                            text: root.tr( "Use spaces for tabs" )
                            checked: AppDialog.editorSpaceTabs
                            onToggled: AppDialog.editorSpaceTabs = checked
                            Binding on checked { value: AppDialog.editorSpaceTabs }
                        }
                        Check {
                            text: root.tr( "Show whitespace" )
                            checked: AppDialog.editorShowSpaces
                            onToggled: AppDialog.editorShowSpaces = checked
                            Binding on checked { value: AppDialog.editorShowSpaces }
                        }
                        Check {
                            text: root.tr( "Show LSP debug output" )
                            checked: AppDialog.editorShowLspDebug
                            onToggled: AppDialog.editorShowLspDebug = checked
                            Binding on checked { value: AppDialog.editorShowLspDebug }
                        }

                        Header { text: root.tr( "Auto-close" ) }

                        Check {
                            text: root.tr( "Parenthesis  ()" )
                            checked: AppDialog.editorCloseParenthesis
                            onToggled: AppDialog.editorCloseParenthesis = checked
                            Binding on checked { value: AppDialog.editorCloseParenthesis }
                        }
                        Check {
                            text: root.tr( "Braces  {}" )
                            checked: AppDialog.editorCloseBraces
                            onToggled: AppDialog.editorCloseBraces = checked
                            Binding on checked { value: AppDialog.editorCloseBraces }
                        }
                        Check {
                            text: root.tr( "Brackets  []" )
                            checked: AppDialog.editorCloseBrackets
                            onToggled: AppDialog.editorCloseBrackets = checked
                            Binding on checked { value: AppDialog.editorCloseBrackets }
                        }
                        Check {
                            text: root.tr( "Quotes  \"\"" )
                            checked: AppDialog.editorCloseQuotes
                            onToggled: AppDialog.editorCloseQuotes = checked
                            Binding on checked { value: AppDialog.editorCloseQuotes }
                        }
                        Check {
                            text: root.tr( "Single quotes  ''" )
                            checked: AppDialog.editorCloseSquotes
                            onToggled: AppDialog.editorCloseSquotes = checked
                            Binding on checked { value: AppDialog.editorCloseSquotes }
                        }
                    }
                }

                // ------------------------------------------------------------ Debug
                Tab {
                    id: tabDebug
                    Header { text: root.tr( "Canvas Diagnostics" ); first: true }

                    Check {
                        text: root.tr( "Enable canvas repaint debug overlay" )
                        checked: AppDialog.repaintOverlayEnabled
                        onToggled: AppDialog.repaintOverlayEnabled = checked
                    }

                    Check {
                        text: root.tr( "Show component bounding rectangles" )
                        checked: AppDialog.showComponentRects
                        onToggled: AppDialog.showComponentRects = checked
                    }

                    Header { text: root.tr( "Language Server & Editor Diagnostics" ) }

                    Check {
                        text: root.tr( "Show LSP debug messages in output panel" )
                        checked: AppDialog.editorShowLspDebug
                        onToggled: AppDialog.editorShowLspDebug = checked
                    }
                }
            }
        }

        RowLayout {
            Layout.fillWidth: true
            spacing: 6
            Item { Layout.fillWidth: true }
            AppButton {
                text: root.tr( "Reset All Settings..." )
                font: root.uiFont
                ToolTip.text: root.tr( "Restore all settings to their default values" )
                ToolTip.visible: hovered
                ToolTip.delay: 600
                onClicked: AppDialog.resetSettings()
            }
        }
    }
}
