import QtQuick
import cs_app
import cs_ui

// Log pane. The C++ side owns the document: it appends through a QTextCursor and
// OutHighlighter colours it, so this file only renders and reports hit positions.
Item {
    id: root
    anchors.fill: parent
    SystemPalette { id: appTheme }

    /* There are two of these -- the simulator log and the compiler log -- so the
     * backend cannot come in as a context property: one engine can only hold one
     * `outPanel`. It is a declared property instead, defaulting to the context
     * one so the existing QQuickWidget host keeps working unchanged.
     */
    property var ctx: null

    FontLoader { id: ubuntuMonoR; source: "qrc:/fonts/UbuntuMono-R.ttf" }
    readonly property int bundledMonoReady: ubuntuMonoR.status
    readonly property font editorFont: {
        var _ = bundledMonoReady
        var family = (ctx && ctx.fontFamily) ? ctx.fontFamily
                    : (EditorPanel.fontFamily || ubuntuMonoR.name || "Ubuntu Mono")
        var size = (ctx && ctx.fontSize) ? ctx.fontSize : (EditorPanel.fontSize || 14)
        return Qt.font({
            family: family,
            pixelSize: size,
            fixedPitch: true
        })
    }

    ScrollableTextEdit {
        id: area
        anchors.fill: parent
        readOnly: true
        selectByMouse: true
        wordWrap: AppDialog.logWordWrap
        contentPadding: 9 // was document()->setDocumentMargin( 9 )
        textFormat: TextEdit.RichText

        font: root.editorFont
        color: (root.ctx && root.ctx.textColor && root.ctx.textColor.length > 0) ? root.ctx.textColor : appTheme.text
        placeholderTextColor: appTheme.placeholderText
        placeholderText: root.ctx ? I18n.tr(root.ctx.placeholder) : ""
        text: root.ctx ? root.ctx.text : ""

        // A compiler error line ("file.c:42:") turns the cursor into a hand and
        // jumps to the source on double click - was mouseMoveEvent/mouseDoubleClickEvent.
        HoverHandler {
            id: hover
            cursorShape: ( root.ctx && root.ctx.isLocationAt( area.positionAt( point.position.x, point.position.y ) ) )
                         ? Qt.PointingHandCursor : Qt.IBeamCursor
        }

        TapHandler {
            acceptedButtons: Qt.LeftButton
            onDoubleTapped: ( point ) => {
                if (root.ctx) root.ctx.activateLocationAt( area.positionAt( point.position.x, point.position.y ) )
            }
        }
        TapHandler {
            acceptedButtons: Qt.RightButton
            onTapped: contextMenu.popup()
        }
    }

    AppContextMenu {
        id: contextMenu
        ContextMenuItem {
            text: I18n.tr("Copy")
            iconLigature: "content_copy"
            shortcutText: "Ctrl+C"
            enabled: area.selectedText.length > 0
            onTriggered: area.copy()
        }
        ContextMenuItem {
            text: I18n.tr("Select All")
            iconLigature: "select_all"
            shortcutText: "Ctrl+A"
            onTriggered: area.selectAll()
        }
        ContextMenuSeparator {}
        ContextMenuItem {
            text: I18n.tr("Wrap Text")
            checkable: true
            checked: AppDialog.logWordWrap
            onTriggered: AppDialog.logWordWrap = checked
            Binding on checked { value: AppDialog.logWordWrap }
        }
        ContextMenuSeparator {}
        ContextMenuItem {
            text: I18n.tr("Clear Log")
            iconLigature: "delete"
            onTriggered: if (root.ctx) root.ctx.clear()
        }
    }

    function scrollToTail(): void {
        area.scrollToTail()
    }

    Connections {
        target: root.ctx
        function onTextChanged() { root.scrollToTail(); }
        function onAppended() { root.scrollToTail(); }
        function onCopyRequested() { area.copy(); }
        function onSelectAllRequested() { area.selectAll(); }
    }
}
