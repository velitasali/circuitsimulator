import QtQuick
import QtQuick.Controls
import QtQuick.Dialogs
import QtQuick.Window
import cs_app

/* Pan/zoom/select canvas backed by native tiny-skia software rasterization
 * (CircuitCanvasItem) composited over the existing GPU grid shader.
 * Forwarding all input to CircuitCanvas (Rust scene + hit-test). Hosted by CircuitPanel.qml. */
Item {
    id: root
    function tr(s) { return App.i18nTick >= 0 ? App.translate(s) : s }
    focus: true
    clip: true

    readonly property bool dark: App.darkTheme

    function rgba(hex) {
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

    function syncSize() {
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
        centerX: CircuitCanvas.centerX
        centerY: CircuitCanvas.centerY
        zoom: CircuitCanvas.zoom
    }

    Timer {
        id: panSettleTimer
        interval: 60
        repeat: false
        onTriggered: {
            if (typeof CircuitCanvas.settlePan === "function") {
                CircuitCanvas.settlePan()
            } else if (typeof CircuitCanvas.settle_pan === "function") {
                CircuitCanvas.settle_pan()
            }
        }
    }

    Connections {
        target: CircuitCanvas
        function onCenterXChanged() {
            panSettleTimer.restart()
        }
        function onCenterYChanged() {
            panSettleTimer.restart()
        }
        function onZoomChanged() {
            panSettleTimer.restart()
        }
    }

    MouseArea {
        id: mouse
        anchors.fill: parent
        hoverEnabled: true
        acceptedButtons: Qt.AllButtons
        cursorShape: {
            switch (CircuitCanvas.cursor) {
            case "openhand":   return Qt.OpenHandCursor
            case "closedhand": return Qt.ClosedHandCursor
            case "cross":      return Qt.CrossCursor
            case "splith":     return Qt.SplitHCursor
            case "splitv":     return Qt.SplitVCursor
            case "sizeall":    return Qt.SizeAllCursor
            default:           return Qt.ArrowCursor
            }
        }
        preventStealing: true

        onPressed: (ev) => {
            root.forceActiveFocus()
            CircuitCanvas.mousePress(ev.button, ev.x, ev.y, ev.modifiers)
            if (CircuitCanvas.linkingFrom && ev.button === Qt.LeftButton) {
                if (CircuitCanvas.hasItemSelection)
                    CircuitCanvas.completeLink()
                else
                    CircuitCanvas.stopLinking()
                return
            }
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
        function onRequestOpenSerialMon(monId, title) {
            SerialMonitor.openMonitor(monId, title)
        }
        function onRequestOpenFile(path) {
            EditorPanel.loadFile(path)
            AppDialog.addRecentFile(path)
            AppMenuBar.rebuild()
            CircuitPanel.showEditor()
        }
        function onRequestShowEditor() {
            CircuitPanel.showEditor()
        }
        function onRequestPauseSim() {
            if (CircuitPanel.simRunning && !CircuitPanel.simPaused)
                CircuitPanel.pauseCirc()
        }
        function onRequestUploadRun(debug) {
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
        readonly property string itemUid: {
            var p = pinId
            var i = p.lastIndexOf("-")
            return i > 0 ? p.substring(0, i) : ""
        }
        readonly property string localPinId: {
            var p = pinId
            var i = p.lastIndexOf("-")
            return i > 0 ? p.substring(i + 1) : p
        }

        ContextMenuItem {
            text: root.tr("Invert Pin")
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
            text: root.tr("Edit Pin")
            iconLigature: "edit"
            visible: pinMenu.isPackage
            onTriggered: {
                editPinContent.loadPin(pinMenu.itemUid, pinMenu.localPinId)
                editPinPopup.open()
            }
        }
        ContextMenuItem {
            text: root.tr("Delete Pin")
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
            text: root.tr("Remove")
            shortcutText: "Del"
            iconLigature: "delete"
            onTriggered: CircuitCanvas.removeSelection()
        }
    }

    AppContextMenu {
        id: labelMenu
        readonly property string uid: CircuitCanvas.hitLabelUid
        readonly property bool isVal: CircuitCanvas.hitLabelIsVal

        ContextMenuItem {
            text: root.tr("Rotate CW")
            shortcutText: "Ctrl+R"
            iconLigature: "rotate_right"
            onTriggered: {
                if (labelMenu.isVal)
                    CircuitCanvas.rotateItemValLabel(labelMenu.uid, 90)
                else
                    CircuitCanvas.rotateItemLabel(labelMenu.uid, 90)
            }
        }
        ContextMenuItem {
            text: root.tr("Rotate CCW")
            shortcutText: "Ctrl+Shift+R"
            iconLigature: "rotate_left"
            onTriggered: {
                if (labelMenu.isVal)
                    CircuitCanvas.rotateItemValLabel(labelMenu.uid, -90)
                else
                    CircuitCanvas.rotateItemLabel(labelMenu.uid, -90)
            }
        }
        ContextMenuItem {
            text: root.tr("Rotate 180°")
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
        readonly property bool isLed: itemType === "Led" || itemType === "LED"
        readonly property bool isDial: itemType === "Dial"
        readonly property bool isLinker: isLed || isDial || isDcMotor || isMcuChip
        readonly property bool hasMonitors: (CircuitCanvas.selectedItemMonitors && CircuitCanvas.selectedItemMonitors.length > 0)
        readonly property bool hasFlash: CircuitCanvas.selectedHasFlash
        readonly property bool hasEeprom: CircuitCanvas.selectedHasEeprom
        readonly property bool isActive: CircuitCanvas.selectedItemActive
        readonly property bool tunnelVisible: CircuitCanvas.selectedTunnelVisible
        readonly property bool probePause: CircuitCanvas.selectedProbePause
        readonly property bool hasSpecial: isWaveGen || isOsc || isLa || isMcu || isSerial || isSubPackage || isSubc
            || isMemory || isFunction || isImage || isSdCard || isTunnel || isProbe || isTestUnit || isDcMotor || isLinker

        function detachSubMenu(sub) {
            for (var i = 0; i < count; i++) {
                if (menuAt(i) === sub) {
                    takeMenu(i)
                    return
                }
            }
        }
        function syncSubMenuRow(sub, shown) {
            for (var i = 0; i < count; i++) {
                var it = itemAt(i)
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
            detachSubMenu(footprintsMenu)
            if (itemMenu.isSubPackage)
                itemMenu.insertMenu(packageFileItem.index !== undefined ? packageFileItem.index : 1, footprintsMenu)
        }
        onClosed: {
            detachSubMenu(footprintsMenu)
        }

        // C++ Chip::addActivateAction — checkable, tick when active, disabled once active.
        ContextMenuItem {
            text: root.tr("Activate")
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
            text: root.tr("Upload and Run")
            iconLigature: "play_arrow"
            visible: itemMenu.isQemu
            onTriggered: if (itemMenu.itemUid) CircuitCanvas.uploadAndRun(itemMenu.itemUid, false)
        }
        ContextMenuItem {
            text: root.tr("Upload and Debug")
            iconLigature: "bug_report"
            visible: itemMenu.isQemu
            onTriggered: if (itemMenu.itemUid) CircuitCanvas.uploadAndRun(itemMenu.itemUid, true)
        }

        ContextMenuItem {
            id: editFirmwareItem
            text: root.tr("Edit firmware")
            iconLigature: "edit"
            visible: itemMenu.showMcuActions
            onTriggered: CircuitCanvas.editFirmware(itemMenu.firmwareUid || "")
        }

        ContextMenuItem {
            text: root.tr("Link to Component")
            iconLigature: "link"
            visible: itemMenu.isLinker
            onTriggered: if (itemMenu.itemUid) CircuitCanvas.startLinking(itemMenu.itemUid)
        }

        ContextMenuSeparator { visible: itemMenu.isQemu }

        ContextMenuItem {
            text: root.tr("Load firmware")
            iconLigature: "file_download"
            visible: itemMenu.showMcuActions && (itemMenu.hasFlash || itemMenu.isQemu)
            onTriggered: loadFirmwareDialog.open()
        }
        ContextMenuItem {
            id: reloadFirmwareItem
            text: root.tr("Reload firmware")
            iconLigature: "refresh"
            visible: itemMenu.showMcuActions && (itemMenu.hasFlash || itemMenu.isQemu)
            onTriggered: CircuitCanvas.reloadFirmware(itemMenu.firmwareUid)
        }

        ContextMenuSeparator { visible: (itemMenu.isMcuChip || itemMenu.hasNestedMcu) && itemMenu.hasFlash }

        ContextMenuItem {
            text: root.tr("Load EEPROM data from file")
            iconLigature: "folder_open"
            visible: (itemMenu.isMcuChip || itemMenu.hasNestedMcu) && itemMenu.hasEeprom
            onTriggered: loadEepromDialog.open()
        }
        ContextMenuItem {
            text: root.tr("Save EEPROM data to file")
            iconLigature: "save"
            visible: (itemMenu.isMcuChip || itemMenu.hasNestedMcu) && itemMenu.hasEeprom
            onTriggered: saveEepromDialog.open()
        }

        ContextMenuSeparator { visible: (itemMenu.isMcuChip || itemMenu.hasNestedMcu) && itemMenu.hasEeprom }

        ContextMenuItem {
            id: openMcuMonitorItem
            text: root.tr("Open Mcu Monitor.")
            iconLigature: "terminal"
            visible: itemMenu.isMcuChip || itemMenu.hasNestedMcu
            onTriggered: {
                if (itemMenu.firmwareUid)
                    CircuitCanvas.activeDeviceId = itemMenu.firmwareUid
                App.showMcu()
            }
        }

        AppContextMenu {
            id: monitorMenu
            title: itemMenu.isQemu ? root.tr("Open Serial Monitor.") : root.tr("Open Monitor.")
            iconLigature: "terminal"
            Instantiator {
                id: monitorRepeater
                model: CircuitCanvas.selectedItemMonitors || []
                delegate: ContextMenuItem {
                    required property var modelData
                    text: "" + modelData
                    onTriggered: {
                        var uid = itemMenu.firmwareUid || itemMenu.itemUid
                        var title = uid ? (uid + "-" + text) : text
                        var monId = uid ? (uid + ":" + text) : text
                        SerialMonitor.openMonitor(monId, title)
                    }
                }
                onObjectAdded: (index, object) => monitorMenu.insertItem(index, object)
                onObjectRemoved: (_, object) => monitorMenu.removeItem(object)
            }
        }

        ContextMenuItem {
            text: root.tr("Open Subcircuit")
            iconLigature: "upload"
            visible: itemMenu.isSubc
            onTriggered: if (itemMenu.itemUid) CircuitCanvas.openSubcircuit(itemMenu.itemUid)
        }

        ContextMenuItem {
            text: root.tr("Load Audio File")
            iconLigature: "file_download"
            visible: itemMenu.isWaveGen
            onTriggered: loadWavDialog.open()
        }
        ContextMenuItem {
            text: root.tr("Open Oscilloscope Window")
            iconLigature: "monitoring"
            visible: itemMenu.isOsc
            onTriggered: App.showOsc()
        }
        ContextMenuItem {
            text: root.tr("Open Logic Analyzer Window")
            iconLigature: "monitoring"
            visible: itemMenu.isLa
            onTriggered: App.showLa()
        }
        ContextMenuItem {
            text: root.tr("Open Serial Monitor.")
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
            text: root.tr("Load data")
            iconLigature: "file_download"
            visible: itemMenu.isMemory
            onTriggered: loadDataDialog.open()
        }
        ContextMenuItem {
            text: root.tr("Save data")
            iconLigature: "save"
            visible: itemMenu.isMemory
            onTriggered: saveDataDialog.open()
        }
        ContextMenuItem {
            text: root.tr("Show Memory Table")
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
            text: root.tr("Save Functions")
            iconLigature: "save"
            visible: itemMenu.isFunction
            onTriggered: saveFunctionsDialog.open()
        }
        ContextMenuItem {
            text: root.tr("Load Functions")
            iconLigature: "folder_open"
            visible: itemMenu.isFunction
            onTriggered: loadFunctionsDialog.open()
        }

        ContextMenuItem {
            text: root.tr("Load Image")
            iconLigature: "file_download"
            visible: itemMenu.isImage
            onTriggered: loadImageDialog.open()
        }
        ContextMenuItem {
            text: root.tr("Save Image")
            iconLigature: "save"
            visible: itemMenu.isImage && CircuitCanvas.selectedHasImageData
            onTriggered: saveImageItemDialog.open()
        }

        ContextMenuItem {
            text: root.tr("Load .img File")
            iconLigature: "file_download"
            visible: itemMenu.isSdCard
            onTriggered: loadSdDialog.open()
        }
        ContextMenuItem {
            text: root.tr("Eject SD card")
            iconLigature: "link_off"
            visible: itemMenu.isSdCard && CircuitCanvas.selectedHasSdImage
            onTriggered: CircuitCanvas.ejectSdCard(itemMenu.itemUid)
        }

        ContextMenuItem {
            text: root.tr("Hide group")
            iconLigature: "visibility_off"
            visible: itemMenu.isTunnel && itemMenu.tunnelVisible
            onTriggered: CircuitCanvas.hideTunnelGroup(itemMenu.itemUid)
        }
        ContextMenuItem {
            text: root.tr("Show group")
            iconLigature: "stop_circle"
            visible: itemMenu.isTunnel && !itemMenu.tunnelVisible
            onTriggered: CircuitCanvas.showTunnelGroup(itemMenu.itemUid)
        }
        ContextMenuItem {
            text: root.tr("Rename group")
            iconLigature: "edit"
            visible: itemMenu.isTunnel
            onTriggered: {
                renameTunnelField.text = ""
                renameTunnelPopup.open()
            }
        }

        ContextMenuItem {
            text: root.tr("Pause at state change")
            iconLigature: itemMenu.probePause ? "play_circle" : "pause_circle"
            visible: itemMenu.isProbe
            checkable: true
            checked: itemMenu.probePause
            onTriggered: CircuitCanvas.toggleProbePause(itemMenu.itemUid)
        }

        ContextMenuItem {
            text: root.tr("Show Table")
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
            text: root.tr("Open Monitor")
            iconLigature: "terminal"
            visible: itemMenu.isDcMotor
            onTriggered: motorMonitorPopup.open()
        }

        ContextMenuItem {
            text: root.tr("Load Package")
            iconLigature: "folder_open"
            visible: itemMenu.isSubPackage
            onTriggered: loadPackageDialog.open()
        }
        ContextMenuItem {
            text: root.tr("Save Package")
            iconLigature: "save"
            visible: itemMenu.isSubPackage
            onTriggered: savePackageDialog.open()
        }
        ContextMenuItem {
            text: root.tr("Generate Pins...")
            iconLigature: "grid_on"
            visible: itemMenu.isSubPackage
            onTriggered: {
                if (itemMenu.itemUid) {
                    generatePinsContent.uid = itemMenu.itemUid
                    generatePinsPopup.open()
                }
            }
        }
        ContextMenuItem {
            id: packageFileItem
            text: root.tr("Select Exposed Components")
            iconLigature: "link"
            visible: itemMenu.isSubPackage
            onTriggered: if (itemMenu.itemUid) CircuitCanvas.startLinking(itemMenu.itemUid)
        }

        ContextMenuSeparator { visible: itemMenu.hasSpecial }

        // C++ Component::contextMenu: Copy, Cut, Remove, Properties, then rotate/flip
        ContextMenuItem {
            text: root.tr("Copy")
            shortcutText: "Ctrl+C"
            iconLigature: "content_copy"
            enabled: CircuitCanvas.hasSelection
            onTriggered: CircuitCanvas.copySelection()
        }
        ContextMenuItem {
            text: root.tr("Cut")
            shortcutText: "Ctrl+X"
            iconLigature: "content_cut"
            enabled: CircuitCanvas.hasSelection
            onTriggered: CircuitCanvas.cutSelection()
        }
        ContextMenuItem {
            text: root.tr("Remove")
            shortcutText: "Del"
            iconLigature: "delete"
            enabled: CircuitCanvas.hasSelection
            onTriggered: CircuitCanvas.removeSelection()
        }
        ContextMenuItem {
            text: root.tr("Properties")
            iconLigature: "settings"
            visible: CircuitCanvas.hasItemSelection
            onTriggered: CircuitCanvas.openSelectedProperties()
        }

        ContextMenuSeparator { visible: CircuitCanvas.hasItemSelection }
        ContextMenuItem {
            text: root.tr("Rotate CW")
            shortcutText: "Ctrl+R"
            iconLigature: "rotate_right"
            visible: CircuitCanvas.hasItemSelection
            onTriggered: CircuitCanvas.rotateCw()
        }
        ContextMenuItem {
            text: root.tr("Rotate CCW")
            shortcutText: "Ctrl+Shift+R"
            iconLigature: "rotate_left"
            visible: CircuitCanvas.hasItemSelection
            onTriggered: CircuitCanvas.rotateCcw()
        }
        ContextMenuItem {
            text: root.tr("Rotate 180")
            iconLigature: "sync_alt"
            visible: CircuitCanvas.hasItemSelection
            onTriggered: CircuitCanvas.rotate180()
        }
        ContextMenuItem {
            text: root.tr("Horizontal Flip")
            shortcutText: "Ctrl+L"
            iconLigature: "flip"
            visible: CircuitCanvas.hasItemSelection
            onTriggered: CircuitCanvas.flipH()
        }
        ContextMenuItem {
            text: root.tr("Vertical Flip")
            shortcutText: "Ctrl+Shift+L"
            iconLigature: "swap_vert"
            visible: CircuitCanvas.hasItemSelection
            onTriggered: CircuitCanvas.flipV()
        }
    }

    AppContextMenu {
        id: footprintsMenu
        title: root.tr("Footprints")
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
            text: root.tr("Logic Symbol (4 In, 2 Out)")
            onTriggered: if (itemMenu.itemUid) CircuitCanvas.setPackageFootprintLs(itemMenu.itemUid, "LS", "A,B,C,D", "Y1,Y2", "VCC", "GND")
        }
    }

    AppContextMenu {
        id: canvasMenu
        ContextMenuItem {
            text: root.tr("Paste")
            shortcutText: "Ctrl+V"
            iconLigature: "content_paste"
            enabled: CircuitCanvas.canPaste
            onTriggered: CircuitCanvas.pasteAtCursor()
        }
        ContextMenuItem {
            text: root.tr("Undo")
            shortcutText: "Ctrl+Z"
            iconLigature: "undo"
            enabled: CircuitCanvas.canUndo
            onTriggered: CircuitCanvas.undo()
        }
        ContextMenuItem {
            text: root.tr("Redo")
            shortcutText: "Ctrl+Y"
            iconLigature: "redo"
            enabled: CircuitCanvas.canRedo
            onTriggered: CircuitCanvas.redo()
        }
        ContextMenuSeparator {}
        ContextMenuItem {
            text: root.tr("Import Circuit")
            iconLigature: "folder_open"
            onTriggered: importCircDialog.open()
        }
        ContextMenuItem {
            text: root.tr("Save Circuit as Image")
            iconLigature: "image"
            onTriggered: saveImageDialog.open()
        }
    }

    FileDialog {
        id: loadWavDialog
        title: root.tr("Load Audio File")
        fileMode: FileDialog.OpenFile
        currentFolder: App.suggestFolderUrl(CircuitCanvas.filePath)
        nameFilters: [ root.tr("WAV files (*.wav)"), root.tr("All files (*)") ]
        onAccepted: {
            if (itemMenu.itemUid) {
                CircuitCanvas.loadWavFile(itemMenu.itemUid, "" + selectedFile)
            }
        }
    }

    FileDialog {
        id: loadFirmwareDialog
        title: root.tr("Load Firmware")
        fileMode: FileDialog.OpenFile
        currentFolder: App.suggestFolderUrl(CircuitCanvas.filePath)
        nameFilters: [ root.tr("All files (*.*)"), root.tr("Hex Files (*.hex)") ]
        onAccepted: CircuitCanvas.loadFirmware(itemMenu.firmwareUid, "" + selectedFile)
    }
    FileDialog {
        id: loadEepromDialog
        title: root.tr("Load EEPROM data from file")
        fileMode: FileDialog.OpenFile
        currentFolder: App.suggestFolderUrl(CircuitCanvas.filePath)
        nameFilters: [ root.tr("All files (*.*)"), root.tr(".data (*.data)"), root.tr(".bin (*.bin)") ]
        onAccepted: CircuitCanvas.loadEeprom(itemMenu.firmwareUid, "" + selectedFile)
    }
    FileDialog {
        id: saveEepromDialog
        title: root.tr("Save EEPROM data to file")
        fileMode: FileDialog.SaveFile
        currentFolder: App.suggestFolderUrl(CircuitCanvas.filePath)
        defaultSuffix: "data"
        nameFilters: [ root.tr("All files (*.*)"), root.tr(".data (*.data)"), root.tr(".bin (*.bin)") ]
        onAccepted: CircuitCanvas.saveEeprom(itemMenu.firmwareUid, "" + selectedFile)
    }
    FileDialog {
        id: loadDataDialog
        title: root.tr("Load data")
        fileMode: FileDialog.OpenFile
        currentFolder: App.suggestFolderUrl(CircuitCanvas.filePath)
        nameFilters: [ root.tr("All files (*.*)"), root.tr(".data (*.data)"), root.tr(".bin (*.bin)") ]
        onAccepted: CircuitCanvas.loadItemData(itemMenu.itemUid, "" + selectedFile)
    }
    FileDialog {
        id: saveDataDialog
        title: root.tr("Save data")
        fileMode: FileDialog.SaveFile
        currentFolder: App.suggestFolderUrl(CircuitCanvas.filePath)
        defaultSuffix: "data"
        nameFilters: [ root.tr("All files (*.*)"), root.tr(".data (*.data)"), root.tr(".bin (*.bin)") ]
        onAccepted: CircuitCanvas.saveItemData(itemMenu.itemUid, "" + selectedFile)
    }
    FileDialog {
        id: loadFunctionsDialog
        title: root.tr("Open Function")
        fileMode: FileDialog.OpenFile
        currentFolder: App.suggestFolderUrl(CircuitCanvas.filePath)
        nameFilters: [ root.tr("Function (*.fnc)"), root.tr("All files (*)") ]
        onAccepted: CircuitCanvas.loadFunctions(itemMenu.itemUid, "" + selectedFile)
    }
    FileDialog {
        id: saveFunctionsDialog
        title: root.tr("Save Function")
        fileMode: FileDialog.SaveFile
        currentFolder: App.suggestFolderUrl(CircuitCanvas.filePath)
        defaultSuffix: "fnc"
        nameFilters: [ root.tr("Function (*.fnc)"), root.tr("All files (*)") ]
        onAccepted: CircuitCanvas.saveFunctions(itemMenu.itemUid, "" + selectedFile)
    }
    FileDialog {
        id: loadImageDialog
        title: root.tr("Load Image")
        fileMode: FileDialog.OpenFile
        currentFolder: App.suggestFolderUrl(CircuitCanvas.filePath)
        nameFilters: [ root.tr("All files (*)") ]
        onAccepted: CircuitCanvas.loadImageFile(itemMenu.itemUid, "" + selectedFile)
    }
    FileDialog {
        id: saveImageItemDialog
        title: root.tr("Save Image")
        fileMode: FileDialog.SaveFile
        currentFolder: App.suggestFolderUrl(CircuitCanvas.filePath)
        onAccepted: CircuitCanvas.saveImageItem(itemMenu.itemUid, "" + selectedFile)
    }
    FileDialog {
        id: loadSdDialog
        title: root.tr("Load .img File")
        fileMode: FileDialog.OpenFile
        currentFolder: App.suggestFolderUrl(CircuitCanvas.filePath)
        nameFilters: [ root.tr("Disk images (*.img)"), root.tr("All files (*)") ]
        onAccepted: CircuitCanvas.loadSdImage(itemMenu.itemUid, "" + selectedFile)
    }

    ListModel { id: truthTableModel }
    Dialog {
        id: truthTablePopup
        title: root.tr("Truth Table")
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
                spacing: 16
                Text { width: 60; text: "" + step; font.family: "Ubuntu Mono" }
                Text { width: 120; text: "0x" + Number(value).toString(16).toUpperCase(); font.family: "Ubuntu Mono" }
            }
        }
    }
    Dialog {
        id: motorMonitorPopup
        title: root.tr("Open Monitor")
        modal: false
        anchors.centerIn: parent
        width: 240
        height: 120
        standardButtons: Dialog.Close
        Column {
            anchors.centerIn: parent
            spacing: 8
            Text { text: root.tr("Speed: ") + (itemMenu.itemUid ? "" : "") }
            Text { text: root.tr("Use the Properties panel for live motor values.") }
        }
    }
    Dialog {
        id: renameTunnelPopup
        title: root.tr("Rename Tunnels")
        modal: true
        anchors.centerIn: parent
        width: 320
        standardButtons: Dialog.Ok | Dialog.Cancel
        onAccepted: {
            if (renameTunnelField.text.length > 0)
                CircuitCanvas.renameTunnelGroup(itemMenu.itemUid, renameTunnelField.text)
        }
        Column {
            anchors.fill: parent
            anchors.margins: 12
            spacing: 8
            Text { text: root.tr("New name:") }
            AppTextField {
                id: renameTunnelField
                width: parent.width
            }
        }
    }

    Dialog {
        id: editPinPopup
        title: root.tr("Edit Package Pin")
        modal: true
        anchors.centerIn: parent
        padding: 0
        EditPinDialog {
            id: editPinContent
            onAccepted: editPinPopup.close()
            onDeleted: editPinPopup.close()
            onRejected: editPinPopup.close()
        }
    }

    Dialog {
        id: generatePinsPopup
        title: root.tr("Generate Pins")
        modal: true
        anchors.centerIn: parent
        padding: 0
        GeneratePinsDialog {
            id: generatePinsContent
            onAccepted: generatePinsPopup.close()
            onRejected: generatePinsPopup.close()
        }
    }

    FileDialog {
        id: loadPackageDialog
        title: root.tr("Load Package File")
        fileMode: FileDialog.OpenFile
        currentFolder: App.suggestFolderUrl(CircuitCanvas.filePath)
        nameFilters: [ root.tr("Package files (*.package)"), root.tr("All files (*)") ]
        onAccepted: {
            if (CircuitCanvas.selectedUid) {
                CircuitCanvas.loadPackageFile(CircuitCanvas.selectedUid, "" + selectedFile)
            }
        }
    }

    FileDialog {
        id: savePackageDialog
        title: root.tr("Export Package File")
        fileMode: FileDialog.SaveFile
        currentFolder: App.suggestFolderUrl(CircuitCanvas.filePath)
        defaultSuffix: "package"
        nameFilters: [ root.tr("Package files (*.package)"), root.tr("All files (*)") ]
        onAccepted: {
            if (CircuitCanvas.selectedUid) {
                CircuitCanvas.savePackageFile(CircuitCanvas.selectedUid, "" + selectedFile)
            }
        }
    }

    FileDialog {
        id: saveImageDialog
        title: root.tr("Save as Image")
        fileMode: FileDialog.SaveFile
        currentFolder: App.suggestFolderUrl(CircuitCanvas.filePath)
        defaultSuffix: "png"
        nameFilters: [
            root.tr("PNG (*.png)"),
            root.tr("JPEG (*.jpeg *.jpg)"),
            root.tr("BMP (*.bmp)"),
            root.tr("SVG (*.svg)"),
            root.tr("All (*.*)")
        ]
        currentFile: CircuitCanvas.suggestImageUrl || ""
        onAccepted: CircuitCanvas.saveImage("" + selectedFile)
    }

    FileDialog {
        id: importCircDialog
        title: root.tr("Import Circuit")
        fileMode: FileDialog.OpenFile
        currentFolder: App.suggestFolderUrl(CircuitCanvas.filePath)
        nameFilters: [ root.tr("Circuits (*.circ1 *.sim2 *.sim1)"), root.tr("All files (*)") ]
        onAccepted: CircuitCanvas.importPath("" + selectedFile)
    }

    WheelHandler {
        target: null
        acceptedDevices: PointerDevice.Mouse | PointerDevice.TouchPad
        acceptedModifiers: Qt.KeyboardModifierMask
        orientation: Qt.Horizontal | Qt.Vertical
        onWheel: (w) => CircuitCanvas.wheel(w.pixelDelta.x, w.pixelDelta.y,
                                            w.angleDelta.x, w.angleDelta.y,
                                            (w.point && w.point.position ? w.point.position.x : (w.position ? w.position.x : (w.x !== undefined ? w.x : mouse.mouseX))),
                                            (w.point && w.point.position ? w.point.position.y : (w.position ? w.position.y : (w.y !== undefined ? w.y : mouse.mouseY))),
                                            w.modifiers)
    }

    Timer {
        id: simTimer
        interval: Math.max(10, Math.round(1000 / Math.max(1, AppDialog.fps)))
        repeat: true
        running: CircuitCanvas.simRunning && !CircuitPanel.simPaused
        onTriggered: {
            CircuitCanvas.tick()
            SerialMonitor.sync()
        }
    }

    Timer {
        id: infoTimer
        interval: 100
        repeat: true
        running: CircuitCanvas.simRunning && !CircuitPanel.simPaused && CircuitPanel.infoVisible
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

    Keys.onPressed: (ev) => {
        if (ev.key === Qt.Key_Escape && CircuitCanvas.linkingFrom) {
            CircuitCanvas.stopLinking()
            ev.accepted = true
            return
        }
        CircuitCanvas.handleKey(ev.key, ev.modifiers)
        ev.accepted = true
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
