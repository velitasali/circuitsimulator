pragma ComponentBehavior: Bound

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

    readonly property bool showLineMarkers: AppDialog.logWordWrap
    property var _lineMarkerRows: []
    property real _markerCacheTop: 0
    property real _markerCacheBottom: -1
    property bool _markerRefreshQueued: false

    function _isLineBreak(ch: string): bool {
        return ch === "\n" || ch === "\r" || ch === "\u2028" || ch === "\u2029"
    }

    function _invalidateLineMarkers(): void {
        _markerCacheBottom = -1
        _scheduleMarkerRefresh()
    }

    function _scheduleMarkerRefresh(): void {
        if (_markerRefreshQueued)
            return
        _markerRefreshQueued = true
        Qt.callLater(_updateLineMarkers)
    }

    function _onViewportScrolled(): void {
        if (!root.showLineMarkers)
            return
        var flick = area.flickable
        if (!flick)
            return
        if (flick.contentY >= _markerCacheTop && (flick.contentY + flick.height) <= _markerCacheBottom)
            return
        _updateLineMarkers()
    }

    function _updateLineMarkers(): void {
        _markerRefreshQueued = false
        if (!root.showLineMarkers) {
            if (_lineMarkerRows.length > 0)
                _lineMarkerRows = []
            _markerCacheBottom = -1
            return
        }
        var edit = area.editor
        var flick = area.flickable
        if (!edit || !flick || edit.length === 0 || flick.width <= 0 || flick.height <= 0) {
            if (_lineMarkerRows.length > 0)
                _lineMarkerRows = []
            _markerCacheBottom = -1
            return
        }

        var overscan = 160
        var topPad = edit.topPadding
        var leftPad = edit.leftPadding
        var scanTop = Math.max(topPad, flick.contentY - overscan)
        var scanBottom = flick.contentY + flick.height + overscan
        var probeY = scanTop + 1
        var lastRowY = -1e9
        var rows = []

        for (var guard = 0; guard < 2000 && probeY <= scanBottom; ++guard) {
            var hitPos = edit.positionAt(leftPad, probeY)
            var hitRect = edit.positionToRectangle(hitPos)
            if (hitRect.height <= 0)
                break
            if (probeY > hitRect.y + hitRect.height + 6)
                break
            if (hitRect.y <= lastRowY + 0.5)
                break
            lastRowY = hitRect.y

            var rowStartPos = edit.positionAt(leftPad, hitRect.y + hitRect.height * 0.5)
            var isLineStart = (rowStartPos === 0)
                              || _isLineBreak(edit.getText(rowStartPos - 1, rowStartPos))
            if (isLineStart && rowStartPos < edit.length) {
                var next = edit.getText(rowStartPos, rowStartPos + 1)
                if (!_isLineBreak(next)) {
                    rows.push({ y: hitRect.y, h: hitRect.height })
                }
            }

            var step = Math.max(1, Math.min(4, hitRect.height * 0.25))
            probeY = hitRect.y + hitRect.height + step
        }

        _markerCacheTop = Math.max(0, flick.contentY - overscan)
        _markerCacheBottom = flick.contentY + flick.height + overscan

        var same = rows.length === _lineMarkerRows.length
        if (same) {
            for (var i = 0; i < rows.length; ++i) {
                if (Math.abs(rows[i].y - _lineMarkerRows[i].y) > 0.5 || Math.abs(rows[i].h - _lineMarkerRows[i].h) > 0.5) {
                    same = false
                    break
                }
            }
        }
        if (!same)
            _lineMarkerRows = rows
    }

    onShowLineMarkersChanged: root._invalidateLineMarkers()
    onEditorFontChanged: root._invalidateLineMarkers()
    onCtxChanged: root._invalidateLineMarkers()
    Component.onCompleted: root._invalidateLineMarkers()

    ScrollableTextEdit {
        id: area
        anchors.fill: parent
        readOnly: true
        selectByMouse: true
        defaultContextMenu: false
        wordWrap: AppDialog.logWordWrap
        contentPadding: 9 // was document()->setDocumentMargin( 9 )
        leftContentPadding: root.showLineMarkers ? 20 : contentPadding
        textFormat: TextEdit.RichText

        font: root.editorFont
        color: (root.ctx && root.ctx.textColor && root.ctx.textColor.length > 0) ? root.ctx.textColor : appTheme.text
        placeholderTextColor: appTheme.placeholderText
        placeholderText: root.ctx ? I18n.tr(root.ctx.placeholder) : ""
        text: root.ctx ? root.ctx.text : ""

        Item {
            id: lineMarkerGutter
            visible: root.showLineMarkers && root._lineMarkerRows.length > 0
            x: 0
            y: 0
            width: area.editor.leftPadding
            height: area.editor.height
            z: 1

            Repeater {
                model: root._lineMarkerRows
                delegate: AppIcon {
                    required property var modelData
                    icon: "chevron_right"
                    size: 16
                    color: appTheme.placeholderText
                    opacity: 0.65
                    x: Math.round((lineMarkerGutter.width - width) / 2)
                    y: Math.round(modelData.y + (modelData.h - height) / 2)
                    scale: modelData.h < 15 ? Math.max(0.65, modelData.h / 15) : 1.0
                }
            }
        }

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

    Connections {
        target: area.editor
        function onTextChanged() { root._invalidateLineMarkers() }
        function onWidthChanged() { root._invalidateLineMarkers() }
        function onContentHeightChanged() { root._invalidateLineMarkers() }
        function onLineCountChanged() { root._invalidateLineMarkers() }
        function onLeftPaddingChanged() { root._invalidateLineMarkers() }
    }

    Connections {
        target: area.flickable
        function onWidthChanged() { root._invalidateLineMarkers() }
        function onHeightChanged() { root._invalidateLineMarkers() }
        function onContentYChanged() { root._onViewportScrolled() }
    }

    AppContextMenu {
        id: contextMenu
        ContextMenuItem {
            text: I18n.tr("Copy")
            iconLigature: "content_copy"
            shortcutText: UiUtils.formatShortcut("Ctrl+C")
            enabled: area.selectedText.length > 0
            onTriggered: area.copy()
        }
        ContextMenuItem {
            text: I18n.tr("Select All")
            iconLigature: "select_all"
            shortcutText: UiUtils.formatShortcut("Ctrl+A")
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
