pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import cs_app
import cs_ui

/* Tab bar + one editor. C++ had a Repeater of CodeEditor ctx objects; the
 * current buffer is EditorPanel.currentText so typing does not rebuild tabs. */
Item {
    id: root
    SystemPalette { id: appTheme }

    readonly property bool anyTabHasMcu: {
        var docs = EditorPanel.documents || []
        for (var i = 0; i < docs.length; ++i) {
            if (docs[i] && docs[i].mcuNames && docs[i].mcuNames.length > 0)
                return true
        }
        return false
    }
    TextMetrics {
        id: titleMetrics
        font.family: App.fontFamily
        font.pixelSize: App.fontNormal
        text: "Ag"
    }
    TextMetrics {
        id: mcuMetrics
        font.family: App.fontFamily
        font.pixelSize: App.fontMicro
        text: "Ag"
    }
    readonly property real twoLineTabHeight: Math.ceil(titleMetrics.height + mcuMetrics.height + 6)

    // pollLsp also drains the background compiler. Keep polling while a
    // compile or board discovery is running even if the editor overlay is hidden,
    // otherwise the job would finish silently until the next time CodeEditor is shown.
    Timer {
        interval: 50
        running: EditorPanel.compiling || EditorPanel.boardLoading
        repeat: true
        onTriggered: EditorPanel.pollLsp()
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 4
        spacing: 4

        RowLayout {
            Layout.fillWidth: true
            spacing: 4

            AppTabBar {
                id: docTabs
                Layout.fillWidth: true
                Layout.alignment: Qt.AlignVCenter
                Layout.minimumHeight: 24
                Layout.preferredHeight: Math.max( 24, docTabs.implicitHeight )
                onCurrentIndexChanged: EditorPanel.currentDocument = currentIndex

                Repeater {
                    model: EditorPanel.documents
                    delegate: AppTabButton {
                        id: tabBtn
                        required property var modelData
                        required property int index
                        readonly property string mcuNames: modelData.mcuNames || ""
                        readonly property bool hasMcu: mcuNames.length > 0
                        text: modelData.title
                        leftPadding: 10
                        rightPadding: 24
                            + (modelData.hasSettings ? 18 : 0)
                            + (modelData.dirty ? 10 : 0)
                        topPadding: hasMcu ? 3 : 4
                        bottomPadding: hasMcu ? 3 : 4
                        implicitHeight: Math.max(
                            contentItem.implicitHeight + topPadding + bottomPadding,
                            root.anyTabHasMcu ? root.twoLineTabHeight : 24
                        )
                        anchors.verticalCenter: parent.verticalCenter

                        readonly property color itemColor: tabBtn.checked ? appTheme.highlightedText : appTheme.windowText
                        onClicked: codeEditor.focusEditor()

                        ToolTip.visible: tabBtn.hovered && !settingsBtn.hovered && !closeBtn.hovered
                                         && ((tabBtn.modelData.path || "") !== "" || tabBtn.hasMcu)
                        ToolTip.text: {
                            var p = tabBtn.modelData.path || tabBtn.modelData.title || ""
                            return tabBtn.hasMcu ? (p + "\n" + I18n.tr("MCUs:") + " " + tabBtn.mcuNames) : p
                        }
                        ToolTip.delay: 600

                        contentItem: Item {
                            implicitWidth: tabLabelCol.implicitWidth
                            implicitHeight: tabLabelCol.implicitHeight

                            Column {
                                id: tabLabelCol
                                anchors.left: parent.left
                                anchors.right: parent.right
                                anchors.verticalCenter: parent.verticalCenter
                                spacing: 0

                                AppText {
                                    id: titleLabel
                                    text: tabBtn.modelData.title
                                    color: tabBtn.itemColor
                                    horizontalAlignment: Text.AlignLeft
                                    elide: Text.ElideRight
                                }
                                AppText {
                                    id: mcuLabel
                                    visible: tabBtn.hasMcu
                                    width: Math.min(implicitWidth, Math.max(titleLabel.implicitWidth, 160))
                                    text: tabBtn.mcuNames
                                    font.pixelSize: App.fontMicro
                                    color: tabBtn.itemColor
                                    opacity: tabBtn.checked ? 0.85 : 0.65
                                    horizontalAlignment: Text.AlignLeft
                                    elide: Text.ElideRight
                                }
                            }
                        }

                        MouseArea {
                            anchors.fill: parent
                            acceptedButtons: Qt.RightButton
                            onPressed: (mouse) => {
                                mouse.accepted = true
                                CircuitPanel.overlayClicked("editorPanel")
                                tabContextMenu.prepareForTab(tabBtn.index)
                                var pt = mapToItem(root, mouse.x, mouse.y)
                                tabContextMenu.popup(pt.x, pt.y)
                            }
                        }

                        AppIcon {
                            id: dirtyIcon
                            visible: tabBtn.modelData.dirty === true
                            anchors.right: settingsBtn.visible ? settingsBtn.left : closeBtn.left
                            anchors.verticalCenter: parent.verticalCenter
                            anchors.rightMargin: 3
                            width: 8
                            icon: "circle"
                            size: 16
                            color: tabBtn.itemColor
                        }

                        AppToolButton {
                            id: settingsBtn
                            visible: tabBtn.modelData.hasSettings === true
                            anchors.right: closeBtn.left
                            anchors.verticalCenter: parent.verticalCenter
                            anchors.rightMargin: 2
                            width: 16; height: 16
                            padding: 0
                            ToolTip.text: I18n.tr("File Settings")
                            ToolTip.delay: 500
                            contentItem: AppIcon {
                                icon: "settings"
                                size: 16
                                color: tabBtn.itemColor
                            }
                            onClicked: {
                                CircuitPanel.promoteFloating("editor")
                                docTabs.currentIndex = tabBtn.index
                                EditorPanel.fileProps()
                            }
                        }
                        AppToolButton {
                            id: closeBtn
                            anchors.right: parent.right
                            anchors.verticalCenter: parent.verticalCenter
                            anchors.rightMargin: 2
                            width: 16; height: 16
                            padding: 0
                            ToolTip.text: I18n.tr( "Close" )
                            ToolTip.delay: 500
                            contentItem: AppText {
                                text: "×"
                                font.bold: true
                                color: tabBtn.itemColor
                                horizontalAlignment: Text.AlignHCenter
                                verticalAlignment: Text.AlignVCenter
                            }
                            onClicked: {
                                // Resolve codeEditor before close. A clean close
                                // rebuilds the tab repeater and clears this
                                // delegate context before the handler returns.
                                codeEditor.focusEditor()
                                EditorPanel.closeDocument( tabBtn.index )
                            }
                        }
                    }
                }
            }

            Rectangle {
                Layout.alignment: Qt.AlignVCenter
                Layout.preferredWidth: 1
                Layout.preferredHeight: 16
                color: appTheme.mid
                opacity: 0.6
            }

            AppToolButton {
                id: newFileBtn
                Layout.alignment: Qt.AlignVCenter
                implicitWidth: 20
                implicitHeight: 20
                padding: 0
                ToolTip.text: I18n.tr( "New File" )
                ToolTip.delay: 500
                contentItem: AppIcon {
                    icon: "add"
                    size: 16
                    color: newFileBtn.textColor
                }
                onClicked: {
                    EditorPanel.newFile()
                    codeEditor.focusEditor()
                }
            }
        }
        AppContextMenu {
            id: tabContextMenu
            property int targetIndex: -1
            readonly property var docs: EditorPanel.documents || []
            readonly property int docCount: docs.length
            readonly property var targetDoc: (targetIndex >= 0 && targetIndex < docCount) ? docs[targetIndex] : null
            readonly property string targetPath: (targetDoc && targetDoc.path) ? String(targetDoc.path) : ""
            property bool targetInProject: false
            property bool targetHasFile: false
            property bool targetHasParentDir: false
            readonly property bool targetHasCompiler: Boolean(targetDoc && targetDoc.hasCompiler === true)
            readonly property bool targetHasSettings: Boolean(targetDoc && targetDoc.hasSettings === true)
            readonly property var allDevices: CircuitCanvas.firmwareDevices || []
            readonly property var activeDevice: {
                for (var i = 0; i < allDevices.length; ++i) {
                    if (allDevices[i] && allDevices[i].isActive)
                        return allDevices[i]
                }
                return allDevices.length > 0 ? allDevices[0] : null
            }
            readonly property var otherDevices: {
                var out = []
                var act = activeDevice
                for (var i = 0; i < allDevices.length; ++i) {
                    var d = allDevices[i]
                    if (d && (!act || d.uid !== act.uid))
                        out.push(d)
                }
                return out
            }
            readonly property bool showCompile: targetHasCompiler
            readonly property bool showLoadFirmware: targetHasCompiler && activeDevice !== null
            readonly property bool showSetupFirmware: !targetHasCompiler && targetHasSettings
            readonly property bool showFileActions: targetInProject || targetPath.length > 0

            function prepareForTab(idx: int): void {
                targetIndex = idx
                var d = (idx >= 0 && idx < docCount) ? docs[idx] : null
                var p = (d && d.path) ? String(d.path) : ""
                targetInProject = p.length > 0 && FileBrowser.isInProject(p)
                targetHasFile = p.length > 0 && FileBrowser.pathExists(p)
                targetHasParentDir = p.length > 0 && FileBrowser.parentDirExists(p)
            }

            function syncSubMenuRow(sub: AppContextMenu, shown: bool): void {
                for (var i = 0; i < count; i++) {
                    const it = itemAt(i) as ContextMenuItem
                    if (it && it.subMenu === sub) {
                        it.visible = shown
                        it.enabled = shown
                        if (shown && sub.iconLigature)
                            it.iconLigature = sub.iconLigature
                        return
                    }
                }
            }

            onAboutToShow: {
                syncSubMenuRow(otherMcuMenu, tabContextMenu.targetHasCompiler && otherMcuRepeater.count > 0)
            }

            ContextMenuItem {
                visible: tabContextMenu.showSetupFirmware
                text: I18n.tr("Set Up Firmware...")
                iconLigature: "settings"
                onTriggered: {
                    if (tabContextMenu.targetIndex >= 0) {
                        CircuitPanel.promoteFloating("editor")
                        docTabs.currentIndex = tabContextMenu.targetIndex
                        EditorPanel.fileProps()
                    }
                }
            }

            ContextMenuItem {
                visible: tabContextMenu.showCompile
                enabled: !EditorPanel.compiling
                text: I18n.tr("Compile")
                iconLigature: "build"
                shortcutText: tabContextMenu.targetIndex === EditorPanel.currentDocument
                              ? UiUtils.shortcutFor("sim.compile")
                              : ""
                onTriggered: {
                    if (tabContextMenu.targetIndex >= 0)
                        EditorPanel.compileDoc(tabContextMenu.targetIndex)
                }
            }

            ContextMenuItem {
                visible: tabContextMenu.showLoadFirmware
                enabled: !EditorPanel.compiling
                text: tabContextMenu.activeDevice
                      ? I18n.tr("Load Firmware for %1").arg(tabContextMenu.activeDevice.label || tabContextMenu.activeDevice.uid || "")
                      : I18n.tr("Load Firmware")
                iconLigature: "upload"
                onTriggered: {
                    if (tabContextMenu.targetIndex >= 0 && tabContextMenu.activeDevice)
                        EditorPanel.uploadDocToDevice(tabContextMenu.targetIndex, String(tabContextMenu.activeDevice.uid || ""))
                }
            }

            AppContextMenu {
                id: otherMcuMenu
                title: I18n.tr("Load Firmware for Other MCU")
                iconLigature: "memory"
                Instantiator {
                    id: otherMcuRepeater
                    model: tabContextMenu.otherDevices
                    delegate: ContextMenuItem {
                        id: otherMcuItem
                        required property var modelData
                        text: String(modelData.label || modelData.uid || "")
                        iconLigature: "upload"
                        enabled: !EditorPanel.compiling
                        onTriggered: {
                            if (tabContextMenu.targetIndex >= 0 && otherMcuItem.modelData.uid)
                                EditorPanel.uploadDocToDevice(tabContextMenu.targetIndex, String(otherMcuItem.modelData.uid))
                        }
                    }
                    onObjectAdded: (index, object) => {
                        const item = object as ContextMenuItem
                        if (item) otherMcuMenu.insertItem(index, item)
                        tabContextMenu.syncSubMenuRow(otherMcuMenu, tabContextMenu.targetHasCompiler && otherMcuRepeater.count > 0)
                    }
                    onObjectRemoved: (_, object) => {
                        const item = object as ContextMenuItem
                        if (item) otherMcuMenu.removeItem(item)
                        tabContextMenu.syncSubMenuRow(otherMcuMenu, tabContextMenu.targetHasCompiler && otherMcuRepeater.count > 0)
                    }
                }
            }

            ContextMenuSeparator {
                visible: tabContextMenu.showCompile || tabContextMenu.showLoadFirmware || tabContextMenu.showSetupFirmware
            }

            ContextMenuItem {
                visible: tabContextMenu.targetInProject
                text: I18n.tr("Focus in File Tree")
                iconLigature: "account_tree"
                onTriggered: {
                    if (tabContextMenu.targetPath.length > 0) {
                        CircuitPanel.showFiles()
                        FileBrowser.revealPath(tabContextMenu.targetPath)
                    }
                }
            }
            ContextMenuItem {
                visible: tabContextMenu.targetPath.length > 0
                enabled: tabContextMenu.targetHasFile
                text: I18n.tr("Open Externally")
                iconLigature: "open_in_new"
                onTriggered: {
                    if (tabContextMenu.targetPath.length > 0)
                        FileBrowser.openExternally(tabContextMenu.targetPath)
                }
            }
            ContextMenuItem {
                visible: tabContextMenu.targetPath.length > 0
                enabled: tabContextMenu.targetHasParentDir
                text: I18n.tr("Open Containing Folder Externally")
                iconLigature: "folder_open"
                onTriggered: {
                    if (tabContextMenu.targetPath.length > 0)
                        FileBrowser.openParentExternally(tabContextMenu.targetPath)
                }
            }
            ContextMenuSeparator {
                visible: tabContextMenu.showFileActions
            }

            ContextMenuItem {
                text: I18n.tr("Close")
                iconLigature: "close"
                enabled: tabContextMenu.targetIndex >= 0
                onTriggered: {
                    codeEditor.focusEditor()
                    EditorPanel.closeDocument(tabContextMenu.targetIndex)
                }
            }
            ContextMenuItem {
                text: I18n.tr("Close Others")
                iconLigature: "tab_close_others"
                enabled: tabContextMenu.targetIndex >= 0 && tabContextMenu.docCount > 1
                onTriggered: {
                    codeEditor.focusEditor()
                    EditorPanel.closeOtherDocuments(tabContextMenu.targetIndex)
                }
            }
            ContextMenuItem {
                text: I18n.tr("Close Tabs to the Left")
                iconLigature: "tab_close_left"
                enabled: tabContextMenu.targetIndex > 0
                onTriggered: {
                    codeEditor.focusEditor()
                    EditorPanel.closeDocumentsToLeft(tabContextMenu.targetIndex)
                }
            }
            ContextMenuItem {
                text: I18n.tr("Close Tabs to the Right")
                iconLigature: "tab_close_right"
                enabled: tabContextMenu.targetIndex >= 0 && tabContextMenu.targetIndex < tabContextMenu.docCount - 1
                onTriggered: {
                    codeEditor.focusEditor()
                    EditorPanel.closeDocumentsToRight(tabContextMenu.targetIndex)
                }
            }
            ContextMenuSeparator {}
            ContextMenuItem {
                text: I18n.tr("Close All")
                iconLigature: "tab_close_all"
                enabled: tabContextMenu.docCount > 0
                onTriggered: {
                    codeEditor.focusEditor()
                    EditorPanel.closeAllDocuments()
                }
            }
        }
        Binding {
            target: docTabs
            property: "currentIndex"
            value: EditorPanel.currentDocument
            restoreMode: Binding.RestoreNone
        }
        Connections {
            target: EditorPanel
            function onDocumentsChanged() {
                Qt.callLater(docTabs.ensureVisibleCurrent)
            }
            function onCurrentDocumentChanged() {
                Qt.callLater(docTabs.ensureVisibleCurrent)
            }
        }

        Rectangle {
            Layout.fillWidth: true
            Layout.fillHeight: true
            radius: 10
            color: appTheme.base
            border.color: appTheme.mid
            border.width: 1
            clip: true
            visible: EditorPanel.currentDocument >= 0

            CodeEditor {
                id: codeEditor
                anchors.fill: parent
            }
        }

        Label {
            Layout.fillWidth: true
            Layout.fillHeight: true
            visible: EditorPanel.currentDocument < 0
            text: I18n.tr( "Open a file from the Files tab, or click + for a new buffer." )
            color: appTheme.windowText
            wrapMode: Text.WordWrap
            horizontalAlignment: Text.AlignHCenter
            verticalAlignment: Text.AlignVCenter
            opacity: 0.6
        }
    }
}
