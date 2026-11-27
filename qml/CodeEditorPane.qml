pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls
import cs_app

/* Pooled per-document Flickable + TextEdit surface owned by CodeEditor.
 * Keeping one QTextDocument per visited tab preserves cursor position,
 * selection, scroll offsets, and native Qt Undo/Redo across tab switches.
 * Uses plain TextEdit rather than TextArea.flickable so selection and text
 * are not clipped to viewport padding margins when scrolled horizontally or
 * vertically (which left the syntax-highlight layer peeking out unselected). */
Item {
    id: pane
    SystemPalette { id: appTheme }

    property int docId: -1
    property bool isActive: false
    property font editorFont
    property real lineHeight: 18
    property int cursorLine: 0
    property real gutterWidth: 52
    property real averageCharacterWidth: 8
    property bool hoverTooltipVisible: false
    property rect hoverTooltipRect: Qt.rect(0, 0, 0, 0)
    property bool _pushingToModel: false
    property bool _pendingCursorSync: false
    property int _hlWinStart: 0
    property int _hlWinEnd: 0

    visible: isActive
    enabled: isActive

    readonly property alias flick: flick
    readonly property alias area: area
    readonly property bool editorHovered: editorMouseArea.containsMouse

    signal hideHoverRequested()
    signal editorExited()
    signal hoverTargetRequested(real targetX, real targetY, real targetHeight, int pos)
    signal contextMenuRequested()
    signal completionAccepted(var edit)

    function syncTextFromModel(modelText: string): void {
        if (area.text !== modelText) {
            pane._pendingCursorSync = false
            area._syncingFromModel = true
            area.text = modelText
            area._syncingFromModel = false
            if (pane.isActive)
                pane.syncHighlightViewport(true)
        }
    }

    function _flushCursorPos(): void {
        if (!pane._pendingCursorSync || !pane.isActive || area._syncingFromModel || area._batchEditing)
            return
        pane._pendingCursorSync = false
        EditorPanel.cursorPos = area.cursorPosition
    }

    function pushBufferToModel(): void {
        if (!pane.isActive || area._syncingFromModel || area._batchEditing)
            return
        pane._pendingCursorSync = false
        pane._pushingToModel = true
        EditorPanel.updateText(area.text, area.cursorPosition)
        pane._pushingToModel = false
    }

    function syncHighlightViewport(force: bool): void {
        if (!pane.isActive)
            return
        var lh = Math.max(1, pane.lineHeight)
        var total = Math.max(1, area.lineCount)
        if (total <= 200) {
            if (force || pane._hlWinStart !== 0 || pane._hlWinEnd !== total) {
                pane._hlWinStart = 0
                pane._hlWinEnd = total
                EditorPanel.setHighlightViewport(0, total)
            }
            return
        }
        var firstVis = Math.max(0, Math.floor((flick.contentY - area.topPadding) / lh))
        var visLines = Math.max(1, Math.ceil(flick.height / lh))
        var lastVis = Math.min(total - 1, firstVis + visLines)
        var margin = 12
        var overscan = 45
        var needUpdate = force
            || pane._hlWinEnd <= pane._hlWinStart
            || (firstVis < pane._hlWinStart + margin && pane._hlWinStart > 0)
            || (lastVis + 1 > pane._hlWinEnd - margin && pane._hlWinEnd < total)
            || pane._hlWinStart >= total
        if (needUpdate) {
            var ws = Math.max(0, firstVis - overscan)
            var we = Math.min(total, lastVis + 1 + overscan)
            pane._hlWinStart = ws
            pane._hlWinEnd = we
            EditorPanel.setHighlightViewport(ws, we)
        }
    }

    function stopHover(): void {
        hoverTimer.stop()
        hoverTimer.targetPos = -1
    }

    function clearHoverLinkState(): void {
        stopHover()
        editorMouseArea.lastMouseX = -1
        editorMouseArea.lastMouseY = -1
        editorMouseArea.hoverLink = false
    }

    function syncEditMenuState(): void {
        AppMenuBar.setEditState(area.canUndo, area.canRedo, area.selectionStart !== area.selectionEnd, true)
    }

    function _syncEditMenuStateIfFocused(): void {
        if (pane.isActive && EditorPanel.focused)
            pane.syncEditMenuState()
    }

    function scheduleEditMenuSync(): void {
        if (pane.isActive && EditorPanel.focused)
            Qt.callLater(pane._syncEditMenuStateIfFocused)
    }

    onIsActiveChanged: {
        if (isActive) {
            syncHighlightViewport(true)
        } else {
            _pendingCursorSync = false
            clearHoverLinkState()
        }
    }

    Flickable {
        id: flick
        anchors.fill: parent
        clip: true
        boundsBehavior: Flickable.StopAtBounds
        onContentXChanged: {
            if (pane.isActive) pane.hideHoverRequested()
        }
        onContentYChanged: {
            if (pane.isActive) {
                pane.hideHoverRequested()
                pane.syncHighlightViewport(false)
            }
        }
        contentWidth: Math.max(width, area.implicitWidth)
        contentHeight: Math.max(height, area.implicitHeight)

        // TextEdit's large-document glyph cache recomputes its visible range
        // from contentX/contentY rather than Flickable width/height.
        onWidthChanged: {
            contentX = Math.max(0, Math.min(contentX, Math.max(0, contentWidth - width)))
            contentX += 1; contentX -= 1
        }
        onHeightChanged: {
            contentY = Math.max(0, Math.min(contentY, Math.max(0, contentHeight - height)))
            contentY += 1; contentY -= 1
            if (pane.isActive)
                pane.syncHighlightViewport(false)
        }
        onContentWidthChanged: {
            if (contentX > contentWidth - width)
                contentX = Math.max(0, contentWidth - width)
        }
        onContentHeightChanged: {
            if (contentY > contentHeight - height)
                contentY = Math.max(0, contentHeight - height)
        }

        function ensureVisible(r: rect): void {
            if (width <= 0 || height <= 0) return
            if (contentX >= r.x)
                contentX = Math.max(0, r.x - area.leftPadding)
            else if (contentX + width <= r.x + r.width)
                contentX = Math.min(Math.max(0, contentWidth - width), r.x + r.width - width + area.rightPadding)
            if (contentY >= r.y)
                contentY = Math.max(0, r.y - area.topPadding)
            else if (contentY + height <= r.y + r.height)
                contentY = Math.min(Math.max(0, contentHeight - height), r.y + r.height - height + area.bottomPadding)
        }

        ScrollBar.vertical: AppScrollBar {
            id: vBar
            policy: ScrollBar.AsNeeded
            Repeater {
                model: pane.isActive ? EditorPanel.scrollMarks : []
                delegate: Rectangle {
                    id: markDelegate
                    required property var modelData
                    parent: vBar
                    x: 0
                    width: vBar.width
                    height: markDelegate.modelData.weight || 2
                    y: (markDelegate.modelData.pos || 0) * (vBar.height - markDelegate.height)
                    color: markDelegate.modelData.color || "transparent"
                }
            }
        }
        ScrollBar.horizontal: AppScrollBar { policy: ScrollBar.AsNeeded }

        TextEdit {
            id: area
            width: Math.max(flick.width, implicitWidth)
            height: Math.max(flick.height, implicitHeight)
            wrapMode: TextEdit.NoWrap
            padding: 8
            font: pane.editorFont
            color: highlight.visible ? "transparent" : EditorPanel.textColor
            selectedTextColor: appTheme.highlightedText
            selectionColor: appTheme.highlight
            // Suppress Qt's built-in cursorDelegate visual: in Qt 6, QQuickTextEdit
            // imperatively hides cursorDelegate when scrolled out of a Flickable
            // viewport and does not re-show it when clicking a visible line without
            // scrolling. Render our own cursor Rectangle bound to cursorRectangle.
            cursorDelegate: Item {
                width: 0
                height: 0
                visible: false
            }
            selectByMouse: true
            persistentSelection: true
            tabStopDistance: Math.max(4, pane.averageCharacterWidth * EditorPanel.tabSize)
            property bool _syncingFromModel: false
            property bool _batchEditing: false

            onLineCountChanged: {
                if (pane.isActive)
                    pane.syncHighlightViewport(false)
            }

            onCursorRectangleChanged: {
                if (pane.isActive && !_syncingFromModel)
                    flick.ensureVisible(cursorRectangle)
            }

            onTextChanged: {
                if (!pane.isActive || _syncingFromModel || _batchEditing) return
                pane.pushBufferToModel()
            }
            onCursorPositionChanged: {
                if (!pane.isActive || _syncingFromModel || _batchEditing) return
                pane._pendingCursorSync = true
                Qt.callLater(pane._flushCursorPos)
            }
            onActiveFocusChanged: {
                if (!pane.isActive) return
                EditorPanel.focused = activeFocus
                if (activeFocus)
                    pane.syncEditMenuState()
            }
            // Defer menu sync and avoid reading area.selectedText inside edit-block
            // signals: Qt emits canUndoChanged mid-edit-block while QTextCursor's
            // selection can still point past the truncated document, causing
            // QQuickTextEdit::selectedText() -> QTextDocumentFragment to loop/crash.
            onCanUndoChanged: pane.scheduleEditMenuSync()
            onCanRedoChanged: pane.scheduleEditMenuSync()
            onSelectionStartChanged: pane.scheduleEditMenuSync()
            onSelectionEndChanged: pane.scheduleEditMenuSync()

            Rectangle {
                z: -2
                visible: pane.isActive
                x: 0
                y: pane.cursorLine * pane.lineHeight + area.topPadding
                width: area.width
                height: pane.lineHeight
                color: EditorPanel.currentLineColor
            }
            Rectangle {
                z: -2
                visible: pane.isActive && EditorPanel.debugLine > 0
                x: 0
                y: (EditorPanel.debugLine - 1) * pane.lineHeight + area.topPadding
                width: area.width
                height: pane.lineHeight
                color: EditorPanel.debugLineColor
            }

            Repeater {
                model: pane.isActive ? EditorPanel.foundRanges : []
                delegate: Rectangle {
                    id: foundDelegate
                    required property var modelData
                    z: -2
                    property rect r: area.positionToRectangle(foundDelegate.modelData.start)
                    x: foundDelegate.r.x
                    y: foundDelegate.r.y
                    width: Math.max(4, area.positionToRectangle(foundDelegate.modelData.end).x - foundDelegate.r.x)
                    height: foundDelegate.r.height
                    color: EditorPanel.foundColor
                }
            }

            AppText {
                id: highlight
                z: -1
                x: area.leftPadding
                y: area.topPadding + EditorPanel.highlightStartLine * pane.lineHeight
                textFormat: Text.RichText
                text: pane.isActive ? EditorPanel.highlightHtml : ""
                font: pane.editorFont
                color: EditorPanel.textColor
                wrapMode: Text.NoWrap
                visible: pane.isActive && EditorPanel.highlightHtml.length > 0
            }

            Rectangle {
                id: editorCursor
                z: 1
                x: area.cursorRectangle.x
                y: area.cursorRectangle.y
                width: 2
                height: area.cursorRectangle.height > 0 ? area.cursorRectangle.height : pane.lineHeight
                color: EditorPanel.textColor
                visible: pane.isActive && area.activeFocus && area.cursorVisible
            }

            Repeater {
                model: pane.isActive ? EditorPanel.diagnosticRanges : []
                delegate: Item {
                    id: diagItem
                    required property var modelData
                    z: 0

                    property rect r1: {
                        var _ = area.length
                        var __ = pane.editorFont
                        return area.positionToRectangle(diagItem.modelData.start)
                    }
                    property rect r2: {
                        var _ = area.length
                        var __ = pane.editorFont
                        return area.positionToRectangle(diagItem.modelData.end)
                    }

                    x: diagItem.r1.x
                    y: diagItem.r1.y + diagItem.r1.height - 2
                    width: Math.max(4, diagItem.r2.x - diagItem.r1.x)
                    height: 2

                    Rectangle {
                        anchors.fill: parent
                        color: diagItem.modelData.color
                        radius: 1
                    }
                }
            }

            Item {
                id: hoverLinkUnderline
                z: 0
                visible: pane.isActive && EditorPanel.hoverLinkStart >= 0 && EditorPanel.hoverLinkEnd > EditorPanel.hoverLinkStart

                property var r1: {
                    var _ = area.length
                    var __ = pane.editorFont
                    return visible ? area.positionToRectangle(EditorPanel.hoverLinkStart) : null
                }
                property var r2: {
                    var _ = area.length
                    var __ = pane.editorFont
                    return visible ? area.positionToRectangle(EditorPanel.hoverLinkEnd) : null
                }

                x: r1 ? r1.x : 0
                y: r1 ? (r1.y + r1.height - 2) : 0
                width: (r1 && r2) ? Math.max(2, r2.x - r1.x) : 0
                height: 1.5

                Rectangle {
                    anchors.fill: parent
                    color: EditorPanel.hoverLinkColor
                    radius: 0.5
                }
            }

            MouseArea {
                id: editorMouseArea
                anchors.fill: parent
                enabled: pane.isActive
                acceptedButtons: Qt.LeftButton | Qt.RightButton
                hoverEnabled: true
                propagateComposedEvents: true
                cursorShape: hoverLink ? Qt.PointingHandCursor : Qt.IBeamCursor
                property bool hoverLink: false
                property int lastMouseX: -1
                property int lastMouseY: -1

                function hideHoverState(): void {
                    pane.hideHoverRequested()
                }

                function updateHoverLink(modifiers: int): void {
                    if (lastMouseX >= 0 && lastMouseY >= 0) {
                        var pos = area.positionAt(lastMouseX, lastMouseY)
                        hoverLink = EditorPanel.textHovered(pos, modifiers)
                        if (hoverLink) {
                            hideHoverState()
                        }
                    } else {
                        hoverLink = false
                        EditorPanel.clearHoverLink()
                    }
                }

                Timer {
                    id: hoverTimer
                    interval: 250
                    repeat: false
                    property int targetPos: -1
                    onTriggered: {
                        if (pane.isActive && targetPos >= 0 && !EditorPanel.completionVisible) {
                            var r = area.positionToRectangle(targetPos)
                            var tx = pane.gutterWidth + r.x - flick.contentX
                            var ty = r.y - flick.contentY
                            var th = r.height > 0 ? r.height : pane.lineHeight
                            pane.hoverTargetRequested(tx, ty, th, targetPos)
                        }
                    }
                }
                onPressed: (mouse) => {
                    hideHoverState()
                    hoverLink = false
                    EditorPanel.clearHoverLink()
                    var pos = area.positionAt(mouse.x, mouse.y)
                    if (mouse.button === Qt.RightButton) {
                        EditorPanel.hideCompletion()
                        if (area.selectionStart === area.selectionEnd)
                            area.cursorPosition = pos
                        pane.contextMenuRequested()
                        mouse.accepted = true
                        return
                    }
                    if (EditorPanel.textClicked(pos, mouse.modifiers))
                        mouse.accepted = true
                    else
                        mouse.accepted = false
                }
                onPositionChanged: (mouse) => {
                    lastMouseX = mouse.x
                    lastMouseY = mouse.y

                    var rootMouseX = pane.gutterWidth + mouse.x - flick.contentX
                    var rootMouseY = mouse.y - flick.contentY
                    if (pane.hoverTooltipVisible &&
                        rootMouseX >= pane.hoverTooltipRect.x && rootMouseX <= pane.hoverTooltipRect.x + pane.hoverTooltipRect.width &&
                        rootMouseY >= pane.hoverTooltipRect.y && rootMouseY <= pane.hoverTooltipRect.y + pane.hoverTooltipRect.height) {
                        hoverTimer.stop()
                        return
                    }

                    var pos = area.positionAt(mouse.x, mouse.y)
                    hoverLink = EditorPanel.textHovered(pos, mouse.modifiers)
                    if (!hoverLink && mouse.buttons === 0) {
                        if (hoverTimer.targetPos !== pos) {
                            hoverTimer.targetPos = pos
                            hoverTimer.restart()
                        }
                    } else {
                        hideHoverState()
                    }
                    mouse.accepted = false
                }
                onExited: {
                    lastMouseX = -1
                    lastMouseY = -1
                    hoverLink = false
                    EditorPanel.clearHoverLink()
                    pane.stopHover()
                    pane.editorExited()
                }
            }

            function insertPair(event: var, open: string, close: string, flag: bool): bool {
                if (!flag || event.text !== open) return false
                area._batchEditing = true
                area.insert(area.cursorPosition, open + close)
                area.cursorPosition = area.cursorPosition - 1
                area._batchEditing = false
                pane.pushBufferToModel()
                event.accepted = true
                if (open === "(") EditorPanel.requestSignature(area.cursorPosition)
                return true
            }

            Keys.onPressed: (event) => {
                if (!pane.isActive) return
                if (event.key === Qt.Key_Control || event.key === Qt.Key_Meta) {
                    editorMouseArea.updateHoverLink(event.modifiers)
                } else {
                    pane.hideHoverRequested()
                }
                if (EditorPanel.completionVisible) {
                    if (event.key === Qt.Key_Return || event.key === Qt.Key_Enter || event.key === Qt.Key_Tab) {
                        pane.completionAccepted(EditorPanel.acceptCompletion(EditorPanel.completionIndex, area.cursorPosition))
                        event.accepted = true
                        return
                    }
                    if (event.key === Qt.Key_Escape) {
                        EditorPanel.hideCompletion()
                        event.accepted = true
                        return
                    }
                    if (event.key === Qt.Key_Up) {
                        EditorPanel.moveCompletion(-1)
                        event.accepted = true
                        return
                    }
                    if (event.key === Qt.Key_Down) {
                        EditorPanel.moveCompletion(1)
                        event.accepted = true
                        return
                    }
                }
                if (event.key === Qt.Key_Return || event.key === Qt.Key_Enter) {
                    var edit = EditorPanel.calculateNewlineForCurrent(area.cursorPosition, area.selectionStart, area.selectionEnd)
                    if (edit && edit.valid) {
                        area._batchEditing = true
                        if (edit.replaceStart < edit.replaceEnd) {
                            area.remove(edit.replaceStart, edit.replaceEnd)
                        }
                        if (edit.insertText.length > 0) {
                            area.insert(edit.replaceStart, edit.insertText)
                        }
                        area.cursorPosition = edit.newCursorPos
                        area._batchEditing = false
                        pane.pushBufferToModel()
                        event.accepted = true
                        return
                    }
                }
                if (event.key === Qt.Key_Escape) {
                    EditorPanel.dismissSignature()
                }
                if (event.key === Qt.Key_Tab && EditorPanel.spaceTabs) {
                    var n = EditorPanel.tabSize
                    var sp = " ".repeat(n)
                    area.insert(area.cursorPosition, sp)
                    event.accepted = true
                    return
                }
                if (area.insertPair(event, "(", ")", EditorPanel.closeParen)) return
                if (area.insertPair(event, "{", "}", EditorPanel.closeBraces)) return
                if (area.insertPair(event, "[", "]", EditorPanel.closeBrackets)) return
                if (area.insertPair(event, "\"", "\"", EditorPanel.closeQuotes)) return
                if (area.insertPair(event, "'", "'", EditorPanel.closeSquotes)) return

                if (event.text === ")") {
                    EditorPanel.dismissSignature()
                }
            }

            Keys.onReleased: (event) => {
                if (!pane.isActive) return
                if (event.key === Qt.Key_Control || event.key === Qt.Key_Meta) {
                    editorMouseArea.hoverLink = false
                    EditorPanel.clearHoverLink()
                }
            }
        }
    }
}
