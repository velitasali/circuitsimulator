import QtQuick
import QtQuick.Controls
import cs_app

// Log pane. The C++ side owns the document: it appends through a QTextCursor and
// OutHighlighter colours it, so this file only renders and reports hit positions.
Item {
    id: root
    anchors.fill: parent
    SystemPalette { id: appTheme }
    function tr(s) { return App.i18nTick >= 0 ? App.translate(s) : s }

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
            fixedPitch: true,
            styleHint: Font.Monospace
        })
    }

    // A bare Flickable + TextArea.flickable, not ScrollView { TextArea {} } --
    // that combination reserved real scrollbar-width padding it never drew
    // text into. codeeditor.qml already uses this pattern; matching it here
    // fixed that half.
    Flickable {
        id: flick
        anchors.fill: parent
        clip: true
        boundsBehavior: Flickable.StopAtBounds
        contentWidth: area.implicitWidth
        contentHeight: area.implicitHeight

        /* TextEdit's large-document support only re-renders the glyphs it
         * thinks are visible, and it recomputes that range off contentX/
         * contentY, not off the Flickable's own width/height -- so widening
         * the panel updates flick.width immediately (verified: flick/root
         * track the live resize correctly) but the newly exposed strip stays
         * blank, already-rendered pixels, until something nudges contentX/Y
         * and forces a recompute. That's exactly what scrolling did by
         * accident. Do the same nudge deliberately on every resize: a
         * same-tick round trip changes contentX twice without an intervening
         * frame, so nothing actually moves on screen.
         */
        onWidthChanged: { contentX += 1; contentX -= 1 }
        onHeightChanged: { contentY += 1; contentY -= 1 }

        ScrollBar.vertical: AppScrollBar { policy: ScrollBar.AsNeeded }
        ScrollBar.horizontal: AppScrollBar { policy: ScrollBar.AsNeeded }

        TextArea.flickable: TextArea {
            id: area
            // Not an AppTextArea: the panel raises its own menu from
            // OutPanelText::showContextMenu(), so Qt's built-in one is dropped
            // outright rather than restyled -- two menus would answer one click.
            ContextMenu.menu: null
            readOnly: true
            selectByMouse: true
            wrapMode: TextEdit.NoWrap
            padding: 9 // was document()->setDocumentMargin( 9 )
            textFormat: TextEdit.RichText

            font: root.editorFont
            color: (ctx && ctx.textColor && ctx.textColor.length > 0) ? ctx.textColor : appTheme.text
            placeholderTextColor: appTheme.placeholderText
            placeholderText: ctx ? root.tr(ctx.placeholder) : ""
            text: ctx ? ctx.text : ""
            background: null

            // A compiler error line ("file.c:42:") turns the cursor into a hand and
            // jumps to the source on double click - was mouseMoveEvent/mouseDoubleClickEvent.
            HoverHandler {
                id: hover
                cursorShape: ( ctx && ctx.isLocationAt( area.positionAt( point.position.x, point.position.y ) ) )
                             ? Qt.PointingHandCursor : Qt.IBeamCursor
            }

            TapHandler {
                acceptedButtons: Qt.LeftButton
                onDoubleTapped: ( point ) =>
                    ctx.activateLocationAt( area.positionAt( point.position.x, point.position.y ) )
            }
            TapHandler {
                acceptedButtons: Qt.RightButton
                onTapped: contextMenu.popup()
            }
        }
    }

    AppContextMenu {
        id: contextMenu
        ContextMenuItem {
            text: root.tr("Copy")
            iconLigature: "content_copy"
            shortcutText: "Ctrl+C"
            enabled: area.selectedText.length > 0
            onTriggered: area.copy()
        }
        ContextMenuItem {
            text: root.tr("Select All")
            iconLigature: "select_all"
            shortcutText: "Ctrl+A"
            onTriggered: area.selectAll()
        }
        ContextMenuSeparator {}
        ContextMenuItem {
            text: root.tr("Clear Log")
            iconLigature: "delete"
            onTriggered: if (ctx) ctx.clear()
        }
    }

    function scrollToTail() {
        if (flick.contentHeight > flick.height) {
            flick.contentY = flick.contentHeight - flick.height
        }
        flick.contentX = 0
        Qt.callLater(function() {
            if (flick.contentHeight > flick.height) {
                flick.contentY = flick.contentHeight - flick.height
            }
            flick.contentX = 0
        })
    }

    Connections {
        target: root.ctx
        // Follow the tail vertically without altering horizontal scroll.
        function onTextChanged() { root.scrollToTail(); }
        function onAppended() { root.scrollToTail(); }
        function onCopyRequested() { area.copy(); }
        function onSelectAllRequested() { area.selectAll(); }
    }
}
