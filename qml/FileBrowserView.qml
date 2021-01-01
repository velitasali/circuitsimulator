import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtQuick.Dialogs
import cs_app

Item {
    id: root
    function tr(s) { return App.i18nTick >= 0 ? App.translate(s) : s }
    SystemPalette { id: appTheme }
    anchors.fill: parent

    property string selectedPath: ""
    property bool selectedIsDir: false

    function syncEntries() {
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

    ListModel { id: entryModel }

    Connections {
        target: FileBrowser
        function onEntriesChanged() { root.syncEntries() }
        function onRootPathChanged() { root.syncEntries() }
    }
    Component.onCompleted: root.syncEntries()

    FolderDialog {
        id: folderDialog
        title: root.tr( "Open Project" )
        currentFolder: App.suggestFolderUrl( FileBrowser.rootPath )
        onAccepted: FileBrowser.rootPath = "" + selectedFolder
    }

    function fileIconForName( name, isDir, expanded ) {
        if ( isDir ) return expanded ? "folder_open" : "folder"
        const ext = ( name || "" ).split( "." ).pop().toLowerCase()
        if ( ext === "circ1" || ext === "circ" || ext === "sim" || ext === "sim2" || ext === "sim1" ) return "memory"
        if ( ext === "c" || ext === "cpp" || ext === "h" || ext === "hpp" ) return "code"
        if ( ext === "asm" || ext === "s" || ext === "inc" ) return "terminal"
        if ( ext === "hex" || ext === "bin" ) return "data_object"
        if ( ext === "png" || ext === "jpg" || ext === "jpeg" || ext === "svg" ) return "image"
        if ( ext === "xml" || ext === "json" || ext === "txt" || ext === "md" ) return "article"
        return "description"
    }

    function getTargetDirectory() {
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

    function destinationDisplayPath() {
        var target = getTargetDirectory()
        if ( !target || target.length === 0 || target === FileBrowser.rootPath ) {
            return root.tr( "Project Root" )
        }
        if ( target.startsWith( FileBrowser.rootPath ) ) {
            var rel = target.substring( FileBrowser.rootPath.length )
            if ( rel.startsWith( "/" ) ) rel = rel.substring( 1 )
            return rel.length > 0 ? rel : root.tr( "Project Root" )
        }
        var idx = target.lastIndexOf( "/" )
        return idx >= 0 ? target.substring( idx + 1 ) : target
    }

    function renameTargetName() {
        if ( !root.selectedPath || root.selectedPath.length === 0 ) return ""
        var parts = root.selectedPath.split( "/" )
        return parts[parts.length - 1]
    }

    function pathExists( targetDir, name ) {
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

    function validateFileName( name, isFolder ) {
        var trimmed = ( name || "" ).trim()
        if ( trimmed.length === 0 ) return ""
        if ( trimmed === "." || trimmed === ".." ) return root.tr( "Name is reserved." )
        if ( /[/\\:*?"<>|]/.test( trimmed ) ) return root.tr( "Name cannot contain / \\ : * ? \" < > |" )
        var target = getTargetDirectory()
        if ( pathExists( target, trimmed ) ) {
            return isFolder ? root.tr( "A folder with this name already exists." ) : root.tr( "A file with this name already exists." )
        }
        return ""
    }

    function validateRename( newName ) {
        var trimmed = ( newName || "" ).trim()
        if ( trimmed.length === 0 ) return ""
        if ( trimmed === "." || trimmed === ".." ) return root.tr( "Name is reserved." )
        if ( /[/\\:*?"<>|]/.test( trimmed ) ) return root.tr( "Name cannot contain / \\ : * ? \" < > |" )
        var parts = root.selectedPath.split( "/" )
        var currentName = parts[parts.length - 1]
        if ( trimmed === currentName ) return root.tr( "Name is unchanged." )
        var parentDir = ""
        var idx = root.selectedPath.lastIndexOf( "/" )
        if ( idx > 0 ) parentDir = root.selectedPath.substring( 0, idx )
        else parentDir = FileBrowser.rootPath
        if ( pathExists( parentDir, trimmed ) ) return root.tr( "An item with this name already exists." )
        return ""
    }

    function applyExtension( textField, ext ) {
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

    function fileIconPreview( name ) {
        var trimmed = ( name || "" ).trim()
        if ( trimmed.length === 0 ) return "note_add"
        return root.fileIconForName( trimmed, false, false )
    }

    function openCreateFileDialog() {
        newFileInput.text = ""
        createFileDialog.open()
    }

    function openCreateFolderDialog() {
        newDirInput.text = ""
        createFolderDialog.open()
    }

    function openRenameDialog() {
        var parts = root.selectedPath.split( "/" )
        renameInput.text = parts[parts.length - 1]
        renameDialog.open()
    }

    AppContextMenu {
        id: contextMenu

        ContextMenuItem {
            text: root.tr( "Open in editor" )
            iconLigature: "folder_open"
            visible: root.selectedPath.length > 0 && !root.selectedIsDir
            onTriggered: FileBrowser.openInEditor( root.selectedPath )
        }
        ContextMenuSeparator {
            visible: root.selectedPath.length > 0 && !root.selectedIsDir
        }
        ContextMenuItem {
            text: root.tr( "New File" )
            iconLigature: "note_add"
            onTriggered: root.openCreateFileDialog()
        }
        ContextMenuItem {
            text: root.tr( "New Directory" )
            iconLigature: "create_new_folder"
            onTriggered: root.openCreateFolderDialog()
        }
        ContextMenuSeparator {}
        ContextMenuItem {
            text: root.tr( "Copy" )
            iconLigature: "content_copy"
            enabled: root.selectedPath.length > 0 && root.selectedPath !== FileBrowser.rootPath
            onTriggered: FileBrowser.copyToClipboard( root.selectedPath )
        }
        ContextMenuItem {
            text: root.tr( "Paste" )
            iconLigature: "content_paste"
            enabled: FileBrowser.canPaste
            onTriggered: FileBrowser.pasteTo( root.selectedPath )
        }
        ContextMenuSeparator {}
        ContextMenuItem {
            text: root.tr( "Rename" )
            iconLigature: "edit"
            enabled: root.selectedPath.length > 0 && root.selectedPath !== FileBrowser.rootPath
            onTriggered: root.openRenameDialog()
        }
        ContextMenuItem {
            text: root.tr( "Move to Trash" )
            iconLigature: "delete"
            enabled: root.selectedPath.length > 0 && root.selectedPath !== FileBrowser.rootPath
            onTriggered: FileBrowser.trash( root.selectedPath )
        }
        ContextMenuSeparator {}
        ContextMenuItem {
            text: root.tr( "Open externally" )
            iconLigature: "folder_open"
            enabled: root.selectedPath.length > 0
            onTriggered: FileBrowser.openExternally( root.selectedPath )
        }
        ContextMenuItem {
            text: root.tr( "Open Parent Dir externally" )
            iconLigature: "folder_open"
            enabled: root.selectedPath.length > 0
            onTriggered: FileBrowser.openParentExternally( root.selectedPath )
        }
        ContextMenuSeparator {}
        ContextMenuItem {
            text: root.tr( "Show Hidden" )
            checkable: true
            checked: FileBrowser.showHidden
            onTriggered: FileBrowser.showHidden = !FileBrowser.showHidden
        }
    }

    Dialog {
        id: createFileDialog
        modal: true
        dim: true
        closePolicy: Popup.CloseOnEscape
        padding: 0
        header: null
        footer: null
        width: Math.min( Math.max( root.width - 16, 220 ), 300 )
        x: Math.round( ( root.width - width ) / 2 )
        y: Math.max( 16, Math.round( ( root.height - height ) / 3 ) )

        background: Rectangle {
            color: appTheme.window
            border.color: appTheme.mid
            border.width: 1
            radius: 10
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
                        radius: 5
                        color: Qt.rgba( appTheme.highlight.r, appTheme.highlight.g, appTheme.highlight.b, 0.18 )

                        AppIcon {
                            anchors.centerIn: parent
                            text: root.fileIconPreview( newFileInput.text )
                            color: appTheme.highlight
                            font.pixelSize: 15
                        }
                    }

                    Text {
                        text: root.tr( "Create File" )
                        font: Qt.font({ family: App.fontFamily || "Ubuntu", pixelSize: ( App.fontSize || 13 ) + 1, bold: true })
                        color: appTheme.windowText
                        Layout.fillWidth: true
                        elide: Text.ElideRight
                    }

                    AppToolButton {
                        implicitWidth: 22
                        implicitHeight: 22
                        padding: 0
                        contentItem: AppIcon {
                            text: "close"
                            font.pixelSize: 14
                            color: appTheme.windowText
                            opacity: 0.6
                        }
                        onClicked: createFileDialog.close()
                    }
                }

                Rectangle {
                    Layout.fillWidth: true
                    implicitHeight: 22
                    radius: 4
                    color: Qt.rgba( appTheme.mid.r, appTheme.mid.g, appTheme.mid.b, 0.18 )
                    border.color: Qt.rgba( appTheme.mid.r, appTheme.mid.g, appTheme.mid.b, 0.35 )
                    border.width: 1

                    RowLayout {
                        anchors.fill: parent
                        anchors.leftMargin: 6
                        anchors.rightMargin: 6
                        spacing: 4

                        AppIcon {
                            text: "folder"
                            font.pixelSize: 12
                            color: appTheme.windowText
                            opacity: 0.7
                        }

                        Text {
                            text: root.tr( "In: %1" ).arg( root.destinationDisplayPath() )
                            font: Qt.font({ family: App.fontFamily || "Ubuntu", pixelSize: ( App.fontSize || 13 ) - 2 })
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

                    Text {
                        text: root.tr( "File Name" )
                        font: Qt.font({ family: App.fontFamily || "Ubuntu", pixelSize: ( App.fontSize || 13 ) - 1, bold: true })
                        color: appTheme.windowText
                        opacity: 0.85
                    }

                    Rectangle {
                        Layout.fillWidth: true
                        implicitHeight: 30
                        radius: 5
                        color: appTheme.base
                        border.width: newFileInput.activeFocus ? 2 : 1
                        border.color: newFileInput.activeFocus ? appTheme.highlight : appTheme.mid

                        RowLayout {
                            anchors.fill: parent
                            anchors.leftMargin: 6
                            anchors.rightMargin: 4
                            spacing: 4

                            AppIcon {
                                text: root.fileIconPreview( newFileInput.text )
                                font.pixelSize: 14
                                color: newFileInput.text.length > 0 ? appTheme.highlight : appTheme.windowText
                                opacity: newFileInput.text.length > 0 ? 1.0 : 0.45
                            }

                            AppTextField {
                                id: newFileInput
                                Layout.fillWidth: true
                                Layout.fillHeight: true
                                placeholderText: root.tr( "circuit.circ1, main.c..." )
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
                                    text: "close"
                                    font.pixelSize: 12
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

                    Text {
                        text: root.tr( "Type:" )
                        font: Qt.font({ family: App.fontFamily || "Ubuntu", pixelSize: ( App.fontSize || 13 ) - 2 })
                        color: appTheme.windowText
                        opacity: 0.6
                    }

                    Repeater {
                        model: [ ".circ1", ".sim2", ".c", ".h", ".txt" ]
                        delegate: Rectangle {
                            id: extChip
                            required property string modelData
                            implicitWidth: chipText.implicitWidth + 8
                            implicitHeight: 18
                            radius: 3
                            color: chipHover.hovered
                                   ? Qt.rgba( appTheme.highlight.r, appTheme.highlight.g, appTheme.highlight.b, 0.25 )
                                   : Qt.rgba( appTheme.mid.r, appTheme.mid.g, appTheme.mid.b, 0.2 )
                            border.color: newFileInput.text.endsWith( modelData )
                                          ? appTheme.highlight
                                          : Qt.rgba( appTheme.mid.r, appTheme.mid.g, appTheme.mid.b, 0.4 )
                            border.width: 1

                            Text {
                                id: chipText
                                anchors.centerIn: parent
                                text: extChip.modelData
                                font: Qt.font({ family: App.fontFamily || "Ubuntu", pixelSize: ( App.fontSize || 13 ) - 2, bold: newFileInput.text.endsWith( extChip.modelData ) })
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

                Text {
                    id: fileValidationText
                    Layout.fillWidth: true
                    visible: text.length > 0
                    text: root.validateFileName( newFileInput.text, false )
                    font: Qt.font({ family: App.fontFamily || "Ubuntu", pixelSize: ( App.fontSize || 13 ) - 2 })
                    color: CircuitCanvas.msgErrorBg || Qt.rgba( 0.9, 0.3, 0.3, 1.0 )
                    wrapMode: Text.WordWrap
                }

                RowLayout {
                    Layout.fillWidth: true
                    Layout.topMargin: 2
                    spacing: 8

                    Item { Layout.fillWidth: true }

                    AppButton {
                        text: root.tr( "Cancel" )
                        font: Qt.font({ family: App.fontFamily || "Ubuntu", pixelSize: ( App.fontSize || 13 ) - 1 })
                        implicitHeight: 26
                        leftPadding: 12
                        rightPadding: 12

                        background: Rectangle {
                            radius: 4
                            color: parent.down ? appTheme.dark : ( parent.hovered ? appTheme.midlight : appTheme.button )
                            border.color: appTheme.mid
                            border.width: 1
                        }
                        contentItem: Text {
                            text: parent.text
                            font: parent.font
                            color: appTheme.buttonText
                            horizontalAlignment: Text.AlignHCenter
                            verticalAlignment: Text.AlignVCenter
                        }
                        onClicked: createFileDialog.close()
                    }

                    AppButton {
                        id: createFileBtn
                        text: root.tr( "Create" )
                        enabled: newFileInput.text.trim().length > 0 && fileValidationText.text.length === 0
                        font: Qt.font({ family: App.fontFamily || "Ubuntu", pixelSize: ( App.fontSize || 13 ) - 1, bold: true })
                        implicitHeight: 26
                        leftPadding: 14
                        rightPadding: 14

                        background: Rectangle {
                            radius: 4
                            color: !createFileBtn.enabled
                                   ? Qt.rgba( appTheme.mid.r, appTheme.mid.g, appTheme.mid.b, 0.4 )
                                   : ( createFileBtn.down
                                       ? Qt.darker( appTheme.highlight, 1.2 )
                                       : ( createFileBtn.hovered ? Qt.lighter( appTheme.highlight, 1.1 ) : appTheme.highlight ) )
                            border.color: createFileBtn.enabled ? Qt.darker( appTheme.highlight, 1.2 ) : "transparent"
                            border.width: 1
                        }
                        contentItem: Text {
                            text: parent.text
                            font: parent.font
                            color: createFileBtn.enabled ? appTheme.highlightedText : Qt.rgba( appTheme.buttonText.r, appTheme.buttonText.g, appTheme.buttonText.b, 0.5 )
                            horizontalAlignment: Text.AlignHCenter
                            verticalAlignment: Text.AlignVCenter
                        }
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
        closePolicy: Popup.CloseOnEscape
        padding: 0
        header: null
        footer: null
        width: Math.min( Math.max( root.width - 16, 220 ), 300 )
        x: Math.round( ( root.width - width ) / 2 )
        y: Math.max( 16, Math.round( ( root.height - height ) / 3 ) )

        background: Rectangle {
            color: appTheme.window
            border.color: appTheme.mid
            border.width: 1
            radius: 10
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
                        radius: 5
                        color: Qt.rgba( appTheme.highlight.r, appTheme.highlight.g, appTheme.highlight.b, 0.18 )

                        AppIcon {
                            anchors.centerIn: parent
                            text: "create_new_folder"
                            color: appTheme.highlight
                            font.pixelSize: 15
                        }
                    }

                    Text {
                        text: root.tr( "Create Folder" )
                        font: Qt.font({ family: App.fontFamily || "Ubuntu", pixelSize: ( App.fontSize || 13 ) + 1, bold: true })
                        color: appTheme.windowText
                        Layout.fillWidth: true
                        elide: Text.ElideRight
                    }

                    AppToolButton {
                        implicitWidth: 22
                        implicitHeight: 22
                        padding: 0
                        contentItem: AppIcon {
                            text: "close"
                            font.pixelSize: 14
                            color: appTheme.windowText
                            opacity: 0.6
                        }
                        onClicked: createFolderDialog.close()
                    }
                }

                Rectangle {
                    Layout.fillWidth: true
                    implicitHeight: 22
                    radius: 4
                    color: Qt.rgba( appTheme.mid.r, appTheme.mid.g, appTheme.mid.b, 0.18 )
                    border.color: Qt.rgba( appTheme.mid.r, appTheme.mid.g, appTheme.mid.b, 0.35 )
                    border.width: 1

                    RowLayout {
                        anchors.fill: parent
                        anchors.leftMargin: 6
                        anchors.rightMargin: 6
                        spacing: 4

                        AppIcon {
                            text: "folder"
                            font.pixelSize: 12
                            color: appTheme.windowText
                            opacity: 0.7
                        }

                        Text {
                            text: root.tr( "In: %1" ).arg( root.destinationDisplayPath() )
                            font: Qt.font({ family: App.fontFamily || "Ubuntu", pixelSize: ( App.fontSize || 13 ) - 2 })
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

                    Text {
                        text: root.tr( "Folder Name" )
                        font: Qt.font({ family: App.fontFamily || "Ubuntu", pixelSize: ( App.fontSize || 13 ) - 1, bold: true })
                        color: appTheme.windowText
                        opacity: 0.85
                    }

                    Rectangle {
                        Layout.fillWidth: true
                        implicitHeight: 30
                        radius: 5
                        color: appTheme.base
                        border.width: newDirInput.activeFocus ? 2 : 1
                        border.color: newDirInput.activeFocus ? appTheme.highlight : appTheme.mid

                        RowLayout {
                            anchors.fill: parent
                            anchors.leftMargin: 6
                            anchors.rightMargin: 4
                            spacing: 4

                            AppIcon {
                                text: "folder"
                                font.pixelSize: 14
                                color: newDirInput.text.length > 0 ? appTheme.highlight : appTheme.windowText
                                opacity: newDirInput.text.length > 0 ? 1.0 : 0.45
                            }

                            AppTextField {
                                id: newDirInput
                                Layout.fillWidth: true
                                Layout.fillHeight: true
                                placeholderText: root.tr( "components, subcircuits..." )
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
                                    text: "close"
                                    font.pixelSize: 12
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

                Text {
                    id: dirValidationText
                    Layout.fillWidth: true
                    visible: text.length > 0
                    text: root.validateFileName( newDirInput.text, true )
                    font: Qt.font({ family: App.fontFamily || "Ubuntu", pixelSize: ( App.fontSize || 13 ) - 2 })
                    color: CircuitCanvas.msgErrorBg || Qt.rgba( 0.9, 0.3, 0.3, 1.0 )
                    wrapMode: Text.WordWrap
                }

                RowLayout {
                    Layout.fillWidth: true
                    Layout.topMargin: 2
                    spacing: 8

                    Item { Layout.fillWidth: true }

                    AppButton {
                        text: root.tr( "Cancel" )
                        font: Qt.font({ family: App.fontFamily || "Ubuntu", pixelSize: ( App.fontSize || 13 ) - 1 })
                        implicitHeight: 26
                        leftPadding: 12
                        rightPadding: 12

                        background: Rectangle {
                            radius: 4
                            color: parent.down ? appTheme.dark : ( parent.hovered ? appTheme.midlight : appTheme.button )
                            border.color: appTheme.mid
                            border.width: 1
                        }
                        contentItem: Text {
                            text: parent.text
                            font: parent.font
                            color: appTheme.buttonText
                            horizontalAlignment: Text.AlignHCenter
                            verticalAlignment: Text.AlignVCenter
                        }
                        onClicked: createFolderDialog.close()
                    }

                    AppButton {
                        id: createFolderBtn
                        text: root.tr( "Create" )
                        enabled: newDirInput.text.trim().length > 0 && dirValidationText.text.length === 0
                        font: Qt.font({ family: App.fontFamily || "Ubuntu", pixelSize: ( App.fontSize || 13 ) - 1, bold: true })
                        implicitHeight: 26
                        leftPadding: 14
                        rightPadding: 14

                        background: Rectangle {
                            radius: 4
                            color: !createFolderBtn.enabled
                                   ? Qt.rgba( appTheme.mid.r, appTheme.mid.g, appTheme.mid.b, 0.4 )
                                   : ( createFolderBtn.down
                                       ? Qt.darker( appTheme.highlight, 1.2 )
                                       : ( createFolderBtn.hovered ? Qt.lighter( appTheme.highlight, 1.1 ) : appTheme.highlight ) )
                            border.color: createFolderBtn.enabled ? Qt.darker( appTheme.highlight, 1.2 ) : "transparent"
                            border.width: 1
                        }
                        contentItem: Text {
                            text: parent.text
                            font: parent.font
                            color: createFolderBtn.enabled ? appTheme.highlightedText : Qt.rgba( appTheme.buttonText.r, appTheme.buttonText.g, appTheme.buttonText.b, 0.5 )
                            horizontalAlignment: Text.AlignHCenter
                            verticalAlignment: Text.AlignVCenter
                        }
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
        closePolicy: Popup.CloseOnEscape
        padding: 0
        header: null
        footer: null
        width: Math.min( Math.max( root.width - 16, 220 ), 300 )
        x: Math.round( ( root.width - width ) / 2 )
        y: Math.max( 16, Math.round( ( root.height - height ) / 3 ) )

        background: Rectangle {
            color: appTheme.window
            border.color: appTheme.mid
            border.width: 1
            radius: 10
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
                        radius: 5
                        color: Qt.rgba( appTheme.highlight.r, appTheme.highlight.g, appTheme.highlight.b, 0.18 )

                        AppIcon {
                            anchors.centerIn: parent
                            text: "edit"
                            color: appTheme.highlight
                            font.pixelSize: 15
                        }
                    }

                    Text {
                        text: root.tr( "Rename" )
                        font: Qt.font({ family: App.fontFamily || "Ubuntu", pixelSize: ( App.fontSize || 13 ) + 1, bold: true })
                        color: appTheme.windowText
                        Layout.fillWidth: true
                        elide: Text.ElideRight
                    }

                    AppToolButton {
                        implicitWidth: 22
                        implicitHeight: 22
                        padding: 0
                        contentItem: AppIcon {
                            text: "close"
                            font.pixelSize: 14
                            color: appTheme.windowText
                            opacity: 0.6
                        }
                        onClicked: renameDialog.close()
                    }
                }

                Rectangle {
                    Layout.fillWidth: true
                    implicitHeight: 22
                    radius: 4
                    color: Qt.rgba( appTheme.mid.r, appTheme.mid.g, appTheme.mid.b, 0.18 )
                    border.color: Qt.rgba( appTheme.mid.r, appTheme.mid.g, appTheme.mid.b, 0.35 )
                    border.width: 1

                    RowLayout {
                        anchors.fill: parent
                        anchors.leftMargin: 6
                        anchors.rightMargin: 6
                        spacing: 4

                        AppIcon {
                            text: root.selectedIsDir ? "folder" : root.fileIconPreview( renameInput.text )
                            font.pixelSize: 12
                            color: appTheme.windowText
                            opacity: 0.7
                        }

                        Text {
                            text: root.tr( "Target: %1" ).arg( root.renameTargetName() )
                            font: Qt.font({ family: App.fontFamily || "Ubuntu", pixelSize: ( App.fontSize || 13 ) - 2 })
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

                    Text {
                        text: root.tr( "New Name" )
                        font: Qt.font({ family: App.fontFamily || "Ubuntu", pixelSize: ( App.fontSize || 13 ) - 1, bold: true })
                        color: appTheme.windowText
                        opacity: 0.85
                    }

                    Rectangle {
                        Layout.fillWidth: true
                        implicitHeight: 30
                        radius: 5
                        color: appTheme.base
                        border.width: renameInput.activeFocus ? 2 : 1
                        border.color: renameInput.activeFocus ? appTheme.highlight : appTheme.mid

                        RowLayout {
                            anchors.fill: parent
                            anchors.leftMargin: 6
                            anchors.rightMargin: 4
                            spacing: 4

                            AppIcon {
                                text: root.selectedIsDir ? "folder" : root.fileIconPreview( renameInput.text )
                                font.pixelSize: 14
                                color: renameInput.text.length > 0 ? appTheme.highlight : appTheme.windowText
                                opacity: renameInput.text.length > 0 ? 1.0 : 0.45
                            }

                            AppTextField {
                                id: renameInput
                                Layout.fillWidth: true
                                Layout.fillHeight: true
                                placeholderText: root.tr( "New name..." )
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
                                    text: "close"
                                    font.pixelSize: 12
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

                Text {
                    id: renameValidationText
                    Layout.fillWidth: true
                    visible: text.length > 0
                    text: root.validateRename( renameInput.text )
                    font: Qt.font({ family: App.fontFamily || "Ubuntu", pixelSize: ( App.fontSize || 13 ) - 2 })
                    color: CircuitCanvas.msgErrorBg || Qt.rgba( 0.9, 0.3, 0.3, 1.0 )
                    wrapMode: Text.WordWrap
                }

                RowLayout {
                    Layout.fillWidth: true
                    Layout.topMargin: 2
                    spacing: 8

                    Item { Layout.fillWidth: true }

                    AppButton {
                        text: root.tr( "Cancel" )
                        font: Qt.font({ family: App.fontFamily || "Ubuntu", pixelSize: ( App.fontSize || 13 ) - 1 })
                        implicitHeight: 26
                        leftPadding: 12
                        rightPadding: 12

                        background: Rectangle {
                            radius: 4
                            color: parent.down ? appTheme.dark : ( parent.hovered ? appTheme.midlight : appTheme.button )
                            border.color: appTheme.mid
                            border.width: 1
                        }
                        contentItem: Text {
                            text: parent.text
                            font: parent.font
                            color: appTheme.buttonText
                            horizontalAlignment: Text.AlignHCenter
                            verticalAlignment: Text.AlignVCenter
                        }
                        onClicked: renameDialog.close()
                    }

                    AppButton {
                        id: renameBtn
                        text: root.tr( "Rename" )
                        enabled: renameInput.text.trim().length > 0 && renameValidationText.text.length === 0
                        font: Qt.font({ family: App.fontFamily || "Ubuntu", pixelSize: ( App.fontSize || 13 ) - 1, bold: true })
                        implicitHeight: 26
                        leftPadding: 14
                        rightPadding: 14

                        background: Rectangle {
                            radius: 4
                            color: !renameBtn.enabled
                                   ? Qt.rgba( appTheme.mid.r, appTheme.mid.g, appTheme.mid.b, 0.4 )
                                   : ( renameBtn.down
                                       ? Qt.darker( appTheme.highlight, 1.2 )
                                       : ( renameBtn.hovered ? Qt.lighter( appTheme.highlight, 1.1 ) : appTheme.highlight ) )
                            border.color: renameBtn.enabled ? Qt.darker( appTheme.highlight, 1.2 ) : "transparent"
                            border.width: 1
                        }
                        contentItem: Text {
                            text: parent.text
                            font: parent.font
                            color: renameBtn.enabled ? appTheme.highlightedText : Qt.rgba( appTheme.buttonText.r, appTheme.buttonText.g, appTheme.buttonText.b, 0.5 )
                            horizontalAlignment: Text.AlignHCenter
                            verticalAlignment: Text.AlignVCenter
                        }
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

    ColumnLayout {
        anchors.fill: parent
        spacing: 4

        RowLayout {
            id: toolbar
            visible: FileBrowser.hasProject
            Layout.fillWidth: true
            Layout.preferredHeight: 28
            spacing: 0

            AppToolButton {
                implicitWidth: 28; implicitHeight: 28
                contentItem: AppIcon { text: "note_add"; color: appTheme.windowText }
                ToolTip.text: root.tr( "New File" )
                ToolTip.visible: hovered
                onClicked: root.openCreateFileDialog()
            }
            AppToolButton {
                implicitWidth: 28; implicitHeight: 28
                contentItem: AppIcon { text: "create_new_folder"; color: appTheme.windowText }
                ToolTip.text: root.tr( "New Folder" )
                ToolTip.visible: hovered
                onClicked: root.openCreateFolderDialog()
            }
            AppToolButton {
                implicitWidth: 28; implicitHeight: 28
                contentItem: AppIcon { text: "refresh"; color: appTheme.windowText }
                ToolTip.text: root.tr( "Refresh" )
                ToolTip.visible: hovered
                onClicked: FileBrowser.reload()
            }
            AppToolButton {
                implicitWidth: 28; implicitHeight: 28
                contentItem: AppIcon { text: "expand_more"; color: appTheme.windowText }
                ToolTip.text: root.tr( "Expand All" )
                ToolTip.visible: hovered
                onClicked: FileBrowser.expandAll()
            }
            AppToolButton {
                implicitWidth: 28; implicitHeight: 28
                contentItem: AppIcon { text: "expand_less"; color: appTheme.windowText }
                ToolTip.text: root.tr( "Collapse All" )
                ToolTip.visible: hovered
                onClicked: FileBrowser.collapseAll()
            }
            AppToolButton {
                implicitWidth: 28; implicitHeight: 28
                contentItem: AppIcon { text: "folder_open"; color: appTheme.windowText }
                ToolTip.text: root.tr( "Open Project Folder" )
                ToolTip.visible: hovered
                onClicked: folderDialog.open()
            }
            Item { Layout.fillWidth: true }
        }

        Rectangle {
            Layout.fillWidth: true
            Layout.fillHeight: true
            radius: 8
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

                    Label {
                        text: root.tr( "Open a project folder to load the file explorer." )
                        font: Qt.font({ family: App.fontFamily, pixelSize: App.fontSize })
                        wrapMode: Text.WordWrap
                        horizontalAlignment: Text.AlignHCenter
                        color: appTheme.windowText
                        Layout.fillWidth: true
                    }
                    AppButton {
                        text: root.tr( "Open Folder" )
                        font: Qt.font({ family: App.fontFamily, pixelSize: App.fontSize })
                        Layout.alignment: Qt.AlignHCenter
                        onClicked: folderDialog.open()
                    }
                }
            }

            MouseArea {
                anchors.fill: parent
                visible: FileBrowser.hasProject
                acceptedButtons: Qt.LeftButton | Qt.RightButton
                onClicked: ( mouse ) => {
                    root.selectedPath = FileBrowser.rootPath
                    root.selectedIsDir = true
                    if ( mouse.button === Qt.RightButton ) {
                        contextMenu.popup()
                    }
                }
            }

            ListView {
                id: listView
                visible: FileBrowser.hasProject
                anchors.fill: parent
                anchors.margins: 1
                model: entryModel
                clip: true
                boundsBehavior: Flickable.StopAtBounds
                ScrollBar.vertical: AppScrollBar {}
                ScrollBar.horizontal: AppScrollBar {}

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
                        radius: 4
                        visible: hoverHandler.hovered || model.path === root.selectedPath
                        color: hoverHandler.hovered
                               ? appTheme.highlight
                               : Qt.rgba( appTheme.highlight.r, appTheme.highlight.g, appTheme.highlight.b, 0.22 )
                    }

                    Row {
                        anchors.fill: parent
                        anchors.leftMargin: 6 + model.depth * 14
                        anchors.rightMargin: 6
                        spacing: 6

                        Item {
                            width: 14
                            height: parent.height
                            anchors.verticalCenter: parent.verticalCenter

                            AppIcon {
                                anchors.centerIn: parent
                                visible: model.isDir
                                text: model.expanded ? "expand_more" : "chevron_right"
                                font.pixelSize: 16
                                color: hoverHandler.hovered
                                       ? appTheme.highlightedText
                                       : ( model.path === root.selectedPath ? appTheme.highlight : appTheme.windowText )
                            }

                            TapHandler {
                                enabled: model.isDir
                                acceptedButtons: Qt.LeftButton
                                onTapped: {
                                    root.selectedPath = model.path
                                    root.selectedIsDir = true
                                    FileBrowser.toggleExpand( model.path )
                                }
                            }
                        }

                        AppIcon {
                            anchors.verticalCenter: parent.verticalCenter
                            text: root.fileIconForName( model.name, model.isDir, model.expanded )
                            color: hoverHandler.hovered
                                   ? appTheme.highlightedText
                                   : ( model.path === root.selectedPath ? appTheme.highlight : appTheme.windowText )
                        }

                        Text {
                            anchors.verticalCenter: parent.verticalCenter
                            text: model.name
                            font: Qt.font({ family: App.fontFamily, pixelSize: 12 })
                            color: hoverHandler.hovered
                                   ? appTheme.highlightedText
                                   : ( model.path === root.selectedPath ? appTheme.highlight : appTheme.windowText )
                            elide: Text.ElideRight
                            width: Math.max( 0, parent.width - 48 )
                        }
                    }

                    HoverHandler { id: hoverHandler }

                    TapHandler {
                        acceptedButtons: Qt.LeftButton
                        onTapped: {
                            root.selectedPath = model.path
                            root.selectedIsDir = model.isDir
                        }
                        onDoubleTapped: {
                            root.selectedPath = model.path
                            root.selectedIsDir = model.isDir
                            if ( model.isDir ) FileBrowser.toggleExpand( model.path )
                            else FileBrowser.openPath( model.path )
                        }
                    }

                    TapHandler {
                        acceptedButtons: Qt.RightButton
                        onTapped: {
                            root.selectedPath = model.path
                            root.selectedIsDir = model.isDir
                            contextMenu.popup()
                        }
                    }
                }
            }
        }
    }
}

