pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls
import QtQuick.Dialogs
import QtQuick.Window
import cs_app
import cs_ui

/* Pan/zoom/select canvas. The picture is the Vello frame in
 * CircuitCanvasItem, so overlays in this file paint above it. The grid
 * shader underneath shows until that frame arrives. Input goes to
 * CircuitCanvas. Hosted by CircuitPanelView.qml. */
Item {
    id: root
    focus: true
    clip: true

    readonly property bool dark: App.darkTheme

    function rgba(hex: string): vector4d {
        var h = ("" + hex).replace("#", "")
        var r = 0, g = 0, b = 0, a = 1
        if (h.length === 6) {
            r = parseInt(h.substr(0, 2), 16) / 255
            g = parseInt(h.substr(2, 2), 16) / 255
            b = parseInt(h.substr(4, 2), 16) / 255
        } else if (h.length >= 8) {
            a = parseInt(h.substr(0, 2), 16) / 255
            r = parseInt(h.substr(2, 2), 16) / 255
            g = parseInt(h.substr(4, 2), 16) / 255
            b = parseInt(h.substr(6, 2), 16) / 255
        }
        return Qt.vector4d(r, g, b, a)
    }

    function syncSize(): void {
        CircuitCanvas.setDpr(Screen.devicePixelRatio)
        CircuitCanvas.setViewSize(width, height)
    }

    Component.onCompleted: {
        CircuitCanvas.dark = dark
        syncSize()
        CircuitCanvas.render()
    }
    onWidthChanged: syncSize()
    onHeightChanged: syncSize()
    onDarkChanged: {
        CircuitCanvas.dark = dark
        CircuitCanvas.render()
    }

    ShaderEffect {
        anchors.fill: parent
        fragmentShader: "qrc:/qt/qml/cs_app/grid.frag.qsb"

        property vector2d itemSize: Qt.vector2d(width, height)
        property vector2d center: Qt.vector2d(CircuitCanvas.centerX, CircuitCanvas.centerY)
        property vector4d sceneRect: Qt.vector4d(CircuitCanvas.sceneRect.x, CircuitCanvas.sceneRect.y,
                                                 CircuitCanvas.sceneRect.w, CircuitCanvas.sceneRect.h)
        property vector4d gridColor: root.rgba(CircuitCanvas.gridColor)
        property vector4d canvasColor: root.rgba(CircuitCanvas.canvasColor)
        property vector4d viewportColor: root.rgba(CircuitCanvas.viewportColor)
        property real zoom: CircuitCanvas.zoom
        property real dpr: Screen.devicePixelRatio
        property real showGrid: CircuitCanvas.showGrid ? 1.0 : 0.0
    }

    CircuitCanvasItem {
        id: nativeCanvas
        anchors.fill: parent
        onHostReady: CircuitCanvas.render()
        onFrameReady: CircuitCanvas.flushDirty()
    }

    MouseArea {
        id: mouse
        anchors.fill: parent
        hoverEnabled: true
        acceptedButtons: Qt.AllButtons
        cursorShape: {
            switch (CircuitCanvas.cursor) {
            case "openhand":     return Qt.OpenHandCursor
            case "closedhand":   return Qt.ClosedHandCursor
            case "cross":        return Qt.CrossCursor
            case "splith":       return Qt.SplitHCursor
            case "splitv":       return Qt.SplitVCursor
            case "sizeall":      return Qt.SizeAllCursor
            case "pointinghand": return Qt.PointingHandCursor
            default:             return Qt.ArrowCursor
            }
        }
        preventStealing: true

        onPressed: (ev) => {
            root.forceActiveFocus()
            const wasLinking = CircuitCanvas.linkingFrom.length > 0
            CircuitCanvas.mousePress(ev.button, ev.x, ev.y, ev.modifiers)
            if (wasLinking)
                return
            if (ev.button === Qt.RightButton) {
                if (CircuitCanvas.hasPinHit)
                    pinMenu.popup()
                else if (CircuitCanvas.hasLabelHit)
                    labelMenu.popup()
                else if (CircuitCanvas.hasItemSelection)
                    itemMenu.popup()
                else if (CircuitCanvas.hasWireSelection)
                    wireMenu.popup()
                else
                    canvasMenu.popup()
            }
        }
        onEntered: CircuitCanvas.mouseMove(mouse.mouseX, mouse.mouseY, 0, 0)
        onPositionChanged: (ev) => CircuitCanvas.mouseMove(ev.x, ev.y, ev.buttons, ev.modifiers)
        onReleased: (ev) => CircuitCanvas.mouseRelease(ev.button, ev.x, ev.y, ev.modifiers)
        onCanceled: CircuitCanvas.mouseCancel()
        onExited: CircuitCanvas.mouseCancel()
        onDoubleClicked: (ev) => CircuitCanvas.mouseDoubleClick(ev.button, ev.x, ev.y, ev.modifiers)
    }

    Connections {
        target: CircuitCanvas
        function onRequestShowScope() {
            App.showOsc()
        }
        function onRequestShowLa() {
            App.showLa()
        }
        function onRequestShowTerminal() {
            App.showTerminal()
        }
        function onRequestOpenSerialMon(monId: string, title: string): void {
            SerialMonitor.openMonitor(monId, title)
        }
        function onRequestOpenFile(path: string): void {
            EditorPanel.loadFile(path)
            EditorPanel.focusEditor()
            AppDialog.addRecentFile(path)
            AppMenuBar.rebuild()
            CircuitPanel.showEditor()
        }
        function onRequestShowEditor(): void {
            CircuitPanel.showEditor()
            EditorPanel.focusEditor()
        }
        function onRequestPauseSim(): void {
            if (CircuitCanvas.simRunning && !CircuitCanvas.simPaused)
                CircuitCanvas.pauseCirc()
        }
        function onRequestUploadRun(debug: bool): void {
            if (debug)
                EditorPanel.debug()
            else
                EditorPanel.uploadRun()
        }
    }

    AppContextMenu {
        id: pinMenu
        readonly property string pinId: CircuitCanvas.hitPinId
        readonly property bool isInverted: CircuitCanvas.hitPinInverted
        readonly property bool isPackage: CircuitCanvas.hitPinIsPackage
        readonly property string itemUid: CircuitCanvas.hitPinItemUid
        readonly property string localPinId: CircuitCanvas.hitPinLocalId

        ContextMenuItem {
            text: I18n.tr("Invert Pin")
            iconLigature: "invert_colors"
            visible: !pinMenu.isPackage
            checkable: true
            checked: pinMenu.isInverted
            onTriggered: {
                if (pinMenu.pinId) {
                    CircuitCanvas.togglePinInverted(pinMenu.pinId)
                }
            }
        }
        ContextMenuItem {
            text: I18n.tr("Edit Pin")
            iconLigature: "edit"
            visible: pinMenu.isPackage
            onTriggered: {
                editPinWin.openForPin(pinMenu.itemUid, pinMenu.localPinId)
            }
        }
        ContextMenuItem {
            text: I18n.tr("Delete Pin")
            iconLigature: "delete"
            visible: pinMenu.isPackage
            onTriggered: {
                if (pinMenu.itemUid && pinMenu.localPinId)
                    CircuitCanvas.removePackagePin(pinMenu.itemUid, pinMenu.localPinId)
            }
        }
    }

    AppContextMenu {
        id: wireMenu
        ContextMenuItem {
            text: I18n.tr("Remove")
            shortcutText: UiUtils.shortcutFor("edit.delete")
            iconLigature: "delete"
            onTriggered: CircuitCanvas.removeSelection()
        }
    }

    AppContextMenu {
        id: labelMenu
        readonly property string uid: CircuitCanvas.hitLabelUid
        readonly property bool isVal: CircuitCanvas.hitLabelIsVal

        ContextMenuItem {
            text: I18n.tr("Rotate CW")
            shortcutText: UiUtils.shortcutFor("edit.rotateCw")
            iconLigature: "rotate_right"
            onTriggered: {
                if (labelMenu.isVal)
                    CircuitCanvas.rotateItemValLabel(labelMenu.uid, 90)
                else
                    CircuitCanvas.rotateItemLabel(labelMenu.uid, 90)
            }
        }
        ContextMenuItem {
            text: I18n.tr("Rotate CCW")
            shortcutText: UiUtils.shortcutFor("edit.rotateCcw")
            iconLigature: "rotate_left"
            onTriggered: {
                if (labelMenu.isVal)
                    CircuitCanvas.rotateItemValLabel(labelMenu.uid, -90)
                else
                    CircuitCanvas.rotateItemLabel(labelMenu.uid, -90)
            }
        }
        ContextMenuItem {
            text: I18n.tr("Rotate 180°")
            iconLigature: "sync_alt"
            onTriggered: {
                if (labelMenu.isVal)
                    CircuitCanvas.rotateItemValLabel(labelMenu.uid, 180)
                else
                    CircuitCanvas.rotateItemLabel(labelMenu.uid, 180)
            }
        }
    }

    AppContextMenu {
        id: itemMenu

        readonly property string itemType: CircuitCanvas.selectedItemType || CircuitCanvas.propItemType
        readonly property string itemUid: CircuitCanvas.selectedItemUid || CircuitCanvas.selectedUid
        readonly property bool isWaveGen: itemType === "WaveGen"
        readonly property bool isOsc: itemType === "Oscilloscope" || itemType === "Oscope"
        readonly property bool isLa: itemType === "LAnalizer" || itemType === "LogicAnalyzer"
        readonly property bool isMcuChip: itemType === "MCU" || itemType === "Mcu"
        readonly property bool isQemu: itemType === "QemuDevice"
        readonly property bool isMcu: isMcuChip || isQemu
        readonly property bool isSerial: itemType === "SerialPort" || itemType === "SerialTerm" || itemType === "Esp01"
        readonly property bool isSubPackage: itemType === "SubPackage"
        readonly property bool isSubc: itemType === "Subcircuit" || itemType === "Board"
        readonly property string nestedUid: CircuitCanvas.selectedNestedMcuUid
        readonly property bool hasNestedMcu: isSubc && nestedUid.length > 0
        readonly property string firmwareUid: hasNestedMcu ? nestedUid : itemUid
        readonly property bool showMcuActions: isMcu || hasNestedMcu
        readonly property bool isMemory: itemType === "Memory" || itemType === "DynamicMemory" || itemType === "I2CRam"
        readonly property bool isFunction: itemType === "Function"
        readonly property bool isImage: itemType === "Image"
        readonly property bool isSdCard: itemType === "SdCard"
        readonly property bool isTunnel: itemType === "Tunnel"
        readonly property bool isProbe: itemType === "Probe"
        readonly property bool isTestUnit: itemType === "TestUnit"
        readonly property bool isDcMotor: itemType === "DcMotor"
        readonly property bool isStepper: itemType === "Stepper"
        readonly property bool isLed: itemType === "Led" || itemType === "LED"
        readonly property bool isDial: itemType === "Dial"
        readonly property bool isLinker: isLed || isDial || isDcMotor || isStepper || isMcuChip
        readonly property bool hasMonitors: (CircuitCanvas.selectedItemMonitors && CircuitCanvas.selectedItemMonitors.length > 0)
        readonly property bool hasFlash: CircuitCanvas.selectedHasFlash
        readonly property bool hasEeprom: CircuitCanvas.selectedHasEeprom
        readonly property bool isActive: CircuitCanvas.selectedItemActive
        readonly property bool tunnelVisible: CircuitCanvas.selectedTunnelVisible
        readonly property bool tunnelHasName: CircuitCanvas.selectedTunnelName.length > 0
        readonly property bool probePause: CircuitCanvas.selectedProbePause
        readonly property bool hasSpecial: isWaveGen || isOsc || isLa || isMcu || isSerial || isSubPackage || isSubc
            || isMemory || isFunction || isImage || isSdCard || isTunnel || isProbe || isTestUnit || isDcMotor || isLinker

        function syncSubMenuRow(sub: AppContextMenu, shown: bool): void {
            for (var i = 0; i < count; i++) {
                const it = itemAt(i) as ContextMenuItem
                if (it && it.subMenu === sub) {
                    it.visible = shown
                    it.enabled = shown
                    if (shown && sub.iconLigature)
                        it.iconLigature = sub.iconLigature
                    return
                }
            }
        }
        onAboutToShow: {
            // Hide the parent row; never bind Menu.visible (that opens the popup).
            syncSubMenuRow(monitorMenu, itemMenu.hasMonitors && itemMenu.showMcuActions && monitorRepeater.count > 0)
            syncSubMenuRow(footprintsMenu, itemMenu.isSubPackage)
        }

        Connections {
            target: CircuitCanvas
            function onHasItemSelectionChanged() {
                if (!CircuitCanvas.hasItemSelection && itemMenu.visible) {
                    itemMenu.close()
                }
            }
        }

        // C++ Chip::addActivateAction — checkable, tick when active, disabled once active.
        ContextMenuItem {
            text: I18n.tr("Activate")
            visible: itemMenu.showMcuActions
            checkable: true
            checked: itemMenu.isActive
            enabled: !itemMenu.isActive
            onTriggered: {
                if (itemMenu.firmwareUid)
                    CircuitCanvas.activeDeviceId = itemMenu.firmwareUid
            }
        }

        // C++ QemuDevice: Upload and Run / Upload and Debug
        ContextMenuItem {
            text: I18n.tr("Upload and Run")
            iconLigature: "play_arrow"
            visible: itemMenu.isQemu
            onTriggered: if (itemMenu.itemUid) CircuitCanvas.uploadAndRun(itemMenu.itemUid, false)
        }
        ContextMenuItem {
            text: I18n.tr("Upload and Debug")
            iconLigature: "bug_report"
            visible: itemMenu.isQemu
            onTriggered: if (itemMenu.itemUid) CircuitCanvas.uploadAndRun(itemMenu.itemUid, true)
        }

        ContextMenuItem {
            id: editFirmwareItem
            text: I18n.tr("Edit firmware")
            iconLigature: "edit"
            visible: itemMenu.showMcuActions
            onTriggered: CircuitCanvas.editFirmware(itemMenu.firmwareUid || "")
        }

        ContextMenuItem {
            text: I18n.tr("Link to Component")
            iconLigature: "link"
            visible: itemMenu.isLinker
            onTriggered: if (itemMenu.itemUid) CircuitCanvas.startLinking(itemMenu.itemUid)
        }

        ContextMenuSeparator { visible: itemMenu.isQemu }

        ContextMenuItem {
            text: I18n.tr("Load firmware")
            iconLigature: "file_download"
            visible: itemMenu.showMcuActions && (itemMenu.hasFlash || itemMenu.isQemu)
            onTriggered: loadFirmwareDialog.open()
        }
        ContextMenuItem {
            id: reloadFirmwareItem
            text: I18n.tr("Reload firmware")
            iconLigature: "refresh"
            visible: itemMenu.showMcuActions && (itemMenu.hasFlash || itemMenu.isQemu)
            onTriggered: CircuitCanvas.reloadFirmware(itemMenu.firmwareUid)
        }

        ContextMenuSeparator { visible: (itemMenu.isMcuChip || itemMenu.hasNestedMcu) && itemMenu.hasFlash }

        ContextMenuItem {
            text: I18n.tr("Load EEPROM data from file")
            iconLigature: "folder_open"
            visible: (itemMenu.isMcuChip || itemMenu.hasNestedMcu) && itemMenu.hasEeprom
            onTriggered: loadEepromDialog.open()
        }
        ContextMenuItem {
            text: I18n.tr("Save EEPROM data to file")
            iconLigature: "save"
            visible: (itemMenu.isMcuChip || itemMenu.hasNestedMcu) && itemMenu.hasEeprom
            onTriggered: saveEepromDialog.open()
        }

        ContextMenuSeparator { visible: (itemMenu.isMcuChip || itemMenu.hasNestedMcu) && itemMenu.hasEeprom }

        ContextMenuItem {
            id: openMcuMonitorItem
            text: I18n.tr("Open Mcu Monitor.")
            iconLigature: "terminal"
            visible: itemMenu.showMcuActions
            onTriggered: {
                if (itemMenu.firmwareUid)
                    CircuitCanvas.activeDeviceId = itemMenu.firmwareUid
                App.showMcu()
            }
        }

        ContextMenuItem {
            id: directSerialMonItem
            text: itemMenu.isQemu ? I18n.tr("Open Serial Monitor.") : I18n.tr("Open Monitor.")
            iconLigature: "terminal"
            visible: itemMenu.showMcuActions && itemMenu.hasMonitors
            onTriggered: {
                var uid = itemMenu.firmwareUid || itemMenu.itemUid
                var list = CircuitCanvas.selectedItemMonitors || []
                var monName = list.length > 0 ? list[0] : (itemMenu.isQemu ? "USART1" : "USART")
                var title = uid ? (uid + "-" + monName) : monName
                var monId = uid ? (uid + ":" + monName) : monName
                SerialMonitor.openMonitor(monId, title)
            }
        }

        AppContextMenu {
            id: monitorMenu
            title: I18n.tr("Other Monitors")
            iconLigature: "terminal"
            Instantiator {
                id: monitorRepeater
                model: (CircuitCanvas.selectedItemMonitors && CircuitCanvas.selectedItemMonitors.length > 1)
                       ? CircuitCanvas.selectedItemMonitors.slice(1)
                       : []
                delegate: ContextMenuItem {
                    id: monCtxItem
                    required property var modelData
                    text: "" + modelData
                    onTriggered: {
                        var uid = itemMenu.firmwareUid || itemMenu.itemUid
                        var title = uid ? (uid + "-" + monCtxItem.text) : monCtxItem.text
                        var monId = uid ? (uid + ":" + monCtxItem.text) : monCtxItem.text
                        SerialMonitor.openMonitor(monId, title)
                    }
                }
                onObjectAdded: (index, object) => {
                    const item = object as ContextMenuItem
                    if (item) monitorMenu.insertItem(index, item)
                    itemMenu.syncSubMenuRow(monitorMenu, itemMenu.hasMonitors && itemMenu.showMcuActions && monitorRepeater.count > 0)
                }
                onObjectRemoved: (_, object) => {
                    const item = object as ContextMenuItem
                    if (item) monitorMenu.removeItem(item)
                    itemMenu.syncSubMenuRow(monitorMenu, itemMenu.hasMonitors && itemMenu.showMcuActions && monitorRepeater.count > 0)
                }
            }
        }

        ContextMenuItem {
            text: I18n.tr("Open Subcircuit")
            iconLigature: "upload"
            visible: itemMenu.isSubc
            onTriggered: if (itemMenu.itemUid) CircuitCanvas.openSubcircuit(itemMenu.itemUid)
        }

        ContextMenuItem {
            text: I18n.tr("Load Audio File")
            iconLigature: "file_download"
            visible: itemMenu.isWaveGen
            onTriggered: loadWavDialog.open()
        }
        ContextMenuItem {
            text: I18n.tr("Open Oscilloscope Window")
            iconLigature: "monitoring"
            visible: itemMenu.isOsc
            onTriggered: App.showOsc()
        }
        ContextMenuItem {
            text: I18n.tr("Open Logic Analyzer Window")
            iconLigature: "monitoring"
            visible: itemMenu.isLa
            onTriggered: App.showLa()
        }
        ContextMenuItem {
            text: I18n.tr("Open Serial Monitor.")
            iconLigature: "terminal"
            visible: itemMenu.isSerial
            onTriggered: {
                var uid = itemMenu.itemUid
                var title = uid ? (uid + "-Serial") : "Serial Monitor"
                var monId = uid ? (uid + ":Serial") : "Serial"
                SerialMonitor.openMonitor(monId, title)
            }
        }

        ContextMenuItem {
            text: I18n.tr("Load data")
            iconLigature: "file_download"
            visible: itemMenu.isMemory
            onTriggered: loadDataDialog.open()
        }
        ContextMenuItem {
            text: I18n.tr("Save data")
            iconLigature: "save"
            visible: itemMenu.isMemory
            onTriggered: saveDataDialog.open()
        }
        ContextMenuItem {
            text: I18n.tr("Show Memory Table")
            iconLigature: "table_chart"
            visible: itemMenu.isMemory
            onTriggered: {
                var info = CircuitCanvas.memoryItemInfo(itemMenu.itemUid)
                if (info) {
                    MemoryTable.openForItem(info.id, info.title, info.isRom, info.cellBytes, info.data)
                }
            }
        }

        ContextMenuItem {
            text: I18n.tr("Save Functions")
            iconLigature: "save"
            visible: itemMenu.isFunction
            onTriggered: saveFunctionsDialog.open()
        }
        ContextMenuItem {
            text: I18n.tr("Load Functions")
            iconLigature: "folder_open"
            visible: itemMenu.isFunction
            onTriggered: loadFunctionsDialog.open()
        }

        ContextMenuItem {
            text: I18n.tr("Load Image")
            iconLigature: "file_download"
            visible: itemMenu.isImage
            onTriggered: loadImageDialog.open()
        }
        ContextMenuItem {
            text: I18n.tr("Save Image")
            iconLigature: "save"
            visible: itemMenu.isImage && CircuitCanvas.selectedHasImageData
            onTriggered: saveImageItemDialog.open()
        }

        ContextMenuItem {
            text: I18n.tr("Load .img File")
            iconLigature: "file_download"
            visible: itemMenu.isSdCard
            onTriggered: loadSdDialog.open()
        }
        ContextMenuItem {
            text: I18n.tr("Eject SD card")
            iconLigature: "link_off"
            visible: itemMenu.isSdCard && CircuitCanvas.selectedHasSdImage
            onTriggered: CircuitCanvas.ejectSdCard(itemMenu.itemUid)
        }

        ContextMenuItem {
            text: I18n.tr("Hide group")
            iconLigature: "visibility_off"
            visible: itemMenu.isTunnel && itemMenu.tunnelVisible
            onTriggered: CircuitCanvas.hideTunnelGroup(itemMenu.itemUid)
        }
        ContextMenuItem {
            text: I18n.tr("Show group")
            iconLigature: "visibility"
            visible: itemMenu.isTunnel && !itemMenu.tunnelVisible
            enabled: itemMenu.tunnelHasName
            onTriggered: CircuitCanvas.showTunnelGroup(itemMenu.itemUid)
        }
        ContextMenuItem {
            text: I18n.tr("Rename group")
            iconLigature: "edit"
            visible: itemMenu.isTunnel
            onTriggered: renameTunnelDialog.openFor(itemMenu.itemUid)
        }

        ContextMenuItem {
            text: I18n.tr("Pause at state change")
            iconLigature: itemMenu.probePause ? "play_circle" : "pause_circle"
            visible: itemMenu.isProbe
            checkable: true
            checked: itemMenu.probePause
            onTriggered: CircuitCanvas.toggleProbePause(itemMenu.itemUid)
        }

        ContextMenuItem {
            text: I18n.tr("Show Table")
            iconLigature: "list"
            visible: itemMenu.isTestUnit
            onTriggered: {
                truthTableModel.clear()
                var rows = CircuitCanvas.testUnitTruth(itemMenu.itemUid)
                for (var i = 0; i < rows.length; ++i)
                    truthTableModel.append({ step: i, value: rows[i] })
                truthTablePopup.open()
            }
        }

        ContextMenuItem {
            text: I18n.tr("Open Monitor")
            iconLigature: "terminal"
            visible: itemMenu.isDcMotor
            onTriggered: motorMonitorPopup.open()
        }

        ContextMenuItem {
            text: I18n.tr("Load Package")
            iconLigature: "folder_open"
            visible: itemMenu.isSubPackage
            onTriggered: loadPackageDialog.open()
        }
        ContextMenuItem {
            text: I18n.tr("Save Package")
            iconLigature: "save"
            visible: itemMenu.isSubPackage
            onTriggered: savePackageDialog.open()
        }
        ContextMenuItem {
            text: I18n.tr("Generate Pins...")
            iconLigature: "grid_on"
            visible: itemMenu.isSubPackage
            onTriggered: {
                if (itemMenu.itemUid) {
                    generatePinsWin.openForItem(itemMenu.itemUid)
                }
            }
        }
        ContextMenuItem {
            id: packageFileItem
            text: I18n.tr("Select Exposed Components")
            iconLigature: "link"
            visible: itemMenu.isSubPackage
            onTriggered: if (itemMenu.itemUid) CircuitCanvas.startLinking(itemMenu.itemUid)
        }
        ContextMenuItem {
            text: CircuitCanvas.boardLayoutPkg.length > 0
                ? I18n.tr("Exit Board Layout")
                : I18n.tr("Edit Board Layout")
            iconLigature: "developer_board"
            visible: itemMenu.isSubPackage
            onTriggered: {
                if (CircuitCanvas.boardLayoutPkg.length > 0)
                    CircuitCanvas.exitBoardLayout()
                else if (itemMenu.itemUid)
                    CircuitCanvas.enterBoardLayout(itemMenu.itemUid)
            }
        }

        AppContextMenu {
            id: footprintsMenu
            title: I18n.tr("Footprints")
            iconLigature: "developer_board"
            ContextMenuItem {
                text: "DIP-8"
                onTriggered: if (itemMenu.itemUid) CircuitCanvas.setPackageFootprintDip(itemMenu.itemUid, "DIP-8", 8, 4)
            }
            ContextMenuItem {
                text: "DIP-14"
                onTriggered: if (itemMenu.itemUid) CircuitCanvas.setPackageFootprintDip(itemMenu.itemUid, "DIP-14", 14, 4)
            }
            ContextMenuItem {
                text: "DIP-16"
                onTriggered: if (itemMenu.itemUid) CircuitCanvas.setPackageFootprintDip(itemMenu.itemUid, "DIP-16", 16, 4)
            }
            ContextMenuItem {
                text: "DIP-20"
                onTriggered: if (itemMenu.itemUid) CircuitCanvas.setPackageFootprintDip(itemMenu.itemUid, "DIP-20", 20, 4)
            }
            ContextMenuItem {
                text: "DIP-24"
                onTriggered: if (itemMenu.itemUid) CircuitCanvas.setPackageFootprintDip(itemMenu.itemUid, "DIP-24", 24, 6)
            }
            ContextMenuItem {
                text: "DIP-28"
                onTriggered: if (itemMenu.itemUid) CircuitCanvas.setPackageFootprintDip(itemMenu.itemUid, "DIP-28", 28, 6)
            }
            ContextMenuItem {
                text: "DIP-40"
                onTriggered: if (itemMenu.itemUid) CircuitCanvas.setPackageFootprintDip(itemMenu.itemUid, "DIP-40", 40, 6)
            }
            ContextMenuItem {
                text: I18n.tr("Logic Symbol (4 In, 2 Out)")
                onTriggered: if (itemMenu.itemUid) CircuitCanvas.setPackageFootprintLs(itemMenu.itemUid, "LS", "A,B,C,D", "Y1,Y2", "VCC", "GND")
            }
        }

        ContextMenuSeparator { visible: itemMenu.hasSpecial && CircuitCanvas.hasItemSelection }

        // C++ Component::contextMenu: Copy, Cut, Remove, Properties, then rotate/flip
        ContextMenuItem {
            text: I18n.tr("Copy")
            shortcutText: UiUtils.shortcutFor("edit.copy")
            iconLigature: "content_copy"
            visible: CircuitCanvas.hasItemSelection
            enabled: CircuitCanvas.hasSelection
            onTriggered: CircuitCanvas.copySelection()
        }
        ContextMenuItem {
            text: I18n.tr("Cut")
            shortcutText: UiUtils.shortcutFor("edit.cut")
            iconLigature: "content_cut"
            visible: CircuitCanvas.hasItemSelection
            enabled: CircuitCanvas.hasSelection
            onTriggered: CircuitCanvas.cutSelection()
        }
        ContextMenuItem {
            text: I18n.tr("Remove")
            shortcutText: UiUtils.shortcutFor("edit.delete")
            iconLigature: "delete"
            visible: CircuitCanvas.hasItemSelection
            enabled: CircuitCanvas.hasSelection
            onTriggered: CircuitCanvas.removeSelection()
        }
        ContextMenuItem {
            text: I18n.tr("Properties")
            shortcutText: UiUtils.shortcutFor("edit.properties")
            iconLigature: "settings"
            visible: CircuitCanvas.hasItemSelection
            onTriggered: AppMenuBar.triggerAction("edit.properties")
        }

        ContextMenuSeparator { visible: CircuitCanvas.hasItemSelection }
        ContextMenuItem {
            text: I18n.tr("Rotate CW")
            shortcutText: UiUtils.shortcutFor("edit.rotateCw")
            iconLigature: "rotate_right"
            visible: CircuitCanvas.hasItemSelection
            onTriggered: AppMenuBar.triggerAction("edit.rotateCw")
        }
        ContextMenuItem {
            text: I18n.tr("Rotate CCW")
            shortcutText: UiUtils.shortcutFor("edit.rotateCcw")
            iconLigature: "rotate_left"
            visible: CircuitCanvas.hasItemSelection
            onTriggered: AppMenuBar.triggerAction("edit.rotateCcw")
        }
        ContextMenuItem {
            text: I18n.tr("Rotate 180")
            iconLigature: "sync_alt"
            visible: CircuitCanvas.hasItemSelection
            onTriggered: CircuitCanvas.rotate180()
        }
        ContextMenuItem {
            text: I18n.tr("Horizontal Flip")
            shortcutText: UiUtils.shortcutFor("edit.flipH")
            iconLigature: "flip"
            visible: CircuitCanvas.hasItemSelection
            onTriggered: AppMenuBar.triggerAction("edit.flipH")
        }
        ContextMenuItem {
            text: I18n.tr("Vertical Flip")
            shortcutText: UiUtils.shortcutFor("edit.flipV")
            iconLigature: "swap_vert"
            visible: CircuitCanvas.hasItemSelection
            onTriggered: AppMenuBar.triggerAction("edit.flipV")
        }
    }

    AppContextMenu {
        id: canvasMenu
        ContextMenuItem {
            text: I18n.tr("Paste")
            shortcutText: UiUtils.shortcutFor("edit.paste")
            iconLigature: "content_paste"
            enabled: CircuitCanvas.canPaste
            onTriggered: CircuitCanvas.pasteAtCursor()
        }
        ContextMenuItem {
            text: I18n.tr("Undo")
            shortcutText: UiUtils.shortcutFor("edit.undo")
            iconLigature: "undo"
            enabled: CircuitCanvas.canUndo
            onTriggered: CircuitCanvas.undo()
        }
        ContextMenuItem {
            text: I18n.tr("Redo")
            shortcutText: UiUtils.shortcutFor("edit.redo")
            iconLigature: "redo"
            enabled: CircuitCanvas.canRedo
            onTriggered: CircuitCanvas.redo()
        }
        ContextMenuSeparator {}
        ContextMenuItem {
            text: I18n.tr("Import Circuit")
            iconLigature: "folder_open"
            onTriggered: importCircDialog.open()
        }
        ContextMenuItem {
            text: I18n.tr("Save Circuit as Image")
            iconLigature: "image"
            onTriggered: saveImageDialog.open()
        }
    }

    FileDialog {
        id: loadWavDialog
        title: I18n.tr("Load Audio File")
        fileMode: FileDialog.OpenFile
        currentFolder: App.suggestFolderUrl(CircuitCanvas.filePath)
        nameFilters: [ I18n.tr("WAV files (*.wav)"), I18n.tr("All files (*)") ]
        onAccepted: {
            if (itemMenu.itemUid) {
                CircuitCanvas.loadWavFile(itemMenu.itemUid, "" + selectedFile)
            }
        }
    }

    FileDialog {
        id: loadFirmwareDialog
        title: I18n.tr("Load Firmware")
        fileMode: FileDialog.OpenFile
        currentFolder: App.suggestFolderUrl(CircuitCanvas.filePath)
        nameFilters: [ I18n.tr("All files (*.*)"), I18n.tr("Hex Files (*.hex)") ]
        onAccepted: CircuitCanvas.loadFirmware(itemMenu.firmwareUid, "" + selectedFile)
    }
    FileDialog {
        id: loadEepromDialog
        title: I18n.tr("Load EEPROM data from file")
        fileMode: FileDialog.OpenFile
        currentFolder: App.suggestFolderUrl(CircuitCanvas.filePath)
        nameFilters: [ I18n.tr("All files (*.*)"), I18n.tr(".data (*.data)"), I18n.tr(".bin (*.bin)") ]
        onAccepted: CircuitCanvas.loadEeprom(itemMenu.firmwareUid, "" + selectedFile)
    }
    FileDialog {
        id: saveEepromDialog
        title: I18n.tr("Save EEPROM data to file")
        fileMode: FileDialog.SaveFile
        currentFolder: App.suggestFolderUrl(CircuitCanvas.filePath)
        defaultSuffix: "data"
        nameFilters: [ I18n.tr("All files (*.*)"), I18n.tr(".data (*.data)"), I18n.tr(".bin (*.bin)") ]
        onAccepted: CircuitCanvas.saveEeprom(itemMenu.firmwareUid, "" + selectedFile)
    }
    FileDialog {
        id: loadDataDialog
        title: I18n.tr("Load data")
        fileMode: FileDialog.OpenFile
        currentFolder: App.suggestFolderUrl(CircuitCanvas.filePath)
        nameFilters: [ I18n.tr("All files (*.*)"), I18n.tr(".data (*.data)"), I18n.tr(".bin (*.bin)") ]
        onAccepted: CircuitCanvas.loadItemData(itemMenu.itemUid, "" + selectedFile)
    }
    FileDialog {
        id: saveDataDialog
        title: I18n.tr("Save data")
        fileMode: FileDialog.SaveFile
        currentFolder: App.suggestFolderUrl(CircuitCanvas.filePath)
        defaultSuffix: "data"
        nameFilters: [ I18n.tr("All files (*.*)"), I18n.tr(".data (*.data)"), I18n.tr(".bin (*.bin)") ]
        onAccepted: CircuitCanvas.saveItemData(itemMenu.itemUid, "" + selectedFile)
    }
    FileDialog {
        id: loadFunctionsDialog
        title: I18n.tr("Open Function")
        fileMode: FileDialog.OpenFile
        currentFolder: App.suggestFolderUrl(CircuitCanvas.filePath)
        nameFilters: [ I18n.tr("Function (*.fnc)"), I18n.tr("All files (*)") ]
        onAccepted: CircuitCanvas.loadFunctions(itemMenu.itemUid, "" + selectedFile)
    }
    FileDialog {
        id: saveFunctionsDialog
        title: I18n.tr("Save Function")
        fileMode: FileDialog.SaveFile
        currentFolder: App.suggestFolderUrl(CircuitCanvas.filePath)
        defaultSuffix: "fnc"
        nameFilters: [ I18n.tr("Function (*.fnc)"), I18n.tr("All files (*)") ]
        onAccepted: CircuitCanvas.saveFunctions(itemMenu.itemUid, "" + selectedFile)
    }
    FileDialog {
        id: loadImageDialog
        title: I18n.tr("Load Image")
        fileMode: FileDialog.OpenFile
        currentFolder: App.suggestFolderUrl(CircuitCanvas.filePath)
        nameFilters: [ I18n.tr("Images (*.png *.jpg *.jpeg *.bmp)"), I18n.tr("All files (*)") ]
        onAccepted: CircuitCanvas.loadImageFile(itemMenu.itemUid, "" + selectedFile)
    }
    FileDialog {
        id: saveImageItemDialog
        title: I18n.tr("Save Image")
        fileMode: FileDialog.SaveFile
        currentFolder: App.suggestFolderUrl(CircuitCanvas.filePath)
        defaultSuffix: "png"
        nameFilters: [ I18n.tr("PNG image (*.png)"), I18n.tr("All files (*)") ]
        onAccepted: CircuitCanvas.saveImageItem(itemMenu.itemUid, "" + selectedFile)
    }
    FileDialog {
        id: loadSdDialog
        title: I18n.tr("Load .img File")
        fileMode: FileDialog.OpenFile
        currentFolder: App.suggestFolderUrl(CircuitCanvas.filePath)
        nameFilters: [ I18n.tr("Disk images (*.img)"), I18n.tr("All files (*)") ]
        onAccepted: CircuitCanvas.loadSdImage(itemMenu.itemUid, "" + selectedFile)
    }

    ListModel { id: truthTableModel }
    Dialog {
        id: truthTablePopup
        title: I18n.tr("Truth Table")
        modal: true
        anchors.centerIn: parent
        width: 320
        height: 360
        standardButtons: Dialog.Close
        ListView {
            anchors.fill: parent
            clip: true
            model: truthTableModel
            delegate: Row {
                id: ttRow
                required property int step
                required property real value
                spacing: 16
                AppText { width: 60; text: "" + ttRow.step; font.family: "Ubuntu Mono" }
                AppText { width: 120; text: "0x" + Number(ttRow.value).toString(16).toUpperCase(); font.family: "Ubuntu Mono" }
            }
        }
    }
    Dialog {
        id: motorMonitorPopup
        title: I18n.tr("Open Monitor")
        modal: false
        anchors.centerIn: parent
        width: 240
        height: 120
        standardButtons: Dialog.Close
        Column {
            anchors.centerIn: parent
            spacing: 8
            AppText { text: I18n.tr("Speed: ") + (itemMenu.itemUid ? "" : "") }
            AppText { text: I18n.tr("Use the Properties panel for live motor values.") }
        }
    }
    RenameTunnelDialog {
        id: renameTunnelDialog
    }

    AppWindow {
        id: editPinWin
        ownerWindow: root.Window.window as Window
        modality: Qt.WindowModal
        title: I18n.tr("Edit Package Pin")
        sizeToContent: true
        contentMinWidth: 380
        panelSource: "EditPinDialog.qml"

        property string targetUid: ""
        property string targetPinId: ""

        function openForPin(itemUid: string, pinId: string): void {
            targetUid = itemUid
            targetPinId = pinId
            var p = panel as EditPinDialog
            if (visible && p) {
                p.loadPin(itemUid, pinId)
                bringToFront()
                p.focusFirstField()
            } else {
                open()
            }
        }

        onAboutToShow: {
            var p = panel as EditPinDialog
            if (p && targetUid && targetPinId)
                p.loadPin(targetUid, targetPinId)
        }

        onVisibleChanged: {
            if (visible) {
                Qt.callLater(function() {
                    var p = editPinWin.panel as EditPinDialog
                    if (editPinWin.visible && p)
                        p.focusFirstField()
                })
            }
        }

        onClosing: (close) => {
            if (!editPinWin.isQuitting) {
                close.accepted = false
                openRequested = false
            }
        }

        Connections {
            target: editPinWin.panel
            ignoreUnknownSignals: true
            function onAccepted(): void { editPinWin.openRequested = false }
            function onDeleted(): void { editPinWin.openRequested = false }
            function onRejected(): void { editPinWin.openRequested = false }
        }
    }

    AppWindow {
        id: generatePinsWin
        ownerWindow: root.Window.window as Window
        modality: Qt.WindowModal
        title: I18n.tr("Generate Package Pins")
        sizeToContent: true
        contentMinWidth: 360
        panelSource: "GeneratePinsDialog.qml"

        property string targetUid: ""

        function openForItem(itemUid: string): void {
            targetUid = itemUid
            var p = panel as GeneratePinsDialog
            if (visible && p) {
                p.uid = itemUid
                bringToFront()
                p.focusFirstField()
            } else {
                open()
            }
        }

        onAboutToShow: {
            var p = panel as GeneratePinsDialog
            if (p && targetUid)
                p.uid = targetUid
        }

        onVisibleChanged: {
            if (visible) {
                Qt.callLater(function() {
                    var p = generatePinsWin.panel as GeneratePinsDialog
                    if (generatePinsWin.visible && p)
                        p.focusFirstField()
                })
            }
        }

        onClosing: (close) => {
            if (!generatePinsWin.isQuitting) {
                close.accepted = false
                openRequested = false
            }
        }

        Connections {
            target: generatePinsWin.panel
            ignoreUnknownSignals: true
            function onAccepted(): void { generatePinsWin.openRequested = false }
            function onRejected(): void { generatePinsWin.openRequested = false }
        }
    }

    FileDialog {
        id: loadPackageDialog
        title: I18n.tr("Load Package File")
        fileMode: FileDialog.OpenFile
        currentFolder: App.suggestFolderUrl(CircuitCanvas.filePath)
        nameFilters: [ I18n.tr("Package files (*.package)"), I18n.tr("All files (*)") ]
        onAccepted: {
            if (CircuitCanvas.selectedUid) {
                CircuitCanvas.loadPackageFile(CircuitCanvas.selectedUid, "" + selectedFile)
            }
        }
    }

    FileDialog {
        id: savePackageDialog
        title: I18n.tr("Export Package File")
        fileMode: FileDialog.SaveFile
        currentFolder: App.suggestFolderUrl(CircuitCanvas.filePath)
        defaultSuffix: "package"
        nameFilters: [ I18n.tr("Package files (*.package)"), I18n.tr("All files (*)") ]
        onAccepted: {
            if (CircuitCanvas.selectedUid) {
                CircuitCanvas.savePackageFile(CircuitCanvas.selectedUid, "" + selectedFile)
            }
        }
    }

    FileDialog {
        id: saveImageDialog
        title: I18n.tr("Save as Image")
        fileMode: FileDialog.SaveFile
        currentFolder: App.suggestFolderUrl(CircuitCanvas.filePath)
        defaultSuffix: "png"
        nameFilters: [
            I18n.tr("PNG (*.png)"),
            I18n.tr("JPEG (*.jpeg *.jpg)"),
            I18n.tr("BMP (*.bmp)"),
            I18n.tr("SVG (*.svg)"),
            I18n.tr("All (*.*)")
        ]
        currentFile: CircuitCanvas.suggestImageUrl || ""
        onAccepted: CircuitCanvas.saveImage("" + selectedFile)
    }

    FileDialog {
        id: importCircDialog
        title: I18n.tr("Import Circuit")
        fileMode: FileDialog.OpenFile
        currentFolder: App.suggestFolderUrl(CircuitCanvas.filePath)
        nameFilters: [ I18n.tr("Circuits (*.circ1 *.sim2 *.sim1)"), I18n.tr("All files (*)") ]
        onAccepted: CircuitCanvas.importPath("" + selectedFile)
    }

    WheelHandler {
        target: null
        acceptedDevices: PointerDevice.Mouse | PointerDevice.TouchPad
        acceptedModifiers: Qt.KeyboardModifierMask
        orientation: Qt.Horizontal | Qt.Vertical
        onWheel: (w) => CircuitCanvas.wheel(w.pixelDelta.x, w.pixelDelta.y,
                                            w.angleDelta.x, w.angleDelta.y,
                                            w.x, w.y,
                                            w.modifiers)
    }

    Timer {
        id: simTimer
        interval: Math.max(10, Math.round(1000 / Math.max(1, AppDialog.fps)))
        repeat: true
        running: CircuitCanvas.simRunning && !CircuitCanvas.simPaused
        onTriggered: {
            CircuitCanvas.tick()
            SerialMonitor.sync()
        }
    }

    Timer {
        id: infoTimer
        interval: 100
        repeat: true
        running: CircuitCanvas.simRunning && !CircuitCanvas.simPaused && CircuitPanel.infoVisible
        onTriggered: {
            InfoWidget.syncSim(
                CircuitCanvas.simCircTime,
                CircuitCanvas.speedPercent,
                CircuitCanvas.simRealSpeed,
                CircuitCanvas.simLoad,
                CircuitCanvas.simGuiLoad,
                CircuitCanvas.simFps,
                CircuitCanvas.simMcuDevice,
                CircuitCanvas.simMcuName,
                CircuitCanvas.simHasMcu
            )
        }
    }

    PinchHandler {
        id: pinch
        target: null
        property real lastScale: 1
        onActiveChanged: {
            if (active) {
                lastScale = scale
            } else {
                lastScale = 1
            }
        }
        onScaleChanged: {
            if (!active)
                return
            var factor = scale / lastScale
            if (factor > 0 && isFinite(factor) && Math.abs(factor - 1.0) > 1e-4)
                CircuitCanvas.pinchZoom(factor, centroid.position.x, centroid.position.y)
            lastScale = scale
        }
    }

    onActiveFocusChanged: {
        if (!activeFocus)
            CircuitCanvas.releaseAllKeys()
    }

    Keys.onShortcutOverride: (ev) => {
        if (CircuitCanvas.ownsKeyShortcut(ev.key, ev.modifiers, ev.text))
            ev.accepted = true
    }

    Keys.onPressed: (ev) => {
        if (ev.key === Qt.Key_Escape && CircuitCanvas.linkingFrom) {
            CircuitCanvas.stopLinking()
            ev.accepted = true
            return
        }
        CircuitCanvas.handleKey(ev.key, ev.modifiers, ev.text, ev.isAutoRepeat)
        ev.accepted = true
    }

    Keys.onReleased: (ev) => {
        CircuitCanvas.handleKeyRelease(ev.key, ev.modifiers, ev.text, ev.isAutoRepeat)
        ev.accepted = true
    }

    Rectangle {
        visible: CircuitCanvas.linkingFrom.length > 0 || CircuitCanvas.boardLayoutPkg.length > 0
        anchors.horizontalCenter: parent.horizontalCenter
        anchors.bottom: parent.bottom
        anchors.bottomMargin: hBar.visible ? hBar.height + 10 : 10
        z: 40
        radius: 6
        height: linkBannerRow.implicitHeight + 10
        width: linkBannerRow.implicitWidth + 20
        color: appTheme.window
        border.color: CircuitCanvas.linkingExposed
            ? CircuitCanvas.exposedHighlightColor
            : CircuitCanvas.linkHighlightColor
        border.width: 1.5

        Row {
            id: linkBannerRow
            anchors.centerIn: parent
            spacing: 10

            Rectangle {
                anchors.verticalCenter: parent.verticalCenter
                width: 8
                height: 8
                radius: 4
                color: CircuitCanvas.linkingExposed
                    ? CircuitCanvas.exposedHighlightColor
                    : CircuitCanvas.linkHighlightColor
            }

            AppText {
                anchors.verticalCenter: parent.verticalCenter
                color: appTheme.windowText
                text: CircuitCanvas.boardLayoutPkg.length > 0
                    ? I18n.tr("Board Layout — drag components onto the package face (R to rotate), press Esc or Done to finish")
                    : (CircuitCanvas.linkingExposed
                        ? I18n.tr("Select Exposed Components — click components to toggle, click empty space or Esc to finish")
                        : I18n.tr("Link to Component") + " (" + CircuitCanvas.linkingFrom + ") — "
                            + I18n.tr("click components to toggle, click empty space or Esc to finish"))
            }

            AppButton {
                anchors.verticalCenter: parent.verticalCenter
                text: I18n.tr("Done")
                onClicked: {
                    if (CircuitCanvas.boardLayoutPkg.length > 0)
                        CircuitCanvas.exitBoardLayout()
                    else
                        CircuitCanvas.stopLinking()
                }
            }
        }
    }

    Rectangle {
        visible: CircuitCanvas.banding
        x: CircuitCanvas.bandRect.x
        y: CircuitCanvas.bandRect.y
        width: CircuitCanvas.bandRect.w
        height: CircuitCanvas.bandRect.h
        color: {
            var v = root.rgba(CircuitCanvas.bandColor)
            return Qt.rgba(v.x, v.y, v.z, 40 / 255)
        }
        border.color: CircuitCanvas.bandColor
        border.width: 1
    }

    AppScrollBar {
        id: vBar
        orientation: Qt.Vertical
        policy: ScrollBar.AlwaysOn
        visible: CircuitCanvas.showScroll
        anchors { right: parent.right; top: parent.top; bottom: parent.bottom }
        size: CircuitCanvas.showScroll ? CircuitCanvas.vScrollSize : 1
        position: CircuitCanvas.showScroll ? CircuitCanvas.vScrollPos : 0
        onPositionChanged: {
            if (!pressed)
                return
            CircuitCanvas.centerY = CircuitCanvas.sceneRect.y
                    + position * CircuitCanvas.sceneRect.h
                    + CircuitCanvas.visibleRect.h / 2
        }
    }

    AppScrollBar {
        id: hBar
        orientation: Qt.Horizontal
        policy: ScrollBar.AlwaysOn
        visible: CircuitCanvas.showScroll
        anchors { left: parent.left; right: parent.right; bottom: parent.bottom }
        size: CircuitCanvas.showScroll ? CircuitCanvas.hScrollSize : 1
        position: CircuitCanvas.showScroll ? CircuitCanvas.hScrollPos : 0
        onPositionChanged: {
            if (!pressed)
                return
            CircuitCanvas.centerX = CircuitCanvas.sceneRect.x
                    + position * CircuitCanvas.sceneRect.w
                    + CircuitCanvas.visibleRect.w / 2
        }
    }

    DropArea {
        anchors.fill: parent
        keys: ["text/plain"]
        onDropped: (drop) => {
            var text = drop.text || (drop.hasText ? drop.text : "")
            if (!text && drop.getDataAsString) text = drop.getDataAsString("text/plain")
            if (text) {
                CircuitCanvas.dropAt(text, drop.x, drop.y)
                ComponentList.addRecent(text)
                drop.acceptProposedAction()
            }
        }
    }

    CircuitTooltip {
        id: tooltip
    }

    RepaintDebugOverlay {
        id: repaintOverlay
        anchors.fill: parent
        z: 50
    }

    SystemPalette { id: appTheme }
}
