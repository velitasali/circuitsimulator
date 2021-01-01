import QtQuick
import QtQuick.Controls
import cs_app

/* The context menu a TextField or TextArea raises, drawn like every other menu
 * in the app.
 */
AppContextMenu {
    id: menu
    function tr(s) { return App.i18nTick >= 0 ? App.translate(s) : s }

    required property var editor

    // Display text only, as everywhere else -- these bindings are the editor's own,
    // not shortcuts registered here.
    readonly property bool _mac: Qt.platform.os === "osx" || Qt.platform.os === "macos"
    readonly property string _mod: _mac ? "⌘" : "Ctrl+"

    readonly property bool _hasSelection: editor && editor.selectedText && editor.selectedText.length > 0
    readonly property bool _editable: editor && !editor.readOnly

    ContextMenuItem {
        iconLigature: "undo"
        text: menu.tr("Undo")
        shortcutText: menu._mod + "Z"
        enabled: menu._editable && (menu.editor ? menu.editor.canUndo : false)
        onTriggered: if (menu.editor) menu.editor.undo()
    }
    ContextMenuItem {
        iconLigature: "redo"
        text: menu.tr("Redo")
        shortcutText: menu._mac ? "⇧⌘Z" : "Ctrl+Y"
        enabled: menu._editable && (menu.editor ? menu.editor.canRedo : false)
        onTriggered: if (menu.editor) menu.editor.redo()
    }

    ContextMenuSeparator {}

    ContextMenuItem {
        iconLigature: "content_cut"
        text: menu.tr("Cut")
        shortcutText: menu._mod + "X"
        enabled: menu._editable && menu._hasSelection
        onTriggered: if (menu.editor) menu.editor.cut()
    }
    ContextMenuItem {
        iconLigature: "content_copy"
        text: menu.tr("Copy")
        shortcutText: menu._mod + "C"
        enabled: menu._hasSelection
        onTriggered: if (menu.editor) menu.editor.copy()
    }
    ContextMenuItem {
        iconLigature: "content_paste"
        text: menu.tr("Paste")
        shortcutText: menu._mod + "V"
        enabled: menu._editable && (menu.editor ? menu.editor.canPaste : false)
        onTriggered: if (menu.editor) menu.editor.paste()
    }
    ContextMenuItem {
        iconLigature: "delete"
        text: menu.tr("Delete")
        enabled: menu._editable && menu._hasSelection
        onTriggered: if (menu.editor) menu.editor.remove(menu.editor.selectionStart, menu.editor.selectionEnd)
    }

    ContextMenuSeparator {}

    ContextMenuItem {
        iconLigature: "select_all"
        text: menu.tr("Select All")
        shortcutText: menu._mod + "A"
        enabled: menu.editor && menu.editor.length > 0
        onTriggered: if (menu.editor) menu.editor.selectAll()
    }
}
