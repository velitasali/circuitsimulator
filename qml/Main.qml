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
        source: visible ? "about.qml" : ""
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

    OwnedWindow {
        id: examplesWin
        title: App.translate("Example circuits")
        sizeToContent: true
        source: visible ? "examples.qml" : ""
        visible: false

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
                visible = false
            }
        }

        Connections {
            target: examplesWin.panel
            ignoreUnknownSignals: true
            function onExampleChosen(id: string): void {
                CircuitCanvas.loadExample(id)
                examplesWin.visible = false
            }
        }
    }

    OwnedWindow {
        id: bodeWin
        title: I18n.tr("AC Sweep")
        width: 820
        height: 560
        source: visible ? "BodeView.qml" : ""
        visible: false

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
                visible = false
            }
        }
    }

    OwnedWindow {
        id: curveTracerWin
        title: I18n.tr("DC Parameter Sweep / Curve Tracer")
        width: 920
        height: 620
        source: visible ? "CurveTracerView.qml" : ""
        visible: false

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
                visible = false
            }
        }
    }

    OwnedWindow {
        id: oscWin
        title: I18n.tr("Oscilloscope")
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

    OwnedWindow {
        id: laWin
        title: I18n.tr("Logic Analyzer")
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

    OwnedWindow {
        id: mcuWin
        title: I18n.tr("MCU Monitor")
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

    property var serialMonWindows: ({})

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
        var count = Object.keys(serialMonWindows).length
        var offset = (count % 8) * 28
        var posX = win.x + Math.round((win.width - 560) / 2) + offset
        var posY = win.y + Math.round((win.height - 420) / 2) + offset
        var winTitle = title ? title : SerialMonitor.titleFor(monId)
        var w = serialMonComp.createObject(win, {
            monId: monId,
            title: winTitle,
            x: posX,
            y: posY
        }) as AppWindow
        if (w) {
            serialMonWindows[monId] = w
        }
    }

    function dismissSerialMonitor(monId: string): void {
        if (!monId)
            return
        var w = serialMonWindows[monId] as AppWindow
        if (w) {
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
            visible: true

            Component.onCompleted: {
                monWin.loadSource("qrc:/qt/qml/cs_app/serialmon.qml", { monitorId: monWin.monId })
            }

            onClosing: (close) => {
                close.accepted = true
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

    OwnedWindow {
        id: memTableWin
        title: MemoryTable.title || I18n.tr("Memory Table")
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

    OwnedWindow {
        id: settingsWin
        title: I18n.tr("Settings")
        sizeToContent: true
        minimumWidth: 420
        minimumHeight: 380
        source: visible ? "appdialog.qml" : ""
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

    OwnedWindow {
        id: circSettingsWin
        title: I18n.tr("Circuit Settings")
        maximizable: false
        width: Math.max(420, minimumWidth)
        height: 560
        minimumWidth: panelItem ? Math.max(360, Math.ceil(panelItem.implicitWidth)) : 360
        minimumHeight: 380
        source: visible ? "circuitdialog.qml" : ""
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

    OwnedWindow {
        id: installerWin
        title: I18n.tr("Library Manager")
        maximizable: false
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
            } else if (id === "file.open" || id === "file.openCirc" || id === "file.openFile") {
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
                examplesWin.visible = true
            } else if (id === "sim.bode") {
                bodeWin.visible = true
            } else if (id === "sim.dcSweep") {
                curveTracerWin.visible = true
            } else if (id === "file.exportSpice") {
                saveSpiceDialog.open()
            } else if (id === "file.exportKicad") {
                saveKicadDialog.open()
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
                EditorPanel.focusEditor()
            } else if (id.indexOf("openProj:") === 0) {
                FileBrowser.rootPath = id.substring(9)
            } else if (id === "help.about") {
                App.showAbout()
            } else if (id === "help.info") {
                CircuitPanel.toggleInfo()
            } else if (id === "sim.power" || id === "circ.power") {
                CircuitCanvas.powerCirc()
            } else if (id === "sim.pause" || id === "circ.pause") {
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
                win.openCommandPalette()
            } else if (id === "view.jumpRef") {
                CommandCenter.setCircuitItems(CircuitCanvas.items)
                CommandCenter.openJumpToReference()
            } else if (id === "view.tabSwitch") {
                if (!TabSwitcher.visible) win.populateTabSwitcher()
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
            } else if (id === "edit.delete") {
                CircuitCanvas.removeSelection()
            } else if (id === "sim.compile" || id === "sim_compile") {
                EditorPanel.compile()
            } else if (id === "sim.load" || id === "sim_load") {
                EditorPanel.upload()
            } else if (id === "sim.uploadRun" || id === "sim.upload_run") {
                if (EditorPanel.documents.length === 0 && !CircuitCanvas.simRunning)
                    CircuitCanvas.editFirmware("")
                EditorPanel.uploadRun()
            } else if (id === "sim.debug" || id === "sim_debug") {
                CircuitPanel.showEditor()
                if (EditorPanel.documents.length === 0)
                    CircuitCanvas.editFirmware("")
                EditorPanel.debug()
            } else if (id === "sim.run" || id === "sim_run") {
                if (EditorPanel.documents.length === 0 && !CircuitCanvas.simRunning)
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
        function onSimChanged() {
            AppMenuBar.setSimState(CircuitCanvas.simRunning, CircuitCanvas.simPaused)
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

    function isCircuitPath(path: string): bool {
        var p = ("" + path).toLowerCase()
        return p.endsWith(".circ1") || p.endsWith(".sim1") || p.endsWith(".sim2")
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
        nameFilters: [ App.translate("Circuits (*.circ1 *.sim2 *.sim1)"), App.translate("All files (*)") ]
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
            minimumWidth: 320
            minimumHeight: 160
            visible: true

            Component.onCompleted: {
                loadSource("PropDialog.qml", { "itemUid": itemUid })
                raise()
                requestActivate()
            }

            onClosing: (close) => {
                if (!win.isQuitting) {
                    close.accepted = false
                    propWin.visible = false
                    propWindowManager.closeDialog(propWin.itemUid)
                }
            }
        }
    }

    QtObject {
        id: propWindowManager
        property var windows: ({})

        function syncOpenDialogs(): void {
            var list = CircuitCanvas.openPropDialogs || []
            var activeUids = {}
            for (var i = 0; i < list.length; i++) {
                var entry = list[i]
                var uid = entry.uid
                activeUids[uid] = true
                if (!windows[uid]) {
                    var w = propWindowComponent.createObject(win, {
                        itemUid: uid,
                        itemTitle: entry.title || entry.typeText || "",
                        x: win.x + Math.round((win.width - 320) / 2),
                        y: win.y + Math.round((win.height - 160) / 2)
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
                windows = win.omitKey(windows, delId)
                if (targetWin) {
                    targetWin.destroy()
                }
            }
        }

        function closeDialog(uid: string): void {
            var targetWin = windows[uid]
            if (targetWin) {
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
        minimumWidth: 460
        source: visible ? "FindReplace.qml" : ""
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
    OwnedWindow {
        id: fileSettingsWin
        title: I18n.tr("File Settings")
        sizeToContent: true
        minimumWidth: 420
        source: visible ? "FileSettings.qml" : ""
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
    OwnedWindow {
        id: compilerSettingsWin
        title: EditorPanel.hasCompiler
               ? I18n.tr("%1 Settings").arg(EditorPanel.compilerName)
               : I18n.tr("Compiler Settings")
        sizeToContent: true
        minimumWidth: 420
        source: visible ? "CompilerSettings.qml" : ""
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
        sequences: ["Ctrl+P", "Meta+P"]
        onActivated: {
            CommandCenter.setCircuitItems(CircuitCanvas.items)
            CommandCenter.openJumpToReference()
        }
    }
    Shortcut {
        sequences: ["Ctrl+Shift+P", "Meta+Shift+P"]
        onActivated: win.openCommandPalette()
    }

    // Every menu row that carries a shortcut gets a real Qt Shortcut here, from
    // the same AppMenuBar list the menu paints its key labels from — so a
    // binding reassigned in the Command Center starts working immediately.
    // macOS is excluded: its NSMenu already owns the key equivalents, and a
    // QML Shortcut on top would fire the action a second time.
    Instantiator {
        id: menuShortcutInstantiator
        model: App.nativeMenus ? [] : AppMenuBar.shortcuts
        delegate: Shortcut {
            id: menuShortcutItem
            // Reassigned keys are stored in Qt notation already ("Ctrl+Shift+S").
            required property var modelData
            property string actionId: menuShortcutItem.modelData.id || ""
            sequences: {
                var s = menuShortcutItem.modelData.shortcut || ""
                if (s === "Ctrl++" || s === "Ctrl+=")
                    return ["Ctrl++", "Ctrl+="]
                return s ? [s] : []
            }
            enabled: !!menuShortcutItem.modelData.enabled
            onActivated: {
                if (menuShortcutItem.actionId !== "")
                    AppMenuBar.triggerAction(menuShortcutItem.actionId)
            }
        }
    }

    CommandCenterDialog {
        id: commandCenterDialog
    }

    Connections {
        target: CommandCenter
        function onShortcutsUpdated(): void {
            AppMenuBar.rebuild()
        }
        function onExecuteItem(typ: string, data: string, compType: string): void {
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
        sequences: ["Ctrl+Tab", "Meta+Tab"]
        onActivated: {
            if (!TabSwitcher.visible) win.populateTabSwitcher()
            TabSwitcher.openSwitcher(true)
        }
    }
    Shortcut {
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
