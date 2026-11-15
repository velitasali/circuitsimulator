import QtQuick
import cs_ui

/* The context menu a TextField or TextArea raises, drawn like every other menu
 * in the app.
 */
AppContextMenu {
    id: menu

    property TextInput textInput: null
    property TextEdit textEdit: null

    // Display text only, as everywhere else -- these bindings are the editor's own,
    // not shortcuts registered here.
    readonly property bool _mac: Qt.platform.os === "osx" || Qt.platform.os === "macos"
    readonly property string _mod: _mac ? "⌘" : "Ctrl+"

    readonly property bool _hasSelection: textInput ? textInput.selectedText !== "" : (textEdit ? textEdit.selectedText !== "" : false)
    readonly property bool _editable: textInput ? !textInput.readOnly : (textEdit ? !textEdit.readOnly : false)
    readonly property bool _canUndo: textInput ? textInput.canUndo : (textEdit ? textEdit.canUndo : false)
    readonly property bool _canRedo: textInput ? textInput.canRedo : (textEdit ? textEdit.canRedo : false)
    readonly property bool _canPaste: textInput ? textInput.canPaste : (textEdit ? textEdit.canPaste : false)
    readonly property bool _hasText: textInput ? textInput.length > 0 : (textEdit ? textEdit.length > 0 : false)

    function execEditAction(action: string): void {
        if (textInput) {
            if (action === "undo") textInput.undo()
            else if (action === "redo") textInput.redo()
            else if (action === "cut") textInput.cut()
            else if (action === "copy") textInput.copy()
            else if (action === "paste") textInput.paste()
            else if (action === "remove") textInput.remove(textInput.selectionStart, textInput.selectionEnd)
            else if (action === "selectAll") textInput.selectAll()
        } else if (textEdit) {
            if (action === "undo") textEdit.undo()
            else if (action === "redo") textEdit.redo()
            else if (action === "cut") textEdit.cut()
            else if (action === "copy") textEdit.copy()
            else if (action === "paste") textEdit.paste()
            else if (action === "remove") textEdit.remove(textEdit.selectionStart, textEdit.selectionEnd)
            else if (action === "selectAll") textEdit.selectAll()
        }
    }

    ContextMenuItem {
        iconLigature: "undo"
        text: I18n.tr("Undo")
        shortcutText: menu._mod + "Z"
        enabled: menu._editable && menu._canUndo
        onTriggered: menu.execEditAction("undo")
    }
    ContextMenuItem {
        iconLigature: "redo"
        text: I18n.tr("Redo")
        shortcutText: menu._mac ? "⇧⌘Z" : "Ctrl+Y"
        enabled: menu._editable && menu._canRedo
        onTriggered: menu.execEditAction("redo")
    }

    ContextMenuSeparator {}

    ContextMenuItem {
        iconLigature: "content_cut"
        text: I18n.tr("Cut")
        shortcutText: menu._mod + "X"
        enabled: menu._editable && menu._hasSelection
        onTriggered: menu.execEditAction("cut")
    }
    ContextMenuItem {
        iconLigature: "content_copy"
        text: I18n.tr("Copy")
        shortcutText: menu._mod + "C"
        enabled: menu._hasSelection
        onTriggered: menu.execEditAction("copy")
    }
    ContextMenuItem {
        iconLigature: "content_paste"
        text: I18n.tr("Paste")
        shortcutText: menu._mod + "V"
        enabled: menu._editable && menu._canPaste
        onTriggered: menu.execEditAction("paste")
    }
    ContextMenuItem {
        iconLigature: "delete"
        text: I18n.tr("Delete")
        enabled: menu._editable && menu._hasSelection
        onTriggered: menu.execEditAction("remove")
    }

    ContextMenuSeparator {}

    ContextMenuItem {
        iconLigature: "select_all"
        text: I18n.tr("Select All")
        shortcutText: menu._mod + "A"
        enabled: menu._hasText
        onTriggered: menu.execEditAction("selectAll")
    }
}
