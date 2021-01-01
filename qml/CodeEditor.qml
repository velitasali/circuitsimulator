import QtQuick
import QtQuick.Controls
import cs_app

/* TextArea + QML gutter + HTML overlay. LineNumberArea / QSyntaxHighlighter
 * are not available through qt-bridge. */
Item {
    id: root
    SystemPalette { id: appTheme }
    function tr(s) { return App.i18nTick >= 0 ? App.translate(s) : s }

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
            fixedPitch: true,
            styleHint: Font.Monospace
        })
    }

    readonly property int lineHeight: {
        var r = area.positionToRectangle(0)
        return r.height > 1 ? r.height : Math.ceil(area.font.pixelSize * 1.4)
    }
    readonly property int gutterWidth: Math.max(48, String(Math.max(1, area.lineCount)).length * fm.averageCharacterWidth + 24)
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

    function containsLine(list, line) {
        if (!list) return false
        for (var i = 0; i < list.length; i++) if (Number(list[i]) === line) return true
        return false
    }

    function hideHover() {
        gutterHoverTimer.stop()
        gutterHoverTimer.targetLine = -1
        hoverTimer.stop()
        hoverTimer.targetPos = -1
        root.hoverTargetX = -1
        root.hoverTargetY = -1
        EditorPanel.hideHover()
    }

    function applyCompletion(edit) {
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

    onVisibleChanged: EditorPanel.dark = CircuitCanvas.dark
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

            Repeater {
                model: Math.max( 1, area.lineCount )
                delegate: Item {
                    required property int index
                    y: area.topPadding + index * root.lineHeight
                    width: gutter.width
                    height: root.lineHeight
                    readonly property int line: index + 1
                    readonly property bool isBp: root.containsLine(root.breakpoints, line)
                    readonly property bool isErr: root.containsLine(root.errors, line)
                    readonly property bool isWarn: root.containsLine(root.warnings, line)
                    readonly property bool isDbg: EditorPanel.debugLine === line

                    Rectangle {
                        id: bpDot
                        visible: isBp && !isErr && !isWarn
                        z: 1
                        width: Math.max(8, root.lineHeight - 6)
                        height: width
                        radius: width / 2
                        anchors.verticalCenter: parent.verticalCenter
                        x: 3
                        color: EditorPanel.errorColor
                    }
                    Rectangle {
                        id: errWarnDot
                        visible: isErr || isWarn
                        z: 2
                        width: Math.max(8, root.lineHeight - 6)
                        height: width
                        radius: width / 2
                        anchors.verticalCenter: parent.verticalCenter
                        x: 3
                        color: isErr ? EditorPanel.errorColor : EditorPanel.warnColor
                        border.width: isBp ? 1.5 : 0
                        border.color: isBp ? (isErr ? "#ffffff" : EditorPanel.errorColor) : "transparent"

                        Text {
                            anchors.centerIn: parent
                            text: "!"
                            color: "#ffffff"
                            font.bold: true
                            font.pixelSize: Math.max(7, Math.round(parent.height * 0.75))
                            font.family: area.font.family
                            verticalAlignment: Text.AlignVCenter
                            horizontalAlignment: Text.AlignHCenter
                        }
                    }
                    Rectangle {
                        id: dbgDot
                        visible: isDbg
                        z: 3
                        width: isBp ? parent.height * 0.28 : parent.height * 0.45
                        height: width * 1.4
                        anchors.verticalCenter: parent.verticalCenter
                        x: isBp ? 6 : 4
                        color: isBp ? appTheme.highlightedText : (CircuitCanvas.msgWarnBg || appTheme.highlight)
                    }
                    Text {
                        anchors.fill: parent
                        anchors.rightMargin: 8
                        horizontalAlignment: Text.AlignRight
                        verticalAlignment: Text.AlignVCenter
                        text: line
                        font.family: area.font.family
                        font.pixelSize: area.font.pixelSize
                        color: index === root.cursorLine ? EditorPanel.gutterActiveNumColor : EditorPanel.gutterNumColor
                        font.bold: index === root.cursorLine
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
            onClicked: (mouse) => {
                root.hideHover()
                var line = Math.floor((mouse.y + gutterFlick.contentY - area.topPadding) / Math.max(1, root.lineHeight)) + 1
                EditorPanel.gutterClicked(line, mouse.button)
            }
            onPositionChanged: (mouse) => {
                var line = Math.floor((mouse.y + gutterFlick.contentY - area.topPadding) / Math.max(1, root.lineHeight)) + 1
                if (line < 1 || line > area.lineCount) {
                    root.hideHover()
                    return
                }
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
                    required property var modelData
                    parent: vBar
                    x: 0
                    width: vBar.width
                    height: modelData.weight || 2
                    y: (modelData.pos || 0) * (vBar.height - height)
                    color: modelData.color || "transparent"
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
            text: EditorPanel.currentText
            onTextChanged: {
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
                    required property var modelData
                    z: -2
                    property var r: area.positionToRectangle(modelData.start)
                    x: r.x
                    y: r.y
                    width: Math.max(4, area.positionToRectangle(modelData.end).x - r.x)
                    height: r.height
                    color: EditorPanel.foundColor
                }
            }

            Text {
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

                    property var r1: {
                        var _ = area.length
                        var __ = root.editorFont
                        return area.positionToRectangle(modelData.start)
                    }
                    property var r2: {
                        var _ = area.length
                        var __ = root.editorFont
                        return area.positionToRectangle(modelData.end)
                    }

                    x: r1.x
                    y: r1.y + r1.height - 2
                    width: Math.max(4, r2.x - r1.x)
                    height: 2

                    Rectangle {
                        anchors.fill: parent
                        color: modelData.color
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

                function hideHoverState() {
                    root.hideHover()
                }

                function updateHoverLink(modifiers) {
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

            Keys.onPressed: (event) => {
                if (event.key === Qt.Key_Control || event.key === Qt.Key_Meta) {
                    editorMouseArea.updateHoverLink(event.modifiers)
                } else {
                    root.hideHover()
                }
                if (EditorPanel.completionVisible) {
                    if (event.key === Qt.Key_Return || event.key === Qt.Key_Enter || event.key === Qt.Key_Tab) {
                        applyCompletion(EditorPanel.acceptCompletion(EditorPanel.completionIndex, area.cursorPosition))
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
                var ctrl = (event.modifiers & Qt.ControlModifier) || (event.modifiers & Qt.MetaModifier)
                if (ctrl && event.key === Qt.Key_Space) {
                    EditorPanel.complete(true)
                    event.accepted = true
                    return
                }
                if (ctrl && event.key === Qt.Key_F) {
                    EditorPanel.findDialog()
                    event.accepted = true
                    return
                }
                if (ctrl && event.key === Qt.Key_S) {
                    if (event.modifiers & Qt.ShiftModifier)
                        AppMenuBar.triggerAction("file.saveFileAs")
                    else
                        AppMenuBar.triggerAction("file.saveFile")
                    event.accepted = true
                    return
                }
                if (ctrl && event.key === Qt.Key_O) {
                    AppMenuBar.triggerAction("file.open")
                    event.accepted = true
                    return
                }
                if (event.key === Qt.Key_Escape) {
                    EditorPanel.hideSignature()
                }
                if (event.key === Qt.Key_Tab && EditorPanel.spaceTabs) {
                    var n = EditorPanel.tabSize
                    var sp = " ".repeat(n)
                    area.insert(area.cursorPosition, sp)
                    event.accepted = true
                    return
                }
                function pair(open, close, flag) {
                    if (!flag || event.text !== open) return false
                    area.insert(area.cursorPosition, open + close)
                    area.cursorPosition = area.cursorPosition - 1
                    event.accepted = true
                    if (open === "(") EditorPanel.requestSignature(area.cursorPosition)
                    return true
                }
                if (pair("(", ")", EditorPanel.closeParen)) return
                if (pair("{", "}", EditorPanel.closeBraces)) return
                if (pair("[", "]", EditorPanel.closeBrackets)) return
                if (pair("\"", "\"", EditorPanel.closeQuotes)) return
                if (pair("'", "'", EditorPanel.closeSquotes)) return

                if (event.text === ")") {
                    EditorPanel.hideSignature()
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

    Shortcut {
        sequence: "Ctrl+Shift+I"
        onActivated: EditorPanel.formatDocument()
    }
    Shortcut {
        sequence: "Ctrl+F12"
        onActivated: EditorPanel.gotoDefinition()
    }

    AppContextMenu {
        id: editorMenu

        readonly property bool _mac: Qt.platform.os === "osx" || Qt.platform.os === "macos"
        readonly property string _mod: _mac ? "⌘" : "Ctrl+"
        readonly property bool _hasSelection: area.selectedText && area.selectedText.length > 0

        ContextMenuItem {
            visible: EditorPanel.lspReady
            height: visible ? implicitHeight : 0
            iconLigature: "search"
            text: root.tr("Go to Definition")
            shortcutText: editorMenu._mod + "F12"
            onTriggered: EditorPanel.gotoDefinition()
        }
        ContextMenuSeparator { visible: EditorPanel.lspReady }

        ContextMenuItem {
            iconLigature: "undo"
            text: root.tr("Undo")
            shortcutText: editorMenu._mod + "Z"
            enabled: area.canUndo
            onTriggered: area.undo()
        }
        ContextMenuItem {
            iconLigature: "redo"
            text: root.tr("Redo")
            shortcutText: editorMenu._mac ? "⇧⌘Z" : "Ctrl+Y"
            enabled: area.canRedo
            onTriggered: area.redo()
        }

        ContextMenuSeparator {}

        ContextMenuItem {
            iconLigature: "content_cut"
            text: root.tr("Cut")
            shortcutText: editorMenu._mod + "X"
            enabled: editorMenu._hasSelection
            onTriggered: area.cut()
        }
        ContextMenuItem {
            iconLigature: "content_copy"
            text: root.tr("Copy")
            shortcutText: editorMenu._mod + "C"
            enabled: editorMenu._hasSelection
            onTriggered: area.copy()
        }
        ContextMenuItem {
            iconLigature: "content_paste"
            text: root.tr("Paste")
            shortcutText: editorMenu._mod + "V"
            enabled: area.canPaste
            onTriggered: area.paste()
        }
        ContextMenuItem {
            iconLigature: "delete"
            text: root.tr("Remove")
            enabled: editorMenu._hasSelection
            onTriggered: area.remove(area.selectionStart, area.selectionEnd)
        }

        ContextMenuSeparator {}

        ContextMenuItem {
            iconLigature: "format_align_left"
            text: root.tr("Format Document")
            shortcutText: editorMenu._mac ? "⇧⌘I" : "Ctrl+Shift+I"
            onTriggered: EditorPanel.formatDocument()
        }

        ContextMenuSeparator { visible: EditorPanel.hasCompiler }
        ContextMenuItem {
            visible: EditorPanel.hasCompiler
            height: visible ? implicitHeight : 0
            iconLigature: "settings"
            text: EditorPanel.compilerName.length > 0
                  ? root.tr("%1 Settings").arg(EditorPanel.compilerName)
                  : root.tr("Compiler Settings")
            onTriggered: EditorPanel.compilerProps()
        }
        ContextMenuItem {
            visible: EditorPanel.hasCompiler
            height: visible ? implicitHeight : 0
            iconLigature: "settings_applications"
            text: root.tr("File Settings")
            onTriggered: EditorPanel.fileProps()
        }

        ContextMenuSeparator {}

        ContextMenuItem {
            iconLigature: "refresh"
            text: root.tr("Reload Document")
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
            radius: 6
        }

        Text {
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
            radius: 6
        }

        Text {
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
        x: Math.round((parent.width - width) / 2)
        y: Math.round((parent.height - height) / 2)
        modal: true
        title: EditorPanel.diskPromptKind === "deleted" ? "File Deleted on Disk" : "File Changed on Disk"
        standardButtons: Dialog.NoButton

        contentItem: Item {
            implicitWidth: 380
            implicitHeight: col.implicitHeight

            Column {
                id: col
                width: parent.width
                spacing: 12
                Text {
                    width: parent.width
                    text: EditorPanel.diskPromptKind === "deleted"
                        ? "\n" + EditorPanel.diskPromptPath + "\n\nhas been deleted or renamed by another program."
                        : "\n" + EditorPanel.diskPromptPath + "\n\nhas been changed by another program,\nand you have unsaved changes here."
                    color: appTheme.windowText
                    font.pixelSize: 13
                    wrapMode: Text.Wrap
                }
                Row {
                    spacing: 10
                    anchors.right: parent.right
                    AppButton {
                        text: EditorPanel.diskPromptKind === "deleted" ? "Keep in Editor" : "Reload"
                        onClicked: {
                            if (EditorPanel.diskPromptKind === "deleted") {
                                EditorPanel.diskPromptKeep()
                            } else {
                                EditorPanel.diskPromptReload()
                            }
                        }
                    }
                    AppButton {
                        text: EditorPanel.diskPromptKind === "deleted" ? "Close Tab" : "Keep my Changes"
                        highlighted: true
                        onClicked: {
                            if (EditorPanel.diskPromptKind === "deleted") {
                                EditorPanel.diskPromptClose()
                            } else {
                                EditorPanel.diskPromptKeep()
                            }
                        }
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
            radius: 4
        }

        ListView {
            id: completionList
            anchors.fill: parent
            clip: true
            model: EditorPanel.completions
            currentIndex: EditorPanel.completionIndex
            onCurrentIndexChanged: positionViewAtIndex(currentIndex, ListView.Contain)
            delegate: Rectangle {
                required property var modelData
                required property int index
                width: completionList.width
                height: 22
                color: index === completionList.currentIndex ? appTheme.highlight : "transparent"
                Row {
                    anchors.verticalCenter: parent.verticalCenter
                    anchors.left: parent.left
                    anchors.leftMargin: 6
                    spacing: 6
                    Text {
                        text: modelData.text !== undefined ? modelData.text : modelData
                        font: area.font
                        color: index === completionList.currentIndex ? appTheme.highlightedText : appTheme.windowText
                    }
                    Text {
                        text: modelData.detail !== undefined ? modelData.detail : ""
                        font: area.font
                        opacity: 0.6
                        color: index === completionList.currentIndex ? appTheme.highlightedText : appTheme.windowText
                    }
                }
                MouseArea {
                    anchors.fill: parent
                    onClicked: applyCompletion(EditorPanel.acceptCompletion(index, area.cursorPosition))
                }
            }
        }
    }

    Connections {
        target: EditorPanel
        function onGotoChanged() {
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
        }
        function onCursorChanged() {
            if (Math.abs(area.cursorPosition - EditorPanel.cursorPos) > 0
                    && EditorPanel.selectStart < 0)
                area.cursorPosition = EditorPanel.cursorPos
        }
    }
}
