pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtQuick.Dialogs
import cs_app
import cs_ui

Item {
    id: root
    SystemPalette { id: appTheme }
    anchors.fill: parent

    property string selectedPath: ""
    property bool selectedIsDir: false
    property string pendingRevealPath: ""
    property string trashTargetPath: ""
    property bool trashTargetIsDir: false
    readonly property string projectName: FileBrowser.rootPath
        ? ( FileBrowser.rootPath.replace( /\\/g, "/" ).split( "/" ).filter( Boolean ).pop() || FileBrowser.rootPath )
        : ""

    function syncEntries(): void {
        var arr = FileBrowser.entries
        if ( !arr )
            return
        if ( entryModel.count === 0 ) {
            for ( var i = 0; i < arr.length; ++i )
                entryModel.append( arr[i] )
            return
        }
        if ( arr.length === 0 ) {
            entryModel.clear()
            return
        }

        var newIndexMap = {}
        for ( var k = 0; k < arr.length; ++k ) {
            newIndexMap[arr[k].path] = k
        }

        var i = 0
        var j = 0
        while ( i < entryModel.count && j < arr.length ) {
            var oldItem = entryModel.get( i )
            var newItem = arr[j]

            if ( oldItem.path === newItem.path ) {
                if ( oldItem.expanded !== newItem.expanded ||
                     oldItem.name !== newItem.name ||
                     oldItem.depth !== newItem.depth ||
                     oldItem.isDir !== newItem.isDir ) {
                    entryModel.set( i, newItem )
                }
                i++
                j++
            } else if ( newIndexMap[oldItem.path] === undefined ) {
                entryModel.remove( i, 1 )
            } else if ( newIndexMap[oldItem.path] > j ) {
                entryModel.insert( i, newItem )
                i++
                j++
            } else {
                entryModel.remove( i, 1 )
            }
        }

        while ( i < entryModel.count ) {
            entryModel.remove( i, 1 )
        }

        while ( j < arr.length ) {
            entryModel.append( arr[j] )
            j++
        }
    }

    function scrollToEntryPath( path: string ): bool {
        if ( !path || path.length === 0 || listView.height <= 0 )
            return false
        for ( var j = 0; j < entryModel.count; ++j ) {
            var cur = entryModel.get( j )
            if ( cur && cur.path === path ) {
                listView.forceLayout()
                listView.positionViewAtIndex( j, ListView.Center )
                listView.forceActiveFocus()
                return true
            }
        }
        return false
    }

    function focusEntryPath( path: string ): void {
        if ( !path || path.length === 0 )
            return
        root.syncEntries()
        for ( var i = 0; i < entryModel.count; ++i ) {
            var item = entryModel.get( i )
            if ( item && item.path === path ) {
                root.selectedPath = item.path
                root.selectedIsDir = item.isDir === true
                break
            }
        }
        root.pendingRevealPath = path
        Qt.callLater( function() {
            if ( root.pendingRevealPath === path && root.scrollToEntryPath( path ) ) {
                root.pendingRevealPath = ""
            }
        } )
    }

    ListModel { id: entryModel }

    Connections {
        target: FileBrowser
        function onEntriesChanged(): void { root.syncEntries() }
        function onRootPathChanged(): void {
            root.pendingRevealPath = ""
            root.syncEntries()
        }
        function onPathRevealed( path: string ): void { root.focusEntryPath( path ) }
    }
    Component.onCompleted: {
        FileBrowser.startWatching()
        root.syncEntries()
    }

    FolderDialog {
        id: folderDialog
        title: I18n.tr( "Open Project" )
        currentFolder: App.suggestFolderUrl( FileBrowser.rootPath )
        onAccepted: FileBrowser.rootPath = "" + folderDialog.selectedFolder
    }

    function fileIconForName( name: string, isDir: bool, expanded: bool ): string {
        if ( isDir ) return expanded ? "folder_open" : "folder"
        if ( App.isCircuitPath( name || "" ) ) return "memory"
        const ext = ( name || "" ).split( "." ).pop().toLowerCase()
        if ( ext === "c" || ext === "cpp" || ext === "h" || ext === "hpp" ) return "code"
        if ( ext === "asm" || ext === "s" || ext === "inc" ) return "terminal"
        if ( ext === "hex" || ext === "bin" ) return "data_object"
        if ( ext === "png" || ext === "jpg" || ext === "jpeg" || ext === "svg" ) return "image"
        if ( ext === "xml" || ext === "json" || ext === "txt" || ext === "md" ) return "article"
        return "description"
    }

    function getTargetDirectory(): string {
        if ( !root.selectedPath || root.selectedPath.length === 0 ) {
            return FileBrowser.rootPath
        }
        if ( root.selectedIsDir ) {
            return root.selectedPath
        }
        var idx = root.selectedPath.lastIndexOf( "/" )
        if ( idx > 0 ) {
            return root.selectedPath.substring( 0, idx )
        }
        return FileBrowser.rootPath
    }

    function destinationDisplayPath(): string {
        var target = getTargetDirectory()
        if ( !target || target.length === 0 || target === FileBrowser.rootPath ) {
            return I18n.tr( "Project Root" )
        }
        if ( target.startsWith( FileBrowser.rootPath ) ) {
            var rel = target.substring( FileBrowser.rootPath.length )
            if ( rel.startsWith( "/" ) ) rel = rel.substring( 1 )
            return rel.length > 0 ? rel : I18n.tr( "Project Root" )
        }
        var idx = target.lastIndexOf( "/" )
        return idx >= 0 ? target.substring( idx + 1 ) : target
    }

    function renameTargetName(): string {
        if ( !root.selectedPath || root.selectedPath.length === 0 ) return ""
        var parts = root.selectedPath.split( "/" )
        return parts[parts.length - 1]
    }

    function pathExists( targetDir: string, name: string ): bool {
        if ( !name || name.trim().length === 0 ) return false
        var trimmed = name.trim()
        var fullPath = targetDir.endsWith( "/" ) ? ( targetDir + trimmed ) : ( targetDir + "/" + trimmed )
        for ( var i = 0; i < entryModel.count; ++i ) {
            var item = entryModel.get( i )
            if ( item && item.path === fullPath ) {
                return true
            }
        }
        return false
    }

    function validateFileName( name: string, isFolder: bool ): string {
        var trimmed = ( name || "" ).trim()
        if ( trimmed.length === 0 ) return ""
        if ( trimmed === "." || trimmed === ".." ) return I18n.tr( "Name is reserved." )
        if ( /[/\\:*?"<>|]/.test( trimmed ) ) return I18n.tr( "Name cannot contain / \\ : * ? \" < > |" )
        var target = getTargetDirectory()
        if ( pathExists( target, trimmed ) ) {
            return isFolder ? I18n.tr( "A folder with this name already exists." ) : I18n.tr( "A file with this name already exists." )
        }
        return ""
    }

    function validateRename( newName: string ): string {
        var trimmed = ( newName || "" ).trim()
        if ( trimmed.length === 0 ) return ""
        if ( trimmed === "." || trimmed === ".." ) return I18n.tr( "Name is reserved." )
        if ( /[/\\:*?"<>|]/.test( trimmed ) ) return I18n.tr( "Name cannot contain / \\ : * ? \" < > |" )
        var parts = root.selectedPath.split( "/" )
        var currentName = parts[parts.length - 1]
        if ( trimmed === currentName ) return I18n.tr( "Name is unchanged." )
        var parentDir = ""
        var idx = root.selectedPath.lastIndexOf( "/" )
        if ( idx > 0 ) parentDir = root.selectedPath.substring( 0, idx )
        else parentDir = FileBrowser.rootPath
        if ( pathExists( parentDir, trimmed ) ) return I18n.tr( "An item with this name already exists." )
        return ""
    }

    function applyExtension( textField: AppTextField, ext: string ): void {
        var text = textField.text.trim()
        if ( text.length === 0 ) {
            textField.text = ext
            textField.cursorPosition = 0
            textField.forceActiveFocus()
            return
        }
        var dotIdx = text.lastIndexOf( "." )
        if ( dotIdx > 0 ) {
            textField.text = text.substring( 0, dotIdx ) + ext
        } else {
            textField.text = text + ext
        }
        textField.cursorPosition = textField.text.length
        textField.forceActiveFocus()
    }

    function fileIconPreview( name: string ): string {
        var trimmed = ( name || "" ).trim()
        if ( trimmed.length === 0 ) return "note_add"
        return root.fileIconForName( trimmed, false, false )
    }

    function openFolderDialog(): void {
        CircuitPanel.promoteFloating( "files" )
        folderDialog.open()
    }

    function openCreateFileDialog(): void {
        CircuitPanel.promoteFloating( "files" )
        newFileInput.text = ""
        createFileDialog.open()
    }

    function openCreateFolderDialog(): void {
        CircuitPanel.promoteFloating( "files" )
        newDirInput.text = ""
        createFolderDialog.open()
    }

    function openRenameDialog(): void {
        CircuitPanel.promoteFloating( "files" )
        var parts = root.selectedPath.split( "/" )
        renameInput.text = parts[parts.length - 1]
        renameDialog.open()
    }

    function trashTargetName(): string {
        if ( !root.trashTargetPath || root.trashTargetPath.length === 0 ) return ""
        var parts = root.trashTargetPath.split( "/" )
        return parts[parts.length - 1]
    }

    function openTrashDialog(): void {
        if ( !root.selectedPath || root.selectedPath.length === 0 || root.selectedPath === FileBrowser.rootPath ) return
        CircuitPanel.promoteFloating( "files" )
        root.trashTargetPath = root.selectedPath
        root.trashTargetIsDir = root.selectedIsDir
        trashDialog.open()
    }

    AppContextMenu {
        id: contextMenu

        ContextMenuItem {
            text: I18n.tr( "Open in editor" )
            iconLigature: "folder_open"
            visible: root.selectedPath.length > 0 && !root.selectedIsDir
            onTriggered: FileBrowser.openInEditor( root.selectedPath )
        }
        ContextMenuSeparator {
            visible: root.selectedPath.length > 0 && !root.selectedIsDir
        }
        ContextMenuItem {
            text: I18n.tr( "New File" )
            iconLigature: "note_add"
            onTriggered: root.openCreateFileDialog()
        }
        ContextMenuItem {
            text: I18n.tr( "New Directory" )
            iconLigature: "create_new_folder"
            onTriggered: root.openCreateFolderDialog()
        }
        ContextMenuSeparator {}
        ContextMenuItem {
            text: I18n.tr( "Copy" )
            iconLigature: "content_copy"
            enabled: root.selectedPath.length > 0 && root.selectedPath !== FileBrowser.rootPath
            onTriggered: FileBrowser.copyToClipboard( root.selectedPath )
        }
        ContextMenuItem {
            text: I18n.tr( "Paste" )
            iconLigature: "content_paste"
            enabled: FileBrowser.canPaste
            onTriggered: FileBrowser.pasteTo( root.selectedPath )
        }
        ContextMenuSeparator {}
        ContextMenuItem {
            text: I18n.tr( "Rename" )
            iconLigature: "edit"
            enabled: root.selectedPath.length > 0 && root.selectedPath !== FileBrowser.rootPath
            onTriggered: root.openRenameDialog()
        }
        ContextMenuItem {
            text: I18n.tr( "Move to Trash" )
            iconLigature: "delete"
            enabled: root.selectedPath.length > 0 && root.selectedPath !== FileBrowser.rootPath
            onTriggered: root.openTrashDialog()
        }
        ContextMenuSeparator {}
        ContextMenuItem {
            text: I18n.tr( "Open externally" )
            iconLigature: "folder_open"
            enabled: root.selectedPath.length > 0
            onTriggered: FileBrowser.openExternally( root.selectedPath )
        }
        ContextMenuItem {
            text: I18n.tr( "Open Parent Dir externally" )
            iconLigature: "folder_open"
            enabled: root.selectedPath.length > 0
            onTriggered: FileBrowser.openParentExternally( root.selectedPath )
        }
        ContextMenuSeparator {}
        ContextMenuItem {
            text: I18n.tr( "Show Hidden" )
            checkable: true
            checked: FileBrowser.showHidden
            onTriggered: FileBrowser.showHidden = checked
            Binding on checked { value: FileBrowser.showHidden }
        }
    }

    Dialog {
        id: createFileDialog
        modal: true
        dim: true
        parent: Overlay.overlay
        closePolicy: Popup.CloseOnEscape
        padding: 0
        header: null
        footer: null
        width: Math.min( Math.max( ( parent ? parent.width : root.width ) - 32, 260 ), 340 )
        x: parent ? Math.round( ( parent.width - width ) / 2 ) : 0
        y: parent ? Math.max( 40, Math.round( ( parent.height - height ) / 2 ) ) : 16

        background: Rectangle {
            color: appTheme.window
            border.color: appTheme.mid
            border.width: 1
            radius: 14
        }

        Overlay.modal: Rectangle {
            color: Qt.rgba( 0, 0, 0, 0.45 )
        }

        onOpened: {
            newFileInput.forceActiveFocus()
            newFileInput.selectAll()
        }

        Shortcut {
            enabled: createFileDialog.visible
            sequences: [ "Return", "Enter" ]
            onActivated: if ( createFileBtn.enabled ) createFileDialog.accept()
        }

        contentItem: Item {
            implicitWidth: createFileDialog.width
            implicitHeight: fileDialogCol.implicitHeight + 24

            ColumnLayout {
                id: fileDialogCol
                anchors.fill: parent
                anchors.margins: 12
                spacing: 8

                RowLayout {
                    Layout.fillWidth: true
                    spacing: 8

                    Rectangle {
                        implicitWidth: 26
                        implicitHeight: 26
                        radius: height / 2
                        color: Qt.rgba( appTheme.highlight.r, appTheme.highlight.g, appTheme.highlight.b, 0.18 )

                        AppIcon {
                            anchors.centerIn: parent
                            icon: root.fileIconPreview( newFileInput.text )
                            color: appTheme.highlight
                            size: 16
                        }
                    }

                    AppText {
                        text: I18n.tr( "Create File" )
                        font.bold: true
                        color: appTheme.windowText
                        Layout.fillWidth: true
                        elide: Text.ElideRight
                    }

                    AppToolButton {
                        implicitWidth: 22
                        implicitHeight: 22
                        padding: 0
                        contentItem: AppIcon {
                            icon: "close"
                            size: 16
                            color: appTheme.windowText
                            opacity: 0.6
                        }
                        onClicked: createFileDialog.close()
                    }
                }

                Rectangle {
                    Layout.fillWidth: true
                    implicitHeight: 22
                    radius: 8
                    color: Qt.rgba( appTheme.mid.r, appTheme.mid.g, appTheme.mid.b, 0.18 )
                    border.color: Qt.rgba( appTheme.mid.r, appTheme.mid.g, appTheme.mid.b, 0.35 )
                    border.width: 1

                    RowLayout {
                        anchors.fill: parent
                        anchors.leftMargin: 6
                        anchors.rightMargin: 6
                        spacing: 4

                        AppIcon {
                            icon: "folder"
                            size: 16
                            color: appTheme.windowText
                            opacity: 0.7
                        }

                        AppText {
                            text: I18n.tr( "In: %1" ).arg( root.destinationDisplayPath() )
                            font.pixelSize: App.fontSmall
                            color: appTheme.windowText
                            opacity: 0.8
                            elide: Text.ElideMiddle
                            Layout.fillWidth: true
                        }
                    }
                }

                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 4

                    AppText {
                        text: I18n.tr( "File Name" )
                        font.bold: true
                        color: appTheme.windowText
                        opacity: 0.85
                    }

                    Rectangle {
                        Layout.fillWidth: true
                        implicitHeight: 30
                        radius: 8
                        color: appTheme.base
                        border.width: newFileInput.activeFocus ? 2 : 1
                        border.color: newFileInput.activeFocus ? appTheme.highlight : appTheme.mid

                        RowLayout {
                            anchors.fill: parent
                            anchors.leftMargin: 6
                            anchors.rightMargin: 4
                            spacing: 4

                            AppIcon {
                                icon: root.fileIconPreview( newFileInput.text )
                                size: 16
                                color: newFileInput.text.length > 0 ? appTheme.highlight : appTheme.windowText
                                opacity: newFileInput.text.length > 0 ? 1.0 : 0.45
                            }

                            AppTextField {
                                id: newFileInput
                                Layout.fillWidth: true
                                Layout.fillHeight: true
                                placeholderText: I18n.tr( "circuit.circ1, main.c..." )
                                selectByMouse: true
                                background: null
                                padding: 0
                                leftPadding: 2
                                rightPadding: 2
                                verticalAlignment: Text.AlignVCenter
                                color: appTheme.windowText
                                placeholderTextColor: Qt.rgba( appTheme.windowText.r, appTheme.windowText.g, appTheme.windowText.b, 0.4 )

                                onAccepted: {
                                    if ( createFileBtn.enabled ) createFileDialog.accept()
                                }
                            }

                            AppToolButton {
                                visible: newFileInput.text.length > 0
                                implicitWidth: 18
                                implicitHeight: 18
                                padding: 0
                                contentItem: AppIcon {
                                    icon: "close"
                                    size: 16
                                    color: appTheme.windowText
                                    opacity: 0.6
                                }
                                onClicked: {
                                    newFileInput.text = ""
                                    newFileInput.forceActiveFocus()
                                }
                            }
                        }
                    }
                }

                RowLayout {
                    Layout.fillWidth: true
                    spacing: 4

                    AppText {
                        text: I18n.tr( "Type:" )
                        font.pixelSize: App.fontSmall
                        color: appTheme.windowText
                        opacity: 0.6
                    }

                    Repeater {
                        model: [ ".circ1", ".c", ".h", ".txt" ]
                        delegate: Rectangle {
                            id: extChip
                            required property string modelData
                            implicitWidth: chipText.implicitWidth + 8
                            implicitHeight: 18
                            radius: 3
                            color: chipHover.hovered
                                   ? Qt.rgba( appTheme.highlight.r, appTheme.highlight.g, appTheme.highlight.b, 0.25 )
                                   : Qt.rgba( appTheme.mid.r, appTheme.mid.g, appTheme.mid.b, 0.2 )
                            border.color: newFileInput.text.endsWith( extChip.modelData )
                                          ? appTheme.highlight
                                          : Qt.rgba( appTheme.mid.r, appTheme.mid.g, appTheme.mid.b, 0.4 )
                            border.width: 1

                            AppText {
                                id: chipText
                                anchors.centerIn: parent
                                text: extChip.modelData
                                font.pixelSize: App.fontSmall
                                font.bold: newFileInput.text.endsWith( extChip.modelData )
                                color: newFileInput.text.endsWith( extChip.modelData ) ? appTheme.highlight : appTheme.windowText
                            }

                            HoverHandler { id: chipHover }
                            TapHandler {
                                onTapped: root.applyExtension( newFileInput, extChip.modelData )
                            }
                        }
                    }
                    Item { Layout.fillWidth: true }
                }

                AppText {
                    id: fileValidationText
                    Layout.fillWidth: true
                    visible: text.length > 0
                    text: root.validateFileName( newFileInput.text, false )
                    font.pixelSize: App.fontSmall
                    color: CircuitCanvas.msgErrorBg || Qt.rgba( 0.9, 0.3, 0.3, 1.0 )
                    wrapMode: Text.WordWrap
                }

                RowLayout {
                    Layout.fillWidth: true
                    Layout.topMargin: 2
                    spacing: 8

                    Item { Layout.fillWidth: true }

                    AppButton {
                        id: cancelFileBtn
                        text: I18n.tr( "Cancel" )
                        Layout.minimumHeight: 26
                        leftPadding: 12
                        rightPadding: 12
                        onClicked: createFileDialog.close()
                    }

                    AppButton {
                        id: createFileBtn
                        text: I18n.tr( "Create" )
                        highlighted: true
                        enabled: newFileInput.text.trim().length > 0 && fileValidationText.text.length === 0
                        font.bold: true
                        Layout.minimumHeight: 26
                        leftPadding: 14
                        rightPadding: 14
                        onClicked: createFileDialog.accept()
                    }
                }
            }
        }

        onAccepted: {
            var trimmed = newFileInput.text.trim()
            if ( trimmed.length > 0 && fileValidationText.text.length === 0 ) {
                FileBrowser.createFile( root.selectedPath, trimmed )
            }
        }
    }

    Dialog {
        id: createFolderDialog
        modal: true
        dim: true
        parent: Overlay.overlay
        closePolicy: Popup.CloseOnEscape
        padding: 0
        header: null
        footer: null
        width: Math.min( Math.max( ( parent ? parent.width : root.width ) - 32, 260 ), 340 )
        x: parent ? Math.round( ( parent.width - width ) / 2 ) : 0
        y: parent ? Math.max( 40, Math.round( ( parent.height - height ) / 2 ) ) : 16

        background: Rectangle {
            color: appTheme.window
            border.color: appTheme.mid
            border.width: 1
            radius: 14
        }

        Overlay.modal: Rectangle {
            color: Qt.rgba( 0, 0, 0, 0.45 )
        }

        onOpened: {
            newDirInput.forceActiveFocus()
            newDirInput.selectAll()
        }

        Shortcut {
            enabled: createFolderDialog.visible
            sequences: [ "Return", "Enter" ]
            onActivated: if ( createFolderBtn.enabled ) createFolderDialog.accept()
        }

        contentItem: Item {
            implicitWidth: createFolderDialog.width
            implicitHeight: folderDialogCol.implicitHeight + 24

            ColumnLayout {
                id: folderDialogCol
                anchors.fill: parent
                anchors.margins: 12
                spacing: 8

                RowLayout {
                    Layout.fillWidth: true
                    spacing: 8

                    Rectangle {
                        implicitWidth: 26
                        implicitHeight: 26
                        radius: height / 2
                        color: Qt.rgba( appTheme.highlight.r, appTheme.highlight.g, appTheme.highlight.b, 0.18 )

                        AppIcon {
                            anchors.centerIn: parent
                            icon: "create_new_folder"
                            color: appTheme.highlight
                            size: 16
                        }
                    }

                    AppText {
                        text: I18n.tr( "Create Folder" )
                        font.bold: true
                        color: appTheme.windowText
                        Layout.fillWidth: true
                        elide: Text.ElideRight
                    }

                    AppToolButton {
                        implicitWidth: 22
                        implicitHeight: 22
                        padding: 0
                        contentItem: AppIcon {
                            icon: "close"
                            size: 16
                            color: appTheme.windowText
                            opacity: 0.6
                        }
                        onClicked: createFolderDialog.close()
                    }
                }

                Rectangle {
                    Layout.fillWidth: true
                    implicitHeight: 22
                    radius: 8
                    color: Qt.rgba( appTheme.mid.r, appTheme.mid.g, appTheme.mid.b, 0.18 )
                    border.color: Qt.rgba( appTheme.mid.r, appTheme.mid.g, appTheme.mid.b, 0.35 )
                    border.width: 1

                    RowLayout {
                        anchors.fill: parent
                        anchors.leftMargin: 6
                        anchors.rightMargin: 6
                        spacing: 4

                        AppIcon {
                            icon: "folder"
                            size: 16
                            color: appTheme.windowText
                            opacity: 0.7
                        }

                        AppText {
                            text: I18n.tr( "In: %1" ).arg( root.destinationDisplayPath() )
                            font.pixelSize: App.fontSmall
                            color: appTheme.windowText
                            opacity: 0.8
                            elide: Text.ElideMiddle
                            Layout.fillWidth: true
                        }
                    }
                }

                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 4

                    AppText {
                        text: I18n.tr( "Folder Name" )
                        font.bold: true
                        color: appTheme.windowText
                        opacity: 0.85
                    }

                    Rectangle {
                        Layout.fillWidth: true
                        implicitHeight: 30
                        radius: 8
                        color: appTheme.base
                        border.width: newDirInput.activeFocus ? 2 : 1
                        border.color: newDirInput.activeFocus ? appTheme.highlight : appTheme.mid

                        RowLayout {
                            anchors.fill: parent
                            anchors.leftMargin: 6
                            anchors.rightMargin: 4
                            spacing: 4

                            AppIcon {
                                icon: "folder"
                                size: 16
                                color: newDirInput.text.length > 0 ? appTheme.highlight : appTheme.windowText
                                opacity: newDirInput.text.length > 0 ? 1.0 : 0.45
                            }

                            AppTextField {
                                id: newDirInput
                                Layout.fillWidth: true
                                Layout.fillHeight: true
                                placeholderText: I18n.tr( "components, subcircuits..." )
                                selectByMouse: true
                                background: null
                                padding: 0
                                leftPadding: 2
                                rightPadding: 2
                                verticalAlignment: Text.AlignVCenter
                                color: appTheme.windowText
                                placeholderTextColor: Qt.rgba( appTheme.windowText.r, appTheme.windowText.g, appTheme.windowText.b, 0.4 )

                                onAccepted: {
                                    if ( createFolderBtn.enabled ) createFolderDialog.accept()
                                }
                            }

                            AppToolButton {
                                visible: newDirInput.text.length > 0
                                implicitWidth: 18
                                implicitHeight: 18
                                padding: 0
                                contentItem: AppIcon {
                                    icon: "close"
                                    size: 16
                                    color: appTheme.windowText
                                    opacity: 0.6
                                }
                                onClicked: {
                                    newDirInput.text = ""
                                    newDirInput.forceActiveFocus()
                                }
                            }
                        }
                    }
                }

                AppText {
                    id: dirValidationText
                    Layout.fillWidth: true
                    visible: text.length > 0
                    text: root.validateFileName( newDirInput.text, true )
                    font.pixelSize: App.fontSmall
                    color: CircuitCanvas.msgErrorBg || Qt.rgba( 0.9, 0.3, 0.3, 1.0 )
                    wrapMode: Text.WordWrap
                }

                RowLayout {
                    Layout.fillWidth: true
                    Layout.topMargin: 2
                    spacing: 8

                    Item { Layout.fillWidth: true }

                    AppButton {
                        id: cancelFolderBtn
                        text: I18n.tr( "Cancel" )
                        Layout.minimumHeight: 26
                        leftPadding: 12
                        rightPadding: 12
                        onClicked: createFolderDialog.close()
                    }

                    AppButton {
                        id: createFolderBtn
                        text: I18n.tr( "Create" )
                        highlighted: true
                        enabled: newDirInput.text.trim().length > 0 && dirValidationText.text.length === 0
                        font.bold: true
                        Layout.minimumHeight: 26
                        leftPadding: 14
                        rightPadding: 14
                        onClicked: createFolderDialog.accept()
                    }
                }
            }
        }

        onAccepted: {
            var trimmed = newDirInput.text.trim()
            if ( trimmed.length > 0 && dirValidationText.text.length === 0 ) {
                FileBrowser.createDirectory( root.selectedPath, trimmed )
            }
        }
    }

    Dialog {
        id: renameDialog
        modal: true
        dim: true
        parent: Overlay.overlay
        closePolicy: Popup.CloseOnEscape
        padding: 0
        header: null
        footer: null
        width: Math.min( Math.max( ( parent ? parent.width : root.width ) - 32, 260 ), 340 )
        x: parent ? Math.round( ( parent.width - width ) / 2 ) : 0
        y: parent ? Math.max( 40, Math.round( ( parent.height - height ) / 2 ) ) : 16

        background: Rectangle {
            color: appTheme.window
            border.color: appTheme.mid
            border.width: 1
            radius: 14
        }

        Overlay.modal: Rectangle {
            color: Qt.rgba( 0, 0, 0, 0.45 )
        }

        onOpened: {
            renameInput.forceActiveFocus()
            renameInput.selectAll()
        }

        Shortcut {
            enabled: renameDialog.visible
            sequences: [ "Return", "Enter" ]
            onActivated: if ( renameBtn.enabled ) renameDialog.accept()
        }

        contentItem: Item {
            implicitWidth: renameDialog.width
            implicitHeight: renameCol.implicitHeight + 24

            ColumnLayout {
                id: renameCol
                anchors.fill: parent
                anchors.margins: 12
                spacing: 8

                RowLayout {
                    Layout.fillWidth: true
                    spacing: 8

                    Rectangle {
                        implicitWidth: 26
                        implicitHeight: 26
                        radius: height / 2
                        color: Qt.rgba( appTheme.highlight.r, appTheme.highlight.g, appTheme.highlight.b, 0.18 )

                        AppIcon {
                            anchors.centerIn: parent
                            icon: "edit"
                            color: appTheme.highlight
                            size: 16
                        }
                    }

                    AppText {
                        text: I18n.tr( "Rename" )
                        font.bold: true
                        color: appTheme.windowText
                        Layout.fillWidth: true
                        elide: Text.ElideRight
                    }

                    AppToolButton {
                        implicitWidth: 22
                        implicitHeight: 22
                        padding: 0
                        contentItem: AppIcon {
                            icon: "close"
                            size: 16
                            color: appTheme.windowText
                            opacity: 0.6
                        }
                        onClicked: renameDialog.close()
                    }
                }

                Rectangle {
                    Layout.fillWidth: true
                    implicitHeight: 22
                    radius: 8
                    color: Qt.rgba( appTheme.mid.r, appTheme.mid.g, appTheme.mid.b, 0.18 )
                    border.color: Qt.rgba( appTheme.mid.r, appTheme.mid.g, appTheme.mid.b, 0.35 )
                    border.width: 1

                    RowLayout {
                        anchors.fill: parent
                        anchors.leftMargin: 6
                        anchors.rightMargin: 6
                        spacing: 4

                        AppIcon {
                            icon: root.selectedIsDir ? "folder" : root.fileIconPreview( renameInput.text )
                            size: 16
                            color: appTheme.windowText
                            opacity: 0.7
                        }

                        AppText {
                            text: I18n.tr( "Target: %1" ).arg( root.renameTargetName() )
                            font.pixelSize: App.fontSmall
                            color: appTheme.windowText
                            opacity: 0.8
                            elide: Text.ElideMiddle
                            Layout.fillWidth: true
                        }
                    }
                }

                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 4

                    AppText {
                        text: I18n.tr( "New Name" )
                        font.bold: true
                        color: appTheme.windowText
                        opacity: 0.85
                    }

                    Rectangle {
                        Layout.fillWidth: true
                        implicitHeight: 30
                        radius: 8
                        color: appTheme.base
                        border.width: renameInput.activeFocus ? 2 : 1
                        border.color: renameInput.activeFocus ? appTheme.highlight : appTheme.mid

                        RowLayout {
                            anchors.fill: parent
                            anchors.leftMargin: 6
                            anchors.rightMargin: 4
                            spacing: 4

                            AppIcon {
                                icon: root.selectedIsDir ? "folder" : root.fileIconPreview( renameInput.text )
                                size: 16
                                color: renameInput.text.length > 0 ? appTheme.highlight : appTheme.windowText
                                opacity: renameInput.text.length > 0 ? 1.0 : 0.45
                            }

                            AppTextField {
                                id: renameInput
                                Layout.fillWidth: true
                                Layout.fillHeight: true
                                placeholderText: I18n.tr( "New name..." )
                                selectByMouse: true
                                background: null
                                padding: 0
                                leftPadding: 2
                                rightPadding: 2
                                verticalAlignment: Text.AlignVCenter
                                color: appTheme.windowText
                                placeholderTextColor: Qt.rgba( appTheme.windowText.r, appTheme.windowText.g, appTheme.windowText.b, 0.4 )

                                onAccepted: {
                                    if ( renameBtn.enabled ) renameDialog.accept()
                                }
                            }

                            AppToolButton {
                                visible: renameInput.text.length > 0
                                implicitWidth: 18
                                implicitHeight: 18
                                padding: 0
                                contentItem: AppIcon {
                                    icon: "close"
                                    size: 16
                                    color: appTheme.windowText
                                    opacity: 0.6
                                }
                                onClicked: {
                                    renameInput.text = ""
                                    renameInput.forceActiveFocus()
                                }
                            }
                        }
                    }
                }

                AppText {
                    id: renameValidationText
                    Layout.fillWidth: true
                    visible: text.length > 0
                    text: root.validateRename( renameInput.text )
                    font.pixelSize: App.fontSmall
                    color: CircuitCanvas.msgErrorBg || Qt.rgba( 0.9, 0.3, 0.3, 1.0 )
                    wrapMode: Text.WordWrap
                }

                RowLayout {
                    Layout.fillWidth: true
                    Layout.topMargin: 2
                    spacing: 8

                    Item { Layout.fillWidth: true }

                    AppButton {
                        id: cancelRenameBtn
                        text: I18n.tr( "Cancel" )
                        Layout.minimumHeight: 26
                        leftPadding: 12
                        rightPadding: 12
                        onClicked: renameDialog.close()
                    }

                    AppButton {
                        id: renameBtn
                        text: I18n.tr( "Rename" )
                        highlighted: true
                        enabled: renameInput.text.trim().length > 0 && renameValidationText.text.length === 0
                        font.bold: true
                        Layout.minimumHeight: 26
                        leftPadding: 14
                        rightPadding: 14
                        onClicked: renameDialog.accept()
                    }
                }
            }
        }

        onAccepted: {
            var trimmed = renameInput.text.trim()
            if ( trimmed.length > 0 && renameValidationText.text.length === 0 ) {
                FileBrowser.renamePath( root.selectedPath, trimmed )
            }
        }
    }

    Dialog {
        id: trashDialog
        modal: true
        dim: true
        parent: Overlay.overlay
        closePolicy: Popup.CloseOnEscape
        padding: 0
        header: null
        footer: null
        width: Math.min( Math.max( ( parent ? parent.width : root.width ) - 32, 260 ), 340 )
        x: parent ? Math.round( ( parent.width - width ) / 2 ) : 0
        y: parent ? Math.max( 40, Math.round( ( parent.height - height ) / 2 ) ) : 16

        background: Rectangle {
            color: appTheme.window
            border.color: appTheme.mid
            border.width: 1
            radius: 14
        }

        Overlay.modal: Rectangle {
            color: Qt.rgba( 0, 0, 0, 0.45 )
        }

        onOpened: confirmTrashBtn.forceActiveFocus()

        Shortcut {
            enabled: trashDialog.visible
            sequences: [ "Return", "Enter" ]
            onActivated: trashDialog.accept()
        }

        contentItem: Item {
            implicitWidth: trashDialog.width
            implicitHeight: trashCol.implicitHeight + 24

            ColumnLayout {
                id: trashCol
                anchors.fill: parent
                anchors.margins: 12
                spacing: 8

                RowLayout {
                    Layout.fillWidth: true
                    spacing: 8

                    Rectangle {
                        implicitWidth: 26
                        implicitHeight: 26
                        radius: height / 2
                        color: Qt.rgba( appTheme.highlight.r, appTheme.highlight.g, appTheme.highlight.b, 0.18 )

                        AppIcon {
                            anchors.centerIn: parent
                            icon: "delete"
                            color: appTheme.highlight
                            size: 16
                        }
                    }

                    AppText {
                        text: I18n.tr( "Move to Trash" )
                        font.bold: true
                        color: appTheme.windowText
                        Layout.fillWidth: true
                        elide: Text.ElideRight
                    }

                    AppToolButton {
                        implicitWidth: 22
                        implicitHeight: 22
                        padding: 0
                        contentItem: AppIcon {
                            icon: "close"
                            size: 16
                            color: appTheme.windowText
                            opacity: 0.6
                        }
                        onClicked: trashDialog.close()
                    }
                }

                Rectangle {
                    Layout.fillWidth: true
                    implicitHeight: 22
                    radius: 8
                    color: Qt.rgba( appTheme.mid.r, appTheme.mid.g, appTheme.mid.b, 0.18 )
                    border.color: Qt.rgba( appTheme.mid.r, appTheme.mid.g, appTheme.mid.b, 0.35 )
                    border.width: 1

                    RowLayout {
                        anchors.fill: parent
                        anchors.leftMargin: 6
                        anchors.rightMargin: 6
                        spacing: 4

                        AppIcon {
                            icon: root.trashTargetIsDir ? "folder" : root.fileIconPreview( root.trashTargetName() )
                            size: 16
                            color: appTheme.windowText
                            opacity: 0.7
                        }

                        AppText {
                            text: I18n.tr( "Target: %1" ).arg( root.trashTargetName() )
                            font.pixelSize: App.fontSmall
                            color: appTheme.windowText
                            opacity: 0.8
                            elide: Text.ElideMiddle
                            Layout.fillWidth: true
                        }
                    }
                }

                AppText {
                    Layout.fillWidth: true
                    text: I18n.tr( "Are you sure you want to move the selected items to trash?" )
                    font.pixelSize: App.fontSmall
                    color: appTheme.windowText
                    opacity: 0.88
                    wrapMode: Text.WordWrap
                }

                RowLayout {
                    Layout.fillWidth: true
                    Layout.topMargin: 2
                    spacing: 8

                    Item { Layout.fillWidth: true }

                    AppButton {
                        text: I18n.tr( "Cancel" )
                        Layout.minimumHeight: 26
                        leftPadding: 12
                        rightPadding: 12
                        onClicked: trashDialog.close()
                    }

                    AppButton {
                        id: confirmTrashBtn
                        text: I18n.tr( "Move to Trash" )
                        highlighted: true
                        font.bold: true
                        Layout.minimumHeight: 26
                        leftPadding: 14
                        rightPadding: 14
                        onClicked: trashDialog.accept()
                    }
                }
            }
        }

        onAccepted: {
            if ( root.trashTargetPath.length > 0 ) {
                if ( !FileBrowser.trash( root.trashTargetPath ) ) {
                    deleteDialog.open()
                }
            }
        }
    }

    Dialog {
        id: deleteDialog
        modal: true
        dim: true
        parent: Overlay.overlay
        closePolicy: Popup.CloseOnEscape
        padding: 0
        header: null
        footer: null
        width: Math.min( Math.max( ( parent ? parent.width : root.width ) - 32, 260 ), 340 )
        x: parent ? Math.round( ( parent.width - width ) / 2 ) : 0
        y: parent ? Math.max( 40, Math.round( ( parent.height - height ) / 2 ) ) : 16

        background: Rectangle {
            color: appTheme.window
            border.color: appTheme.mid
            border.width: 1
            radius: 14
        }

        Overlay.modal: Rectangle {
            color: Qt.rgba( 0, 0, 0, 0.45 )
        }

        onOpened: confirmDeleteBtn.forceActiveFocus()

        Shortcut {
            enabled: deleteDialog.visible
            sequences: [ "Return", "Enter" ]
            onActivated: deleteDialog.accept()
        }

        contentItem: Item {
            implicitWidth: deleteDialog.width
            implicitHeight: deleteCol.implicitHeight + 24

            ColumnLayout {
                id: deleteCol
                anchors.fill: parent
                anchors.margins: 12
                spacing: 8

                RowLayout {
                    Layout.fillWidth: true
                    spacing: 8

                    Rectangle {
                        id: deleteBadgeRect
                        readonly property color errColor: CircuitCanvas.msgErrorBg || appTheme.highlight
                        implicitWidth: 26
                        implicitHeight: 26
                        radius: height / 2
                        color: Qt.rgba( errColor.r, errColor.g, errColor.b, 0.18 )

                        AppIcon {
                            anchors.centerIn: parent
                            icon: "warning"
                            color: deleteBadgeRect.errColor
                            size: 16
                        }
                    }

                    AppText {
                        text: I18n.tr( "Delete" )
                        font.bold: true
                        color: appTheme.windowText
                        Layout.fillWidth: true
                        elide: Text.ElideRight
                    }

                    AppToolButton {
                        implicitWidth: 22
                        implicitHeight: 22
                        padding: 0
                        contentItem: AppIcon {
                            icon: "close"
                            size: 16
                            color: appTheme.windowText
                            opacity: 0.6
                        }
                        onClicked: deleteDialog.close()
                    }
                }

                Rectangle {
                    Layout.fillWidth: true
                    implicitHeight: 22
                    radius: 8
                    color: Qt.rgba( appTheme.mid.r, appTheme.mid.g, appTheme.mid.b, 0.18 )
                    border.color: Qt.rgba( appTheme.mid.r, appTheme.mid.g, appTheme.mid.b, 0.35 )
                    border.width: 1

                    RowLayout {
                        anchors.fill: parent
                        anchors.leftMargin: 6
                        anchors.rightMargin: 6
                        spacing: 4

                        AppIcon {
                            icon: root.trashTargetIsDir ? "folder" : root.fileIconPreview( root.trashTargetName() )
                            size: 16
                            color: appTheme.windowText
                            opacity: 0.7
                        }

                        AppText {
                            text: I18n.tr( "Target: %1" ).arg( root.trashTargetName() )
                            font.pixelSize: App.fontSmall
                            color: appTheme.windowText
                            opacity: 0.8
                            elide: Text.ElideMiddle
                            Layout.fillWidth: true
                        }
                    }
                }

                AppText {
                    Layout.fillWidth: true
                    text: I18n.tr( "Failed to move to trash." )
                    font.pixelSize: App.fontSmall
                    color: deleteBadgeRect.errColor
                    wrapMode: Text.WordWrap
                }

                AppText {
                    Layout.fillWidth: true
                    text: I18n.tr( "Are you sure you want to permanently delete the selected items?" )
                    font.pixelSize: App.fontSmall
                    color: appTheme.windowText
                    opacity: 0.88
                    wrapMode: Text.WordWrap
                }

                RowLayout {
                    Layout.fillWidth: true
                    Layout.topMargin: 2
                    spacing: 8

                    Item { Layout.fillWidth: true }

                    AppButton {
                        text: I18n.tr( "Cancel" )
                        Layout.minimumHeight: 26
                        leftPadding: 12
                        rightPadding: 12
                        onClicked: deleteDialog.close()
                    }

                    AppButton {
                        id: confirmDeleteBtn
                        text: I18n.tr( "Delete" )
                        highlighted: true
                        font.bold: true
                        Layout.minimumHeight: 26
                        leftPadding: 14
                        rightPadding: 14
                        onClicked: deleteDialog.accept()
                    }
                }
            }
        }

        onAccepted: {
            if ( root.trashTargetPath.length > 0 ) {
                FileBrowser.deletePath( root.trashTargetPath )
            }
        }
    }

    Rectangle {
        id: browserCard
        anchors.fill: parent
        radius: 10
        color: appTheme.base
        border.color: appTheme.mid
        border.width: 1
        clip: true

        Item {
            id: emptyState
            anchors.fill: parent
            visible: !FileBrowser.hasProject

            ColumnLayout {
                anchors.centerIn: parent
                width: Math.min( parent.width - 40, 260 )
                spacing: 12

                AppText {
                    text: I18n.tr( "Open a project folder to load the file explorer." )
                    wrapMode: Text.WordWrap
                    horizontalAlignment: Text.AlignHCenter
                    color: appTheme.windowText
                    Layout.fillWidth: true
                }
                AppButton {
                    text: I18n.tr( "Open Folder" )
                    Layout.alignment: Qt.AlignHCenter
                    onClicked: root.openFolderDialog()
                }
            }
        }

        ColumnLayout {
            anchors.fill: parent
            anchors.margins: 1
            spacing: 0
            visible: FileBrowser.hasProject

            Rectangle {
                id: headerBar
                Layout.fillWidth: true
                Layout.preferredHeight: 30
                topLeftRadius: 9
                topRightRadius: 9
                color: appTheme.window

                RowLayout {
                    id: toolbar
                    anchors.fill: parent
                    anchors.leftMargin: 4
                    anchors.rightMargin: 4
                    anchors.bottomMargin: 1
                    spacing: 1

                    Rectangle {
                        id: projectHeaderBtn
                        readonly property int naturalWidth: Math.ceil( projectNameLabel.implicitWidth ) + 26
                        Layout.fillWidth: true
                        Layout.preferredWidth: naturalWidth
                        Layout.maximumWidth: naturalWidth
                        Layout.minimumWidth: Math.min( naturalWidth, 40 )
                        Layout.preferredHeight: 22
                        Layout.alignment: Qt.AlignVCenter
                        radius: 6
                        color: ( projectBtnMouse.pressed || projectMenu.opened )
                               ? Qt.rgba( appTheme.windowText.r, appTheme.windowText.g, appTheme.windowText.b, 0.18 )
                               : projectBtnMouse.containsMouse
                                   ? Qt.rgba( appTheme.windowText.r, appTheme.windowText.g, appTheme.windowText.b, 0.10 )
                                   : "transparent"

                        Behavior on color { ColorAnimation { duration: 90 } }

                        RowLayout {
                            anchors.fill: parent
                            anchors.leftMargin: 6
                            anchors.rightMargin: 2
                            spacing: 2

                            AppText {
                                id: projectNameLabel
                                text: root.projectName
                                font.bold: true
                                font.pixelSize: App.fontSmall
                                color: appTheme.windowText
                                elide: Text.ElideRight
                                Layout.fillWidth: true
                                Layout.minimumWidth: 0
                            }

                            AppIcon {
                                icon: "arrow_drop_down"
                                size: 16
                                color: appTheme.windowText
                                opacity: 0.65
                                Layout.alignment: Qt.AlignVCenter
                            }
                        }

                        MouseArea {
                            id: projectBtnMouse
                            anchors.fill: parent
                            hoverEnabled: true
                            cursorShape: Qt.PointingHandCursor
                            onClicked: {
                                if ( projectMenu.opened ) {
                                    projectMenu.close()
                                    return
                                }
                                projectMenu.recentItems = FileBrowser.recentProjectItems() || []
                                projectMenu.x = 0
                                projectMenu.y = projectHeaderBtn.height + 4
                                projectMenu.open()
                            }
                        }

                        ToolTip.text: FileBrowser.rootPath
                        ToolTip.visible: projectBtnMouse.containsMouse && !projectMenu.opened
                        ToolTip.delay: 600

                        AppContextMenu {
                            id: projectMenu
                            closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutsideParent
                            property var recentItems: []

                            Instantiator {
                                model: projectMenu.recentItems
                                delegate: ContextMenuItem {
                                    id: recentProjItem
                                    required property var modelData
                                    text: recentProjItem.modelData.name || ""
                                    iconLigature: recentProjItem.modelData.current ? "check" : "folder"
                                    onTriggered: {
                                        if ( !recentProjItem.modelData.current && recentProjItem.modelData.path ) {
                                            FileBrowser.rootPath = String( recentProjItem.modelData.path )
                                        }
                                    }
                                }
                                onObjectAdded: ( index, object ) => {
                                    const item = object as ContextMenuItem
                                    if ( item ) projectMenu.insertItem( index, item )
                                }
                                onObjectRemoved: ( _, object ) => {
                                    const item = object as ContextMenuItem
                                    if ( item ) projectMenu.removeItem( item )
                                }
                            }

                            ContextMenuSeparator {
                                visible: projectMenu.recentItems.length > 0
                            }
                            ContextMenuItem {
                                text: I18n.tr( "Open Project Folder..." )
                                iconLigature: "folder_open"
                                onTriggered: root.openFolderDialog()
                            }
                            ContextMenuItem {
                                text: I18n.tr( "Show Hidden" )
                                checkable: true
                                checked: FileBrowser.showHidden
                                onTriggered: FileBrowser.showHidden = checked
                                Binding on checked { value: FileBrowser.showHidden }
                            }
                            ContextMenuSeparator {}
                            ContextMenuItem {
                                text: I18n.tr( "Close Project" )
                                iconLigature: "folder_off"
                                onTriggered: FileBrowser.closeProject()
                            }
                        }
                    }

                    Item {
                        Layout.fillWidth: true
                        Layout.minimumWidth: 0
                        Layout.preferredWidth: 0
                    }

                    AppToolButton {
                        implicitWidth: 22
                        implicitHeight: 22
                        padding: 0
                        Layout.alignment: Qt.AlignVCenter
                        contentItem: AppIcon { icon: "note_add"; size: 16; color: appTheme.windowText }
                        ToolTip.text: I18n.tr( "New File" )
                        ToolTip.visible: hovered
                        onClicked: root.openCreateFileDialog()
                    }
                    AppToolButton {
                        implicitWidth: 22
                        implicitHeight: 22
                        padding: 0
                        Layout.alignment: Qt.AlignVCenter
                        contentItem: AppIcon { icon: "create_new_folder"; size: 16; color: appTheme.windowText }
                        ToolTip.text: I18n.tr( "New Folder" )
                        ToolTip.visible: hovered
                        onClicked: root.openCreateFolderDialog()
                    }

                    Rectangle {
                        Layout.alignment: Qt.AlignVCenter
                        Layout.leftMargin: 2
                        Layout.rightMargin: 2
                        implicitWidth: 1
                        implicitHeight: 12
                        color: appTheme.mid
                        opacity: 0.45
                    }

                    AppToolButton {
                        implicitWidth: 22
                        implicitHeight: 22
                        padding: 0
                        Layout.alignment: Qt.AlignVCenter
                        contentItem: AppIcon { icon: "refresh"; size: 16; color: appTheme.windowText }
                        ToolTip.text: I18n.tr( "Refresh" )
                        ToolTip.visible: hovered
                        onClicked: FileBrowser.reload()
                    }
                    AppToolButton {
                        implicitWidth: 22
                        implicitHeight: 22
                        padding: 0
                        Layout.alignment: Qt.AlignVCenter
                        contentItem: AppIcon { icon: "expand_more"; size: 16; color: appTheme.windowText }
                        ToolTip.text: I18n.tr( "Expand All" )
                        ToolTip.visible: hovered
                        onClicked: FileBrowser.expandAll()
                    }
                    AppToolButton {
                        implicitWidth: 22
                        implicitHeight: 22
                        padding: 0
                        Layout.alignment: Qt.AlignVCenter
                        contentItem: AppIcon { icon: "expand_less"; size: 16; color: appTheme.windowText }
                        ToolTip.text: I18n.tr( "Collapse All" )
                        ToolTip.visible: hovered
                        onClicked: FileBrowser.collapseAll()
                    }
                }

                Rectangle {
                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.bottom: parent.bottom
                    height: 1
                    color: appTheme.mid
                    opacity: 0.6
                }
            }

            Item {
                Layout.fillWidth: true
                Layout.fillHeight: true
                clip: true

                MouseArea {
                    anchors.fill: parent
                    acceptedButtons: Qt.LeftButton | Qt.RightButton
                    onClicked: ( mouse ) => {
                        root.pendingRevealPath = ""
                        root.selectedPath = FileBrowser.rootPath
                        root.selectedIsDir = true
                        if ( mouse.button === Qt.RightButton ) {
                            contextMenu.popup()
                        }
                    }
                }

                ListView {
                    id: listView
                    anchors.fill: parent
                    topMargin: 2
                    bottomMargin: 2
                    model: entryModel
                    clip: true
                    boundsBehavior: Flickable.StopAtBounds
                    ScrollBar.vertical: AppScrollBar {}
                    ScrollBar.horizontal: AppScrollBar {}
                    onHeightChanged: {
                        if ( height > 0 && root.pendingRevealPath.length > 0 ) {
                            var p = root.pendingRevealPath
                            if ( root.scrollToEntryPath( p ) ) {
                                root.pendingRevealPath = ""
                            }
                        }
                    }

                    delegate: Item {
                        id: rowDelegate
                        required property var model
                        required property int index
                        width: listView.width
                        height: 24

                        Rectangle {
                            anchors.fill: parent
                            anchors.leftMargin: 4
                            anchors.rightMargin: 4
                            radius: 6
                            visible: hoverHandler.hovered || rowDelegate.model.path === root.selectedPath
                            color: hoverHandler.hovered
                                   ? appTheme.highlight
                                   : Qt.rgba( appTheme.highlight.r, appTheme.highlight.g, appTheme.highlight.b, 0.22 )
                        }

                        Row {
                            anchors.fill: parent
                            anchors.leftMargin: 6 + rowDelegate.model.depth * 14
                            anchors.rightMargin: 6
                            spacing: 6

                            Item {
                                width: 14
                                height: rowDelegate.height
                                anchors.verticalCenter: parent.verticalCenter

                                AppIcon {
                                    anchors.centerIn: parent
                                    visible: rowDelegate.model.isDir
                                    icon: rowDelegate.model.expanded ? "expand_more" : "chevron_right"
                                    size: 16
                                    color: hoverHandler.hovered
                                           ? appTheme.highlightedText
                                           : ( rowDelegate.model.path === root.selectedPath ? appTheme.highlight : appTheme.windowText )
                                }

                                TapHandler {
                                    enabled: rowDelegate.model.isDir
                                    acceptedButtons: Qt.LeftButton
                                    onTapped: {
                                        root.selectedPath = rowDelegate.model.path
                                        root.selectedIsDir = true
                                        FileBrowser.toggleExpand( rowDelegate.model.path )
                                    }
                                }
                            }

                            AppIcon {
                                anchors.verticalCenter: parent.verticalCenter
                                icon: root.fileIconForName( rowDelegate.model.name, rowDelegate.model.isDir, rowDelegate.model.expanded )
                                color: hoverHandler.hovered
                                       ? appTheme.highlightedText
                                       : ( rowDelegate.model.path === root.selectedPath ? appTheme.highlight : appTheme.windowText )
                            }

                            AppText {
                                anchors.verticalCenter: parent.verticalCenter
                                text: rowDelegate.model.name
                                color: hoverHandler.hovered
                                       ? appTheme.highlightedText
                                       : ( rowDelegate.model.path === root.selectedPath ? appTheme.highlight : appTheme.windowText )
                                elide: Text.ElideRight
                                width: Math.max( 0, rowDelegate.width - 48 )
                            }
                        }

                        HoverHandler { id: hoverHandler }

                        TapHandler {
                            acceptedButtons: Qt.LeftButton
                            onTapped: {
                                root.pendingRevealPath = ""
                                root.selectedPath = rowDelegate.model.path
                                root.selectedIsDir = rowDelegate.model.isDir
                            }
                            onDoubleTapped: {
                                root.pendingRevealPath = ""
                                root.selectedPath = rowDelegate.model.path
                                root.selectedIsDir = rowDelegate.model.isDir
                                if ( rowDelegate.model.isDir ) FileBrowser.toggleExpand( rowDelegate.model.path )
                                else FileBrowser.openPath( rowDelegate.model.path )
                            }
                        }

                        TapHandler {
                            acceptedButtons: Qt.RightButton
                            gesturePolicy: TapHandler.ReleaseWithinBounds
                            onTapped: {
                                root.pendingRevealPath = ""
                                root.selectedPath = rowDelegate.model.path
                                root.selectedIsDir = rowDelegate.model.isDir
                                contextMenu.popup()
                            }
                        }
                    }
                }
            }
        }
    }
}

