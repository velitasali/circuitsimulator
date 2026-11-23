pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import cs_app
import cs_ui

/* TextArea + QML gutter + HTML overlay. LineNumberArea / QSyntaxHighlighter
 * are not available through qt-bridge. */
Item {
    id: root
    SystemPalette { id: appTheme }

    function _doFocusEditor(): void {
        if (root.visible && EditorPanel.currentDocument >= 0 && area) {
            area.forceActiveFocus()
        }
    }

    function focusEditor(): void {
        Qt.callLater(root._doFocusEditor)
    }

    // C++ registered these with QFontDatabase::addApplicationFont before any
    // QML ran. FontLoader is async, so the editor font binding must depend on
    // status or it keeps the first (missed) resolution forever.
    FontLoader { id: ubuntuMonoR; source: "qrc:/fonts/UbuntuMono-R.ttf" }
    FontLoader { id: ubuntuMonoI; source: "qrc:/fonts/UbuntuMono-RI.ttf" }
    FontLoader { id: ubuntuMonoB; source: "qrc:/fonts/UbuntuMono-B.ttf" }
    FontLoader { id: ubuntuMonoBI; source: "qrc:/fonts/UbuntuMono-BI.ttf" }
    readonly property int bundledMonoReady: ubuntuMonoR.status + ubuntuMonoI.status + ubuntuMonoB.status + ubuntuMonoBI.status
    readonly property font editorFont: {
        var _ = bundledMonoReady
        return Qt.font({
            family: EditorPanel.fontFamily || ubuntuMonoR.name || "Ubuntu Mono",
            pixelSize: EditorPanel.fontSize || 14,
            fixedPitch: true
        })
    }

    readonly property real lineHeight: {
        var _f = root.editorFont
        var _b = root.bundledMonoReady
        var _l = area.lineCount
        var r = area.positionToRectangle(0)
        return r.height > 1 ? r.height : (fm.lineSpacing > 1 ? fm.lineSpacing : Math.ceil(area.font.pixelSize * 1.4))
    }
    readonly property int markerLaneWidth: Math.max(18, Math.round(root.lineHeight))
    readonly property int markerSize: Math.max(8, Math.min(12, Math.round(root.lineHeight * 0.58 / 2) * 2))
    readonly property int gutterWidth: Math.max(52, root.markerLaneWidth + Math.ceil(String(Math.max(1, area.lineCount)).length * fm.averageCharacterWidth) + 14)
    property int hoveredGutterLine: -1
    readonly property int cursorLine: {
        var t = area.text.substring(0, area.cursorPosition)
        return t.length === 0 ? 0 : t.split('\n').length - 1
    }
    readonly property var breakpoints: EditorPanel.breakpoints
    readonly property var errors: EditorPanel.errors
    readonly property var warnings: EditorPanel.warnings

    property int hoverTargetX: -1
    property int hoverTargetY: -1
    property int hoverTargetHeight: root.lineHeight
    readonly property int effectiveHoverX: hoverTargetX >= 0
        ? hoverTargetX
        : (gutter.width + area.cursorRectangle.x - flick.contentX)
    readonly property int effectiveHoverY: hoverTargetY >= 0
        ? hoverTargetY
        : (area.cursorRectangle.y - flick.contentY)
    readonly property int effectiveHoverHeight: hoverTargetHeight > 0
        ? hoverTargetHeight
        : (area.cursorRectangle.height > 0 ? area.cursorRectangle.height : root.lineHeight)

    function containsLine(list: var, line: int): bool {
        if (!list) return false
        for (var i = 0; i < list.length; i++) if (Number(list[i]) === line) return true
        return false
    }

    function hideHover(): void {
        gutterHoverTimer.stop()
        gutterHoverTimer.targetLine = -1
        hoverTimer.stop()
        hoverTimer.targetPos = -1
        root.hoverTargetX = -1
        root.hoverTargetY = -1
        EditorPanel.hideHover()
    }

    function applyCompletion(edit: var): void {
        if (!edit || !edit.valid) return
        if (edit.replaceStart < edit.replaceEnd) {
            area.remove(edit.replaceStart, edit.replaceEnd)
        }
        if (edit.insertText.length > 0) {
            area.insert(edit.replaceStart, edit.insertText)
        }
        area.cursorPosition = edit.newCursorPos
        EditorPanel.cursorPos = edit.newCursorPos
        EditorPanel.finishCompletion()
        area.forceActiveFocus()
    }

    FontMetrics {
        id: fm
        font: area.font
    }

    onVisibleChanged: {
        EditorPanel.dark = CircuitCanvas.dark
        if (visible && EditorPanel.currentDocument >= 0) {
            root.focusEditor()
        }
    }
    Component.onCompleted: EditorPanel.dark = CircuitCanvas.dark
    Connections {
        target: CircuitCanvas
        function onAppearanceChanged() { EditorPanel.dark = CircuitCanvas.dark }
    }

    Timer {
        interval: 100
        running: root.visible
        repeat: true
        onTriggered: EditorPanel.pollLsp()
    }

    Rectangle {
        id: gutter
        width: root.gutterWidth
        height: parent.height
        color: "transparent"

        Rectangle {
            anchors.right: parent.right
            width: 1
            height: parent.height
            color: EditorPanel.gutterBorderColor
        }

        Flickable {
            id: gutterFlick
            anchors.fill: parent
            clip: true
            interactive: false
            contentY: flick.contentY
            contentHeight: area.implicitHeight

            Rectangle {
                id: hoverBpDot
                visible: root.hoveredGutterLine >= 1
                         && root.hoveredGutterLine <= area.lineCount
                         && !root.containsLine(root.breakpoints, root.hoveredGutterLine)
                         && !root.containsLine(root.errors, root.hoveredGutterLine)
                         && !root.containsLine(root.warnings, root.hoveredGutterLine)
                         && EditorPanel.debugLine !== root.hoveredGutterLine
                z: 1
                width: root.markerSize
                height: root.markerSize
                radius: width / 2
                x: Math.round((root.markerLaneWidth - width) / 2)
                y: Math.round(area.topPadding + (root.hoveredGutterLine - 1) * root.lineHeight + (root.lineHeight - height) / 2)
                color: Qt.alpha(EditorPanel.errorColor, 0.32)
                border.width: 1
                border.color: Qt.alpha(EditorPanel.errorColor, 0.65)
            }

            Repeater {
                model: Math.max( 1, area.lineCount )
                delegate: Item {
                    id: gutterLine
                    required property int index
                    y: area.topPadding + gutterLine.index * root.lineHeight
                    width: gutter.width
                    height: root.lineHeight
                    readonly property int line: gutterLine.index + 1
                    readonly property bool isBp: root.containsLine(root.breakpoints, gutterLine.line)
                    readonly property bool isErr: root.containsLine(root.errors, gutterLine.line)
                    readonly property bool isWarn: root.containsLine(root.warnings, gutterLine.line)
                    readonly property bool isDbg: EditorPanel.debugLine === gutterLine.line

                    Rectangle {
                        id: bpDot
                        visible: gutterLine.isBp && !gutterLine.isErr && !gutterLine.isWarn
                        z: 2
                        width: root.markerSize
                        height: root.markerSize
                        radius: width / 2
                        x: Math.round((root.markerLaneWidth - width) / 2)
                        y: Math.round((gutterLine.height - height) / 2)
                        color: EditorPanel.errorColor
                        border.width: gutterLine.isDbg ? 1.5 : 1
                        border.color: gutterLine.isDbg
                                      ? (CircuitCanvas.msgWarnBg || appTheme.highlight)
                                      : Qt.darker(EditorPanel.errorColor, 1.35)
                    }
                    Rectangle {
                        id: errWarnDot
                        visible: gutterLine.isErr || gutterLine.isWarn
                        z: 3
                        width: Math.max(10, Math.min(13, Math.round(root.lineHeight * 0.66)))
                        height: errWarnDot.width
                        radius: errWarnDot.width / 2
                        x: Math.round((root.markerLaneWidth - width) / 2)
                        y: Math.round((gutterLine.height - height) / 2)
                        color: gutterLine.isErr ? EditorPanel.errorColor : EditorPanel.warnColor
                        border.width: gutterLine.isBp ? 1.5 : 0
                        border.color: gutterLine.isBp ? (gutterLine.isErr ? appTheme.highlightedText : EditorPanel.errorColor) : "transparent"

                        AppText {
                            anchors.centerIn: parent
                            text: "!"
                            color: appTheme.highlightedText
                            font.bold: true
                            font.pixelSize: Math.max(7, Math.round(errWarnDot.height * 0.72))
                            font.family: area.font.family
                            verticalAlignment: Text.AlignVCenter
                            horizontalAlignment: Text.AlignHCenter
                        }
                    }
                    AppText {
                        anchors.left: parent.left
                        anchors.leftMargin: root.markerLaneWidth
                        anchors.right: parent.right
                        anchors.rightMargin: 8
                        anchors.top: parent.top
                        anchors.bottom: parent.bottom
                        horizontalAlignment: Text.AlignRight
                        verticalAlignment: Text.AlignVCenter
                        text: gutterLine.line
                        font.family: area.font.family
                        font.pixelSize: area.font.pixelSize
                        color: gutterLine.index === root.cursorLine ? EditorPanel.gutterActiveNumColor : EditorPanel.gutterNumColor
                        font.bold: gutterLine.index === root.cursorLine
                    }
                }
            }

            Canvas {
                id: dbgArrow
                contextType: "2d"
                readonly property int dbgLine: EditorPanel.debugLine
                readonly property bool dbgOnBp: dbgLine >= 1 && root.containsLine(root.breakpoints, dbgLine)
                visible: dbgLine >= 1 && dbgLine <= area.lineCount
                z: 4
                x: 0
                y: Math.round(area.topPadding + (dbgLine - 1) * root.lineHeight)
                width: root.markerLaneWidth
                height: Math.max(1, Math.round(root.lineHeight))
                antialiasing: true
                onDbgLineChanged: requestPaint()
                onDbgOnBpChanged: requestPaint()
                onWidthChanged: requestPaint()
                onHeightChanged: requestPaint()
                onVisibleChanged: if (visible) requestPaint()
                Connections {
                    target: CircuitCanvas
                    function onAppearanceChanged() { dbgArrow.requestPaint() }
                }
                onPaint: {
                    var ctx = getContext("2d")
                    if (!ctx)
                        return
                    ctx.clearRect(0, 0, width, height)
                    var cx = width / 2
                    var cy = height / 2
                    ctx.beginPath()
                    if (dbgOnBp) {
                        var r = Math.max(2.4, root.markerSize * 0.27)
                        ctx.moveTo(cx - r * 0.65, cy - r)
                        ctx.lineTo(cx + r * 0.95, cy)
                        ctx.lineTo(cx - r * 0.65, cy + r)
                        ctx.closePath()
                        ctx.fillStyle = appTheme.highlightedText
                        ctx.fill()
                    } else {
                        var aw = Math.max(9, Math.min(12, Math.round(root.markerSize * 1.1)))
                        var ah = Math.max(8, Math.min(10, Math.round(root.markerSize * 0.88)))
                        var x0 = cx - aw / 2
                        var xm = cx + aw * 0.12
                        var x1 = cx + aw / 2
                        var y0 = cy - ah / 2
                        var y1 = cy + ah / 2
                        ctx.moveTo(x0, y0)
                        ctx.lineTo(xm, y0)
                        ctx.lineTo(x1, cy)
                        ctx.lineTo(xm, y1)
                        ctx.lineTo(x0, y1)
                        ctx.closePath()
                        var fillCol = CircuitCanvas.msgWarnBg || appTheme.highlight
                        ctx.fillStyle = fillCol
                        ctx.fill()
                        ctx.lineWidth = 1
                        ctx.strokeStyle = Qt.darker(fillCol, 1.35)
                        ctx.stroke()
                    }
                }
            }
        }

        Timer {
            id: gutterHoverTimer
            interval: 200
            repeat: false
            property int targetLine: -1
            onTriggered: {
                if (targetLine >= 1 && targetLine <= area.lineCount && !EditorPanel.completionVisible) {
                    var tip = EditorPanel.gutterTooltip(targetLine)
                    if (tip && tip.length > 0) {
                        root.hoverTargetX = gutter.width + 4
                        root.hoverTargetY = area.topPadding + (targetLine - 1) * root.lineHeight - gutterFlick.contentY
                        root.hoverTargetHeight = root.lineHeight
                        EditorPanel.showGutterHover(targetLine)
                    } else {
                        root.hideHover()
                    }
                }
            }
        }

        MouseArea {
            anchors.fill: parent
            acceptedButtons: Qt.LeftButton | Qt.RightButton
            hoverEnabled: true
            cursorShape: root.hoveredGutterLine >= 1 ? Qt.PointingHandCursor : Qt.ArrowCursor
            onClicked: (mouse) => {
                root.hideHover()
                var line = Math.floor((mouse.y + gutterFlick.contentY - area.topPadding) / Math.max(1, root.lineHeight)) + 1
                EditorPanel.gutterClicked(line, mouse.button)
            }
            onPositionChanged: (mouse) => {
                var line = Math.floor((mouse.y + gutterFlick.contentY - area.topPadding) / Math.max(1, root.lineHeight)) + 1
                if (line < 1 || line > area.lineCount) {
                    root.hoveredGutterLine = -1
                    root.hideHover()
                    return
                }
                root.hoveredGutterLine = line
                var tip = EditorPanel.gutterTooltip(line)
                if (!tip || tip.length === 0) {
                    root.hideHover()
                    return
                }
                if (gutterHoverTimer.targetLine !== line) {
                    gutterHoverTimer.targetLine = line
                    gutterHoverTimer.restart()
                }
            }
            onExited: {
                root.hoveredGutterLine = -1
                root.hideHover()
            }
        }
    }

    Flickable {
        id: flick
        anchors.left: gutter.right
        anchors.right: parent.right
        anchors.top: parent.top
        anchors.bottom: parent.bottom
        clip: true
        boundsBehavior: Flickable.StopAtBounds
        onContentXChanged: {
            root.hideHover()
        }
        onContentYChanged: {
            root.hideHover()
        }
        contentWidth: Math.max(width, area.implicitWidth)
        contentHeight: Math.max(height, area.implicitHeight)
        ScrollBar.vertical: AppScrollBar {
            id: vBar
            policy: ScrollBar.AsNeeded
            Repeater {
                model: EditorPanel.scrollMarks
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

        TextArea.flickable: TextArea {
            id: area
            wrapMode: TextEdit.NoWrap
            padding: 8
            font: root.editorFont
            color: highlight.visible ? "transparent" : EditorPanel.textColor
            selectedTextColor: appTheme.highlightedText
            selectionColor: appTheme.highlight
            background: null
            cursorDelegate: Rectangle {
                width: 2
                color: EditorPanel.textColor
                visible: area.cursorVisible
            }
            selectByMouse: true
            persistentSelection: true
            // CodeEditor raises its own menu (C++ CodeEditor::showContextMenu),
            // so Qt's built-in one is dropped — two menus would answer one click.
            ContextMenu.menu: null
            tabStopDistance: Math.max(4, fm.averageCharacterWidth * EditorPanel.tabSize)
            property bool _syncingFromModel: false
            Component.onCompleted: {
                _syncingFromModel = true
                text = EditorPanel.currentText
                _syncingFromModel = false
            }
            onTextChanged: {
                if (_syncingFromModel) return
                EditorPanel.cursorPos = cursorPosition
                if ( text !== EditorPanel.currentText )
                    EditorPanel.currentText = text
            }
            onCursorPositionChanged: EditorPanel.cursorPos = cursorPosition
            onActiveFocusChanged: {
                EditorPanel.focused = activeFocus
                if (activeFocus)
                    AppMenuBar.setEditState(canUndo, canRedo, selectedText.length > 0, true)
            }
            onCanUndoChanged: if (EditorPanel.focused) AppMenuBar.setEditState(canUndo, canRedo, selectedText.length > 0, true)
            onCanRedoChanged: if (EditorPanel.focused) AppMenuBar.setEditState(canUndo, canRedo, selectedText.length > 0, true)
            onSelectedTextChanged: if (EditorPanel.focused) AppMenuBar.setEditState(canUndo, canRedo, selectedText.length > 0, true)

            Rectangle {
                z: -2
                x: flick.contentX
                y: root.cursorLine * root.lineHeight + area.topPadding
                width: Math.max(flick.width, area.implicitWidth)
                height: root.lineHeight
                color: EditorPanel.currentLineColor
            }
            Rectangle {
                z: -2
                visible: EditorPanel.debugLine > 0
                x: flick.contentX
                y: (EditorPanel.debugLine - 1) * root.lineHeight + area.topPadding
                width: Math.max(flick.width, area.implicitWidth)
                height: root.lineHeight
                color: EditorPanel.debugLineColor
            }

            Repeater {
                model: EditorPanel.foundRanges
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
                y: area.topPadding
                textFormat: Text.RichText
                text: EditorPanel.highlightHtml
                font: root.editorFont
                color: EditorPanel.textColor
                wrapMode: Text.NoWrap
                visible: EditorPanel.highlightHtml.length > 0
            }

            Repeater {
                model: EditorPanel.diagnosticRanges
                delegate: Item {
                    id: diagItem
                    required property var modelData
                    z: 0

                    property rect r1: {
                        var _ = area.length
                        var __ = root.editorFont
                        return area.positionToRectangle(diagItem.modelData.start)
                    }
                    property rect r2: {
                        var _ = area.length
                        var __ = root.editorFont
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
                visible: EditorPanel.hoverLinkStart >= 0 && EditorPanel.hoverLinkEnd > EditorPanel.hoverLinkStart

                property var r1: {
                    var _ = area.length
                    var __ = root.editorFont
                    return visible ? area.positionToRectangle(EditorPanel.hoverLinkStart) : null
                }
                property var r2: {
                    var _ = area.length
                    var __ = root.editorFont
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
                acceptedButtons: Qt.LeftButton | Qt.RightButton
                hoverEnabled: true
                propagateComposedEvents: true
                cursorShape: hoverLink ? Qt.PointingHandCursor : Qt.IBeamCursor
                property bool hoverLink: false
                property int lastMouseX: -1
                property int lastMouseY: -1

                function hideHoverState(): void {
                    root.hideHover()
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
                        if (targetPos >= 0 && !EditorPanel.completionVisible) {
                            var r = area.positionToRectangle(targetPos)
                            root.hoverTargetX = gutter.width + r.x - flick.contentX
                            root.hoverTargetY = r.y - flick.contentY
                            root.hoverTargetHeight = r.height > 0 ? r.height : root.lineHeight
                            EditorPanel.showHover(targetPos)
                        }
                    }
                }
                onPressed: (mouse) => {
                    hideHoverState()
                    hoverLink = false
                    EditorPanel.clearHoverLink()
                    var pos = area.positionAt(mouse.x, mouse.y)
                    if (mouse.button === Qt.RightButton) {
                        if (!area.selectedText || area.selectedText.length === 0)
                            area.cursorPosition = pos
                        editorMenu.popup()
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

                    var rootMouseX = gutter.width + mouse.x - flick.contentX
                    var rootMouseY = mouse.y - flick.contentY
                    if (hoverTooltip.visible &&
                        rootMouseX >= hoverTooltip.x && rootMouseX <= hoverTooltip.x + hoverTooltip.width &&
                        rootMouseY >= hoverTooltip.y && rootMouseY <= hoverTooltip.y + hoverTooltip.height) {
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
                    hideHoverState()
                }
            }

            function insertPair(event: var, open: string, close: string, flag: bool): bool {
                if (!flag || event.text !== open) return false
                area.insert(area.cursorPosition, open + close)
                area.cursorPosition = area.cursorPosition - 1
                event.accepted = true
                if (open === "(") EditorPanel.requestSignature(area.cursorPosition)
                return true
            }

            Keys.onPressed: (event) => {
                if (event.key === Qt.Key_Control || event.key === Qt.Key_Meta) {
                    editorMouseArea.updateHoverLink(event.modifiers)
                } else {
                    root.hideHover()
                }
                if (EditorPanel.completionVisible) {
                    if (event.key === Qt.Key_Return || event.key === Qt.Key_Enter || event.key === Qt.Key_Tab) {
                        root.applyCompletion(EditorPanel.acceptCompletion(EditorPanel.completionIndex, area.cursorPosition))
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
                    var edit = EditorPanel.calculateNewline(area.text, area.cursorPosition, area.selectionStart, area.selectionEnd)
                    if (edit && edit.valid) {
                        if (edit.replaceStart < edit.replaceEnd) {
                            area.remove(edit.replaceStart, edit.replaceEnd)
                        }
                        if (edit.insertText.length > 0) {
                            area.insert(edit.replaceStart, edit.insertText)
                        }
                        area.cursorPosition = edit.newCursorPos
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
                if (event.key === Qt.Key_Control || event.key === Qt.Key_Meta) {
                    editorMouseArea.hoverLink = false
                    EditorPanel.clearHoverLink()
                }
            }
        }
    }

    AppContextMenu {
        id: editorMenu

        readonly property bool _hasSelection: area.selectedText && area.selectedText.length > 0

        ContextMenuItem {
            visible: EditorPanel.lspReady
            height: visible ? implicitHeight : 0
            iconLigature: "search"
            text: I18n.tr("Go to Definition")
            shortcutText: UiUtils.shortcutFor("edit.gotoDefinition")
            onTriggered: AppMenuBar.triggerAction("edit.gotoDefinition")
        }
        ContextMenuSeparator { visible: EditorPanel.lspReady }

        ContextMenuItem {
            iconLigature: "undo"
            text: I18n.tr("Undo")
            shortcutText: UiUtils.formatShortcut("Ctrl+Z")
            enabled: area.canUndo
            onTriggered: area.undo()
        }
        ContextMenuItem {
            iconLigature: "redo"
            text: I18n.tr("Redo")
            shortcutText: UiUtils.formatShortcut(UiUtils.isMacOS ? "Ctrl+Shift+Z" : "Ctrl+Y")
            enabled: area.canRedo
            onTriggered: area.redo()
        }

        ContextMenuSeparator {}

        ContextMenuItem {
            iconLigature: "content_cut"
            text: I18n.tr("Cut")
            shortcutText: UiUtils.formatShortcut("Ctrl+X")
            enabled: editorMenu._hasSelection
            onTriggered: area.cut()
        }
        ContextMenuItem {
            iconLigature: "content_copy"
            text: I18n.tr("Copy")
            shortcutText: UiUtils.formatShortcut("Ctrl+C")
            enabled: editorMenu._hasSelection
            onTriggered: area.copy()
        }
        ContextMenuItem {
            iconLigature: "content_paste"
            text: I18n.tr("Paste")
            shortcutText: UiUtils.formatShortcut("Ctrl+V")
            enabled: area.canPaste
            onTriggered: area.paste()
        }
        ContextMenuItem {
            iconLigature: "delete"
            text: I18n.tr("Remove")
            enabled: editorMenu._hasSelection
            onTriggered: area.remove(area.selectionStart, area.selectionEnd)
        }

        ContextMenuSeparator {}

        ContextMenuItem {
            iconLigature: "format_align_left"
            text: I18n.tr("Format Document")
            shortcutText: UiUtils.shortcutFor("edit.format")
            onTriggered: EditorPanel.formatDocument()
        }

        ContextMenuSeparator { visible: EditorPanel.hasCompiler }
        ContextMenuItem {
            visible: EditorPanel.hasCompiler
            height: visible ? implicitHeight : 0
            iconLigature: "settings"
            text: EditorPanel.compilerName.length > 0
                  ? I18n.tr("%1 Settings").arg(EditorPanel.compilerName)
                  : I18n.tr("Compiler Settings")
            onTriggered: EditorPanel.compilerProps()
        }
        ContextMenuItem {
            visible: EditorPanel.hasCompiler
            height: visible ? implicitHeight : 0
            iconLigature: "settings_applications"
            text: I18n.tr("File Settings")
            onTriggered: EditorPanel.fileProps()
        }

        ContextMenuSeparator {}

        ContextMenuItem {
            iconLigature: "refresh"
            text: I18n.tr("Reload Document")
            enabled: EditorPanel.currentPath.length > 0
            onTriggered: EditorPanel.reloadDocument()
        }
    }

    Popup {
        id: signaturePopup
        visible: EditorPanel.signatureVisible && EditorPanel.signatureHtml.length > 0
        z: 1000
        x: Math.max(gutter.width + 4, Math.min(root.width - width - 10, gutter.width + area.cursorRectangle.x + area.leftPadding - flick.contentX))
        y: {
            var curY = area.cursorRectangle.y + area.topPadding - flick.contentY
            if (curY >= height + 6) {
                return curY - height - 4
            } else {
                return curY + area.cursorRectangle.height + 4
            }
        }
        width: Math.min(500, Math.max(120, sigText.implicitWidth + 16))
        height: sigText.implicitHeight + 12
        padding: 6
        closePolicy: Popup.NoAutoClose

        background: Rectangle {
            color: appTheme.base
            border.color: appTheme.mid
            border.width: 1
            radius: 10
        }

        AppText {
            id: sigText
            text: EditorPanel.signatureHtml
            textFormat: Text.RichText
            color: appTheme.text
            font: area.font
            wrapMode: Text.Wrap
            width: parent.width - 12
        }
    }

    Item {
        id: hoverTooltip
        z: 1000
        visible: EditorPanel.hoverHtml.length > 0 && !EditorPanel.completionVisible

        readonly property int padX: 10
        readonly property int padY: 8
        readonly property int maxTooltipWidth: Math.min(520, Math.max(160, root.width - gutter.width - 20))

        x: Math.max(gutter.width + 4, Math.min(root.width - width - 10, root.effectiveHoverX))
        y: {
            var below = root.effectiveHoverY + root.effectiveHoverHeight + 4
            if (below + height > root.height - 10) {
                var above = root.effectiveHoverY - height - 4
                return Math.max(0, above)
            }
            return Math.max(0, below)
        }

        implicitWidth: hoverText.implicitWidth + 2 * padX + 2
        implicitHeight: hoverText.implicitHeight + 2 * padY + 2
        width: Math.min(implicitWidth, maxTooltipWidth)
        height: implicitHeight

        Rectangle {
            anchors.fill: parent
            color: appTheme.base
            opacity: 0.96
            border.color: appTheme.mid
            border.width: 1
            radius: 10
        }

        AppText {
            id: hoverText
            x: hoverTooltip.padX
            y: hoverTooltip.padY
            width: Math.min(implicitWidth, hoverTooltip.maxTooltipWidth - 2 * hoverTooltip.padX - 2)
            text: EditorPanel.hoverHtml
            textFormat: Text.RichText
            color: appTheme.text
            font: area.font
            wrapMode: Text.Wrap
        }
    }

    Dialog {
        id: diskPromptDialog
        visible: EditorPanel.diskPromptVisible
        parent: Overlay.overlay
        modal: true
        dim: true
        padding: 0
        header: null
        footer: null
        standardButtons: Dialog.NoButton
        closePolicy: Popup.NoAutoClose
        width: Math.min(Math.max((parent ? parent.width : root.width) - 32, 300), 420)
        x: parent ? Math.round((parent.width - width) / 2) : 0
        y: parent ? Math.max(40, Math.round((parent.height - height) / 2)) : 0

        readonly property bool isDeleted: EditorPanel.diskPromptKind === "deleted"
        readonly property color accentColor: isDeleted
            ? (CircuitCanvas.msgErrorBg || appTheme.highlight)
            : (CircuitCanvas.msgWarnBg || appTheme.highlight)

        function fileNameForPath(path: string): string {
            if (!path || path.length === 0)
                return ""
            var parts = path.replace(/\\/g, "/").split("/")
            return parts[parts.length - 1] || path
        }

        function fileIconForPath(path: string): string {
            var name = fileNameForPath(path)
            if (App.isCircuitPath(name))
                return "memory"
            var ext = name.split(".").pop().toLowerCase()
            if (ext === "c" || ext === "cpp" || ext === "h" || ext === "hpp" || ext === "ino" || ext === "rs" || ext === "py")
                return "code"
            if (ext === "asm" || ext === "s" || ext === "inc")
                return "terminal"
            if (ext === "hex" || ext === "bin")
                return "data_object"
            if (ext === "xml" || ext === "json" || ext === "txt" || ext === "md")
                return "article"
            return "description"
        }

        function triggerSecondary(): void {
            if (isDeleted) {
                EditorPanel.diskPromptKeep()
            } else {
                EditorPanel.diskPromptReload()
            }
        }

        function triggerPrimary(): void {
            if (isDeleted) {
                EditorPanel.diskPromptClose()
            } else {
                EditorPanel.diskPromptKeep()
            }
        }

        background: Rectangle {
            color: appTheme.window
            border.color: appTheme.mid
            border.width: 1
            radius: 14
        }

        Overlay.modal: Rectangle {
            color: Qt.rgba(0, 0, 0, 0.45)
        }

        onOpened: primaryDiskPromptBtn.forceActiveFocus()

        Shortcut {
            enabled: diskPromptDialog.visible
            sequences: ["Return", "Enter"]
            onActivated: diskPromptDialog.triggerPrimary()
        }

        Shortcut {
            enabled: diskPromptDialog.visible
            sequences: ["Escape"]
            onActivated: EditorPanel.diskPromptKeep()
        }

        contentItem: Item {
            implicitWidth: diskPromptDialog.width
            implicitHeight: diskPromptCol.implicitHeight + 32

            ColumnLayout {
                id: diskPromptCol
                anchors.fill: parent
                anchors.margins: 16
                spacing: 12

                RowLayout {
                    Layout.fillWidth: true
                    spacing: 10

                    Rectangle {
                        implicitWidth: 28
                        implicitHeight: 28
                        radius: height / 2
                        color: Qt.alpha(diskPromptDialog.accentColor, 0.18)

                        AppIcon {
                            anchors.centerIn: parent
                            icon: "warning"
                            color: diskPromptDialog.accentColor
                            size: 16
                        }
                    }

                    AppText {
                        text: diskPromptDialog.isDeleted
                            ? I18n.tr("File Deleted on Disk")
                            : I18n.tr("File Changed on Disk")
                        font.pixelSize: App.fontTitle
                        font.bold: true
                        color: appTheme.windowText
                        Layout.fillWidth: true
                        elide: Text.ElideRight
                    }

                    AppToolButton {
                        implicitWidth: 24
                        implicitHeight: 24
                        padding: 0
                        contentItem: AppIcon {
                            icon: "close"
                            size: 16
                            color: appTheme.windowText
                            opacity: 0.65
                        }
                        onClicked: EditorPanel.diskPromptKeep()
                    }
                }

                Rectangle {
                    Layout.fillWidth: true
                    implicitHeight: Math.max(32, diskTargetRow.implicitHeight + 12)
                    radius: 8
                    color: Qt.rgba(appTheme.mid.r, appTheme.mid.g, appTheme.mid.b, 0.16)
                    border.color: Qt.rgba(appTheme.mid.r, appTheme.mid.g, appTheme.mid.b, 0.32)
                    border.width: 1

                    RowLayout {
                        id: diskTargetRow
                        anchors.fill: parent
                        anchors.leftMargin: 8
                        anchors.rightMargin: 8
                        spacing: 8

                        AppIcon {
                            icon: diskPromptDialog.fileIconForPath(EditorPanel.diskPromptPath)
                            size: 16
                            color: appTheme.windowText
                            opacity: 0.7
                        }

                        ColumnLayout {
                            Layout.fillWidth: true
                            spacing: 1

                            AppText {
                                Layout.fillWidth: true
                                text: diskPromptDialog.fileNameForPath(EditorPanel.diskPromptPath)
                                font.pixelSize: App.fontSmall
                                font.bold: true
                                color: appTheme.windowText
                                elide: Text.ElideMiddle
                            }

                            AppText {
                                visible: text.length > 0
                                Layout.fillWidth: true
                                text: EditorPanel.diskPromptPath
                                font.pixelSize: App.fontMicro
                                color: appTheme.windowText
                                opacity: 0.65
                                elide: Text.ElideMiddle
                            }
                        }
                    }
                }

                AppText {
                    Layout.fillWidth: true
                    text: diskPromptDialog.isDeleted
                        ? I18n.tr("This file has been deleted or renamed on disk by another program.")
                        : I18n.tr("This file has been modified on disk by another program, and you have unsaved changes in the editor.")
                    color: appTheme.windowText
                    opacity: 0.88
                    wrapMode: Text.WordWrap
                    lineHeight: 1.2
                }

                RowLayout {
                    Layout.fillWidth: true
                    Layout.topMargin: 2
                    spacing: 8

                    Item { Layout.fillWidth: true }

                    AppButton {
                        id: secondaryDiskPromptBtn
                        text: diskPromptDialog.isDeleted ? I18n.tr("Keep in Editor") : I18n.tr("Reload")
                        Layout.minimumHeight: 28
                        leftPadding: 14
                        rightPadding: 14
                        onClicked: diskPromptDialog.triggerSecondary()
                    }

                    AppButton {
                        id: primaryDiskPromptBtn
                        text: diskPromptDialog.isDeleted ? I18n.tr("Close Tab") : I18n.tr("Keep My Changes")
                        highlighted: true
                        font.bold: true
                        Layout.minimumHeight: 28
                        leftPadding: 16
                        rightPadding: 16
                        onClicked: diskPromptDialog.triggerPrimary()
                    }
                }
            }
        }
    }

    Popup {
        id: completionPopup
        visible: EditorPanel.completionVisible
        x: gutter.width + area.cursorRectangle.x + area.leftPadding - flick.contentX
        y: area.cursorRectangle.y + area.cursorRectangle.height + area.topPadding - flick.contentY
        width: 380
        height: Math.min(240, Math.max(24, completionList.count * 22 + 2))
        padding: 1
        closePolicy: Popup.NoAutoClose

        background: Rectangle {
            color: appTheme.base
            border.color: appTheme.mid
            border.width: 1
            radius: 8
        }

        ListView {
            id: completionList
            anchors.fill: parent
            clip: true
            model: EditorPanel.completions
            currentIndex: EditorPanel.completionIndex
            onCurrentIndexChanged: completionList.positionViewAtIndex(completionList.currentIndex, ListView.Contain)
            delegate: Rectangle {
                id: completionDelegate
                required property var modelData
                required property int index
                width: completionList.width
                height: 22
                color: completionDelegate.index === completionList.currentIndex ? appTheme.highlight : "transparent"
                Row {
                    anchors.verticalCenter: parent.verticalCenter
                    anchors.left: parent.left
                    anchors.leftMargin: 6
                    spacing: 6
                    AppText {
                        text: completionDelegate.modelData.text !== undefined ? completionDelegate.modelData.text : completionDelegate.modelData
                        font: area.font
                        color: completionDelegate.index === completionList.currentIndex ? appTheme.highlightedText : appTheme.windowText
                    }
                    AppText {
                        text: completionDelegate.modelData.detail !== undefined ? completionDelegate.modelData.detail : ""
                        font: area.font
                        opacity: 0.6
                        color: completionDelegate.index === completionList.currentIndex ? appTheme.highlightedText : appTheme.windowText
                    }
                }
                MouseArea {
                    anchors.fill: parent
                    onClicked: root.applyCompletion(EditorPanel.acceptCompletion(completionDelegate.index, area.cursorPosition))
                }
            }
        }
    }

    Connections {
        target: EditorPanel
        function onRequestFocus(): void {
            root.focusEditor()
        }
        function onCurrentDocumentChanged(): void {
            if (EditorPanel.currentDocument >= 0) {
                root.focusEditor()
            }
        }
        function onCurrentTextChanged(): void {
            var modelText = EditorPanel.currentText
            if (area.text !== modelText) {
                area._syncingFromModel = true
                area.text = modelText
                area._syncingFromModel = false
            }
        }
        function onGotoChanged() {
            root.focusEditor()
            if (EditorPanel.selectStart >= 0 && EditorPanel.selectEnd > EditorPanel.selectStart) {
                area.select(EditorPanel.selectStart, EditorPanel.selectEnd)
                return
            }
            if (EditorPanel.gotoLine > 0) {
                var t = area.text
                var line = 1
                var pos = 0
                while (line < EditorPanel.gotoLine) {
                    var n = t.indexOf("\n", pos)
                    if (n < 0) { pos = t.length; break }
                    pos = n + 1
                    line++
                }
                area.cursorPosition = pos
            }
        }
        function onEditActionChanged() {
            var a = EditorPanel.editAction
            if (a === "undo") area.undo()
            else if (a === "redo") area.redo()
            else if (a === "cut") area.cut()
            else if (a === "copy") area.copy()
            else if (a === "paste") area.paste()
            else if (a === "selectAll") area.selectAll()
        }
        function onCursorChanged() {
            if (Math.abs(area.cursorPosition - EditorPanel.cursorPos) > 0
                    && EditorPanel.selectStart < 0)
                area.cursorPosition = EditorPanel.cursorPos
        }
    }
}
