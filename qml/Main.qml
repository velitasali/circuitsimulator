pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls
import QtQuick.Effects
import QtQuick.Window
import QtQuick.Dialogs
import cs_app
import cs_ui

/* Spike of src/gui/main.qml. CircuitPanel owns the canvas and overlays.
 * Extra windows (About, plots, MCU monitor, serial) are QML Window, not C++ QmlWindow.
 *
 * The menu bar is platform-split: macOS gets a real NSMenu (App.nativeMenus),
 * while non-macOS systems show it inline in the TitleBar when there is enough
 * space and fall back to the compact menu button inside CircuitPanel's toolbar.
 */
ApplicationWindow {
    id: win
    objectName: "mainWindow"
    SystemPalette { id: appTheme }


    readonly property bool darkChrome: appTheme.window.hslLightness < 0.5
    property bool saveAllPending: false
    property bool circSaveThenReplace: false
    property bool fileSaveThenClose: false
    property bool isQuitting: false
    property list<AppWindow> ownedWindows: []

    component OwnedWindow: AppWindow {
        id: ownedWin
        ownerWindow: win
        Component.onCompleted: win.registerOwnedWindow(ownedWin)
        Component.onDestruction: win.unregisterOwnedWindow(ownedWin)
    }

    function registerOwnedWindow(w: AppWindow): void {
        if (!w)
            return
        for (let i = 0; i < ownedWindows.length; i++) {
            if (ownedWindows[i] === w)
                return
        }
        ownedWindows.push(w)
    }

    function unregisterOwnedWindow(w: AppWindow): void {
        const next = []
        for (let i = 0; i < ownedWindows.length; i++) {
            if (ownedWindows[i] !== w)
                next.push(ownedWindows[i])
        }
        ownedWindows = next
    }

    function closeOwnedWindows(): void {
        const list = []
        for (let i = 0; i < ownedWindows.length; i++)
            list.push(ownedWindows[i])
        for (let i = 0; i < list.length; i++) {
            const w = list[i] as AppWindow
            if (w && w.closeWithOwner !== false)
                w.dismiss()
        }
    }

    // Same set MainWindow registers through QFontDatabase in the C++ app.
    // FontLoader adds each family to the app font database, so App.fontFamily
    // (default "Ubuntu") and the settings font combo resolve them.
    FontLoader { source: "qrc:/fonts/Ubuntu-R.ttf" }
    FontLoader { source: "qrc:/fonts/Ubuntu-B.ttf" }
    FontLoader { source: "qrc:/fonts/UbuntuMono-R.ttf" }
    FontLoader { source: "qrc:/fonts/UbuntuMono-RI.ttf" }
    FontLoader { source: "qrc:/fonts/UbuntuMono-B.ttf" }
    FontLoader { source: "qrc:/fonts/UbuntuMono-BI.ttf" }

    readonly property bool linuxCustomChrome: Qt.platform.os === "linux" && App.customTitlebar
    readonly property bool linuxWayland: linuxCustomChrome && Qt.platform.pluginName === "wayland"
    readonly property bool linuxMaximized: win.visibility === Window.Maximized || win.visibility === Window.FullScreen
    property int linuxBaseShadowMargin: linuxCustomChrome ? 20 : 0
    readonly property int linuxShadowMargin: (linuxCustomChrome && !linuxMaximized) ? linuxBaseShadowMargin : 0
    // Grab handles hang this far outside windowFrame. theme.cpp reads the same property.
    readonly property int linuxResizeOverhang: 3
    readonly property int linuxResizeThickness: 5

    // macOS: draw the client area under the traffic lights. The QML title strip
    // (TitleBar) replaces the centered title. Linux: frameless with custom titlebar,
    // client-side shadow gutter, and rounded corners. Windows removes its caption in native code.
    flags: (Qt.platform.os === "osx" && App.customTitlebar)
           ? (Qt.Window | Qt.ExpandedClientAreaHint | Qt.NoTitleBarBackgroundHint)
           : win.linuxCustomChrome
           ? (Qt.Window | Qt.FramelessWindowHint)
           : Qt.Window
    // An explicit 0 stops ApplicationWindow from binding padding to the safe
    // area, which would push the canvas down below the title strip.
    topPadding: 0
    leftPadding: 0
    rightPadding: 0
    bottomPadding: 0

    visible: true
    minimumWidth: App.minWindowWidth + win.linuxShadowMargin * 2
    minimumHeight: App.minWindowHeight + win.linuxShadowMargin * 2
    // setCustomMargins grows a Wayland window by the gutter. X11 does not, so only X11 adds it here.
    width: 1200 + ((win.linuxCustomChrome && !win.linuxWayland) ? win.linuxBaseShadowMargin * 2 : 0)
    height: 800 + ((win.linuxCustomChrome && !win.linuxWayland) ? win.linuxBaseShadowMargin * 2 : 0)
    title: {
        var _tick = App.i18nTick
        var state = CircuitCanvas.simRunning ? (CircuitCanvas.simPaused ? App.translate("[Paused] ") : App.translate("[Running] ")) : ""
        var proj = FileBrowser.rootPath ? ("[" + FileBrowser.rootPath.split("/").filter(Boolean).pop() + "] ") : ""
        var file = CircuitCanvas.fileName ? CircuitCanvas.fileName : App.translate("Untitled")
        var changed = CircuitCanvas.modified ? "*" : ""
        return state + proj + file + changed + " - " + App.translate("Circuit Simulator")
    }
    font: Qt.font({ family: App.fontFamily, pixelSize: App.fontSize })
    color: win.linuxCustomChrome ? "transparent" : appTheme.window
    background: null
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

    Item {
        id: windowMask
        anchors.fill: windowFrame
        visible: false
        layer.enabled: win.linuxShadowMargin > 0

        Rectangle {
            anchors.fill: parent
            radius: 14
        }
    }

    RectangularShadow {
        anchors.fill: windowFrame
        visible: win.linuxShadowMargin > 0
        radius: 14
        blur: 14
        offset: Qt.vector2d(0, 3)
        color: win.active ? App.windowShadow : App.windowShadowInactive
    }

    Rectangle {
        id: windowFrame
        anchors.fill: parent
        anchors.margins: win.linuxShadowMargin
        radius: win.linuxShadowMargin > 0 ? 14 : 0
        color: appTheme.window
        layer.enabled: win.linuxShadowMargin > 0
        layer.effect: MultiEffect {
            maskEnabled: true
            maskSource: windowMask
        }

        TitleBar {
            id: titleBar
            visible: App.customTitlebar
            window: win
            anchors.top: parent.top
            anchors.left: parent.left
            anchors.right: parent.right
            z: 10
        }

        Loader {
            id: circuitPanelLoader
            anchors.fill: parent
            source: "CircuitPanelView.qml"
            focus: true
        }

        Binding {
            target: circuitPanelLoader.item
            property: "topInset"
            value: titleBar.visible ? titleBar.height : 0
        }

        Binding {
            target: circuitPanelLoader.item
            property: "inlineMenuBarVisible"
            value: titleBar.visible && titleBar.menuBarVisible
        }
    }

    Rectangle {
        anchors.fill: windowFrame
        z: 20
        visible: win.linuxShadowMargin > 0
        radius: 14
        color: "transparent"
        border.color: appTheme.mid
        border.width: 1
    }

    WindowResizeHandles {
        targetWindow: win
        targetItem: windowFrame
        overhang: win.linuxResizeOverhang
        handleThickness: win.linuxResizeThickness
        visible: win.linuxShadowMargin > 0
    }

    OwnedWindow {
        id: aboutWin
        title: App.translate("About Circuit Simulator")
        sizeToContent: true
        panelSource: "about.qml"
        openRequested: App.aboutVisible

        onClosing: (close) => {
            if (!win.isQuitting) {
                close.accepted = false
                App.aboutVisible = false
            }
        }
    }

    OwnedWindow {
        id: examplesWin
        title: App.translate("Example circuits")
        sizeToContent: true
        panelSource: "examples.qml"

        onClosing: (close) => {
            if (!win.isQuitting) {
                close.accepted = false
                openRequested = false
            }
        }

        Connections {
            target: examplesWin.panel
            ignoreUnknownSignals: true
            function onExampleChosen(id: string): void {
                CircuitCanvas.loadExample(id)
                examplesWin.openRequested = false
            }
        }
    }

    OwnedWindow {
        id: bodeWin
        title: I18n.tr("AC Sweep")
        width: 820
        height: 560
        panelSource: "BodeView.qml"

        onClosing: (close) => {
            if (!win.isQuitting) {
                close.accepted = false
                openRequested = false
            }
        }
    }

    OwnedWindow {
        id: curveTracerWin
        title: I18n.tr("DC Parameter Sweep / Curve Tracer")
        width: 920
        height: 620
        panelSource: "CurveTracerView.qml"

        onClosing: (close) => {
            if (!win.isQuitting) {
                close.accepted = false
                openRequested = false
            }
        }
    }

    OwnedWindow {
        id: oscWin
        title: I18n.tr("Oscilloscope")
        width: 900
        height: 520
        panelSource: "OscView.qml"
        openRequested: App.oscVisible

        onClosing: (close) => {
            if (!win.isQuitting) {
                close.accepted = false
                App.oscVisible = false
            }
        }
    }

    OwnedWindow {
        id: laWin
        title: I18n.tr("Logic Analyzer")
        width: 900
        height: 520
        panelSource: "LaView.qml"
        openRequested: App.laVisible

        onClosing: (close) => {
            if (!win.isQuitting) {
                close.accepted = false
                App.laVisible = false
            }
        }
    }

    OwnedWindow {
        id: mcuWin
        title: I18n.tr("MCU Monitor")
        width: 900
        height: 520
        panelSource: "McuView.qml"
        openRequested: App.mcuVisible

        onClosing: (close) => {
            if (!win.isQuitting) {
                close.accepted = false
                App.mcuVisible = false
            }
        }
    }

    property var serialMonWindows: ({})
    property var serialMonLastPos: null

    function omitKey(map: var, key: string): var {
        var copy = {}
        if (!map) return copy
        for (var k in map) {
            if (k !== key && map[k] !== undefined)
                copy[k] = map[k]
        }
        return copy
    }

    function showSerialMonitor(monId: string, title: string): void {
        if (!monId)
            return
        var existing = serialMonWindows[monId] as AppWindow
        if (existing) {
            if (title && title.length > 0)
                existing.title = title
            existing.bringToFront()
            return
        }
        var winTitle = title ? title : SerialMonitor.titleFor(monId)
        var props = {
            monId: monId,
            title: winTitle
        }
        if (win.serialMonLastPos) {
            props.hasSavedPosition = true
            props.savedX = win.serialMonLastPos.x
            props.savedY = win.serialMonLastPos.y
        }
        var w = serialMonComp.createObject(win, props) as AppWindow
        if (w) {
            serialMonWindows[monId] = w
        }
    }

    function dismissSerialMonitor(monId: string): void {
        if (!monId)
            return
        var w = serialMonWindows[monId] as AppWindow
        if (w) {
            if (w.hasSavedPosition) {
                win.serialMonLastPos = { x: w.savedX, y: w.savedY }
            }
            serialMonWindows = win.omitKey(serialMonWindows, monId)
            w.destroy()
        }
    }

    Connections {
        target: SerialMonitor
        function onRequestShowMonitor(id: string, title: string): void {
            win.showSerialMonitor(id, title)
        }
        function onMonitorClosed(id: string): void {
            win.dismissSerialMonitor(id)
        }
    }

    Component {
        id: serialMonComp
        OwnedWindow {
            id: monWin
            property string monId: ""
            width: 560
            height: 420
            panelSource: "qrc:/qt/qml/cs_app/serialmon.qml"
            panelProperties: ({ monitorId: monWin.monId })
            openRequested: true

            onSavedXChanged: if (monWin.hasSavedPosition) win.serialMonLastPos = { x: monWin.savedX, y: monWin.savedY }
            onSavedYChanged: if (monWin.hasSavedPosition) win.serialMonLastPos = { x: monWin.savedX, y: monWin.savedY }

            onClosing: (close) => {
                close.accepted = true
                if (monWin.hasSavedPosition) {
                    win.serialMonLastPos = { x: monWin.savedX, y: monWin.savedY }
                }
                if (win.serialMonWindows && win.serialMonWindows[monWin.monId]) {
                    win.serialMonWindows = win.omitKey(win.serialMonWindows, monWin.monId)
                }
                if (!win.isQuitting) {
                    SerialMonitor.closeMonitor(monWin.monId)
                }
                monWin.destroy()
            }
        }
    }

    OwnedWindow {
        id: terminalWin
        title: I18n.tr("Serial Terminal")
        width: 560
        height: 420
        panelSource: "terminal.qml"
        openRequested: App.terminalVisible

        onAboutToShow: SerialTerminal.setDark(win.darkChrome)
        onClosing: (close) => {
            if (!win.isQuitting) {
                close.accepted = false
                App.terminalVisible = false
            }
        }
    }

    OwnedWindow {
        id: memTableWin
        title: MemoryTable.title || I18n.tr("Memory Table")
        width: 720
        height: 480
        contentMinWidth: 520
        contentMinHeight: 300
        panelSource: "MemTablePanel.qml"
        openRequested: MemoryTable.visible

        onClosing: (close) => {
            if (!win.isQuitting) {
                close.accepted = false
                MemoryTable.visible = false
            }
        }
    }

    OwnedWindow {
        id: settingsWin
        title: I18n.tr("Settings")
        sizeToContent: true
        contentMinWidth: 420
        contentMinHeight: 380
        panelSource: "appdialog.qml"
        openRequested: App.settingsVisible

        onClosing: (close) => {
            if (!win.isQuitting) {
                close.accepted = false
                App.settingsVisible = false
            }
        }
    }

    OwnedWindow {
        id: circSettingsWin
        title: I18n.tr("Circuit Settings")
        maximizable: false
        width: 420
        height: 560
        contentMinWidth: panelItem ? Math.max(360, Math.ceil(panelItem.implicitWidth)) : 360
        contentMinHeight: 380
        panelSource: "circuitdialog.qml"
        openRequested: App.circuitSettingsVisible

        onClosing: (close) => {
            if (!win.isQuitting) {
                close.accepted = false
                App.circuitSettingsVisible = false
            }
        }
    }

    OwnedWindow {
        id: installerWin
        title: I18n.tr("Library Manager")
        maximizable: false
        width: 600
        height: 520
        contentMinWidth: 480
        contentMinHeight: 360
        panelSource: "installer.qml"
        openRequested: App.installerVisible

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
        function onPackageInstalled(name: string): void { ComponentList.reloadComponents() }
        function onPackageUninstalled(name: string): void { ComponentList.reloadComponents() }
    }

    Connections {
        target: AppMenuBar
        function onAction(id: string): void {
            if (id === "file.quit") {
                CircuitCanvas.powerOff()
                if (App.requestClose()) {
                    win.closeOwnedWindows()
                    Qt.quit()
                }
            } else if (id === "file.new") {
                if (EditorPanel.focused)
                    EditorPanel.newFile()
                else
                    CircuitCanvas.newCircuit()
            } else if (id === "file.newCirc") {
                CircuitCanvas.newCircuit()
            } else if (id === "file.open") {
                openDialog.open()
            } else if (id === "file.save") {
                if (EditorPanel.focused)
                    EditorPanel.save()
                else if (CircuitCanvas.filePath)
                    CircuitCanvas.save()
                else
                    win.openSaveCircAs()
            } else if (id === "file.saveCirc") {
                if (CircuitCanvas.filePath)
                    CircuitCanvas.save()
                else
                    win.openSaveCircAs()
            } else if (id === "file.saveAs") {
                if (EditorPanel.focused)
                    win.openSaveFileAs()
                else
                    win.openSaveCircAs()
            } else if (id === "file.saveCircAs") {
                win.openSaveCircAs()
            } else if (id === "file.newFile") {
                EditorPanel.newFile()
                EditorPanel.focusEditor()
            } else if (id === "file.saveFile") {
                EditorPanel.save()
            } else if (id === "file.saveFileAs") {
                win.openSaveFileAs()
            } else if (id === "file.close") {
                if (EditorPanel.focused)
                    EditorPanel.closeCurrent()
                else
                    CircuitCanvas.closeCircuit()
            } else if (id === "file.closeFile") {
                EditorPanel.closeCurrent()
            } else if (id === "file.closeCirc") {
                CircuitCanvas.closeCircuit()
            } else if (id === "file.saveAll") {
                win.runSaveAll()
            } else if (id === "file.newWindow") {
                App.newWindow()
            } else if (id === "file.openFolder") {
                openFolderDialog.open()
            } else if (id === "file.examples") {
                examplesWin.open()
            } else if (id === "sim.bode") {
                bodeWin.open()
            } else if (id === "sim.dcSweep") {
                curveTracerWin.open()
            } else if (id === "file.exportSpice") {
                saveSpiceDialog.open()
            } else if (id === "file.exportKicad") {
                saveKicadDialog.open()
            } else if (id === "file.closeProject") {
                FileBrowser.closeProject()
            } else if (id === "file.settings") {
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
                EditorPanel.focusEditor()
            } else if (id.indexOf("openProj:") === 0) {
                FileBrowser.rootPath = id.substring(9)
            } else if (id.indexOf("insertSubc:") === 0) {
                var subcName = id.substring(11)
                win.insertComponent(subcName + ",Subcircuit")
            } else if (id === "help.about") {
                App.showAbout()
            } else if (id === "help.info") {
                CircuitPanel.toggleInfo()
            } else if (id === "sim.power") {
                CircuitCanvas.powerCirc()
            } else if (id === "sim.pause") {
                CircuitCanvas.pauseCirc()
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
                EditorPanel.focusEditor()
            } else if (id === "view.focusSimLog") {
                CircuitPanel.showSidePanel()
                App.sidePanelTab = 2
            } else if (id === "view.focusCompLog") {
                CircuitPanel.showSidePanel()
                App.sidePanelTab = 3
            } else if (id === "view.sidePanel") {
                CircuitPanel.togglePanel("sidePanel")
            } else if (id === "view.editorPanel") {
                CircuitPanel.togglePanel("editorPanel")
            } else if (id === "view.toggleActivePanel") {
                CircuitPanel.toggleActiveOverlay()
            } else if (id === "view.libManager") {
                App.showInstaller()

            } else if (id === "view.cmdPalette") {
                win.openCommandPalette()
            } else if (id === "view.jumpRef") {
                CommandCenter.setCircuitItems(CircuitCanvas.items)
                CommandCenter.openJumpToReference()
            } else if (id === "view.tabSwitch") {
                if (!TabSwitcher.visible) win.populateTabSwitcher()
                TabSwitcher.openSwitcher(true)
            } else if (id === "debug.toggleRepaintOverlay") {
                AppDialog.repaintOverlayEnabled = !AppDialog.repaintOverlayEnabled
            } else if (id === "debug.toggleComponentRects") {
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
            } else if (id === "edit.selectAll") {
                if (EditorPanel.focused) EditorPanel.selectAll()
                else CircuitCanvas.selectAll()
            } else if (id === "edit.find") {
                EditorPanel.findDialog()
            } else if (id === "edit.format") {
                EditorPanel.formatDocument()
            } else if (id === "edit.triggerCompletion") {
                if (EditorPanel.focused) EditorPanel.complete(true)
            } else if (id === "edit.gotoDefinition") {
                EditorPanel.gotoDefinition()
            } else if (id === "edit.rotateCw") {
                CircuitCanvas.rotateCw()
            } else if (id === "edit.rotateCcw") {
                CircuitCanvas.rotateCcw()
            } else if (id === "edit.flipH") {
                CircuitCanvas.flipH()
            } else if (id === "edit.flipV") {
                CircuitCanvas.flipV()
            } else if (id === "edit.properties") {
                CircuitCanvas.openSelectedProperties()
            } else if (id === "edit.delete") {
                CircuitCanvas.removeSelection()
            } else if (id === "sim.compile") {
                EditorPanel.compile()
            } else if (id === "sim.load") {
                EditorPanel.upload()
            } else if (id === "sim.uploadRun") {
                if (EditorPanel.documents.length === 0 && !CircuitCanvas.simRunning)
                    CircuitCanvas.editFirmware("")
                EditorPanel.uploadRun()
            } else if (id === "sim.debug") {
                CircuitPanel.showEditor()
                if (EditorPanel.documents.length === 0)
                    CircuitCanvas.editFirmware("")
                EditorPanel.debug()
            } else if (id === "sim.run") {
                if (EditorPanel.documents.length === 0 && !CircuitCanvas.simRunning)
                    CircuitCanvas.editFirmware("")
                EditorPanel.run()
            } else if (id === "sim.step") {
                EditorPanel.debugStep()
            } else if (id === "sim.stepOver") {
                EditorPanel.debugStepOver()
            } else if (id === "sim.debugPause") {
                EditorPanel.debugPause()
            } else if (id === "sim.stop") {
                EditorPanel.debugStop()
            } else if (id === "sim.reset") {
                EditorPanel.debugReset()
            } else if (id === "circ.addAllComponents") {
                CircuitCanvas.addAllComponents()
            }
        }
    }

    Connections {
        target: CircuitCanvas
        function onRequestInsertComponent(spec: string): void {
            win.insertComponent(spec)
        }
        function onRequestShowProperties() {
            var uid = CircuitCanvas.selectedUid
            if (uid && propWindowManager.windows[uid]) {
                propWindowManager.windows[uid].bringToFront()
            }
        }
        function onHistoryChanged() {
            if (!EditorPanel.focused)
                AppMenuBar.setEditState(CircuitCanvas.canUndo, CircuitCanvas.canRedo,
                                        CircuitCanvas.hasSelection, CircuitCanvas.canPaste)
        }
        function onItemsChanged() {
            if (!EditorPanel.focused)
                AppMenuBar.setEditState(CircuitCanvas.canUndo, CircuitCanvas.canRedo,
                                        CircuitCanvas.hasSelection, CircuitCanvas.canPaste)
            CommandCenter.setCircuitItems(CircuitCanvas.items)
        }
        function onSimChanged() {
            AppMenuBar.setSimState(CircuitCanvas.simRunning, CircuitCanvas.simPaused)
        }
        function onFilePathChanged() {
            App.windowTitle = CircuitCanvas.fileName
                    ? CircuitCanvas.fileName + " — Circuit Simulator"
                    : "Circuit Simulator"
            ComponentList.setCircuitPath(CircuitCanvas.filePath)
            CommandCenter.setCircuitPath(CircuitCanvas.filePath)
            AppMenuBar.setCircuitPath(CircuitCanvas.filePath)
            if (CircuitCanvas.filePath) {
                AppDialog.addRecentCircuit(CircuitCanvas.filePath)
                AppMenuBar.rebuild()
            }
            AppMenuBar.setChecked("sim.animateLogic", CircuitCanvas.animateLogic)
            AppMenuBar.setChecked("sim.animateCurr", CircuitCanvas.animateCurr)
        }
    }

    function isCircuitPath(path: string): bool {
        return App.isCircuitPath("" + path)
    }

    function openSelectedPath(url: var): void {
        var path = "" + url
        if (win.isCircuitPath(path)) {
            CircuitCanvas.loadPath(path)
            return
        }
        CircuitPanel.showEditor()
        EditorPanel.loadFile(path)
        EditorPanel.focusEditor()
        AppDialog.addRecentFile(path)
        AppMenuBar.rebuild()
    }

    FileDialog {
        id: openDialog
        title: App.translate("Open")
        fileMode: FileDialog.OpenFile
        currentFolder: App.suggestFolderUrl(FileBrowser.rootPath || CircuitCanvas.filePath || EditorPanel.currentPath)
        nameFilters: [ App.translate("Circuits (*.circ1 *.sim2 *.sim1)"), App.translate("All files (*)") ]
        onAccepted: win.openSelectedPath(openDialog.selectedFile)
    }
    FileDialog {
        id: saveCircDialog
        title: App.translate("Save Circuit")
        fileMode: FileDialog.SaveFile
        currentFolder: App.suggestFolderUrl(CircuitCanvas.filePath)
        defaultSuffix: "circ1"
        nameFilters: [ App.translate("Circuits (*.circ1)"), App.translate("All files (*)") ]
        currentFile: CircuitCanvas.filePath ? App.suggestFileUrl(CircuitCanvas.filePath) : ""
        onAccepted: {
            CircuitCanvas.savePath("" + saveCircDialog.selectedFile)
            win.circSaveThenReplace = false
            if (win.saveAllPending)
                win.continueSaveAllFiles()
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
            EditorPanel.savePath("" + saveFileDialog.selectedFile)
            AppDialog.addRecentFile("" + saveFileDialog.selectedFile)
            AppMenuBar.rebuild()
            if (win.fileSaveThenClose) {
                win.fileSaveThenClose = false
                EditorPanel.finishPendingClose()
            }
            if (win.saveAllPending)
                win.continueSaveAllFiles()
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
    FileDialog {
        id: saveSpiceDialog
        title: App.translate("Export SPICE Netlist")
        fileMode: FileDialog.SaveFile
        currentFolder: App.suggestFolderUrl(CircuitCanvas.filePath)
        defaultSuffix: "cir"
        nameFilters: [ App.translate("SPICE netlist (*.cir *.sp)"), App.translate("All files (*)") ]
        onAccepted: CircuitCanvas.saveSpice("" + selectedFile)
    }
    FileDialog {
        id: saveKicadDialog
        title: App.translate("Export KiCad Netlist")
        fileMode: FileDialog.SaveFile
        currentFolder: App.suggestFolderUrl(CircuitCanvas.filePath)
        defaultSuffix: "net"
        nameFilters: [ App.translate("KiCad netlist (*.net)"), App.translate("All files (*)") ]
        onAccepted: CircuitCanvas.saveKicad("" + selectedFile)
    }

    Component {
        id: propWindowComponent
        OwnedWindow {
            id: propWin
            property string itemUid: ""
            property string itemTitle: ""
            title: itemTitle.length > 0
                   ? App.translate("Properties: %1").arg(itemTitle)
                   : App.translate("Properties")
            sizeToContent: true
            contentMinWidth: 320
            contentMinHeight: 160
            panelSource: "PropDialog.qml"
            panelProperties: ({ "itemUid": itemUid })
            openRequested: true

            onSavedXChanged: if (propWin.hasSavedPosition) propWindowManager.recordPosition(propWin.itemUid, propWin.savedX, propWin.savedY)
            onSavedYChanged: if (propWin.hasSavedPosition) propWindowManager.recordPosition(propWin.itemUid, propWin.savedX, propWin.savedY)

            onClosing: (close) => {
                if (!win.isQuitting) {
                    close.accepted = false
                    if (propWin.hasSavedPosition) {
                        propWindowManager.recordPosition(propWin.itemUid, propWin.savedX, propWin.savedY)
                    }
                    propWin.openRequested = false
                    propWindowManager.closeDialog(propWin.itemUid)
                }
            }
        }
    }

    QtObject {
        id: propWindowManager
        property var windows: ({})
        property var lastPos: null
        property var lastPosByUid: ({})

        function recordPosition(uid: string, px: int, py: int): void {
            var pos = { x: px, y: py }
            lastPos = pos
            if (uid && uid.length > 0) {
                lastPosByUid[uid] = pos
            }
        }

        function syncOpenDialogs(): void {
            var list = CircuitCanvas.openPropDialogs || []
            var activeUids = {}
            for (var i = 0; i < list.length; i++) {
                var entry = list[i]
                var uid = entry.uid
                activeUids[uid] = true
                if (!windows[uid]) {
                    var props = {
                        itemUid: uid,
                        itemTitle: entry.title || entry.typeText || ""
                    }
                    var saved = lastPosByUid[uid] || lastPos
                    if (saved) {
                        props.hasSavedPosition = true
                        props.savedX = saved.x
                        props.savedY = saved.y
                    }
                    var w = propWindowComponent.createObject(win, props)
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
                windows = win.omitKey(windows, delId)
                if (targetWin) {
                    if (targetWin.hasSavedPosition) {
                        recordPosition(delId, targetWin.savedX, targetWin.savedY)
                    }
                    targetWin.destroy()
                }
            }
        }

        function closeDialog(uid: string): void {
            var targetWin = windows[uid]
            if (targetWin) {
                if (targetWin.hasSavedPosition) {
                    recordPosition(uid, targetWin.savedX, targetWin.savedY)
                }
                windows = win.omitKey(windows, uid)
                targetWin.destroy()
            }
            CircuitCanvas.closePropertyDialog(uid)
        }

        function bringToFront(uid: string): void {
            if (windows[uid]) {
                windows[uid].bringToFront()
            }
        }
    }

    Connections {
        target: CircuitCanvas
        function onOpenPropDialogsChanged(): void {
            propWindowManager.syncOpenDialogs()
        }
        function onRequestBringPropToFront(uid: string): void {
            propWindowManager.bringToFront(uid)
        }
    }

    Connections {
        target: CircuitPanel
        function onToolbarChanged(): void {
            AppMenuBar.setSimState(CircuitCanvas.simRunning, CircuitCanvas.simPaused)
        }
        function onAction(id: string): void {
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

    // Poll the background compiler and board discovery while running so that logs
    // and completion events drain even if the code editor tab/window is hidden.
    Timer {
        interval: 50
        running: EditorPanel.compiling || EditorPanel.boardLoading
        repeat: true
        onTriggered: EditorPanel.pollLsp()
    }

    Connections {
        target: EditorPanel
        function onRequestShowFind() { findWin.bringToFront() }
        function onRequestShowFileSettings() { fileSettingsWin.bringToFront() }
        function onRequestShowCompilerSettings() { compilerSettingsWin.bringToFront() }
        function onRequestSaveAs(): void { win.openSaveFileAs() }
        function onLogChanged() {
            var lines = EditorPanel.takeLogLines()
            if (lines && lines.length > 0) {
                CompilerLog.appendLines(lines)
            }
        }
        function onCompilingChanged() {
            var lines = EditorPanel.takeLogLines()
            if (lines && lines.length > 0) {
                CompilerLog.appendLines(lines)
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
            if (!CircuitCanvas.simRunning)
                CircuitCanvas.powerCirc()
            else if (CircuitCanvas.simPaused)
                CircuitCanvas.pauseCirc()
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

    OwnedWindow {
        id: findWin
        title: I18n.tr("Find/Replace")
        sizeToContent: true
        contentMinWidth: 460
        panelSource: "FindReplace.qml"
        openRequested: EditorPanel.findVisible

        onClosing: (close) => {
            if (!win.isQuitting) {
                close.accepted = false
                EditorPanel.closeFind()
            }
        }
    }
    OwnedWindow {
        id: fileSettingsWin
        title: I18n.tr("File Settings")
        sizeToContent: true
        contentMinWidth: 420
        panelSource: "FileSettings.qml"
        openRequested: EditorPanel.fileSettingsVisible

        onClosing: (close) => {
            if (!win.isQuitting) {
                close.accepted = false
                EditorPanel.fileSettingsVisible = false
            }
        }
    }
    OwnedWindow {
        id: compilerSettingsWin
        title: EditorPanel.hasCompiler
               ? I18n.tr("%1 Settings").arg(EditorPanel.compilerName)
               : I18n.tr("Compiler Settings")
        sizeToContent: true
        contentMinWidth: 420
        panelSource: "CompilerSettings.qml"
        openRequested: EditorPanel.compilerSettingsVisible

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
            ComponentList.setProjectDir(FileBrowser.rootPath)
            AppMenuBar.setProjectDir(FileBrowser.rootPath)
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
        Qt.callLater(titleBar.refreshChrome)
        CircuitPanel.installNativeChrome()
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
        ComponentList.setProjectDir(FileBrowser.rootPath)
        AppMenuBar.setProjectDir(FileBrowser.rootPath)
        ComponentList.setCircuitPath(CircuitCanvas.filePath)
        CommandCenter.setCircuitPath(CircuitCanvas.filePath)
        AppMenuBar.setCircuitPath(CircuitCanvas.filePath)
        CommandCenter.setCircuitItems(CircuitCanvas.items)
        CommandCenter.editorFocused = EditorPanel.focused
        var simLine
        while ((simLine = CircuitCanvas.takeSimLogLine()) !== "")
            SimulatorLog.appendLine(simLine)
        var compLines = EditorPanel.takeLogLines()
        if (compLines && compLines.length > 0) {
            CompilerLog.appendLines(compLines)
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

    function joinFolderFile(folderUrl: var, fileName: string): string {
        var folder = ("" + folderUrl).replace(/\/$/, "")
        return folder + "/" + fileName
    }

    function openSaveCircAs(): void {
        var folder = App.suggestFolderUrl(CircuitCanvas.filePath)
        var url = CircuitCanvas.filePath
            ? App.suggestFileUrl(CircuitCanvas.filePath)
            : win.joinFolderFile(folder, "untitled.circ1")
        saveCircDialog.currentFolder = folder
        saveCircDialog.selectedFile = url
        saveCircDialog.currentFile = url
        saveCircDialog.open()
    }

    function openSaveFileAs(): void {
        var path = EditorPanel.currentPath
        var folder = App.suggestFolderUrl(path)
        var url = path
            ? App.suggestFileUrl(path)
            : win.joinFolderFile(folder, EditorPanel.currentTitle || "untitled")
        saveFileDialog.currentFolder = folder
        saveFileDialog.selectedFile = url
        saveFileDialog.currentFile = url
        saveFileDialog.open()
    }

    Shortcut {
        enabled: !CommandCenter.recordingShortcut
        sequences: ["Ctrl+P", "Meta+P"]
        onActivated: {
            CommandCenter.setCircuitItems(CircuitCanvas.items)
            CommandCenter.openJumpToReference()
        }
    }
    Shortcut {
        enabled: !CommandCenter.recordingShortcut
        sequences: ["Ctrl+Shift+P", "Meta+Shift+P"]
        onActivated: win.openCommandPalette()
    }

    // QML Shortcut registrations sourced from AppMenuBar.shortcuts (already
    // filtered by platform: on macOS, NSMenu-owned chords are omitted while
    // non-menu and bare-key actions remain registered here).
    Instantiator {
        id: menuShortcutInstantiator
        model: AppMenuBar.shortcuts
        delegate: Shortcut {
            id: menuShortcutItem
            // Stored in canonical Qt notation ("Ctrl+Shift+S").
            required property var modelData
            property string actionId: menuShortcutItem.modelData.id || ""
            sequences: {
                var s = menuShortcutItem.modelData.shortcut || ""
                if (s === "Ctrl++" || s === "Ctrl+=")
                    return ["Ctrl++", "Ctrl+="]
                if (s === "Return" || s.endsWith("+Return"))
                    return [s, s.slice(0, -6) + "Enter"]
                return s ? [s] : []
            }
            enabled: !!menuShortcutItem.modelData.enabled && !CommandCenter.recordingShortcut
            onActivated: {
                if (menuShortcutItem.actionId !== "")
                    AppMenuBar.triggerAction(menuShortcutItem.actionId)
            }
        }
    }

    CommandCenterDialog {
        id: commandCenterDialog
    }

    function insertComponent(spec: string): void {
        if (!spec)
            return
        var comma = spec.indexOf(",")
        var name = comma >= 0 ? spec.substring(0, comma).trim() : spec.trim()
        var compType = comma >= 0 ? spec.substring(comma + 1).trim() : spec.trim()
        ComponentList.addRecent(spec)
        if (compType === "MCU" && name === "MCU") {
            App.showMcu()
            return
        }
        CircuitCanvas.addComponent(spec)
        if (compType === "Oscope" || compType === "Oscilloscope") {
            App.showOsc()
        } else if (compType === "LogicAnalyzer" || compType === "LAnalizer") {
            App.showLa()
        } else if (compType === "QemuDevice") {
            App.showMcu()
        } else if (compType === "SerialPort") {
            App.showSerialMon()
        } else if (compType === "SerialTerm") {
            App.showTerminal()
        }
    }

    Connections {
        target: CommandCenter
        function onShortcutsUpdated(): void {
            AppMenuBar.rebuild()
            ComponentList.refreshShortcuts()
        }
        function onExecuteItem(typ: string, data: string, compType: string): void {
            if (typ === "action") {
                AppMenuBar.triggerAction(data)
            } else if (typ === "component") {
                var spec = (data === compType || !compType) ? data : (data + "," + compType)
                win.insertComponent(spec)
            } else if (typ === "jump_comp") {
                CircuitCanvas.focusComponent(data)
            } else if (typ === "jump_file") {
                if (win.isCircuitPath(data)) {
                    CircuitCanvas.loadPath(data)
                } else {
                    CircuitPanel.showEditor()
                    EditorPanel.loadFile(data)
                    EditorPanel.focusEditor()
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
        enabled: !CommandCenter.recordingShortcut
        sequences: ["Ctrl+Tab", "Meta+Tab"]
        onActivated: {
            if (!TabSwitcher.visible) win.populateTabSwitcher()
            TabSwitcher.openSwitcher(true)
        }
    }
    Shortcut {
        enabled: !CommandCenter.recordingShortcut
        sequences: ["Ctrl+Shift+Tab", "Meta+Shift+Tab"]
        onActivated: {
            if (!TabSwitcher.visible) win.populateTabSwitcher()
            TabSwitcher.openSwitcher(false)
        }
    }

    TabSwitcherDialog {
        id: tabSwitcherDialog
    }

    Connections {
        target: TabSwitcher
        function onExecuteItem(kind: string, id: string, index: int): void {
            if (kind === "circuit") {
                if (id && id.length > 0 && id !== CircuitCanvas.filePath) {
                    CircuitCanvas.loadPath(id)
                }
            } else if (kind === "editor") {
                CircuitPanel.showEditor()
                if (index >= 0) {
                    EditorPanel.currentDocument = index
                } else if (id && id.length > 0) {
                    EditorPanel.loadFile(id)
                }
                EditorPanel.focusEditor()
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
        function onRequestSaveBeforeReplace(): void { circuitReplaceDialog.open() }
        function onRequestSaveAsThenReplace(): void {
            win.circSaveThenReplace = true
            win.openSaveCircAs()
        }
    }

    Connections {
        target: EditorPanel
        function onRequestSaveBeforeClose(): void { fileCloseDialog.open() }
        function onRequestSaveAsThenClose(): void {
            win.fileSaveThenClose = true
            win.openSaveFileAs()
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
        CircuitCanvas.powerOff()
        if (!App.requestClose()) {
            close.accepted = false
            return
        }
        win.isQuitting = true
        closeOwnedWindows()
        Qt.quit()
    }
}
