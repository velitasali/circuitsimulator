pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtQuick.Window
import cs_app
import cs_ui

/* TextEdit + QML gutter + HTML overlay. LineNumberArea / QSyntaxHighlighter
 * are not available through qt-bridge. */
Item {
    id: root
    SystemPalette { id: appTheme }

    property var _panePool: ({})
    property int activeDocId: -1
    property CodeEditorPane activePane: null
    readonly property var hostWindow: Window.window
    readonly property Flickable activeFlick: activePane ? activePane.flick : null
    readonly property TextEdit activeArea: activePane ? activePane.area : null
    readonly property int activeLineCount: activeArea ? activeArea.lineCount : 1
    readonly property real activeTopPadding: activeArea ? activeArea.topPadding : 8
    readonly property real activeLeftPadding: activeArea ? activeArea.leftPadding : 8
    readonly property real activeImplicitHeight: activeArea ? activeArea.implicitHeight : 0
    readonly property real activeContentX: activeFlick ? activeFlick.contentX : 0
    readonly property real activeContentY: activeFlick ? activeFlick.contentY : 0
    readonly property rect activeCursorRect: activeArea ? activeArea.cursorRectangle : Qt.rect(0, 0, 0, root.lineHeight)
    readonly property bool cursorInView: {
        var f = root.activeFlick
        var a = root.activeArea
        if (!f || !a || f.width <= 0 || f.height <= 0)
            return false
        var r = a.cursorRectangle
        var vx = r.x - f.contentX
        var vy = r.y - f.contentY
        var cw = Math.max(1, r.width)
        var ch = r.height > 0 ? r.height : root.lineHeight
        return vy + ch > 0 && vy < f.height && vx + cw > 0 && vx < f.width
    }

    function _hideCompletionIfCursorOutOfView(): void {
        if (!root.cursorInView && EditorPanel.completionVisible) {
            EditorPanel.hideCompletion()
        }
    }

    onCursorInViewChanged: {
        if (!cursorInView && EditorPanel.completionVisible) {
            Qt.callLater(root._hideCompletionIfCursorOutOfView)
        }
    }

    function _doFocusEditor(): void {
        if (root.visible && EditorPanel.currentDocument >= 0 && root.activeArea) {
            root.activeArea.forceActiveFocus()
        }
    }

    function focusEditor(): void {
        Qt.callLater(root._doFocusEditor)
    }

    function syncActivePane(): void {
        var docId = EditorPanel.currentDocId
        if (docId < 0) {
            if (root.activePane) {
                root.activePane.isActive = false
                root.activePane = null
            }
            root.activeDocId = -1
            root.hideHover()
            EditorPanel.hideCompletion()
            EditorPanel.focused = false
            return
        }
        var key = String(docId)
        var pane = root._panePool[key] as CodeEditorPane
        var switched = (root.activeDocId !== docId) || (root.activePane !== pane)
        if (!pane) {
            pane = paneComponent.createObject(paneHost, { docId: docId }) as CodeEditorPane
            if (!pane) return
            pane.syncTextFromModel(EditorPanel.currentText)
            root._panePool[key] = pane
            switched = true
        } else if (switched) {
            pane.syncTextFromModel(EditorPanel.currentText)
        }
        if (switched) {
            root.hideHover()
            EditorPanel.hideCompletion()
            if (root.activePane && root.activePane !== pane) {
                root.activePane.isActive = false
            }
            root.activePane = pane
            root.activeDocId = docId
            pane.isActive = true
            EditorPanel.cursorPos = pane.area.cursorPosition
            if (EditorPanel.focused || pane.area.activeFocus) {
                pane.syncEditMenuState()
            }
        }
    }

    function pruneClosedPanes(): void {
        var docs = EditorPanel.documents || []
        var openIds = ({})
        for (var i = 0; i < docs.length; ++i) {
            if (docs[i] && docs[i].id !== undefined) {
                openIds[String(docs[i].id)] = true
            }
        }
        var keys = Object.keys(root._panePool)
        for (var k = 0; k < keys.length; ++k) {
            var key = keys[k]
            if (!openIds[key]) {
                var pane = root._panePool[key] as CodeEditorPane
                delete root._panePool[key]
                if (pane) {
                    if (root.activePane === pane) {
                        root.activePane = null
                        root.activeDocId = -1
                    }
                    pane.isActive = false
                    pane.destroy()
                }
            }
        }
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
        var a = root.activeArea
        var _l = a ? a.lineCount : 1
        var r = a ? a.positionToRectangle(0) : Qt.rect(0, 0, 0, 0)
        return r.height > 1 ? r.height : (fm.lineSpacing > 1 ? fm.lineSpacing : Math.ceil(root.editorFont.pixelSize * 1.4))
    }
    readonly property int markerLaneWidth: Math.max(18, Math.round(root.lineHeight))
    readonly property int markerSize: Math.max(8, Math.min(12, Math.round(root.lineHeight * 0.58 / 2) * 2))
    readonly property int gutterWidth: Math.max(52, root.markerLaneWidth + Math.ceil(String(Math.max(1, root.activeLineCount)).length * fm.averageCharacterWidth) + 14)
    property int hoveredGutterLine: -1
    readonly property int cursorLine: {
        var a = root.activeArea
        if (!a) return 0
        var t = a.text.substring(0, a.cursorPosition)
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
        : (gutter.width + root.activeCursorRect.x - root.activeContentX)
    readonly property int effectiveHoverY: hoverTargetY >= 0
        ? hoverTargetY
        : (root.activeCursorRect.y - root.activeContentY)
    readonly property int effectiveHoverHeight: hoverTargetHeight > 0
        ? hoverTargetHeight
        : (root.activeCursorRect.height > 0 ? root.activeCursorRect.height : root.lineHeight)

    function containsLine(list: var, line: int): bool {
        if (!list) return false
        for (var i = 0; i < list.length; i++) if (Number(list[i]) === line) return true
        return false
    }

    function hideHover(): void {
        gutterHoverTimer.stop()
        gutterHoverTimer.targetLine = -1
        if (root.activePane) {
            root.activePane.stopHover()
        }
        root.hoverTargetX = -1
        root.hoverTargetY = -1
        EditorPanel.hideHover()
    }

    function applyCompletion(edit: var): void {
        var a = root.activeArea
        if (!a || !edit || !edit.valid) return
        if (edit.replaceStart < edit.replaceEnd) {
            a.remove(edit.replaceStart, edit.replaceEnd)
        }
        if (edit.insertText.length > 0) {
            a.insert(edit.replaceStart, edit.insertText)
        }
        a.cursorPosition = edit.newCursorPos
        EditorPanel.cursorPos = edit.newCursorPos
        EditorPanel.finishCompletion()
        a.forceActiveFocus()
    }

    FontMetrics {
        id: fm
        font: root.editorFont
    }

    onVisibleChanged: {
        EditorPanel.dark = CircuitCanvas.dark
        if (visible && EditorPanel.currentDocument >= 0) {
            root.syncActivePane()
            root.focusEditor()
        } else if (!visible) {
            root.hideHover()
            EditorPanel.hideCompletion()
        }
    }
    Component.onCompleted: {
        EditorPanel.dark = CircuitCanvas.dark
        root.syncActivePane()
    }
    Component.onDestruction: {
        EditorPanel.hideCompletion()
        var keys = Object.keys(root._panePool)
        for (var k = 0; k < keys.length; ++k) {
            var pane = root._panePool[keys[k]] as CodeEditorPane
            if (pane) pane.destroy()
        }
        root._panePool = ({})
        root.activePane = null
        root.activeDocId = -1
    }
    Connections {
        target: CircuitCanvas
        function onAppearanceChanged() { EditorPanel.dark = CircuitCanvas.dark }
    }
    Connections {
        target: root.hostWindow
        function onActiveChanged(): void {
            if (root.hostWindow && !root.hostWindow.active) {
                EditorPanel.hideCompletion()
            }
        }
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
            contentY: root.activeContentY
            contentHeight: root.activeImplicitHeight

            Rectangle {
                id: hoverBpDot
                visible: root.hoveredGutterLine >= 1
                         && root.hoveredGutterLine <= root.activeLineCount
                         && !root.containsLine(root.breakpoints, root.hoveredGutterLine)
                         && !root.containsLine(root.errors, root.hoveredGutterLine)
                         && !root.containsLine(root.warnings, root.hoveredGutterLine)
                         && EditorPanel.debugLine !== root.hoveredGutterLine
                z: 1
                width: root.markerSize
                height: root.markerSize
                radius: width / 2
                x: Math.round((root.markerLaneWidth - width) / 2)
                y: Math.round(root.activeTopPadding + (root.hoveredGutterLine - 1) * root.lineHeight + (root.lineHeight - height) / 2)
                color: Qt.alpha(EditorPanel.errorColor, 0.32)
                border.width: 1
                border.color: Qt.alpha(EditorPanel.errorColor, 0.65)
            }

            Repeater {
                model: Math.max( 1, root.activeLineCount )
                delegate: Item {
                    id: gutterLine
                    required property int index
                    y: root.activeTopPadding + gutterLine.index * root.lineHeight
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
                            font.family: root.editorFont.family
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
                        font.family: root.editorFont.family
                        font.pixelSize: root.editorFont.pixelSize
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
                visible: dbgLine >= 1 && dbgLine <= root.activeLineCount
                z: 4
                x: 0
                y: Math.round(root.activeTopPadding + (dbgLine - 1) * root.lineHeight)
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
                if (targetLine >= 1 && targetLine <= root.activeLineCount && !EditorPanel.completionVisible) {
                    var tip = EditorPanel.gutterTooltip(targetLine)
                    if (tip && tip.length > 0) {
                        root.hoverTargetX = gutter.width + 4
                        root.hoverTargetY = root.activeTopPadding + (targetLine - 1) * root.lineHeight - gutterFlick.contentY
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
                var line = Math.floor((mouse.y + gutterFlick.contentY - root.activeTopPadding) / Math.max(1, root.lineHeight)) + 1
                EditorPanel.gutterClicked(line, mouse.button)
            }
            onPositionChanged: (mouse) => {
                var line = Math.floor((mouse.y + gutterFlick.contentY - root.activeTopPadding) / Math.max(1, root.lineHeight)) + 1
                if (line < 1 || line > root.activeLineCount) {
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

    Item {
        id: paneHost
        anchors.left: gutter.right
        anchors.right: parent.right
        anchors.top: parent.top
        anchors.bottom: parent.bottom
        clip: true
    }

    Component {
        id: paneComponent
        CodeEditorPane {
            anchors.fill: parent
            editorFont: root.editorFont
            lineHeight: root.lineHeight
            cursorLine: root.cursorLine
            gutterWidth: gutter.width
            averageCharacterWidth: fm.averageCharacterWidth
            hoverTooltipVisible: hoverTooltip.visible
            hoverTooltipRect: Qt.rect(hoverTooltip.x, hoverTooltip.y, hoverTooltip.width, hoverTooltip.height)
            onHideHoverRequested: root.hideHover()
            onHoverTargetRequested: (tx, ty, th, pos) => {
                root.hoverTargetX = tx
                root.hoverTargetY = ty
                root.hoverTargetHeight = th
                EditorPanel.showHover(pos)
            }
            onContextMenuRequested: editorMenu.popup()
            onCompletionAccepted: (edit) => root.applyCompletion(edit)
        }
    }

    AppContextMenu {
        id: editorMenu

        readonly property bool _hasSelection: Boolean(root.activeArea && root.activeArea.selectionStart !== root.activeArea.selectionEnd)

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
            enabled: Boolean(root.activeArea && root.activeArea.canUndo)
            onTriggered: if (root.activeArea) root.activeArea.undo()
        }
        ContextMenuItem {
            iconLigature: "redo"
            text: I18n.tr("Redo")
            shortcutText: UiUtils.formatShortcut(UiUtils.isMacOS ? "Ctrl+Shift+Z" : "Ctrl+Y")
            enabled: Boolean(root.activeArea && root.activeArea.canRedo)
            onTriggered: if (root.activeArea) root.activeArea.redo()
        }

        ContextMenuSeparator {}

        ContextMenuItem {
            iconLigature: "content_cut"
            text: I18n.tr("Cut")
            shortcutText: UiUtils.formatShortcut("Ctrl+X")
            enabled: editorMenu._hasSelection
            onTriggered: if (root.activeArea) root.activeArea.cut()
        }
        ContextMenuItem {
            iconLigature: "content_copy"
            text: I18n.tr("Copy")
            shortcutText: UiUtils.formatShortcut("Ctrl+C")
            enabled: editorMenu._hasSelection
            onTriggered: if (root.activeArea) root.activeArea.copy()
        }
        ContextMenuItem {
            iconLigature: "content_paste"
            text: I18n.tr("Paste")
            shortcutText: UiUtils.formatShortcut("Ctrl+V")
            enabled: Boolean(root.activeArea && root.activeArea.canPaste)
            onTriggered: if (root.activeArea) root.activeArea.paste()
        }
        ContextMenuItem {
            iconLigature: "delete"
            text: I18n.tr("Remove")
            enabled: editorMenu._hasSelection
            onTriggered: if (root.activeArea) root.activeArea.remove(root.activeArea.selectionStart, root.activeArea.selectionEnd)
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
        x: Math.max(gutter.width + 4, Math.min(root.width - width - 10, gutter.width + root.activeCursorRect.x + root.activeLeftPadding - root.activeContentX))
        y: {
            var curY = root.activeCursorRect.y + root.activeTopPadding - root.activeContentY
            if (curY >= height + 6) {
                return curY - height - 4
            } else {
                return curY + root.activeCursorRect.height + 4
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
            font: root.editorFont
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
            font: root.editorFont
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
        visible: root.visible && root.activePane !== null && root.cursorInView && EditorPanel.completionVisible
        focus: false
        readonly property real edgeMargin: 8
        readonly property real cursorGap: 2
        readonly property real windowW: root.Overlay.overlay ? root.Overlay.overlay.width : (root.hostWindow ? root.hostWindow.width : root.width)
        readonly property real windowH: root.Overlay.overlay ? root.Overlay.overlay.height : (root.hostWindow ? root.hostWindow.height : root.height)
        readonly property point rootOrigin: {
            var _rects = CircuitPanel.overlayRects
            var _w = root.width
            var _h = root.height
            var _ww = completionPopup.windowW
            var _wh = completionPopup.windowH
            var _v = completionPopup.visible
            return root.mapToItem(root.Overlay.overlay, 0, 0)
        }
        readonly property real cursorLocalX: gutter.width + root.activeCursorRect.x - root.activeContentX
        readonly property real cursorLocalY: root.activeCursorRect.y - root.activeContentY
        readonly property real cursorH: root.activeCursorRect.height > 0 ? root.activeCursorRect.height : root.lineHeight
        width: Math.min(380, Math.max(160, completionPopup.windowW - 2 * completionPopup.edgeMargin))
        height: {
            var naturalH = Math.min(240, Math.max(24, completionList.count * 22 + 2))
            var maxH = Math.max(24, completionPopup.windowH - 2 * completionPopup.edgeMargin)
            return Math.min(naturalH, maxH)
        }
        x: {
            var winX = completionPopup.rootOrigin.x + completionPopup.cursorLocalX
            var targetWinX = (winX + width <= completionPopup.windowW - completionPopup.edgeMargin)
                ? Math.max(completionPopup.edgeMargin, winX)
                : Math.max(completionPopup.edgeMargin, Math.min(completionPopup.windowW - width - completionPopup.edgeMargin, winX - width))
            return Math.round(targetWinX - completionPopup.rootOrigin.x)
        }
        y: {
            var cursorWinY = completionPopup.rootOrigin.y + completionPopup.cursorLocalY
            var belowWinY = cursorWinY + completionPopup.cursorH + completionPopup.cursorGap
            var aboveWinY = cursorWinY - height - completionPopup.cursorGap
            var targetWinY
            if (belowWinY + height <= completionPopup.windowH - completionPopup.edgeMargin) {
                targetWinY = Math.max(completionPopup.edgeMargin, belowWinY)
            } else if (aboveWinY >= completionPopup.edgeMargin) {
                targetWinY = aboveWinY
            } else {
                var spaceBelow = completionPopup.windowH - completionPopup.edgeMargin - belowWinY
                var spaceAbove = cursorWinY - completionPopup.cursorGap - completionPopup.edgeMargin
                targetWinY = (spaceAbove >= spaceBelow)
                    ? Math.max(completionPopup.edgeMargin, aboveWinY)
                    : Math.max(completionPopup.edgeMargin, Math.min(completionPopup.windowH - height - completionPopup.edgeMargin, belowWinY))
            }
            return Math.round(targetWinY - completionPopup.rootOrigin.y)
        }
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
            boundsBehavior: Flickable.StopAtBounds
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
                        font: root.editorFont
                        color: completionDelegate.index === completionList.currentIndex ? appTheme.highlightedText : appTheme.windowText
                    }
                    AppText {
                        text: completionDelegate.modelData.detail !== undefined ? completionDelegate.modelData.detail : ""
                        font: root.editorFont
                        opacity: 0.6
                        color: completionDelegate.index === completionList.currentIndex ? appTheme.highlightedText : appTheme.windowText
                    }
                }
                MouseArea {
                    anchors.fill: parent
                    onClicked: root.applyCompletion(EditorPanel.acceptCompletion(completionDelegate.index, root.activeArea ? root.activeArea.cursorPosition : 0))
                }
            }
        }
    }

    Connections {
        target: EditorPanel
        function onRequestFocus(): void {
            root.focusEditor()
        }
        function onDocumentsChanged(): void {
            root.syncActivePane()
            root.pruneClosedPanes()
        }
        function onCurrentDocumentChanged(): void {
            root.syncActivePane()
            if (EditorPanel.currentDocument >= 0) {
                root.focusEditor()
            }
        }
        function onCurrentTextChanged(): void {
            root.syncActivePane()
            if (root.activePane) {
                root.activePane.syncTextFromModel(EditorPanel.currentText)
            }
        }
        function onGotoChanged(): void {
            root.syncActivePane()
            var a = root.activeArea
            if (!a) return
            root.focusEditor()
            if (EditorPanel.selectStart >= 0 && EditorPanel.selectEnd > EditorPanel.selectStart) {
                a.select(EditorPanel.selectStart, EditorPanel.selectEnd)
                return
            }
            if (EditorPanel.gotoLine > 0) {
                var t = a.text
                var line = 1
                var pos = 0
                while (line < EditorPanel.gotoLine) {
                    var n = t.indexOf("\n", pos)
                    if (n < 0) { pos = t.length; break }
                    pos = n + 1
                    line++
                }
                a.cursorPosition = pos
            }
        }
        function onEditActionChanged(): void {
            var a = root.activeArea
            if (!a) return
            var act = EditorPanel.editAction
            if (act === "undo") a.undo()
            else if (act === "redo") a.redo()
            else if (act === "cut") a.cut()
            else if (act === "copy") a.copy()
            else if (act === "paste") a.paste()
            else if (act === "selectAll") a.selectAll()
        }
        function onCursorChanged(): void {
            var a = root.activeArea
            if (a && Math.abs(a.cursorPosition - EditorPanel.cursorPos) > 0
                    && EditorPanel.selectStart < 0) {
                a.cursorPosition = EditorPanel.cursorPos
            }
        }
    }
}
