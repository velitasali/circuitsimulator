import QtQuick
import cs_ui

/* The context menu a TextField or TextArea raises, drawn like every other menu
 * in the app.
 */
AppContextMenu {
    id: menu

    property TextInput textInput: null
    property TextEdit textEdit: null

    readonly property bool _hasSelection: textInput
        ? (textInput.selectionStart !== textInput.selectionEnd)
        : (textEdit ? (textEdit.selectionStart !== textEdit.selectionEnd) : false)
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
        shortcutText: UiUtils.formatShortcut("Ctrl+Z")
        enabled: menu._editable && menu._canUndo
        onTriggered: menu.execEditAction("undo")
    }
    ContextMenuItem {
        iconLigature: "redo"
        text: I18n.tr("Redo")
        shortcutText: UiUtils.formatShortcut(UiUtils.isMacOS ? "Ctrl+Shift+Z" : "Ctrl+Y")
        enabled: menu._editable && menu._canRedo
        onTriggered: menu.execEditAction("redo")
    }

    ContextMenuSeparator {}

    ContextMenuItem {
        iconLigature: "content_cut"
        text: I18n.tr("Cut")
        shortcutText: UiUtils.formatShortcut("Ctrl+X")
        enabled: menu._editable && menu._hasSelection
        onTriggered: menu.execEditAction("cut")
    }
    ContextMenuItem {
        iconLigature: "content_copy"
        text: I18n.tr("Copy")
        shortcutText: UiUtils.formatShortcut("Ctrl+C")
        enabled: menu._hasSelection
        onTriggered: menu.execEditAction("copy")
    }
    ContextMenuItem {
        iconLigature: "content_paste"
        text: I18n.tr("Paste")
        shortcutText: UiUtils.formatShortcut("Ctrl+V")
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
        shortcutText: UiUtils.formatShortcut("Ctrl+A")
        enabled: menu._hasText
        onTriggered: menu.execEditAction("selectAll")
    }
}
