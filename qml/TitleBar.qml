pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtQuick.Templates as T
import QtQuick.Window
import cs_app
import cs_ui

/* Transparent main-window title strip over the live canvas.
 *
 * Windows: WinChromeNativeFilter returns HTCLIENT on caption MouseAreas
 * (titleBarInteractive), HTTOP along the top resize band, and HTCAPTION on
 * the icon, title, and empty strip so the window drags. The strip draws
 * Minimize / Maximize / Close.
 *
 * macOS: traffic lights stay. Qt.ExpandedClientAreaHint extends the canvas
 * under them. A press on this strip drags; a double-click zooms. The
 * caption buttons stay hidden.
 */
Item {
    id: bar
    objectName: "titleBar"
    SystemPalette { id: appTheme }

    required property Window window

    readonly property bool macOS: Qt.platform.os === "osx"
    // Full left margin. 74 matches the native fallback until the zoom
    // button is measured. 0 means the default 8px margin on Windows.
    property int leadingInset: macOS ? 74 : 0

    function refreshChrome() {
        if (!macOS)
            return
        leadingInset = App.titlebarLeadingInset()
    }

    Connections {
        target: bar.window
        function onVisibilityChanged() { Qt.callLater(bar.refreshChrome) }
    }

    readonly property bool windowActive: window ? window.active : true
    readonly property bool maximized: window ? (window.visibility === Window.Maximized || window.visibility === Window.FullScreen) : false
    readonly property string projectName: FileBrowser.rootPath
        ? (FileBrowser.rootPath.split("/").filter(Boolean).pop() || "")
        : ""
    readonly property string fileLabel: CircuitCanvas.fileName ? CircuitCanvas.fileName : I18n.tr("Untitled")

    readonly property int leftEdge: bar.leadingInset > 0 ? bar.leadingInset : 8
    readonly property int updatePillWidth: 26
    readonly property int baseRightEdge: (captionCluster.visible ? captionCluster.width : 0)
        + (Updater.updateAvailable ? (bar.updatePillWidth + bar.pillGap) : 0) + 12
    readonly property int coordsPillWidth: Math.ceil(coordsRow.implicitWidth) + 14
    readonly property bool coordsVisible: (bar.width - bar.leftEdge - bar.baseRightEdge
        - bar.leftUtilityWidth - bar.pillGap - bar.coordsPillWidth - bar.pillGap) >= bar.titleContentMinWidth
    readonly property int rightEdge: bar.baseRightEdge
        + (bar.coordsVisible ? (bar.coordsPillWidth + bar.pillGap) : 0)
    readonly property int usableWidth: Math.max(0, bar.width - bar.leftEdge - bar.rightEdge)
    readonly property int pillGap: 8
    // 2px padding on each side + two 22px icon buttons + 2px spacing.
    readonly property int historyPillWidth: 50
    // 2px padding on each side + one 22px icon button.
    readonly property int commandCenterPillWidth: 26
    readonly property int leftUtilityWidth: bar.historyPillWidth + bar.pillGap + bar.commandCenterPillWidth
    // icon 16 + row spacing 4. The row drops a hidden MenuBar,
    // so the fit check cannot read menuPill.implicitWidth.
    readonly property int menuPillChrome: 20
    readonly property int menuPillWidth: Math.ceil(menuBar.implicitWidth) + bar.menuPillChrome
    // Floors of the title pill once the menu icon has moved onto the menu pill:
    // sim chip, project badge (48), file label (64), plus the pill's own padding.
    readonly property int titleContentMinWidth: {
        var w = 12 + Math.min(fileBtn.naturalWidth, 64)
        if (bar.projectName !== "")
            w += Math.min(Math.ceil(projText.implicitWidth) + 10, 48) + leftGroup.spacing
        if (CircuitCanvas.simRunning)
            w += Math.ceil(simStateText.implicitWidth) + 12 + leftGroup.spacing
        return w
    }
    readonly property bool menuBarVisible: !App.nativeMenus
        && menuBar.implicitWidth > 0
        && (bar.usableWidth - bar.menuPillWidth - bar.pillGap - bar.leftUtilityWidth - bar.pillGap >= bar.titleContentMinWidth)

    function actionTooltip(label: string, actionId: string): string {
        var sc = UiUtils.shortcutFor(actionId)
        return sc !== "" ? (label + " (" + sc + ")") : label
    }

    onMenuBarVisibleChanged: {
        if (!menuBarVisible)
            closeAllMenus()
    }

    implicitHeight: 34
    height: visible ? 34 : 0

    AppMenuBuilder {
        id: menuBuilder
    }

    function anyMenuOpen(): bool {
        for (var i = 0; i < menuBar.count; ++i) {
            var m = menuBar.menuAt(i)
            if (m && m.opened)
                return true
        }
        return false
    }

    function syncAltMenuOpen(): void {
        if (bar.menuBarVisible)
            AppMenuBar.setAltMenuOpen(bar.anyMenuOpen())
    }

    function closeSwitcherPopups(): void {
        if (projectPopup.opened)
            projectPopup.close()
        if (circuitPopup.opened)
            circuitPopup.close()
    }

    function toggleProjectPopup(): void {
        var wasOpen = projectPopup.opened
        closeAllMenus()
        if (wasOpen)
            return
        projectPopup.openUnder(projBadge, FileBrowser.recentProjectItems())
    }

    function toggleCircuitPopup(): void {
        var wasOpen = circuitPopup.opened
        closeAllMenus()
        if (wasOpen)
            return
        circuitPopup.openUnder(fileBtn, FileBrowser.circuitSwitcherItems(CircuitCanvas.filePath))
    }

    function closeAllMenus(): void {
        closeSwitcherPopups()
        for (var i = 0; i < menuBar.count; ++i) {
            var m = menuBar.menuAt(i) as AppContextMenu
            if (m && m.opened)
                m.closeTree()
        }
        if (!App.nativeMenus)
            AppMenuBar.setAltMenuOpen(false)
    }

    component SwitcherPopup: Popup {
        id: popup

        property var rows: []
        property string emptyText: I18n.tr("(None)")

        signal activated(string path)

        popupType: Popup.Item
        modal: false
        focus: true
        padding: 4
        margins: 0
        closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutsideParent

        readonly property int minPopupWidth: 220
        readonly property int maxPopupWidth: Math.max(minPopupWidth, Math.min(420, Math.floor(bar.width) - 16))
        readonly property int maxPopupHeight: Math.max(80, Math.min(360, (bar.window ? bar.window.height : 600) - bar.height - 16))

        readonly property int contentNaturalHeight: {
            var arr = popup.rows
            if (!arr || arr.length === 0)
                return 28
            var h = 0
            for (var i = 0; i < arr.length; ++i)
                h += (arr[i] && arr[i].kind === "header") ? 22 : 28
            return h
        }

        width: {
            var arr = popup.rows
            var maxChars = 0
            if (arr) {
                for (var i = 0; i < arr.length; ++i) {
                    var r = arr[i]
                    if (!r || r.kind === "header")
                        continue
                    var nameLen = String(r.name || "").length
                    var detailLen = String(r.detail || "").length
                    maxChars = Math.max(maxChars, nameLen + (detailLen > 0 ? Math.min(detailLen, 28) + 4 : 0))
                }
            }
            return Math.max(minPopupWidth, Math.min(maxPopupWidth, maxChars * 7 + 68))
        }
        height: Math.min(contentNaturalHeight + topPadding + bottomPadding, maxPopupHeight)

        function firstSelectableIndex(): int {
            var arr = popup.rows
            if (!arr)
                return -1
            var first = -1
            for (var i = 0; i < arr.length; ++i) {
                var r = arr[i]
                if (!r || r.kind !== "item")
                    continue
                if (first < 0)
                    first = i
                if (r.current)
                    return i
            }
            return first
        }

        function moveSelection(delta: int): void {
            var arr = popup.rows
            if (!arr || arr.length === 0)
                return
            var idx = list.currentIndex
            for (var step = 0; step < arr.length; ++step) {
                idx = (idx + delta + arr.length) % arr.length
                if (arr[idx] && arr[idx].kind === "item") {
                    list.currentIndex = idx
                    list.positionViewAtIndex(idx, ListView.Contain)
                    return
                }
            }
        }

        function activateIndex(idx: int): void {
            var arr = popup.rows
            if (!arr || idx < 0 || idx >= arr.length)
                return
            var r = arr[idx]
            if (!r || r.kind !== "item")
                return
            popup.close()
            if (!r.current && r.path)
                popup.activated(String(r.path))
        }

        function openUnder(anchorItem: Item, items: var): void {
            popup.rows = items || []
            list.currentIndex = firstSelectableIndex()
            var pt = anchorItem.mapToItem(bar, 0, anchorItem.height)
            var desiredSceneX = Math.max(8, Math.min(bar.width - popup.width - 8, Math.round(pt.x)))
            popup.x = desiredSceneX - pt.x
            popup.y = anchorItem.height + 6
            popup.open()
            list.forceActiveFocus()
        }

        background: AppSurface {
            radius: 10
            elevation: 0.85
            shadowEnabled: true
        }

        contentItem: Item {
            implicitWidth: popup.width - popup.leftPadding - popup.rightPadding
            implicitHeight: popup.contentNaturalHeight

            AppText {
                anchors.fill: parent
                anchors.leftMargin: 10
                anchors.rightMargin: 10
                visible: !popup.rows || popup.rows.length === 0
                text: popup.emptyText
                font.pixelSize: App.fontSmall
                color: appTheme.windowText
                opacity: 0.55
                verticalAlignment: Text.AlignVCenter
            }

            ListView {
                id: list
                anchors.fill: parent
                visible: Boolean(popup.rows && popup.rows.length > 0)
                clip: true
                model: popup.rows
                boundsBehavior: Flickable.StopAtBounds
                currentIndex: -1
                highlightFollowsCurrentItem: false

                ScrollBar.vertical: AppScrollBar {
                    id: vbar
                    policy: ScrollBar.AsNeeded
                }

                Keys.onUpPressed: (event) => {
                    popup.moveSelection(-1)
                    event.accepted = true
                }
                Keys.onDownPressed: (event) => {
                    popup.moveSelection(1)
                    event.accepted = true
                }
                Keys.onReturnPressed: (event) => {
                    popup.activateIndex(list.currentIndex)
                    event.accepted = true
                }
                Keys.onEnterPressed: (event) => {
                    popup.activateIndex(list.currentIndex)
                    event.accepted = true
                }

                delegate: Item {
                    id: rowItem
                    required property var modelData
                    required property int index

                    readonly property bool isHeader: Boolean(rowItem.modelData && rowItem.modelData.kind === "header")
                    readonly property bool isCurrent: Boolean(rowItem.modelData && rowItem.modelData.current)
                    readonly property bool isHighlighted: !isHeader && (rowMouse.containsMouse || list.currentIndex === rowItem.index)

                    width: list.width - (vbar.visible ? vbar.width : 0)
                    height: isHeader ? 22 : 28

                    Rectangle {
                        anchors.fill: parent
                        visible: !rowItem.isHeader
                        radius: 6
                        color: rowItem.isHighlighted
                            ? Qt.rgba(appTheme.windowText.r, appTheme.windowText.g, appTheme.windowText.b, 0.12)
                            : "transparent"
                    }

                    AppText {
                        visible: rowItem.isHeader
                        anchors.fill: parent
                        anchors.leftMargin: 8
                        anchors.rightMargin: 8
                        text: rowItem.modelData ? String(rowItem.modelData.text || "") : ""
                        font.pixelSize: App.fontMicro
                        font.bold: true
                        color: appTheme.windowText
                        opacity: 0.6
                        verticalAlignment: Text.AlignVCenter
                        elide: Text.ElideRight
                    }

                    RowLayout {
                        visible: !rowItem.isHeader
                        anchors.fill: parent
                        anchors.leftMargin: 8
                        anchors.rightMargin: 8
                        spacing: 6

                        AppIcon {
                            icon: rowItem.modelData ? String(rowItem.modelData.icon || "") : ""
                            size: 16
                            color: appTheme.windowText
                            opacity: rowItem.isCurrent ? 1.0 : 0.85
                            Layout.preferredWidth: 16
                            Layout.preferredHeight: 16
                        }

                        AppText {
                            text: rowItem.modelData ? String(rowItem.modelData.name || "") : ""
                            font.pixelSize: App.fontSmall
                            font.bold: rowItem.isCurrent
                            color: appTheme.windowText
                            elide: Text.ElideRight
                            verticalAlignment: Text.AlignVCenter
                            Layout.fillWidth: !(rowItem.modelData && rowItem.modelData.detail)
                            Layout.maximumWidth: (rowItem.modelData && rowItem.modelData.detail)
                                ? Math.max(96, Math.round(rowItem.width * 0.58))
                                : rowItem.width
                        }

                        AppText {
                            visible: Boolean(rowItem.modelData && rowItem.modelData.detail)
                            text: rowItem.modelData ? String(rowItem.modelData.detail || "") : ""
                            font.pixelSize: App.fontMicro
                            color: appTheme.windowText
                            opacity: 0.55
                            elide: Text.ElideLeft
                            horizontalAlignment: Text.AlignRight
                            verticalAlignment: Text.AlignVCenter
                            Layout.fillWidth: true
                        }

                        AppIcon {
                            visible: rowItem.isCurrent
                            icon: "check"
                            size: 16
                            color: appTheme.highlight
                            Layout.preferredWidth: 16
                            Layout.preferredHeight: 16
                        }
                    }

                    MouseArea {
                        id: rowMouse
                        anchors.fill: parent
                        enabled: !rowItem.isHeader
                        hoverEnabled: !rowItem.isHeader
                        acceptedButtons: Qt.LeftButton
                        ToolTip.visible: containsMouse && Boolean(rowItem.modelData && rowItem.modelData.tooltip)
                        ToolTip.delay: 600
                        ToolTip.text: rowItem.modelData ? String(rowItem.modelData.tooltip || "") : ""
                        onEntered: list.currentIndex = rowItem.index
                        onClicked: popup.activateIndex(rowItem.index)
                    }
                }
            }
        }
    }

    Connections {
        target: AppMenuBar
        function onMenusChanged(): void {
            for (var i = 0; i < menuBar.count; ++i) {
                var m = menuBar.menuAt(i) as AppContextMenu
                if (m)
                    m.menuDirty = true
            }
        }
        function onTogglePopupRequested(): void {
            // The bar is already on screen, so this release must not open a menu.
            // Closing is the release half of a press the filter ate while a menu was open.
            if (!bar.visible || !bar.menuBarVisible)
                return
            if (bar.anyMenuOpen()) {
                bar.closeAllMenus()
                AppMenuBar.noteAltToggled(true)
                return
            }
            AppMenuBar.noteAltToggled(false)
        }
    }

    // Subtle top-edge canvas-colored veil easing out smoothly over the live canvas.
    readonly property color backdropColor: CircuitCanvas.canvasColor
    Rectangle {
        anchors.top: parent.top
        anchors.left: parent.left
        anchors.right: parent.right
        height: bar.height + 24
        gradient: Gradient {
            GradientStop { position: 0.0; color: Qt.alpha(bar.backdropColor, 0.75) }
            GradientStop { position: 0.42; color: Qt.alpha(bar.backdropColor, 0.30) }
            GradientStop { position: 0.72; color: Qt.alpha(bar.backdropColor, 0.15) }
            GradientStop { position: 1.0; color: Qt.alpha(bar.backdropColor, 0.0) }
        }
    }

    // Linux drag / double-click-maximize background (below interactive pills and caption buttons).
    Item {
        anchors.fill: parent
        anchors.rightMargin: captionCluster.visible ? captionCluster.width : 0
        visible: Qt.platform.os === "linux"

        TapHandler {
            acceptedButtons: Qt.LeftButton
            onDoubleTapped: {
                if (!bar.window)
                    return
                if (bar.maximized)
                    bar.window.showNormal()
                else
                    bar.window.showMaximized()
            }
        }

        DragHandler {
            target: null
            acceptedButtons: Qt.LeftButton
            onActiveChanged: {
                if (active && bar.window)
                    bar.window.startSystemMove()
            }
        }
    }

    // macOS drag handle (below interactive pills so Undo/Redo receive clicks).
    MouseArea {
        anchors.fill: parent
        anchors.rightMargin: captionCluster.visible ? captionCluster.width : 0
        visible: bar.macOS
        // Keep the press here until titlebarMouseDown starts the native drag.
        preventStealing: true
        acceptedButtons: Qt.LeftButton
        onPressed: App.titlebarMouseDown()
    }

    // Left (non-macOS when wide enough): app icon + inline menu bar (no pill background).
    // The container stays visible so the bar's contentWidth can be measured. The MenuBar
    // itself hides when it does not fit; that width does not collapse with visible.
    Item {
        id: menuContainer
        visible: !App.nativeMenus
        enabled: bar.menuBarVisible
        opacity: bar.menuBarVisible ? 1.0 : 0.0
        anchors.left: parent.left
        anchors.leftMargin: bar.leftEdge
        anchors.verticalCenter: parent.verticalCenter
        height: 26
        width: bar.menuBarVisible ? menuPill.implicitWidth : 0
        clip: true

        Rectangle {
            id: menuPill
            anchors.left: parent.left
            anchors.verticalCenter: parent.verticalCenter
            height: 26
            implicitWidth: Math.ceil(menuPillRow.implicitWidth)
            width: implicitWidth
            radius: height / 2
            color: "transparent"
            antialiasing: true
            opacity: bar.windowActive ? 1.0 : 0.7

            Row {
                id: menuPillRow
                anchors.left: parent.left
                anchors.verticalCenter: parent.verticalCenter
                spacing: 4

                Image {
                    anchors.verticalCenter: parent.verticalCenter
                    source: "qrc:/icons/circuitsimulator.png"
                    width: 16
                    height: 16
                    sourceSize.width: 32
                    sourceSize.height: 32
                    fillMode: Image.PreserveAspectFit
                    smooth: true
                }

                T.MenuBar {
                    id: menuBar
                    // The Alt filter keys off isVisible(), not enabled or opacity.
                    // A collapsed bar would still take the hamburger's Alt tap.
                    visible: bar.menuBarVisible
                    anchors.verticalCenter: parent.verticalCenter
                    implicitWidth: contentWidth + leftPadding + rightPadding
                    implicitHeight: Math.max(22, contentHeight + topPadding + bottomPadding)
                    leftPadding: 0
                    rightPadding: 0
                    topPadding: 0
                    bottomPadding: 0
                    spacing: 1
                    font: Qt.font({ family: App.fontFamily, pixelSize: App.fontNormal })
                    background: null

                    Keys.onPressed: (event) => {
                        if ((event.modifiers & (Qt.ControlModifier | Qt.AltModifier | Qt.MetaModifier)) !== 0)
                            return
                        var keyText = String(event.text || "").toLowerCase()
                        if (keyText === "")
                            return
                        for (var i = 0; i < menuBar.count; ++i) {
                            var it = menuBar.itemAt(i) as T.MenuBarItem
                            var m = menuBar.menuAt(i) as AppContextMenu
                            if (!it || !m)
                                continue
                            var mark = UiUtils.splitMnemonic(m.rawTitle).mark.toLowerCase()
                            if (mark !== "" && mark === keyText) {
                                it.triggered()
                                event.accepted = true
                                return
                            }
                        }
                    }

                    contentItem: Row {
                        spacing: menuBar.spacing
                        Repeater {
                            id: menuBarRepeater
                            model: menuBar.contentModel
                        }
                    }

                    delegate: T.MenuBarItem {
                        id: barItem
                        property bool titleBarInteractive: true
                        enabled: bar.menuBarVisible
                        implicitWidth: contentItem.implicitWidth + leftPadding + rightPadding
                        implicitHeight: 22
                        font: menuBar.font
                        hoverEnabled: true
                        leftPadding: 7
                        rightPadding: 7
                        topPadding: 0
                        bottomPadding: 0

                        readonly property string rawTitle: (barItem.menu as AppContextMenu)?.rawTitle ?? String(barItem.text)
                        readonly property var mnemonic: UiUtils.splitMnemonic(rawTitle)
                        readonly property real labelOpacity: bar.windowActive ? (enabled ? 1.0 : 0.4) : 0.75

                        // Qt's MenuBarItem fails to activate Alt+<key> mnemonics when enabled/visible
                        // starts false during initial layout, and stripping '&' from `menu.title`
                        // prevents an ambiguous overload if it ever did grab one.
                        Shortcut {
                            enabled: bar.visible && bar.menuBarVisible
                                && barItem.mnemonic.mark !== ""
                                && !CommandCenter.visible && !CommandCenter.recordingShortcut && !TabSwitcher.visible
                            sequences: barItem.mnemonic.mark !== "" ? ["Alt+" + barItem.mnemonic.mark] : []
                            onActivated: {
                                if (UiUtils.hasModalOverlay(bar.Overlay.overlay, null))
                                    return
                                for (var i = 0; i < menuBar.count; ++i) {
                                    var m = menuBar.menuAt(i) as AppContextMenu
                                    if (m && m !== barItem.menu && m.opened)
                                        m.closeTree()
                                }
                                var ownMenu = barItem.menu as AppContextMenu
                                if (ownMenu && ownMenu.opened)
                                    ownMenu.closeTree()
                                else
                                    barItem.triggered()
                            }
                        }

                        contentItem: Item {
                            implicitWidth: mnemonicRow.implicitWidth
                            implicitHeight: mnemonicRow.implicitHeight

                            Row {
                                id: mnemonicRow
                                anchors.centerIn: parent
                                spacing: 0

                                AppText {
                                    text: barItem.mnemonic.lead
                                    color: appTheme.windowText
                                    opacity: barItem.labelOpacity
                                }
                                AppText {
                                    text: barItem.mnemonic.mark
                                    font.underline: true
                                    visible: text !== ""
                                    color: appTheme.windowText
                                    opacity: barItem.labelOpacity
                                }
                                AppText {
                                    text: barItem.mnemonic.tail
                                    color: appTheme.windowText
                                    opacity: barItem.labelOpacity
                                }
                            }
                        }

                        background: Rectangle {
                            radius: height / 2
                            color: (barItem.down || barItem.highlighted)
                                ? Qt.rgba(appTheme.windowText.r, appTheme.windowText.g, appTheme.windowText.b, 0.16)
                                : barItem.hovered
                                    ? Qt.rgba(appTheme.windowText.r, appTheme.windowText.g, appTheme.windowText.b, 0.10)
                                    : "transparent"
                            Behavior on color { ColorAnimation { duration: 80 } }
                        }
                    }

                    Instantiator {
                        model: (!App.nativeMenus && AppMenuBar.menus) ? AppMenuBar.menus.length : 0
                        onObjectAdded: (index, object) => menuBar.insertMenu(index, object)
                        onObjectRemoved: (_, object) => menuBar.removeMenu(object)
                        delegate: AppContextMenu {
                            id: topMenu
                            required property int index
                            property bool menuDirty: true
                            popupType: Popup.Item
                            focus: true
                            rawTitle: {
                                var ms = AppMenuBar.menus
                                return (ms && index >= 0 && index < ms.length && ms[index])
                                    ? String(ms[index].title || "")
                                    : ""
                            }
                            title: UiUtils.stripMnemonic(rawTitle)
                            onOpenedChanged: bar.syncAltMenuOpen()
                            onAboutToShow: {
                                bar.closeSwitcherPopups()
                                if (topMenu.menuDirty || topMenu.count === 0) {
                                    var ms = AppMenuBar.menus
                                    var items = (ms && index >= 0 && index < ms.length && ms[index])
                                        ? ms[index].items
                                        : []
                                    menuBuilder.populateMenu(topMenu, items)
                                    topMenu.menuDirty = false
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Compact Undo / Redo pill right after the inline menu bar (Windows/Linux)
    // or traffic lights (macOS). Follows the same focus-aware target as Edit > Undo/Redo.
    Rectangle {
        id: historyPill
        property bool titleBarInteractive: true
        anchors.verticalCenter: parent.verticalCenter
        x: bar.menuBarVisible
            ? (bar.leftEdge + menuContainer.width + bar.pillGap)
            : bar.leftEdge
        width: bar.historyPillWidth
        height: 26
        visible: bar.usableWidth - width - bar.pillGap >= bar.titleContentMinWidth
        radius: height / 2
        color: Qt.alpha(bar.backdropColor, 0.70)
        antialiasing: true
        opacity: bar.windowActive ? 1.0 : 0.7

        // Absorb presses on pill padding or disabled buttons so they do not drag the window.
        MouseArea {
            anchors.fill: parent
            acceptedButtons: Qt.LeftButton
        }

        Row {
            anchors.centerIn: parent
            spacing: 2

            AppToolButton {
                id: undoBtn
                property bool titleBarInteractive: true
                width: 22
                height: 22
                padding: 0
                focusPolicy: Qt.NoFocus
                enabled: AppMenuBar.canUndo
                ToolTip.visible: hovered
                ToolTip.text: bar.actionTooltip(I18n.tr("Undo"), "edit.undo")
                onClicked: AppMenuBar.triggerAction("edit.undo")

                contentItem: Item {
                    implicitWidth: 16
                    implicitHeight: 16
                    AppIcon {
                        anchors.centerIn: parent
                        icon: "undo"
                        size: 16
                        color: appTheme.windowText
                    }
                }
            }

            AppToolButton {
                id: redoBtn
                property bool titleBarInteractive: true
                width: 22
                height: 22
                padding: 0
                focusPolicy: Qt.NoFocus
                enabled: AppMenuBar.canRedo
                ToolTip.visible: hovered
                ToolTip.text: bar.actionTooltip(I18n.tr("Redo"), "edit.redo")
                onClicked: AppMenuBar.triggerAction("edit.redo")

                contentItem: Item {
                    implicitWidth: 16
                    implicitHeight: 16
                    AppIcon {
                        anchors.centerIn: parent
                        icon: "redo"
                        size: 16
                        color: appTheme.windowText
                    }
                }
            }
        }
    }

    // Compact Command Center button right after Undo / Redo.
    Rectangle {
        id: commandCenterPill
        property bool titleBarInteractive: true
        anchors.verticalCenter: parent.verticalCenter
        x: historyPill.x + historyPill.width + bar.pillGap
        width: bar.commandCenterPillWidth
        height: 26
        visible: historyPill.visible
            && (bar.usableWidth - bar.historyPillWidth - bar.pillGap - width - bar.pillGap >= bar.titleContentMinWidth)
        radius: height / 2
        color: Qt.alpha(bar.backdropColor, 0.70)
        antialiasing: true
        opacity: bar.windowActive ? 1.0 : 0.7

        // Absorb presses on pill padding so they do not drag the window.
        MouseArea {
            anchors.fill: parent
            acceptedButtons: Qt.LeftButton
        }

        AppToolButton {
            id: commandCenterBtn
            property bool titleBarInteractive: true
            anchors.centerIn: parent
            width: 22
            height: 22
            padding: 0
            focusPolicy: Qt.NoFocus
            checked: CommandCenter.visible
            ToolTip.visible: hovered && !CommandCenter.visible
            ToolTip.text: bar.actionTooltip(I18n.tr("Command Center"), "view.cmdPalette")
            onClicked: {
                bar.closeAllMenus()
                AppMenuBar.triggerAction("view.cmdPalette")
            }

            contentItem: Item {
                implicitWidth: 16
                implicitHeight: 16
                AppIcon {
                    anchors.centerIn: parent
                    icon: "search"
                    size: 16
                    color: commandCenterBtn.isCheckedActive ? appTheme.highlight : appTheme.windowText
                }
            }
        }
    }

    // Active file title pill: live simulation state badge, project badge & active file title
    // (plus app icon when the inline menu bar is not shown). Centers in the titlebar and
    // clamps between the left controls (menu bar / traffic lights / history & command center pills)
    // and right caption buttons, expanding to fit directory and file names and truncating with
    // ellipsis only when constrained.
    Rectangle {
        id: leftPill
        anchors.verticalCenter: parent.verticalCenter
        height: 26
        readonly property int minLeftX: commandCenterPill.visible
            ? (commandCenterPill.x + commandCenterPill.width + bar.pillGap)
            : historyPill.visible
                ? (historyPill.x + historyPill.width + bar.pillGap)
                : (bar.menuBarVisible ? (bar.leftEdge + menuContainer.width + bar.pillGap) : bar.leftEdge)
        readonly property int maxRightX: Math.max(minLeftX, bar.width - bar.rightEdge)
        readonly property int maxWidth: Math.max(0, maxRightX - minLeftX)
        width: Math.min(Math.ceil(leftGroup.implicitWidth) + 12, maxWidth)
        x: Math.max(minLeftX, Math.min(maxRightX - width, Math.round((bar.width - width) / 2)))
        visible: width > 0
        clip: true
        radius: height / 2
        color: Qt.alpha(bar.backdropColor, 0.70)
        antialiasing: true
        opacity: bar.windowActive ? 1.0 : 0.7

        RowLayout {
            id: leftGroup
            anchors.fill: parent
            anchors.leftMargin: 8
            anchors.rightMargin: 4
            spacing: 6

            Image {
                visible: !bar.menuBarVisible
                source: "qrc:/icons/circuitsimulator.png"
                Layout.preferredWidth: 16
                Layout.preferredHeight: 16
                sourceSize.width: 32
                sourceSize.height: 32
                fillMode: Image.PreserveAspectFit
                smooth: true
            }

            Rectangle {
                visible: CircuitCanvas.simRunning
                radius: height / 2
                color: (CircuitCanvas.simPaused || CircuitPanel.simAtBreakpoint) ? CircuitCanvas.msgWarnBg : CircuitCanvas.msgOkBg
                implicitWidth: Math.ceil(simStateText.implicitWidth) + 12
                implicitHeight: Math.ceil(simStateText.implicitHeight) + 4
                Layout.preferredWidth: implicitWidth
                Layout.minimumWidth: implicitWidth

                AppText {
                    id: simStateText
                    anchors.centerIn: parent
                    text: CircuitPanel.simAtBreakpoint
                        ? I18n.tr("Breakpoint")
                        : CircuitCanvas.simPaused
                            ? I18n.tr("Paused")
                            : I18n.tr("Running")
                    color: (CircuitCanvas.simPaused || CircuitPanel.simAtBreakpoint) ? CircuitCanvas.msgWarnText : CircuitCanvas.msgOkText
                }
            }

            Rectangle {
                id: projBadge
                property bool titleBarInteractive: true
                visible: bar.projectName !== ""
                radius: height / 2
                color: (projMouse.pressed || projectPopup.opened)
                    ? Qt.rgba(appTheme.windowText.r, appTheme.windowText.g, appTheme.windowText.b, 0.18)
                    : projMouse.containsMouse
                        ? Qt.rgba(appTheme.windowText.r, appTheme.windowText.g, appTheme.windowText.b, 0.14)
                        : Qt.rgba(appTheme.windowText.r, appTheme.windowText.g, appTheme.windowText.b, 0.08)
                Behavior on color { ColorAnimation { duration: 80 } }
                // Round up CoreText's fractional implicitWidth so RowLayout's integer
                // geometry snapping never shrinks the label below its text advance.
                readonly property int naturalWidth: Math.ceil(projText.implicitWidth) + 10
                implicitWidth: naturalWidth
                implicitHeight: Math.ceil(projText.implicitHeight) + 4
                Layout.fillWidth: true
                Layout.preferredWidth: naturalWidth
                Layout.maximumWidth: naturalWidth
                Layout.minimumWidth: Math.min(naturalWidth, 48)

                AppText {
                    id: projText
                    anchors.centerIn: parent
                    width: Math.min(Math.ceil(implicitWidth), Math.max(0, parent.width - 10))
                    text: bar.projectName
                    color: appTheme.windowText
                    elide: Text.ElideRight
                }

                MouseArea {
                    id: projMouse
                    anchors.fill: parent
                    hoverEnabled: true
                    acceptedButtons: Qt.LeftButton
                    property bool titleBarInteractive: true
                    onClicked: bar.toggleProjectPopup()
                }

                SwitcherPopup {
                    id: projectPopup
                    parent: projBadge
                    onActivated: (path) => {
                        if (path !== "")
                            FileBrowser.rootPath = path
                    }
                }
            }

            Rectangle {
                id: fileBtn
                property bool titleBarInteractive: true
                radius: height / 2
                color: (fileMouse.pressed || circuitPopup.opened)
                    ? Qt.rgba(appTheme.windowText.r, appTheme.windowText.g, appTheme.windowText.b, 0.16)
                    : fileMouse.containsMouse
                        ? Qt.rgba(appTheme.windowText.r, appTheme.windowText.g, appTheme.windowText.b, 0.10)
                        : "transparent"
                Behavior on color { ColorAnimation { duration: 80 } }
                readonly property int modifiedExtra: CircuitCanvas.modified ? 10 : 0
                readonly property int naturalWidth: Math.ceil(fileLabelText.implicitWidth) + 8 + modifiedExtra
                implicitWidth: naturalWidth
                implicitHeight: Math.ceil(fileLabelText.implicitHeight) + 4
                Layout.fillWidth: true
                Layout.preferredWidth: naturalWidth
                Layout.maximumWidth: naturalWidth
                Layout.minimumWidth: Math.min(naturalWidth, 64)

                Row {
                    anchors.centerIn: parent
                    spacing: 2

                    AppText {
                        id: fileLabelText
                        anchors.verticalCenter: parent.verticalCenter
                        width: Math.min(Math.ceil(implicitWidth), Math.max(0, fileBtn.width - 8 - fileBtn.modifiedExtra))
                        text: bar.fileLabel
                        color: appTheme.windowText
                        elide: Text.ElideMiddle
                        maximumLineCount: 1
                    }

                    AppIcon {
                        visible: CircuitCanvas.modified
                        anchors.verticalCenter: parent.verticalCenter
                        width: 8
                        icon: "circle"
                        size: 16
                        color: appTheme.windowText
                    }
                }

                MouseArea {
                    id: fileMouse
                    anchors.fill: parent
                    hoverEnabled: true
                    acceptedButtons: Qt.LeftButton
                    property bool titleBarInteractive: true
                    onClicked: bar.toggleCircuitPopup()
                }

                SwitcherPopup {
                    id: circuitPopup
                    parent: fileBtn
                    onActivated: (path) => {
                        if (path !== "")
                            CircuitCanvas.loadPath(path)
                    }
                }
            }
        }
    }

    // Live canvas cursor coordinates pill on the right side of the custom title bar.
    Rectangle {
        id: coordsPill
        visible: bar.coordsVisible
        anchors.verticalCenter: parent.verticalCenter
        anchors.right: updatePill.visible
            ? updatePill.left
            : (captionCluster.visible ? captionCluster.left : parent.right)
        anchors.rightMargin: 8
        width: bar.coordsPillWidth
        height: Math.ceil(coordsRow.implicitHeight) + 4
        radius: height / 2
        color: Qt.alpha(bar.backdropColor, 0.70)
        antialiasing: true
        opacity: bar.windowActive ? 1.0 : 0.7

        // Distinguishing neutral tint matching the overlay toolbar's coordinates pill.
        Rectangle {
            anchors.fill: parent
            radius: height / 2
            color: Qt.rgba(appTheme.windowText.r, appTheme.windowText.g, appTheme.windowText.b, 0.10)
            antialiasing: true
        }

        TextMetrics {
            id: coordsValueMetrics
            font.family: App.fontFamily
            font.pixelSize: App.fontNormal
            text: "-99999"
        }

        Row {
            id: coordsRow
            anchors.centerIn: parent
            spacing: 3

            AppText {
                anchors.verticalCenter: parent.verticalCenter
                text: App.translate("X")
                color: Qt.rgba(appTheme.windowText.r, appTheme.windowText.g, appTheme.windowText.b, 0.6)
            }
            AppText {
                anchors.verticalCenter: parent.verticalCenter
                width: Math.max(coordsValueMetrics.width, implicitWidth)
                horizontalAlignment: Text.AlignRight
                text: CircuitPanel.coordsX
                color: appTheme.windowText
            }
            AppText {
                anchors.verticalCenter: parent.verticalCenter
                leftPadding: 7
                text: App.translate("Y")
                color: Qt.rgba(appTheme.windowText.r, appTheme.windowText.g, appTheme.windowText.b, 0.6)
            }
            AppText {
                anchors.verticalCenter: parent.verticalCenter
                width: Math.max(coordsValueMetrics.width, implicitWidth)
                horizontalAlignment: Text.AlignRight
                text: CircuitPanel.coordsY
                color: appTheme.windowText
            }
        }
    }

    // Update available indicator pill on the right side of the title bar
    // (just to the left of the caption buttons on Windows/Linux, or at the right edge on macOS).
    Rectangle {
        id: updatePill
        property bool titleBarInteractive: true
        visible: Updater.updateAvailable
        anchors.verticalCenter: parent.verticalCenter
        anchors.right: captionCluster.visible ? captionCluster.left : parent.right
        anchors.rightMargin: 8
        width: bar.updatePillWidth
        height: 26
        radius: height / 2
        color: Qt.alpha(bar.backdropColor, 0.70)
        antialiasing: true
        opacity: bar.windowActive ? 1.0 : 0.7

        // Absorb presses on pill padding so they do not drag the window.
        MouseArea {
            anchors.fill: parent
            acceptedButtons: Qt.LeftButton
        }

        AppToolButton {
            id: updateBtn
            property bool titleBarInteractive: true
            anchors.centerIn: parent
            width: 22
            height: 22
            padding: 0
            focusPolicy: Qt.NoFocus
            checked: Updater.dialogVisible
            ToolTip.visible: hovered
            ToolTip.text: I18n.tr("Update available: %1").replace("%1", Updater.latestVersion)
            onClicked: {
                bar.closeAllMenus()
                Updater.openUpdateDialog()
            }

            contentItem: Item {
                implicitWidth: 16
                implicitHeight: 16
                AppIcon {
                    anchors.centerIn: parent
                    icon: "file_download"
                    size: 16
                    color: appTheme.highlight
                }
            }
        }
    }

    // Right: Windows-style Minimize / Maximize-Restore / Close caption buttons.
    // macOS keeps the traffic lights, so this cluster stays hidden.
    Rectangle {
        id: captionCluster
        visible: !bar.macOS
        anchors.top: parent.top
        anchors.right: parent.right
        width: captionButtons.width
        height: parent.height
        color: Qt.alpha(bar.backdropColor, 0.70)
        topRightRadius: (Qt.platform.os === "linux" && !bar.maximized) ? 14 : 0

        WindowCaptionButtons {
            id: captionButtons
            height: parent.height
            window: bar.window
            windowActive: bar.windowActive
            topRightRadius: captionCluster.topRightRadius
        }
    }
}
