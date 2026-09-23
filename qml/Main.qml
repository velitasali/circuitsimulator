import QtQuick
import QtQuick.Controls
import QtQuick.Controls.impl
import QtQuick.Templates as T
import QtQuick.Window
import QtQuick.Dialogs
import cs_app

/* Spike of src/gui/main.qml. CircuitPanel owns the canvas and overlays.
 * Extra windows (About, plots, MCU monitor, serial) are QML Window, not C++ QmlWindow.
 *
 * The menu bar is platform-split: macOS gets a real NSMenu (App.nativeMenus),
 * everyone else draws a template MenuBar from the same AppMenuBar JSON so the
 * app palette (including dark theme) actually applies on Windows.
 */
ApplicationWindow {
    id: win
    SystemPalette { id: appTheme }

    function tr(s) { return App.i18nTick >= 0 ? App.translate(s) : s }

    readonly property bool darkChrome: appTheme.window.hslLightness < 0.5
    property bool saveAllPending: false
    property bool circSaveThenReplace: false
    property bool fileSaveThenClose: false
    property bool isQuitting: false
    property var ownedWindows: []

    function registerOwnedWindow(w) {
        if (!w)
            return
        if (ownedWindows.indexOf(w) < 0)
            ownedWindows.push(w)
    }

    function unregisterOwnedWindow(w) {
        var i = ownedWindows.indexOf(w)
        if (i >= 0)
            ownedWindows.splice(i, 1)
    }

    function closeOwnedWindows() {
        var list = ownedWindows.slice()
        for (var i = 0; i < list.length; i++) {
            var w = list[i]
            if (w && w.closeWithOwner !== false && w.dismiss)
                w.dismiss()
        }
    }

    // Same set MainWindow registers through QFontDatabase in the C++ app.
    // FontLoader adds each family to the app font database, so App.fontFamily
    // (default "Ubuntu") and the settings font combo resolve them.
    FontLoader { source: "qrc:/fonts/MaterialSymbolsRounded.ttf" }
    FontLoader { source: "qrc:/fonts/Ubuntu-R.ttf" }
    FontLoader { source: "qrc:/fonts/Ubuntu-B.ttf" }
    FontLoader { source: "qrc:/fonts/UbuntuMono-R.ttf" }
    FontLoader { source: "qrc:/fonts/UbuntuMono-RI.ttf" }
    FontLoader { source: "qrc:/fonts/UbuntuMono-B.ttf" }
    FontLoader { source: "qrc:/fonts/UbuntuMono-BI.ttf" }

    visible: true
    minimumWidth: App.minWindowWidth
    minimumHeight: App.minWindowHeight
    width: App.windowWidth > 0 ? App.windowWidth : 1200
    height: App.windowHeight > 0 ? App.windowHeight : 800
    title: {
        var _tick = App.i18nTick
        var state = CircuitPanel.simRunning ? (CircuitPanel.simPaused ? App.translate("[Paused] ") : App.translate("[Running] ")) : ""
        var proj = FileBrowser.rootPath ? ("[" + FileBrowser.rootPath.split("/").filter(Boolean).pop() + "] ") : ""
        var file = CircuitCanvas.fileName ? CircuitCanvas.fileName : App.translate("Untitled")
        var changed = CircuitCanvas.modified ? "*" : ""
        return state + proj + file + changed + " - " + App.translate("Circuit Simulator")
    }
    font: Qt.font({ family: App.fontFamily, pixelSize: App.fontSize })
    color: appTheme.window
    palette.window: appTheme.window
    palette.windowText: appTheme.windowText
    palette.button: appTheme.button
    palette.buttonText: appTheme.buttonText
    palette.highlight: appTheme.highlight
    palette.highlightedText: appTheme.highlightedText
    palette.mid: appTheme.mid
    palette.midlight: appTheme.midlight
    palette.text: appTheme.windowText
    palette.base: appTheme.base

    Timer {
        id: saveGeometryTimer
        interval: 400
        repeat: false
        onTriggered: {
            if (win.visibility === Window.Maximized) {
                App.saveWindowState(win.x, win.y, win.width, win.height, true)
            } else if (win.visibility === Window.Windowed) {
                App.saveWindowState(win.x, win.y, win.width, win.height, false)
            }
        }
    }

    onWidthChanged: if (visible && visibility === Window.Windowed) saveGeometryTimer.restart()
    onHeightChanged: if (visible && visibility === Window.Windowed) saveGeometryTimer.restart()
    onXChanged: if (visible && visibility === Window.Windowed) saveGeometryTimer.restart()
    onYChanged: if (visible && visibility === Window.Windowed) saveGeometryTimer.restart()
    onVisibilityChanged: (visibility) => {
        if (visible) {
            if (visibility === Window.Maximized) {
                App.saveWindowState(win.x, win.y, win.width, win.height, true)
            } else if (visibility === Window.Windowed) {
                saveGeometryTimer.restart()
            }
        }
    }

    menuBar: App.nativeMenus ? null : inWindowBar
    Loader { id: inWindowBar; sourceComponent: App.nativeMenus ? null : inWindowBarComponent }

    // Repeater parents Menus as visual children; MenuBar only turns a Menu into a
    // MenuBarItem through addMenu/insertMenu (contentData). Instantiator + insertMenu
    // is the documented dynamic path — without it Windows gets an empty strip.
    //
    // Template types, not QtQuick.Controls.Windows: that style paints a light
    // MenuBar and accent (often purple) hover from the OS palette, ignoring
    // QGuiApplication::setPalette / dark theme.
    Component {
        id: inWindowBarComponent
        T.MenuBar {
            id: bar
            // Templates do not bind implicit size; without this the Loader
            // reports 0 height and ApplicationWindow hides the bar.
            implicitWidth: Math.max(implicitBackgroundWidth + leftInset + rightInset,
                                    contentWidth + leftPadding + rightPadding)
            implicitHeight: Math.max(implicitBackgroundHeight + topInset + bottomInset,
                                     contentHeight + topPadding + bottomPadding)
            width: parent ? parent.width : implicitWidth
            leftPadding: 4
            rightPadding: 4
            topPadding: 2
            bottomPadding: 2
            spacing: 0
            font: Qt.font({ family: App.fontFamily, pixelSize: App.fontSize })

            palette.window: appTheme.window
            palette.windowText: appTheme.windowText
            palette.button: appTheme.button
            palette.buttonText: appTheme.buttonText
            palette.highlight: appTheme.highlight
            palette.highlightedText: appTheme.highlightedText
            palette.mid: appTheme.mid
            palette.midlight: appTheme.midlight
            palette.text: appTheme.windowText
            palette.base: appTheme.base

            contentItem: Row {
                spacing: bar.spacing
                Repeater { model: bar.contentModel }
            }

            background: Rectangle {
                implicitWidth: 200
                implicitHeight: 28
                color: appTheme.window
            }

            delegate: T.MenuBarItem {
                id: barItem
                implicitWidth: Math.max(implicitBackgroundWidth + leftInset + rightInset,
                                        implicitContentWidth + leftPadding + rightPadding)
                implicitHeight: Math.max(implicitBackgroundHeight + topInset + bottomInset,
                                         implicitContentHeight + topPadding + bottomPadding)
                font: bar.font
                hoverEnabled: true
                padding: 6
                leftPadding: 10
                rightPadding: 10

                contentItem: IconLabel {
                    spacing: barItem.spacing
                    mirrored: barItem.mirrored
                    display: barItem.display
                    alignment: Qt.AlignLeft
                    icon: barItem.icon
                    text: barItem.text
                    font: barItem.font
                    color: barItem.enabled ? appTheme.windowText : appTheme.mid
                }

                background: Rectangle {
                    implicitWidth: 20
                    implicitHeight: 22
                    radius: 4
                    color: (barItem.highlighted || barItem.down) ? appTheme.midlight : "transparent"
                }
            }

            Component {
                id: nestedMenuComp
                AppContextMenu {
                    id: nestedMenu
                    popupType: Popup.Item
                    property var itemsModel: []
                    Instantiator {
                        model: nestedMenu.itemsModel
                        onObjectAdded: (index, object) => nestedMenu.insertItem(index, object)
                        onObjectRemoved: (_, object) => nestedMenu.removeItem(object)
                        delegate: ContextMenuItem {
                            required property var modelData
                            text: modelData.separator ? "" : String(modelData.text)
                            enabled: !modelData.separator && modelData.enabled
                            visible: modelData.visible
                            isSeparator: modelData.separator
                            checkable: !modelData.separator && modelData.checkable
                            checked: modelData.checked
                            shortcutText: modelData.shortcut ? String(modelData.shortcut) : ""
                            onTriggered: AppMenuBar.trigger(modelData.path)
                        }
                    }
                }
            }

            Instantiator {
                model: AppMenuBar.menus
                onObjectAdded: (index, object) => bar.insertMenu(index, object)
                onObjectRemoved: (_, object) => bar.removeMenu(object)
                delegate: AppContextMenu {
                    id: topMenu
                    popupType: Popup.Item
                    required property var modelData
                    title: modelData.title
                    onAboutToShow: AppMenuBar.aboutToShow(modelData.path)
                    Instantiator {
                        model: topMenu.modelData.items
                        onObjectAdded: (index, object) => topMenu.insertItem(index, object)
                        onObjectRemoved: (_, object) => topMenu.removeItem(object)
                        delegate: ContextMenuItem {
                            id: menuItem
                            required property var modelData
                            text: modelData.separator ? "" : String(modelData.text)
                            enabled: !modelData.separator && modelData.enabled
                            visible: modelData.visible
                            isSeparator: modelData.separator
                            checkable: !modelData.separator && modelData.checkable && !modelData.submenu
                            checked: modelData.checked
                            shortcutText: modelData.shortcut ? String(modelData.shortcut) : ""
                            onTriggered: {
                                if (!modelData.submenu)
                                    AppMenuBar.trigger(modelData.path)
                            }
                            Component.onCompleted: {
                                if (modelData.submenu)
                                    nestedMenuComp.createObject(menuItem, { itemsModel: modelData.submenu })
                            }
                        }
                    }
                }
            }
        }
    }

    Loader {
        anchors.fill: parent
        source: "CircuitPanel.qml"
        focus: true
    }

    AppWindow {
        id: aboutWin
        title: App.translate("About Circuit Simulator")
        sizeToContent: true
        source: "about.qml"
        visible: App.aboutVisible

        onVisibleChanged: {
            if (visible) {
                x = win.x + Math.round((win.width - width) / 2)
                y = win.y + Math.round((win.height - height) / 2)
                raise()
                requestActivate()
            }
        }
        onClosing: (close) => {
            if (!win.isQuitting) {
                close.accepted = false
                App.aboutVisible = false
            }
        }
    }

    AppWindow {
        id: aboutQtWin
        title: App.translate("About Qt")
        width: 580
        height: 480
        minimumWidth: 400
        minimumHeight: 320
        source: "aboutqt.qml"
        visible: App.aboutQtVisible

        onVisibleChanged: {
            if (visible) {
                x = win.x + Math.round((win.width - width) / 2)
                y = win.y + Math.round((win.height - height) / 2)
                raise()
                requestActivate()
            }
        }
        onClosing: (close) => {
            if (!win.isQuitting) {
                close.accepted = false
                App.aboutQtVisible = false
            }
        }
    }

    AppWindow {
        id: oscWin
        title: win.tr("Oscilloscope")
        width: 900
        height: 520
        source: visible ? "OscView.qml" : ""
        visible: App.oscVisible
        onVisibleChanged: {
            if (visible) {
                x = win.x + Math.round((win.width - width) / 2)
                y = win.y + Math.round((win.height - height) / 2)
                raise()
                requestActivate()
            }
        }
        onClosing: (close) => {
            if (!win.isQuitting) {
                close.accepted = false
                App.oscVisible = false
            }
        }
    }

    AppWindow {
        id: laWin
        title: win.tr("Logic Analyzer")
        width: 900
        height: 520
        source: visible ? "LaView.qml" : ""
        visible: App.laVisible
        onVisibleChanged: {
            if (visible) {
                x = win.x + Math.round((win.width - width) / 2)
                y = win.y + Math.round((win.height - height) / 2)
                raise()
                requestActivate()
            }
        }
        onClosing: (close) => {
            if (!win.isQuitting) {
                close.accepted = false
                App.laVisible = false
            }
        }
    }

    AppWindow {
        id: mcuWin
        title: win.tr("MCU Monitor")
        width: 720
        height: 520
        source: visible ? "McuView.qml" : ""
        visible: App.mcuVisible
        onVisibleChanged: {
            if (visible) {
                x = win.x + Math.round((win.width - width) / 2)
                y = win.y + Math.round((win.height - height) / 2)
                raise()
                requestActivate()
            }
        }
        onClosing: (close) => {
            if (!win.isQuitting) {
                close.accepted = false
                App.mcuVisible = false
            }
        }
    }

    Instantiator {
        id: serialMonInstantiator
        model: SerialMonitor.openMonitors || []
        delegate: AppWindow {
            id: monWin
            required property var modelData
            required property int index
            readonly property string monId: modelData.id || ""
            title: modelData.title || win.tr("Serial Monitor")
            width: 560
            height: 420
            source: "serialmon.qml"
            visible: true
            ownerWindow: win

            Component.onCompleted: {
                var offset = (index % 8) * 28
                x = win.x + Math.round((win.width - width) / 2) + offset
                y = win.y + Math.round((win.height - height) / 2) + offset
                if (panel)
                    panel.monitorId = monId
            }

            Connections {
                target: monWin
                function onPanelChanged() {
                    if (monWin.panel)
                        monWin.panel.monitorId = monWin.monId
                }
            }

            Connections {
                target: SerialMonitor
                function onRequestShowMonitor(id) {
                    if (id === monWin.monId) {
                        monWin.bringToFront()
                    }
                }
            }

            onClosing: (close) => {
                if (!win.isQuitting) {
                    close.accepted = false
                    visible = false
                    SerialMonitor.closeMonitor(monId)
                }
            }
        }
    }

    AppWindow {
        id: terminalWin
        title: win.tr("Serial Terminal")
        width: 560
        height: 420
        source: visible ? "terminal.qml" : ""
        visible: App.terminalVisible
        onVisibleChanged: {
            if (visible) {
                SerialTerminal.setDark(win.darkChrome)
                x = win.x + Math.round((win.width - width) / 2)
                y = win.y + Math.round((win.height - height) / 2)
                raise()
                requestActivate()
            }
        }
        onClosing: (close) => {
            if (!win.isQuitting) {
                close.accepted = false
                App.terminalVisible = false
            }
        }
    }

    AppWindow {
        id: memTableWin
        title: MemoryTable.title || win.tr("Memory Table")
        width: 720
        height: 480
        minimumWidth: 520
        minimumHeight: 300
        source: visible ? "MemTablePanel.qml" : ""
        visible: MemoryTable.visible
        onVisibleChanged: {
            if (visible) {
                x = win.x + Math.round((win.width - width) / 2)
                y = win.y + Math.round((win.height - height) / 2)
                raise()
                requestActivate()
            }
        }
        onClosing: (close) => {
            if (!win.isQuitting) {
                close.accepted = false
                MemoryTable.visible = false
            }
        }
    }

    AppWindow {
        id: settingsWin
        title: win.tr("Settings")
        sizeToContent: true
        minimumWidth: 420
        minimumHeight: 380
        source: "appdialog.qml"
        visible: App.settingsVisible
        onVisibleChanged: {
            if (visible) {
                x = win.x + Math.round((win.width - width) / 2)
                y = win.y + Math.round((win.height - height) / 2)
                raise()
                requestActivate()
            }
        }
        onClosing: (close) => {
            if (!win.isQuitting) {
                close.accepted = false
                App.settingsVisible = false
            }
        }
    }

    AppWindow {
        id: circSettingsWin
        title: win.tr("Circuit Settings")
        width: Math.max(420, minimumWidth)
        height: 560
        minimumWidth: panel ? Math.max(360, Math.ceil(panel.implicitWidth)) : 360
        minimumHeight: 380
        source: "circuitdialog.qml"
        visible: App.circuitSettingsVisible
        onVisibleChanged: {
            if (visible) {
                x = win.x + Math.round((win.width - width) / 2)
                y = win.y + Math.round((win.height - height) / 2)
                raise()
                requestActivate()
            }
        }
        onClosing: (close) => {
            if (!win.isQuitting) {
                close.accepted = false
                App.circuitSettingsVisible = false
            }
        }
    }

    AppWindow {
        id: installerWin
        title: win.tr("Library Manager")
        width: 600
        height: 520
        minimumWidth: 480
        minimumHeight: 360
        source: visible ? "installer.qml" : ""
        visible: App.installerVisible
        onVisibleChanged: {
            if (visible) {
                x = win.x + Math.round((win.width - width) / 2)
                y = win.y + Math.round((win.height - height) / 2)
                raise()
                requestActivate()
            }
        }
        onClosing: (close) => {
            if (!win.isQuitting) {
                close.accepted = false
                App.installerVisible = false
            }
        }
    }

    Connections {
        target: App
        function onRequestShowAbout() { aboutWin.bringToFront() }
        function onRequestShowAboutQt() { aboutQtWin.bringToFront() }
        function onRequestShowSettings() { settingsWin.bringToFront() }
        function onRequestShowCircuitSettings() { circSettingsWin.bringToFront() }
        function onRequestShowInstaller() { installerWin.bringToFront() }
        function onRequestShowOsc() { oscWin.bringToFront() }
        function onRequestShowLa() { laWin.bringToFront() }
        function onRequestShowMcu() { mcuWin.bringToFront() }
        function onRequestShowSerialMon() { SerialMonitor.openMonitor("default", "Serial Monitor") }
        function onRequestShowTerminal() { terminalWin.bringToFront() }
        function onThemeChanged() {
            var isDark = App.darkTheme
            CircuitCanvas.dark = isDark
            Oscilloscope.dark = isDark
            LogicAnalyzer.dark = isDark
            EditorPanel.dark = isDark
            SerialTerminal.setDark(isDark)
            SimulatorLog.setDark(isDark)
            CompilerLog.setDark(isDark)
            InfoWidget.updateThemeColors(isDark)
        }
        function onDarkThemeChanged() {
            var isDark = App.darkTheme
            CircuitCanvas.dark = isDark
            Oscilloscope.dark = isDark
            LogicAnalyzer.dark = isDark
            EditorPanel.dark = isDark
            SerialTerminal.setDark(isDark)
            SimulatorLog.setDark(isDark)
            CompilerLog.setDark(isDark)
            InfoWidget.updateThemeColors(isDark)
        }
    }

    Connections {
        target: Application.styleHints
        function onColorSchemeChanged() {
            App.reloadTheme()
        }
    }

    Connections {
        target: Installer
        function onPackageInstalled(name) { ComponentList.reloadComponents() }
        function onPackageUninstalled(name) { ComponentList.reloadComponents() }
    }

    Connections {
        target: AppMenuBar
        function onAction(id) {
            if (id === "file.quit") {
                if (App.requestClose()) {
                    closeOwnedWindows()
                    Qt.quit()
                }
            } else if (id === "file.new") {
                if (EditorPanel.focused)
                    EditorPanel.newFile()
                else
                    CircuitCanvas.newCircuit()
            } else if (id === "file.newCirc") {
                CircuitCanvas.newCircuit()
            } else if (id === "file.open" || id === "file.openCirc" || id === "file.openFile") {
                openDialog.open()
            } else if (id === "file.save") {
                if (EditorPanel.focused)
                    EditorPanel.save()
                else if (CircuitCanvas.filePath)
                    CircuitCanvas.save()
                else
                    openSaveCircAs()
            } else if (id === "file.saveCirc") {
                if (CircuitCanvas.filePath)
                    CircuitCanvas.save()
                else
                    openSaveCircAs()
            } else if (id === "file.saveAs") {
                if (EditorPanel.focused)
                    openSaveFileAs()
                else
                    openSaveCircAs()
            } else if (id === "file.saveCircAs") {
                openSaveCircAs()
            } else if (id === "file.newFile") {
                EditorPanel.newFile()
            } else if (id === "file.saveFile") {
                EditorPanel.save()
            } else if (id === "file.saveFileAs") {
                openSaveFileAs()
            } else if (id === "file.closeFile") {
                EditorPanel.closeCurrent()
            } else if (id === "file.closeCirc") {
                CircuitCanvas.closeCircuit()
            } else if (id === "file.saveAll") {
                runSaveAll()
            } else if (id === "file.newWindow") {
                App.newWindow()
            } else if (id === "file.openFolder") {
                openFolderDialog.open()
            } else if (id === "file.closeProject") {
                FileBrowser.closeProject()
            } else if (id === "file.settings" || id === "file.appSettings") {
                App.showSettings()
            } else if (id === "file.clearRecentCircuits") {
                AppDialog.clearRecentCircuits(); AppMenuBar.rebuild()
            } else if (id === "file.clearRecentFiles") {
                AppDialog.clearRecentFiles(); AppMenuBar.rebuild()
            } else if (id === "file.clearRecentProjects") {
                AppDialog.clearRecentProjects(); AppMenuBar.rebuild()
            } else if (id.indexOf("openCirc:") === 0) {
                CircuitCanvas.loadPath(id.substring(9))
            } else if (id.indexOf("openFile:") === 0) {
                EditorPanel.loadFile(id.substring(9))
            } else if (id.indexOf("openProj:") === 0) {
                FileBrowser.rootPath = id.substring(9)
            } else if (id === "help.about") {
                App.showAbout()
            } else if (id === "help.aboutQt") {
                App.showAboutQt()
            } else if (id === "help.info") {
                CircuitPanel.toggleInfo()
            } else if (id === "sim.power" || id === "circ.power") {
                CircuitPanel.powerCirc()
            } else if (id === "sim.pause" || id === "circ.pause") {
                CircuitPanel.pauseCirc()
            } else if (id === "circ.settings") {
                App.showCircuitSettings()
            } else if (id === "view.zoomIn") {
                CircuitCanvas.zoomIn()
            } else if (id === "view.zoomOut") {
                CircuitCanvas.zoomOut()
            } else if (id === "view.zoomFit") {
                CircuitCanvas.zoomToFit()
            } else if (id === "view.zoomSel") {
                CircuitCanvas.zoomSelected()
            } else if (id === "view.zoomOne") {
                CircuitCanvas.zoomOne()
            } else if (id === "view.showGrid") {
                CircuitCanvas.showGrid = !CircuitCanvas.showGrid
                AppDialog.drawGrid = CircuitCanvas.showGrid
                AppMenuBar.setChecked("view.showGrid", CircuitCanvas.showGrid)
            } else if (id === "view.showScroll") {
                CircuitCanvas.showScroll = !CircuitCanvas.showScroll
                AppDialog.showScroll = CircuitCanvas.showScroll
                AppMenuBar.setChecked("view.showScroll", CircuitCanvas.showScroll)
            } else if (id === "sim.animateLogic") {
                CircuitCanvas.animateLogic = !CircuitCanvas.animateLogic
                AppMenuBar.setChecked("sim.animateLogic", CircuitCanvas.animateLogic)
            } else if (id === "sim.animateCurr") {
                CircuitCanvas.animateCurr = !CircuitCanvas.animateCurr
                AppMenuBar.setChecked("sim.animateCurr", CircuitCanvas.animateCurr)
            } else if (id === "view.searchComp") {
                CircuitPanel.showSidePanel()
                App.focusSearchComponent()
            } else if (id === "view.focusFiles") {
                CircuitPanel.showSidePanel()
                App.sidePanelTab = 1
            } else if (id === "view.focusEditor") {
                CircuitPanel.showEditor()
            } else if (id === "view.focusSimLog" || id === "view.simLog") {
                CircuitPanel.showSidePanel()
                App.sidePanelTab = 2
            } else if (id === "view.focusCompLog" || id === "view.compLog") {
                CircuitPanel.showSidePanel()
                App.sidePanelTab = 3
            } else if (id === "view.sidePanel") {
                CircuitPanel.togglePanel("sidePanel")
            } else if (id === "view.editorPanel") {
                CircuitPanel.togglePanel("editorPanel")
            } else if (id === "view.toggleActivePanel" || id === "view.collapsePanel") {
                CircuitPanel.toggleActiveOverlay()
            } else if (id === "view.libManager") {
                App.showInstaller()

            } else if (id === "view.cmdPalette") {
                openCommandPalette()
            } else if (id === "view.jumpRef") {
                CommandCenter.setCircuitItems(CircuitCanvas.items)
                CommandCenter.openJumpToReference()
            } else if (id === "view.tabSwitch") {
                if (!TabSwitcher.visible) populateTabSwitcher()
                TabSwitcher.openSwitcher(true)
            } else if (id === "debug.toggleRepaintOverlay" || id === "debug_toggle_repaint_overlay") {
                AppDialog.repaintOverlayEnabled = !AppDialog.repaintOverlayEnabled
            } else if (id === "debug.toggleComponentRects" || id === "debug_toggle_component_rects") {
                AppDialog.showComponentRects = !AppDialog.showComponentRects
            } else if (id === "edit.undo") {
                if (EditorPanel.focused) EditorPanel.undo()
                else CircuitCanvas.undo()
            } else if (id === "edit.redo") {
                if (EditorPanel.focused) EditorPanel.redo()
                else CircuitCanvas.redo()
            } else if (id === "edit.cut") {
                if (EditorPanel.focused) EditorPanel.cut()
                else CircuitCanvas.cutSelection()
            } else if (id === "edit.copy") {
                if (EditorPanel.focused) EditorPanel.copy()
                else CircuitCanvas.copySelection()
            } else if (id === "edit.paste") {
                if (EditorPanel.focused) EditorPanel.paste()
                else CircuitCanvas.pasteAtCursor()
            } else if (id === "edit.selectAll" || id === "edit.select_all") {
                if (EditorPanel.focused) EditorPanel.selectAll()
                else CircuitCanvas.selectAll()
            } else if (id === "edit.find") {
                EditorPanel.findDialog()
            } else if (id === "edit.format") {
                EditorPanel.formatDocument()
            } else if (id === "sim.compile" || id === "sim_compile") {
                EditorPanel.compile()
            } else if (id === "sim.load" || id === "sim_load") {
                EditorPanel.upload()
            } else if (id === "sim.uploadRun" || id === "sim.upload_run") {
                if (EditorPanel.documents.length === 0 && !CircuitPanel.simRunning)
                    CircuitCanvas.editFirmware("")
                EditorPanel.uploadRun()
            } else if (id === "sim.debug" || id === "sim_debug") {
                CircuitPanel.showEditor()
                if (EditorPanel.documents.length === 0)
                    CircuitCanvas.editFirmware("")
                EditorPanel.debug()
            } else if (id === "sim.run" || id === "sim_run") {
                if (EditorPanel.documents.length === 0 && !CircuitPanel.simRunning)
                    CircuitCanvas.editFirmware("")
                EditorPanel.run()
            } else if (id === "sim.step" || id === "sim_step") {
                EditorPanel.debugStep()
            } else if (id === "sim.stepOver" || id === "sim_step_over") {
                EditorPanel.debugStepOver()
            } else if (id === "sim.debugPause" || id === "sim_debug_pause") {
                EditorPanel.debugPause()
            } else if (id === "sim.stop" || id === "sim_stop") {
                EditorPanel.debugStop()
            } else if (id === "sim.reset" || id === "sim_reset") {
                EditorPanel.debugReset()
            } else if (id === "circ.addAllComponents") {
                CircuitCanvas.addAllComponents()
            }
        }
    }

    Connections {
        target: CircuitCanvas
        function onRequestShowProperties() {
            var uid = CircuitCanvas.selectedUid
            if (uid && propWindowManager.windows[uid]) {
                propWindowManager.windows[uid].bringToFront()
            }
        }
        function onHistoryChanged() {
            AppMenuBar.setEditState(CircuitCanvas.canUndo, CircuitCanvas.canRedo,
                                    CircuitCanvas.hasSelection, CircuitCanvas.canPaste)
        }
        function onItemsChanged() {
            AppMenuBar.setEditState(CircuitCanvas.canUndo, CircuitCanvas.canRedo,
                                    CircuitCanvas.hasSelection, CircuitCanvas.canPaste)
            CommandCenter.setCircuitItems(CircuitCanvas.items)
        }
        function onFilePathChanged() {
            App.windowTitle = CircuitCanvas.fileName
                    ? CircuitCanvas.fileName + " — Circuit Simulator"
                    : "Circuit Simulator"
            if (CircuitCanvas.filePath) {
                AppDialog.addRecentCircuit(CircuitCanvas.filePath)
                AppMenuBar.rebuild()
            }
            AppMenuBar.setChecked("sim.animateLogic", CircuitCanvas.animateLogic)
            AppMenuBar.setChecked("sim.animateCurr", CircuitCanvas.animateCurr)
        }
    }

    function isCircuitPath(path) {
        var p = ("" + path).toLowerCase()
        return p.endsWith(".circ1") || p.endsWith(".sim1") || p.endsWith(".sim2")
    }

    function openSelectedPath(url) {
        var path = "" + url
        if (isCircuitPath(path)) {
            CircuitCanvas.loadPath(path)
            return
        }
        CircuitPanel.showEditor()
        EditorPanel.loadFile(path)
        AppDialog.addRecentFile(path)
        AppMenuBar.rebuild()
    }

    FileDialog {
        id: openDialog
        title: App.translate("Open")
        fileMode: FileDialog.OpenFile
        currentFolder: App.suggestFolderUrl(FileBrowser.rootPath || CircuitCanvas.filePath || EditorPanel.currentPath)
        nameFilters: [ App.translate("Circuits (*.circ1 *.sim2 *.sim1)"), App.translate("All files (*)") ]
        onAccepted: openSelectedPath(selectedFile)
    }
    FileDialog {
        id: saveCircDialog
        title: App.translate("Save Circuit")
        fileMode: FileDialog.SaveFile
        currentFolder: App.suggestFolderUrl(CircuitCanvas.filePath)
        defaultSuffix: "circ1"
        nameFilters: [ App.translate("Circuits (*.circ1 *.sim2 *.sim1)"), App.translate("All files (*)") ]
        currentFile: CircuitCanvas.filePath ? App.suggestFileUrl(CircuitCanvas.filePath) : ""
        onAccepted: {
            CircuitCanvas.savePath("" + selectedFile)
            win.circSaveThenReplace = false
            if (win.saveAllPending)
                continueSaveAllFiles()
        }
        onRejected: {
            if (win.circSaveThenReplace) {
                win.circSaveThenReplace = false
                CircuitCanvas.confirmReplaceCancel()
            }
            win.saveAllPending = false
        }
    }

    FileDialog {
        id: saveFileDialog
        title: App.translate("Save File")
        fileMode: FileDialog.SaveFile
        currentFolder: App.suggestFolderUrl(EditorPanel.currentPath)
        nameFilters: [ App.translate("All files (*)") ]
        currentFile: EditorPanel.currentPath ? App.suggestFileUrl(EditorPanel.currentPath) : ""
        onAccepted: {
            EditorPanel.savePath("" + selectedFile)
            AppDialog.addRecentFile("" + selectedFile)
            AppMenuBar.rebuild()
            if (win.fileSaveThenClose) {
                win.fileSaveThenClose = false
                EditorPanel.finishPendingClose()
            }
            if (win.saveAllPending)
                continueSaveAllFiles()
        }
        onRejected: {
            if (win.fileSaveThenClose) {
                win.fileSaveThenClose = false
                EditorPanel.cancelPendingClose()
            }
            win.saveAllPending = false
        }
    }
    FolderDialog {
        id: openFolderDialog
        title: App.translate("Open Project")
        currentFolder: App.suggestFolderUrl(FileBrowser.rootPath)
        onAccepted: FileBrowser.rootPath = "" + selectedFolder
    }
    FileDialog {
        id: terminalLoadDialog
        title: App.translate("Send File")
        fileMode: FileDialog.OpenFile
        currentFolder: App.suggestFolderUrl("")
        nameFilters: [ App.translate("All files (*)") ]
        onAccepted: SerialTerminal.loadFile("" + selectedFile)
    }
    FileDialog {
        id: terminalSaveDialog
        title: App.translate("Save Log")
        fileMode: FileDialog.SaveFile
        currentFolder: App.suggestFolderUrl("")
        defaultSuffix: "txt"
        nameFilters: [ App.translate("Text files (*.txt)"), App.translate("All files (*)") ]
        onAccepted: SerialTerminal.saveLog("" + selectedFile)
    }

    Component {
        id: propWindowComponent
        AppWindow {
            id: propWin
            property string itemUid: ""
            property string itemTitle: ""
            title: itemTitle.length > 0
                   ? App.translate("Properties: %1").arg(itemTitle)
                   : App.translate("Properties")
            sizeToContent: true
            resizable: false
            flags: Qt.Dialog | Qt.CustomizeWindowHint | Qt.WindowTitleHint | Qt.WindowCloseButtonHint
            minimumWidth: 320
            minimumHeight: 160
            visible: true
            ownerWindow: win

            Component.onCompleted: {
                loadSource("PropDialog.qml", { "itemUid": itemUid })
                raise()
                requestActivate()
            }

            onClosing: (close) => {
                if (!win.isQuitting) {
                    close.accepted = false
                    visible = false
                    propWindowManager.closeDialog(itemUid)
                }
            }
        }
    }

    QtObject {
        id: propWindowManager
        property var windows: ({})
        property int cascadeCounter: 0

        function syncOpenDialogs() {
            var list = CircuitCanvas.openPropDialogs || []
            var activeUids = {}
            for (var i = 0; i < list.length; i++) {
                var entry = list[i]
                var uid = entry.uid
                activeUids[uid] = true
                if (!windows[uid]) {
                    var offset = (cascadeCounter % 8) * 28
                    cascadeCounter++
                    var w = propWindowComponent.createObject(win, {
                        itemUid: uid,
                        itemTitle: entry.title || entry.typeText || "",
                        x: win.x + Math.round((win.width - 320) / 2) + offset,
                        y: win.y + Math.round((win.height - 160) / 2) + offset
                    })
                    if (w) {
                        windows[uid] = w
                    }
                } else {
                    var newTitle = entry.title || entry.typeText || ""
                    if (newTitle.length > 0 && windows[uid].itemTitle !== newTitle) {
                        windows[uid].itemTitle = newTitle
                    }
                }
            }

            var toDelete = []
            for (var id in windows) {
                if (!activeUids[id]) {
                    toDelete.push(id)
                }
            }
            for (var j = 0; j < toDelete.length; j++) {
                var delId = toDelete[j]
                var targetWin = windows[delId]
                delete windows[delId]
                if (targetWin) {
                    targetWin.destroy()
                }
            }
        }

        function closeDialog(uid) {
            var targetWin = windows[uid]
            if (targetWin) {
                delete windows[uid]
                targetWin.destroy()
            }
            CircuitCanvas.closePropertyDialog(uid)
        }

        function bringToFront(uid) {
            if (windows[uid]) {
                windows[uid].bringToFront()
            }
        }
    }

    Connections {
        target: CircuitCanvas
        function onOpenPropDialogsChanged() {
            propWindowManager.syncOpenDialogs()
        }
        function onRequestBringPropToFront(uid) {
            propWindowManager.bringToFront(uid)
        }
    }

    Connections {
        target: CircuitPanel
        function onToolbarChanged() {
            AppMenuBar.setSimState(CircuitPanel.simRunning, CircuitPanel.simPaused)
        }
        function onAction(id) {
            if (id === "settCircuit") App.showCircuitSettings()
        }
    }

    Connections {
        target: AppDialog
        function onAppChanged() {
            App.reloadI18n()
            AppMenuBar.rebuild()
            ComponentList.reloadComponents()
            SimulatorLog.reloadSettings()
            CompilerLog.reloadSettings()
            EditorPanel.reloadSettings()
            var isDark = App.darkTheme
            CircuitCanvas.dark = isDark
            Oscilloscope.dark = isDark
            LogicAnalyzer.dark = isDark
            EditorPanel.dark = isDark
            SerialTerminal.setDark(isDark)
            SimulatorLog.setDark(isDark)
            CompilerLog.setDark(isDark)
            InfoWidget.updateThemeColors(isDark)
        }
        function onCircuitChanged() {
            CircuitCanvas.showGrid = AppDialog.drawGrid
            CircuitCanvas.showScroll = AppDialog.showScroll
            AppMenuBar.setChecked("view.showGrid", AppDialog.drawGrid)
            AppMenuBar.setChecked("view.showScroll", AppDialog.showScroll)
        }
        function onDebugChanged() {
            CircuitCanvas.showComponentRects = AppDialog.showComponentRects
        }
        function onEditorChanged() {
            EditorPanel.reloadSettings()
            SimulatorLog.reloadSettings()
            CompilerLog.reloadSettings()
        }
    }

    Connections {
        target: App
        function onI18nTickChanged() {
            AppMenuBar.rebuild()
            ComponentList.reloadComponents()
            SimulatorLog.reloadSettings()
            CompilerLog.reloadSettings()
            EditorPanel.reloadSettings()
        }
    }

    // Poll the background compiler while a compilation is running so that logs
    // and completion events drain even if the code editor tab/window is hidden.
    Timer {
        interval: 50
        running: EditorPanel.compiling
        repeat: true
        onTriggered: EditorPanel.pollLsp()
    }

    Connections {
        target: EditorPanel
        function onRequestShowFind() { findWin.bringToFront() }
        function onRequestShowFileSettings() { fileSettingsWin.bringToFront() }
        function onRequestShowCompilerSettings() { compilerSettingsWin.bringToFront() }
        function onRequestSaveAs() { openSaveFileAs() }
        function onLogChanged() {
            var line
            while ((line = EditorPanel.takeLogLine()) !== "") {
                CompilerLog.appendLine(line)
            }
        }
        function onCompilingChanged() {
            var line
            while ((line = EditorPanel.takeLogLine()) !== "") {
                CompilerLog.appendLine(line)
            }
        }
        function onRequestCompilerLog() {
            CompilerLog.clear()
            CircuitPanel.showSidePanel()
            App.sidePanelTab = 3
        }
        function onRequestUpload() {
            CircuitCanvas.uploadFirmware(EditorPanel.lastFirmware)
        }
        function onRequestPowerOn() {
            if (!CircuitPanel.simRunning)
                CircuitPanel.powerCirc()
            else if (CircuitPanel.simPaused)
                CircuitPanel.pauseCirc()
        }
        function onFocusedChanged() {
            if (!CommandCenter.visible)
                CommandCenter.editorFocused = EditorPanel.focused
            if (!EditorPanel.focused)
                AppMenuBar.setEditState(CircuitCanvas.canUndo, CircuitCanvas.canRedo,
                                        CircuitCanvas.hasSelection, CircuitCanvas.canPaste)
        }
    }

    Connections {
        target: CircuitCanvas
        function onSimLogChanged() {
            var line
            while ((line = CircuitCanvas.takeSimLogLine()) !== "")
                SimulatorLog.appendLine(line)
        }
    }

    Connections {
        target: CompilerLog
        function onLocationClicked() {
            if (CompilerLog.locationFile !== "") {
                EditorPanel.navigateToSource(CompilerLog.locationFile, CompilerLog.locationLine)
            }
        }
    }

    Connections {
        target: SimulatorLog
        function onLocationClicked() {
            if (SimulatorLog.locationFile !== "") {
                EditorPanel.navigateToSource(SimulatorLog.locationFile, SimulatorLog.locationLine)
            }
        }
    }

    AppWindow {
        id: findWin
        title: win.tr("Find/Replace")
        sizeToContent: true
        minimumWidth: 460
        source: "FindReplace.qml"
        visible: EditorPanel.findVisible
        onVisibleChanged: {
            if (visible) {
                x = win.x + Math.round((win.width - width) / 2)
                y = win.y + Math.round((win.height - height) / 2)
                raise()
                requestActivate()
            }
        }
        onClosing: (close) => {
            if (!win.isQuitting) {
                close.accepted = false
                EditorPanel.closeFind()
            }
        }
    }
    AppWindow {
        id: fileSettingsWin
        title: win.tr("File Settings")
        sizeToContent: true
        minimumWidth: 420
        source: "FileSettings.qml"
        visible: EditorPanel.fileSettingsVisible
        onVisibleChanged: {
            if (visible) {
                x = win.x + Math.round((win.width - width) / 2)
                y = win.y + Math.round((win.height - height) / 2)
                raise()
                requestActivate()
            }
        }
        onClosing: (close) => {
            if (!win.isQuitting) {
                close.accepted = false
                EditorPanel.fileSettingsVisible = false
            }
        }
    }
    AppWindow {
        id: compilerSettingsWin
        title: EditorPanel.hasCompiler
               ? win.tr("%1 Settings").arg(EditorPanel.compilerName)
               : win.tr("Compiler Settings")
        sizeToContent: true
        minimumWidth: 420
        source: "CompilerSettings.qml"
        visible: EditorPanel.compilerSettingsVisible
        onVisibleChanged: {
            if (visible) {
                x = win.x + Math.round((win.width - width) / 2)
                y = win.y + Math.round((win.height - height) / 2)
                raise()
                requestActivate()
            }
        }
        onClosing: (close) => {
            if (!win.isQuitting) {
                close.accepted = false
                EditorPanel.compilerSettingsVisible = false
            }
        }
    }

    Connections {
        target: SerialTerminal
        function onRequestLoadFile() { terminalLoadDialog.open() }
        function onRequestSaveLog() { terminalSaveDialog.open() }
    }

    Connections {
        target: FileBrowser
        function onRootPathChanged() {
            AppMenuBar.setHasProject(FileBrowser.hasProject)
            if (FileBrowser.rootPath) {
                AppDialog.addRecentProject(FileBrowser.rootPath)
                AppMenuBar.rebuild()
            }
            CommandCenter.projectDir = FileBrowser.rootPath
            EditorPanel.projectPath = FileBrowser.rootPath
            CircuitCanvas.projectPath = FileBrowser.rootPath
        }
    }

    onDarkChromeChanged: {
        var isDark = darkChrome
        CircuitCanvas.dark = isDark
        Oscilloscope.dark = isDark
        LogicAnalyzer.dark = isDark
        EditorPanel.dark = isDark
        SerialTerminal.setDark(isDark)
        SimulatorLog.setDark(isDark)
        CompilerLog.setDark(isDark)
        InfoWidget.updateThemeColors(isDark)
    }
    Component.onCompleted: {
        var isDark = App.darkTheme
        CircuitCanvas.dark = isDark
        Oscilloscope.dark = isDark
        LogicAnalyzer.dark = isDark
        EditorPanel.dark = isDark
        SerialTerminal.setDark(isDark)
        SimulatorLog.setDark(isDark)
        CompilerLog.setDark(isDark)
        InfoWidget.updateThemeColors(isDark)
        App.installNativeChrome()
        CircuitPanel.installNativeChrome()
        if (App.windowMaximized) {
            win.visibility = Window.Maximized
        } else if (App.windowX >= 0 && App.windowY >= 0) {
            win.x = App.windowX
            win.y = App.windowY
        }
        win.show()
        win.raise()
        win.requestActivate()
        AppMenuBar.setHasProject(FileBrowser.hasProject)
        AppMenuBar.installNativeMenus()
        CircuitCanvas.showGrid = AppDialog.drawGrid
        CircuitCanvas.showScroll = AppDialog.showScroll
        CircuitCanvas.showComponentRects = AppDialog.showComponentRects
        AppMenuBar.setChecked("view.showGrid", CircuitCanvas.showGrid)
        AppMenuBar.setChecked("view.showScroll", CircuitCanvas.showScroll)
        AppMenuBar.setChecked("sim.animateLogic", CircuitCanvas.animateLogic)
        AppMenuBar.setChecked("sim.animateCurr", CircuitCanvas.animateCurr)
        CommandCenter.projectDir = FileBrowser.rootPath
        EditorPanel.projectPath = FileBrowser.rootPath
        CircuitCanvas.projectPath = FileBrowser.rootPath
        CommandCenter.setCircuitItems(CircuitCanvas.items)
        CommandCenter.editorFocused = EditorPanel.focused
        var simLine
        while ((simLine = CircuitCanvas.takeSimLogLine()) !== "")
            SimulatorLog.appendLine(simLine)
        var compLine
        while ((compLine = EditorPanel.takeLogLine()) !== "") {
            CompilerLog.appendLine(compLine)
        }
        CircuitCanvas.restoreDraft()
        EditorPanel.restoreDrafts()

    }

    function openCommandPalette() {
        CommandCenter.editorFocused = EditorPanel.focused
        CommandCenter.openCommandPalette()
    }

    function runSaveAll() {
        if (CircuitCanvas.modified && !CircuitCanvas.filePath) {
            win.saveAllPending = true
            openSaveCircAs()
            return
        }
        if (CircuitCanvas.filePath && CircuitCanvas.modified)
            CircuitCanvas.save()
        continueSaveAllFiles()
    }

    function continueSaveAllFiles() {
        if (EditorPanel.saveAll())
            win.saveAllPending = false
        else
            win.saveAllPending = true
    }

    function joinFolderFile(folderUrl, fileName) {
        var folder = ("" + folderUrl).replace(/\/$/, "")
        return folder + "/" + fileName
    }

    function openSaveCircAs() {
        var folder = App.suggestFolderUrl(CircuitCanvas.filePath)
        var url = CircuitCanvas.filePath
            ? App.suggestFileUrl(CircuitCanvas.filePath)
            : joinFolderFile(folder, "untitled.circ1")
        saveCircDialog.currentFolder = folder
        saveCircDialog.selectedFile = url
        saveCircDialog.currentFile = url
        saveCircDialog.open()
    }

    function openSaveFileAs() {
        var path = EditorPanel.currentPath
        var folder = App.suggestFolderUrl(path)
        var url = path
            ? App.suggestFileUrl(path)
            : joinFolderFile(folder, EditorPanel.currentTitle || "untitled")
        saveFileDialog.currentFolder = folder
        saveFileDialog.selectedFile = url
        saveFileDialog.currentFile = url
        saveFileDialog.open()
    }

    Shortcut {
        sequences: ["Ctrl+P", "Meta+P"]
        onActivated: {
            CommandCenter.setCircuitItems(CircuitCanvas.items)
            CommandCenter.openJumpToReference()
        }
    }
    Shortcut {
        sequences: ["Ctrl+Shift+P", "Meta+Shift+P"]
        onActivated: openCommandPalette()
    }
    Shortcut {
        sequences: ["Ctrl+N", "Meta+N"]
        onActivated: AppMenuBar.triggerAction("file.new")
    }
    Shortcut {
        sequences: ["Ctrl+Alt+S", "Meta+Alt+S"]
        onActivated: runSaveAll()
    }
    Shortcut {
        sequences: ["Ctrl+W", "Meta+W"]
        enabled: EditorPanel.focused
        onActivated: EditorPanel.closeCurrent()
    }
    Shortcut {
        sequences: ["Ctrl+Alt+D", "Meta+Alt+D"]
        onActivated: AppDialog.repaintOverlayEnabled = !AppDialog.repaintOverlayEnabled
    }
    Shortcut {
        sequences: ["Ctrl+Alt+C", "Meta+Alt+C"]
        onActivated: AppDialog.showComponentRects = !AppDialog.showComponentRects
    }
    Shortcut {
        sequences: ["F12"]
        onActivated: AppMenuBar.triggerAction("sim.debug")
    }
    Shortcut {
        sequences: ["F5"]
        onActivated: AppMenuBar.triggerAction("sim.run")
    }
    Shortcut {
        sequences: ["Ctrl+,", "Meta+,"]
        onActivated: App.showSettings()
    }
    Shortcut {
        sequences: ["Ctrl+J", "Meta+J"]
        onActivated: CircuitPanel.toggleActiveOverlay()
    }

    CommandCenterDialog {
        id: commandCenterDialog
    }

    Connections {
        target: CommandCenter
        function onShortcutsUpdated() {
            AppMenuBar.rebuild()
        }
        function onExecuteItem(typ, data, compType) {
            if (typ === "action") {
                AppMenuBar.triggerAction(data)
            } else if (typ === "component") {
                var spec = (data === compType || !compType) ? data : (data + "," + compType)
                ComponentList.addRecent(spec)
                CircuitCanvas.addComponent(spec)
                if (compType === "Oscope" || compType === "Oscilloscope") {
                    App.showOsc()
                } else if (compType === "LAnalizer") {
                    App.showLa()
                } else if (compType === "MCU") {
                    if (data === "MCU")
                        App.showMcu()
                } else if (compType === "QemuDevice") {
                    App.showMcu()
                } else if (compType === "SerialPort") {
                    App.showSerialMon()
                } else if (compType === "SerialTerm") {
                    App.showTerminal()
                }
            } else if (typ === "jump_comp") {
                CircuitCanvas.focusComponent(data)
            } else if (typ === "jump_file") {
                if (data.endsWith(".circ1") || data.endsWith(".sim1") || data.endsWith(".sim2")) {
                    CircuitCanvas.loadPath(data)
                } else {
                    CircuitPanel.showEditor()
                    EditorPanel.openPath(data)
                }
            }
        }
    }

    function populateTabSwitcher() {
        var items = []
        var curCirc = CircuitCanvas.fileName ? CircuitCanvas.fileName : App.translate("Untitled Circuit")
        items.push({
            "label": curCirc,
            "kind": "circuit",
            "id": CircuitCanvas.filePath || "",
            "icon": "account_tree",
            "badge": App.translate("Circuit"),
            "tooltip": CircuitCanvas.filePath || curCirc
        })

        var docs = EditorPanel.documents || []
        for (var i = 0; i < docs.length; ++i) {
            var d = docs[i]
            items.push({
                "label": d.title || (App.translate("Untitled") + " " + (i + 1)),
                "kind": "editor",
                "id": d.path || "",
                "index": i,
                "icon": "code",
                "badge": App.translate("Editor"),
                "tooltip": d.path || d.title
            })
        }

        var recents = AppDialog.recentCircuits || []
        for (var r = 0; r < recents.length && items.length < 15; ++r) {
            var rPath = recents[r]
            if (rPath !== CircuitCanvas.filePath) {
                var rName = rPath.split("/").filter(Boolean).pop() || rPath
                items.push({
                    "label": rName,
                    "kind": "circuit",
                    "id": rPath,
                    "icon": "schema",
                    "badge": App.translate("Recent"),
                    "tooltip": rPath
                })
            }
        }

        TabSwitcher.populate(items)
    }

    Shortcut {
        sequences: ["Ctrl+Tab", "Meta+Tab"]
        onActivated: {
            if (!TabSwitcher.visible) populateTabSwitcher()
            TabSwitcher.openSwitcher(true)
        }
    }
    Shortcut {
        sequences: ["Ctrl+Shift+Tab", "Meta+Shift+Tab"]
        onActivated: {
            if (!TabSwitcher.visible) populateTabSwitcher()
            TabSwitcher.openSwitcher(false)
        }
    }

    TabSwitcherDialog {
        id: tabSwitcherDialog
    }

    Connections {
        target: TabSwitcher
        function onExecuteItem(kind, id, index) {
            if (kind === "circuit") {
                if (id && id.length > 0 && id !== CircuitCanvas.filePath) {
                    CircuitCanvas.loadPath(id)
                }
            } else if (kind === "editor") {
                CircuitPanel.showEditor()
                if (index >= 0) {
                    EditorPanel.currentDocument = index
                } else if (id && id.length > 0) {
                    EditorPanel.openPath(id)
                }
            }
        }
    }

    AppConfirmDialog {
        id: circuitReplaceDialog
        dialogTitle: App.translate("Save Circuit")
        dialogText: App.translate("Circuit has been modified.\nDo you want to save your changes?")
        acceptText: App.translate("Save")
        discardText: App.translate("Discard")
        cancelText: App.translate("Cancel")
        onAccepted: CircuitCanvas.confirmReplaceSave()
        onDiscarded: CircuitCanvas.confirmReplaceDiscard()
        onCancelled: CircuitCanvas.confirmReplaceCancel()
    }

    AppConfirmDialog {
        id: fileCloseDialog
        dialogTitle: App.translate("Save File")
        dialogText: App.translate("The Document has been modified.\nDo you want to save your changes?")
        acceptText: App.translate("Save")
        discardText: App.translate("Discard")
        cancelText: App.translate("Cancel")
        onAccepted: EditorPanel.confirmCloseSave()
        onDiscarded: EditorPanel.confirmCloseDiscard()
        onCancelled: EditorPanel.confirmCloseCancel()
    }

    Connections {
        target: CircuitCanvas
        function onRequestSaveBeforeReplace() { circuitReplaceDialog.open() }
        function onRequestSaveAsThenReplace() {
            win.circSaveThenReplace = true
            openSaveCircAs()
        }
    }

    Connections {
        target: EditorPanel
        function onRequestSaveBeforeClose() { fileCloseDialog.open() }
        function onRequestSaveAsThenClose() {
            win.fileSaveThenClose = true
            openSaveFileAs()
        }
    }

    Connections {
        target: App
        function onFlushSession() {
            EditorPanel.flushDrafts()
            CircuitCanvas.flushDraft()
        }
    }

    onClosing: (close) => {
        saveGeometryTimer.stop()
        if (win.visibility === Window.Maximized) {
            App.saveWindowState(win.x, win.y, win.width, win.height, true)
        } else if (win.visibility === Window.Windowed) {
            App.saveWindowState(win.x, win.y, win.width, win.height, false)
        }
        if (!App.requestClose()) {
            close.accepted = false
            return
        }
        win.isQuitting = true
        closeOwnedWindows()
        Qt.quit()
    }
}
