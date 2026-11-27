pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtQuick.Templates as T
import cs_app
import cs_ui

/* The circuit area: the canvas, and the 4-corner overlays floating over it.
 *
 * Five independent overlays (components, files, editor, simLog, compLog) live
 * in four corners (tl, bl, tr, br). Each overlay can be toggled between hover
 * mode (floating with drop shadow) and always-visible pinned mode (docked flush
 * with the window margin). All geometry lives in `cs_engine::overlays`.
 */
Item {
    id: root
    objectName: "circuitPanelRoot"
    SystemPalette { id: appTheme }
    readonly property var rects: CircuitPanel.overlayRects
    readonly property var emptyRectObj: ({ x: 0, y: 0, w: 0, h: 0 })
    readonly property var emptyPanelObj: ({
        visible: false, floating: false, pinned: false, activePinned: false,
        corner: "tl", isLeft: true, x: 0, y: 0, w: 0, h: 0, z: 1,
        button: { x: 0, y: 0, w: 0, h: 0 },
        resize: { x: 0, y: 0, w: 0, h: 0 }
    })
    readonly property var emptyCornerObj: ({
        x: 0, y: 0, w: 0, h: 0,
        pinned: { x: 0, y: 0, w: 0, h: 0 },
        hover: { x: 0, y: 0, w: 0, h: 0 }
    })
    readonly property var emptyHandleObj: ({ visible: false, x: 0, y: 0, w: 0, h: 0 })

    readonly property string floatingPanelId: (rects && rects.floatingPanel) ? String(rects.floatingPanel) : ""
    readonly property string floatingReason: (rects && rects.floatingReason) ? String(rects.floatingReason) : ""

    property string hoveredButtonId: ""
    property string hoveredPanelId: ""

    property bool dragActive: false
    property string dragPanelId: ""
    property real dragX: 0
    property real dragY: 0
    property string dragTargetCorner: ""

    // Overlay rects are JSON {x,y,w,h}, not QRectF.
    function r( key: string ): rect {
        var v = rects ? rects[key] : null
        if ( v === undefined || v === null )
            return Qt.rect( 0, 0, 0, 0 )
        var w = ( v.w !== undefined ) ? v.w : v.width
        var h = ( v.h !== undefined ) ? v.h : v.height
        return Qt.rect( v.x, v.y, w, h )
    }
    function overlayZ( key: string ): real { return (rects && rects[key + "Z"] !== undefined) ? rects[key + "Z"] : 5 }

    function panelInfo( id: string ): var {
        if ( rects && rects.panels && rects.panels[id] )
            return rects.panels[id]
        return root.emptyPanelObj
    }

    function cornerInfo( key: string ): var {
        if ( rects && rects.corners && rects.corners[key] )
            return rects.corners[key]
        return root.emptyCornerObj
    }

    function handleInfo( key: string ): var {
        if ( rects && rects[key] )
            return rects[key]
        return root.emptyHandleObj
    }

    function panelIcon( id: string ): string {
        if ( id === "components" ) return "developer_board"
        if ( id === "files" ) return "folder_open"
        if ( id === "editor" ) return "code"
        if ( id === "simLog" ) return "terminal"
        if ( id === "compLog" ) return "bug_report"
        return "dock_to_left"
    }

    function panelLabel( id: string ): string {
        var _tick = App.i18nTick
        if ( id === "components" ) return I18n.tr( "Components" )
        if ( id === "files" ) return I18n.tr( "Files" )
        if ( id === "editor" ) return I18n.tr( "Editor" )
        if ( id === "simLog" ) return I18n.tr( "Simulator Messages" )
        if ( id === "compLog" ) return I18n.tr( "Compiler Messages" )
        return id
    }

    function cornerLabel( key: string ): string {
        var _tick = App.i18nTick
        if ( key === "tl" ) return I18n.tr( "Top Left" )
        if ( key === "bl" ) return I18n.tr( "Bottom Left" )
        if ( key === "tr" ) return I18n.tr( "Top Right" )
        if ( key === "br" ) return I18n.tr( "Bottom Right" )
        return key
    }

    function cornerAtPoint( px: real, py: real ): string {
        var left = px < root.width / 2
        var top = py < ( root.topInset + root.height ) / 2
        return ( top ? "t" : "b" ) + ( left ? "l" : "r" )
    }

    function floatingInputFocused(): bool {
        if ( root.floatingPanelId === "components" && componentSearch.activeFocus )
            return true
        if ( root.floatingPanelId === "editor" && EditorPanel.focused )
            return true
        return false
    }

    function scheduleHoverClose(): void {
        if ( root.hoveredButtonId === "" && root.hoveredPanelId === "" && !floatResize.pressed )
            hoverCloseTimer.restart()
    }

    Timer {
        id: hoverCloseTimer
        interval: 90
        repeat: false
        onTriggered: {
            if ( root.hoveredButtonId === "" && root.hoveredPanelId === ""
                 && !floatResize.pressed && !cornerBtnMenu.opened && !root.floatingInputFocused() ) {
                CircuitPanel.setHoverPanel( "" )
            }
        }
    }

    Shortcut {
        sequence: "Escape"
        enabled: root.dragActive
                 || ( root.floatingPanelId !== ""
                      && !EditorPanel.completionVisible
                      && !CommandCenter.visible
                      && !TabSwitcher.visible
                      && !UiUtils.hasModalOverlay( root.Overlay.overlay, null ) )
        onActivated: {
            if ( root.dragActive ) {
                root.dragActive = false
                root.dragPanelId = ""
                root.dragTargetCorner = ""
            } else {
                CircuitPanel.dismissFloating()
            }
        }
    }

    function toolbarOpensUpward(): bool {
        return toolbar.y + toolbar.height / 2 > ( root.topInset + root.height ) / 2
    }

    // Free gap on the side the popup opens into, inside the 12px screen inset
    // (equal top padding below the custom title bar when one is active).
    function toolbarPopupRoom(): real {
        var inset = 12
        var gap = 4
        var minY = root.topInset + inset
        var limitBottom = root.height - inset
        if ( root.toolbarOpensUpward() )
            return Math.max( 0, toolbar.y - gap - minY )
        return Math.max( 0, limitBottom - ( toolbar.y + toolbar.height + gap ) )
    }

    // Top of a popup whose height is already the fitted one. The edge that
    // faces the toolbar stays 4px clear; clamping this top used to slide a
    // tall popup onto the bar.
    function toolbarPopupY( shownHeight: real ): real {
        var gap = 4
        if ( root.toolbarOpensUpward() )
            return toolbar.y - gap - shownHeight
        return toolbar.y + toolbar.height + gap
    }

    // Shrink a menu only when it does not fit. A menu that fits keeps its
    // implicit height; a previous shrink is cleared so the next open can grow.
    function fitMenuHeight( menu: T.Popup, naturalHeight: real, shownHeight: real ): void {
        if ( shownHeight < naturalHeight )
            menu.height = shownHeight
        else if ( menu.height + 0.5 < naturalHeight )
            menu.height = Qt.binding( function() { return menu.implicitHeight } )
    }

    // Place a popup on the toolbar's near side. `naturalHeight` is the popup's
    // own size (a closed Menu reports 0, so pass the measured one). Popups whose
    // height stays bound to their content take `capToRoom` and a `heightCap`
    // property; writing `height` would drop that binding.
    function clampPopupX( rawX: real, w: real ): real {
        var avail = root.r( "availableArea" )
        if ( avail.width >= w && avail.width > 0 )
            return Math.max( avail.x, Math.min( avail.x + avail.width - w, rawX ) )
        return Math.max( 8, Math.min( root.width - w - 8, rawX ) )
    }

    function placeToolbarPopup( popup: T.Popup, anchor: Item, alignRight: bool,
                                width: real, naturalHeight: real, capToRoom: bool ): void {
        var room = root.toolbarPopupRoom()
        var shownH = Math.max( 0, Math.min( naturalHeight, room ) )
        if ( capToRoom )
            popup.heightCap = shownH
        else
            root.fitMenuHeight( popup, naturalHeight, shownH )
        var w = width > 0 ? width : popup.width
        var localX = alignRight ? anchor.width - w : 0
        var pt = anchor.mapToItem( root, localX, 0 )
        popup.x = root.clampPopupX( pt.x, w )
        popup.y = root.toolbarPopupY( shownH )
    }

    function positionAppMenu(): void {
        appMenu.updateGeometry()
        var menuW = Math.max( appMenu.implicitWidth,
                              appMenu.contentImplicitWidth + appMenu.leftPadding + appMenu.rightPadding )
        var menuH = Math.max( appMenu.implicitHeight,
                              appMenu.contentImplicitHeight + appMenu.topPadding + appMenu.bottomPadding )
        var shownH = Math.max( 0, Math.min( menuH, root.toolbarPopupRoom() ) )
        root.fitMenuHeight( appMenu, menuH, shownH )
        var anchorPt = appMenuBtn.mapToItem( root, 0, 0 )
        var targetX = root.clampPopupX( anchorPt.x, menuW )
        var targetY = root.toolbarPopupY( shownH )
        var localPt = root.mapToItem( appMenuBtn, targetX, targetY )
        appMenu.x = localPt.x
        appMenu.y = localPt.y
    }

    property int topInset: 0
    onTopInsetChanged: CircuitPanel.setTopInset( topInset )

    property bool customTitlebar: App.customTitlebar
    onCustomTitlebarChanged: CircuitPanel.setCustomTitlebar( customTitlebar )

    property bool inlineMenuBarVisible: false
    onInlineMenuBarVisibleChanged: {
        if ( !inlineMenuBarVisible )
            return
        if ( appMenu.opened )
            root.closeAppMenu()
        // opened clears after this is already true, so onOpenedChanged skips
        // the flag and would leave the next bare Alt eaten.
        AppMenuBar.setAltMenuOpen( false )
    }

    // CircuitWidget lays every overlay out against the canvas area's size, which
    // it used to take from resizeEvent(). Nothing reported it after the port, so
    // m_viewSize stayed (-1,-1) and every panel came out with a negative height,
    // which is why the side and editor panels never appeared.
    function publishViewSize(): void { CircuitPanel.setViewSize( root.width, root.height ) }
    function syncCanvasFitArea(): void {
        var fa = root.r( "fitArea" )
        CircuitCanvas.setFitArea( fa.x, fa.y, fa.width, fa.height )
    }
    onRectsChanged: root.syncCanvasFitArea()
    onWidthChanged: root.publishViewSize()
    onHeightChanged: root.publishViewSize()
    Component.onCompleted: {
        CircuitPanel.setTopInset( root.topInset )
        CircuitPanel.setCustomTitlebar( root.customTitlebar )
        root.publishViewSize()
        root.syncCanvasFitArea()
        CircuitPanel.setZoomPercent( CircuitCanvas.zoom )
        CircuitPanel.resetSubcircuits( CircuitCanvas.filePath, CircuitCanvas.fileName, CircuitCanvas.subcircuitTree )
        CircuitPanel.updateDevices( CircuitCanvas.programmableDevices )
        CircuitPanel.setCanvasOverflow( CircuitCanvas.canvasOverflow ? App.translate("Canvas Overflow") : "" )
        CircuitPanel.setCompileStatus( EditorPanel.compileStatus )
    }



    // ---- Canvas ----------------------------------------------------------
    Loader {
        id: canvasLoader
        anchors.fill: parent
        focus: true
        source: "CircuitView.qml"
    }

    // ---- Reusable 4-corner overlay shell ---------------------------------
    component OverlayShell: AppSurface {
        id: shell
        required property string panelId
        property int contentMargin: 6
        readonly property var info: root.panelInfo( shell.panelId )
        readonly property bool shown: shell.info.visible === true
        readonly property bool isFloatingNow: shell.info.floating === true
        default property alias bodyContent: bodyHost.data

        // Snapshot of last live rect so hover close animations fade & slide in place.
        property real lastX: 0
        property real lastY: 0
        property real lastW: 0
        property real lastH: 0
        property real lastZ: 7
        property bool lastIsLeft: true
        property bool wasFloating: false
        property real animProgress: 0.0

        readonly property bool animateMode: shell.shown ? shell.isFloatingNow : shell.wasFloating
        readonly property bool effectiveIsLeft: shell.shown ? ( shell.info.isLeft !== false ) : shell.lastIsLeft

        Component.onCompleted: {
            if ( shell.info && shell.info.visible === true ) {
                shell.lastX = shell.info.x
                shell.lastY = shell.info.y
                shell.lastW = shell.info.w
                shell.lastH = shell.info.h
                shell.lastZ = shell.info.z
                shell.lastIsLeft = shell.info.isLeft !== false
                shell.wasFloating = shell.info.floating === true
                shell.animProgress = 1.0
            }
        }

        onInfoChanged: {
            if ( shell.info && shell.info.visible === true ) {
                shell.lastX = shell.info.x
                shell.lastY = shell.info.y
                shell.lastW = shell.info.w
                shell.lastH = shell.info.h
                shell.lastZ = shell.info.z
                shell.lastIsLeft = shell.info.isLeft !== false
                shell.wasFloating = shell.info.floating === true
            }
        }

        onShownChanged: {
            hoverOpenAnim.stop()
            hoverCloseAnim.stop()
            if ( shell.shown ) {
                if ( shell.isFloatingNow ) {
                    hoverOpenAnim.from = shell.animProgress
                    hoverOpenAnim.start()
                } else {
                    shell.animProgress = 1.0
                }
            } else {
                if ( root.hoveredPanelId === shell.panelId )
                    root.hoveredPanelId = ""
                if ( shell.wasFloating ) {
                    hoverCloseAnim.from = shell.animProgress
                    hoverCloseAnim.start()
                } else {
                    shell.animProgress = 0.0
                }
            }
        }

        NumberAnimation {
            id: hoverOpenAnim
            target: shell
            property: "animProgress"
            to: 1.0
            duration: 130
            easing.type: Easing.OutCubic
        }

        NumberAnimation {
            id: hoverCloseAnim
            target: shell
            property: "animProgress"
            to: 0.0
            duration: 100
            easing.type: Easing.InCubic
        }

        visible: shell.shown || shell.animProgress > 0.005
        enabled: shell.shown
        x: shell.shown ? shell.info.x : shell.lastX
        y: shell.shown ? shell.info.y : shell.lastY
        width: shell.shown ? shell.info.w : shell.lastW
        height: shell.shown ? shell.info.h : shell.lastH
        z: shell.shown ? shell.info.z : ( shell.lastZ - 0.2 )
        opacity: ( shell.shown && !shell.isFloatingNow ) ? 1.0 : shell.animProgress
        surfaceColor: appTheme.window
        radius: 14
        shadowEnabled: shell.animateMode
        elevation: 1.0

        transform: Translate {
            x: shell.animateMode
               ? ( 1.0 - shell.animProgress ) * ( shell.effectiveIsLeft ? -12 : 12 )
               : 0
        }

        // Absorbs unhandled pointer events so clicks/scrolls never leak to the canvas.
        MouseArea {
            anchors.fill: parent
            hoverEnabled: true
            acceptedButtons: Qt.AllButtons
            onPressed: (m) => { CircuitPanel.overlayClicked( shell.panelId ) }
            onWheel: (wheel) => { wheel.accepted = true }
        }

        WheelHandler {
            target: null
            acceptedDevices: PointerDevice.Mouse | PointerDevice.TouchPad
            orientation: Qt.Horizontal | Qt.Vertical
        }

        HoverHandler {
            id: shellHover
            enabled: shell.shown
            onHoveredChanged: {
                if ( shell.shown && shellHover.hovered ) {
                    hoverCloseTimer.stop()
                    root.hoveredPanelId = shell.panelId
                } else if ( root.hoveredPanelId === shell.panelId ) {
                    root.hoveredPanelId = ""
                    root.scheduleHoverClose()
                }
            }
        }

        DropArea {
            anchors.fill: parent
            onEntered: (drag) => { drag.accepted = true }
            onDropped: (drop) => { drop.accept() }
        }

        Item {
            id: bodyHost
            anchors.fill: parent
            anchors.margins: shell.contentMargin
        }
    }

    // ---- Components overlay ----------------------------------------------
    OverlayShell {
        id: componentsOverlay
        objectName: "componentsPanel"
        panelId: "components"

        function clearComponentHover(): void {
            var view = componentListLoader.item as ComponentListView
            if ( view )
                view.clearHover()
        }
        onVisibleChanged: if ( !visible ) clearComponentHover()
        onXChanged: clearComponentHover()
        onYChanged: clearComponentHover()
        onWidthChanged: clearComponentHover()
        onHeightChanged: clearComponentHover()

        ColumnLayout {
            anchors.fill: parent
            spacing: 4

            AppTextField {
                id: componentSearch
                objectName: "componentSearch"
                Layout.fillWidth: true
                placeholderText: I18n.tr( "Search components" )
                onPressed: CircuitPanel.overlayClicked( "components" )
                onActiveFocusChanged: {
                    if ( activeFocus && componentsOverlay.info.floating === true )
                        CircuitPanel.openPanel( "components" )
                    else if ( !activeFocus )
                        root.scheduleHoverClose()
                }
                onTextChanged: {
                    App.setSearchFilter( componentSearch.text )
                    ComponentList.search( componentSearch.text )
                }

                Connections {
                    target: App
                    function onFocusSearchRequested(): void {
                        CircuitPanel.showComponents()
                        componentSearch.forceActiveFocus()
                        componentSearch.selectAll()
                    }
                }
            }

            Rectangle {
                Layout.fillWidth: true
                Layout.fillHeight: true
                radius: 10
                color: appTheme.base
                border.color: appTheme.mid
                border.width: 1
                clip: true

                Loader {
                    id: componentListLoader
                    anchors.fill: parent
                    source: "ComponentListView.qml"
                }
            }
        }
    }

    ComponentHoverCard {
        id: componentHoverCard
        readonly property ComponentListView listView: componentListLoader.item as ComponentListView
        active: componentsOverlay.visible && listView !== null && listView.hoverCardVisible
        info: listView ? listView.hoveredItemInfo : null
        anchorY: {
            var _panelY = componentsOverlay.y
            return ( listView && listView.hoverCardVisible )
                ? componentListLoader.mapToItem( root, 0, listView.hoveredRowCenterY ).y
                : 0
        }
        panelRect: Qt.rect( componentsOverlay.x, componentsOverlay.y, componentsOverlay.width, componentsOverlay.height )
        preferRight: componentsOverlay.info.isLeft !== false
        topInset: root.topInset
    }

    // ---- File Tree overlay -----------------------------------------------
    OverlayShell {
        id: filesOverlay
        objectName: "filesPanel"
        panelId: "files"

        Loader {
            objectName: "fileBrowser"
            anchors.fill: parent
            source: "FileBrowserView.qml"
        }
    }

    // ---- File Editor overlay ---------------------------------------------
    OverlayShell {
        id: editorOverlay
        objectName: "editorPanel"
        panelId: "editor"
        contentMargin: 1

        Loader {
            anchors.fill: parent
            source: "EditorPanelView.qml"
        }
    }

    // ---- Simulator Messages overlay --------------------------------------
    OverlayShell {
        id: simLogOverlay
        objectName: "simLogPanel"
        panelId: "simLog"

        ColumnLayout {
            anchors.fill: parent
            spacing: 4

            RowLayout {
                Layout.fillWidth: true
                Layout.leftMargin: 4
                Layout.rightMargin: 2
                spacing: 6

                AppIcon {
                    icon: "terminal"
                    size: 16
                    color: appTheme.windowText
                    opacity: 0.8
                }
                AppText {
                    text: I18n.tr( "Simulator Messages" )
                    font.pixelSize: App.fontSmall
                    font.bold: true
                    color: appTheme.windowText
                    elide: Text.ElideRight
                    Layout.fillWidth: true
                }
                AppToolButton {
                    Layout.preferredWidth: 22
                    Layout.preferredHeight: 22
                    padding: 0
                    ToolTip.visible: hovered
                    ToolTip.text: I18n.tr( "Clear Log" )
                    onClicked: SimulatorLog.clear()
                    contentItem: Item {
                        implicitWidth: 16
                        implicitHeight: 16
                        AppIcon {
                            anchors.centerIn: parent
                            icon: "delete"
                            size: 15
                            color: appTheme.windowText
                        }
                    }
                }
            }

            Rectangle {
                Layout.fillWidth: true
                Layout.fillHeight: true
                radius: 10
                color: appTheme.base
                border.color: appTheme.mid
                border.width: 1
                clip: true

                Loader {
                    id: simLogLoader
                    anchors.fill: parent
                    source: "OutPanelText.qml"
                    onLoaded: (simLogLoader.item as OutPanelText).ctx = SimulatorLog
                }
            }
        }
    }

    // ---- Compiler Messages overlay ---------------------------------------
    OverlayShell {
        id: compLogOverlay
        objectName: "compLogPanel"
        panelId: "compLog"

        ColumnLayout {
            anchors.fill: parent
            spacing: 4

            RowLayout {
                Layout.fillWidth: true
                Layout.leftMargin: 4
                Layout.rightMargin: 2
                spacing: 6

                AppIcon {
                    icon: "bug_report"
                    size: 16
                    color: appTheme.windowText
                    opacity: 0.8
                }
                AppText {
                    text: I18n.tr( "Compiler Messages" )
                    font.pixelSize: App.fontSmall
                    font.bold: true
                    color: appTheme.windowText
                    elide: Text.ElideRight
                    Layout.fillWidth: true
                }
                AppToolButton {
                    Layout.preferredWidth: 22
                    Layout.preferredHeight: 22
                    padding: 0
                    ToolTip.visible: hovered
                    ToolTip.text: I18n.tr( "Clear Log" )
                    onClicked: CompilerLog.clear()
                    contentItem: Item {
                        implicitWidth: 16
                        implicitHeight: 16
                        AppIcon {
                            anchors.centerIn: parent
                            icon: "delete"
                            size: 15
                            color: appTheme.windowText
                        }
                    }
                }
            }

            Rectangle {
                Layout.fillWidth: true
                Layout.fillHeight: true
                radius: 10
                color: appTheme.base
                border.color: appTheme.mid
                border.width: 1
                clip: true

                Loader {
                    id: compLogLoader
                    anchors.fill: parent
                    source: "OutPanelText.qml"
                    onLoaded: (compLogLoader.item as OutPanelText).ctx = CompilerLog
                }
            }
        }
    }

    // ---- Vertical split drag handles (MCU Monitor SplitView style) -------
    Repeater {
        model: ["leftSplit", "rightSplit"]
        delegate: Item {
            id: splitHandle
            required property string modelData
            readonly property var hInfo: root.handleInfo( splitHandle.modelData )
            visible: hInfo.visible === true
            x: hInfo.x
            y: hInfo.y
            width: hInfo.w
            height: hInfo.h
            z: 3

            Rectangle {
                anchors.centerIn: parent
                width: Math.min( 36, parent.width * 0.3 )
                height: 2
                radius: 1
                color: ( splitMouse.pressed || splitMouse.containsMouse ) ? appTheme.highlight : appTheme.mid
            }

            MouseArea {
                id: splitMouse
                anchors.fill: parent
                hoverEnabled: true
                cursorShape: Qt.SizeVerCursor
                onPressed: (m) => {
                    var p = splitMouse.mapToItem( root, m.x, m.y )
                    CircuitPanel.gestureBegin( splitHandle.modelData, p.x, p.y )
                }
                onPositionChanged: (m) => {
                    if ( splitMouse.pressed ) {
                        var p = splitMouse.mapToItem( root, m.x, m.y )
                        CircuitPanel.gestureMove( p.x, p.y )
                    }
                }
                onReleased: CircuitPanel.gestureEnd()
                onDoubleClicked: CircuitPanel.resetOverlaySize( splitHandle.modelData )
            }
        }
    }

    // ---- Horizontal width resize strips (docked columns + floating panel) -
    Repeater {
        model: ["leftResize", "rightResize"]
        delegate: MouseArea {
            id: colResize
            required property string modelData
            readonly property var hInfo: root.handleInfo( colResize.modelData )
            visible: hInfo.visible === true
            x: hInfo.x
            y: hInfo.y
            width: hInfo.w
            height: hInfo.h
            z: 3
            cursorShape: Qt.SizeHorCursor
            onPressed: (m) => {
                var p = colResize.mapToItem( root, m.x, m.y )
                CircuitPanel.gestureBegin( colResize.modelData, p.x, p.y )
            }
            onPositionChanged: (m) => {
                if ( colResize.pressed ) {
                    var p = colResize.mapToItem( root, m.x, m.y )
                    CircuitPanel.gestureMove( p.x, p.y )
                }
            }
            onReleased: CircuitPanel.gestureEnd()
            onDoubleClicked: CircuitPanel.resetOverlaySize( colResize.modelData )
        }
    }

    MouseArea {
        id: floatResize
        readonly property var fpInfo: root.floatingPanelId !== "" ? root.panelInfo( root.floatingPanelId ) : root.emptyPanelObj
        readonly property var rr: fpInfo.resize || root.emptyRectObj
        visible: root.floatingPanelId !== "" && fpInfo.visible === true && fpInfo.floating === true && rr.w > 0
        x: rr.x
        y: rr.y
        width: rr.w
        height: rr.h
        z: 7.5
        cursorShape: Qt.SizeHorCursor
        onPressed: (m) => {
            hoverCloseTimer.stop()
            var p = floatResize.mapToItem( root, m.x, m.y )
            CircuitPanel.gestureBegin( "floatResize", p.x, p.y )
        }
        onPositionChanged: (m) => {
            if ( floatResize.pressed ) {
                var p = floatResize.mapToItem( root, m.x, m.y )
                CircuitPanel.gestureMove( p.x, p.y )
            }
        }
        onReleased: {
            CircuitPanel.gestureEnd()
            root.scheduleHoverClose()
        }
        onDoubleClicked: CircuitPanel.resetOverlaySize( "floatResize" )
    }

    // ---- 4 Corner pill backgrounds (separate pinned & hover pills) -------
    Repeater {
        model: ["tl", "bl", "tr", "br"]
        delegate: Item {
            id: cornerGroupBg
            required property string modelData
            readonly property var cInfo: root.cornerInfo( cornerGroupBg.modelData )
            readonly property var pinRect: cInfo.pinned || root.emptyRectObj
            readonly property var hovRect: cInfo.hover || root.emptyRectObj

            // Pinned (always-visible) group pill: subtle accent border distinguishes it from hover pills.
            AppSurface {
                visible: cornerGroupBg.pinRect.w > 0 && cornerGroupBg.pinRect.h > 0
                x: cornerGroupBg.pinRect.x
                y: cornerGroupBg.pinRect.y
                width: cornerGroupBg.pinRect.w
                height: cornerGroupBg.pinRect.h
                radius: 10
                surfaceColor: appTheme.base
                border.color: Qt.rgba( appTheme.highlight.r, appTheme.highlight.g, appTheme.highlight.b, 0.45 )
                z: 4
            }

            // Hover-mode group pill: neutral surface.
            AppSurface {
                visible: cornerGroupBg.hovRect.w > 0 && cornerGroupBg.hovRect.h > 0
                x: cornerGroupBg.hovRect.x
                y: cornerGroupBg.hovRect.y
                width: cornerGroupBg.hovRect.w
                height: cornerGroupBg.hovRect.h
                radius: 10
                surfaceColor: appTheme.base
                z: 4
            }
        }
    }

    // ---- 5 Persistent corner icon buttons (never recreated on layout updates)
    Repeater {
        model: ["components", "files", "editor", "simLog", "compLog"]
        delegate: Rectangle {
            id: cornerBtn
            required property string modelData
            readonly property string panelId: cornerBtn.modelData
            readonly property var info: root.panelInfo( cornerBtn.panelId )
            readonly property var btnRect: info.button || root.emptyRectObj
            readonly property bool isPinned: info.pinned === true
            readonly property bool isActivePinned: info.activePinned === true
            readonly property bool isFloating: info.floating === true
            readonly property bool isDraggingThis: root.dragActive && root.dragPanelId === cornerBtn.panelId

            visible: cornerBtn.btnRect.w > 0 && cornerBtn.btnRect.h > 0
            x: cornerBtn.btnRect.x
            y: cornerBtn.btnRect.y
            width: cornerBtn.btnRect.w
            height: cornerBtn.btnRect.h
            radius: 7
            z: 5
            opacity: cornerBtn.isDraggingThis ? 0.35 : 1.0

            // Clear visual hierarchy:
            // - Active pinned: solid highlight fill + highlightedText icon
            // - Inactive pinned (same corner): soft highlight tint + highlight border + pin dot
            // - Hover mode: neutral transparent button + muted icon (brightens on hover/floating)
            color: cornerBtn.isActivePinned
                     ? ( btnMouse.pressed ? Qt.darker( appTheme.highlight, 1.12 ) : appTheme.highlight )
                 : cornerBtn.isPinned
                     ? ( btnMouse.pressed
                           ? Qt.rgba( appTheme.highlight.r, appTheme.highlight.g, appTheme.highlight.b, 0.30 )
                           : btnMouse.containsMouse
                               ? Qt.rgba( appTheme.highlight.r, appTheme.highlight.g, appTheme.highlight.b, 0.24 )
                               : Qt.rgba( appTheme.highlight.r, appTheme.highlight.g, appTheme.highlight.b, 0.14 ) )
                 : btnMouse.pressed
                     ? Qt.rgba( appTheme.windowText.r, appTheme.windowText.g, appTheme.windowText.b, 0.18 )
                 : ( btnMouse.containsMouse || cornerBtn.isFloating )
                     ? Qt.rgba( appTheme.windowText.r, appTheme.windowText.g, appTheme.windowText.b, 0.10 )
                     : "transparent"

            border.width: ( cornerBtn.isPinned && !cornerBtn.isActivePinned ) ? 1 : 0
            border.color: Qt.rgba( appTheme.highlight.r, appTheme.highlight.g, appTheme.highlight.b, 0.55 )

            Behavior on color { ColorAnimation { duration: 80 } }

            AppIcon {
                anchors.centerIn: parent
                icon: root.panelIcon( cornerBtn.panelId )
                size: 16
                color: cornerBtn.isActivePinned
                       ? appTheme.highlightedText
                       : cornerBtn.isPinned
                           ? appTheme.highlight
                           : appTheme.windowText
                opacity: ( cornerBtn.isPinned || btnMouse.containsMouse || cornerBtn.isFloating ) ? 1.0 : 0.68
            }

            // Pin indicator dot when another pinned panel is active in the same corner.
            Rectangle {
                visible: cornerBtn.isPinned && !cornerBtn.isActivePinned
                width: 4
                height: 4
                radius: 2
                anchors.right: parent.right
                anchors.bottom: parent.bottom
                anchors.rightMargin: 3
                anchors.bottomMargin: 3
                color: appTheme.highlight
            }

            Timer {
                id: singleClickTimer
                interval: 210
                repeat: false
                onTriggered: CircuitPanel.clickCornerButton( cornerBtn.panelId )
            }

            MouseArea {
                id: btnMouse
                anchors.fill: parent
                hoverEnabled: true
                acceptedButtons: Qt.LeftButton | Qt.RightButton
                cursorShape: root.dragActive ? Qt.ClosedHandCursor : Qt.PointingHandCursor

                property real pressRootX: 0
                property real pressRootY: 0
                property bool leftDown: false

                onEntered: {
                    if ( !root.dragActive ) {
                        hoverCloseTimer.stop()
                        root.hoveredButtonId = cornerBtn.panelId
                        CircuitPanel.setHoverPanel( cornerBtn.panelId )
                    }
                }
                onExited: {
                    if ( root.hoveredButtonId === cornerBtn.panelId ) {
                        root.hoveredButtonId = ""
                        root.scheduleHoverClose()
                    }
                }
                onPressed: (m) => {
                    if ( m.button === Qt.RightButton ) {
                        singleClickTimer.stop()
                        hoverCloseTimer.stop()
                        cornerBtnMenu.openFor( cornerBtn.panelId, String( cornerBtn.info.corner || "tl" ), cornerBtn.isPinned )
                        return
                    }
                    var pt = btnMouse.mapToItem( root, m.x, m.y )
                    pressRootX = pt.x
                    pressRootY = pt.y
                    leftDown = true
                }
                onPositionChanged: (m) => {
                    if ( !leftDown )
                        return
                    var pt = btnMouse.mapToItem( root, m.x, m.y )
                    if ( !root.dragActive ) {
                        var dx = pt.x - pressRootX
                        var dy = pt.y - pressRootY
                        if ( dx * dx + dy * dy >= 36 ) {
                            singleClickTimer.stop()
                            root.dragActive = true
                            root.dragPanelId = cornerBtn.panelId
                            CircuitPanel.setHoverPanel( "" )
                        }
                    }
                    if ( root.dragActive ) {
                        root.dragX = pt.x
                        root.dragY = pt.y
                        root.dragTargetCorner = root.cornerAtPoint( pt.x, pt.y )
                    }
                }
                onReleased: (m) => {
                    if ( m.button !== Qt.LeftButton )
                        return
                    var wasDrag = root.dragActive
                    var draggedId = root.dragPanelId
                    var targetCorner = root.dragTargetCorner
                    leftDown = false
                    root.dragActive = false
                    root.dragPanelId = ""
                    root.dragTargetCorner = ""

                    if ( wasDrag ) {
                        singleClickTimer.stop()
                        if ( draggedId !== "" && targetCorner !== "" )
                            CircuitPanel.movePanelToCorner( draggedId, targetCorner )
                    } else if ( singleClickTimer.running ) {
                        singleClickTimer.stop()
                        CircuitPanel.doubleClickCornerButton( cornerBtn.panelId )
                    } else {
                        singleClickTimer.restart()
                    }
                }
                onCanceled: {
                    leftDown = false
                    singleClickTimer.stop()
                    root.dragActive = false
                    root.dragPanelId = ""
                    root.dragTargetCorner = ""
                }

                ToolTip.visible: btnMouse.containsMouse && !root.dragActive && !cornerBtnMenu.opened
                ToolTip.delay: 500
                ToolTip.text: {
                    var name = root.panelLabel( cornerBtn.panelId )
                    if ( cornerBtn.isActivePinned )
                        return name + " — " + I18n.tr( "Docked (click to collapse, double-click for hover mode, drag to move)" )
                    if ( cornerBtn.isPinned )
                        return name + " — " + I18n.tr( "Docked (click to show, double-click for hover mode, drag to move)" )
                    return name + " — " + I18n.tr( "Hover mode (click to open/close, double-click to dock, drag to move)" )
                }
            }
        }
    }

    // ---- Corner button context menu --------------------------------------
    AppContextMenu {
        id: cornerBtnMenu
        popupType: Popup.Item
        property string targetPanelId: ""
        property string targetCorner: "tl"
        property bool targetPinned: false

        function openFor( panelId: string, corner: string, pinned: bool ): void {
            targetPanelId = panelId
            targetCorner = corner
            targetPinned = pinned
            popup()
        }

        ContextMenuItem {
            text: cornerBtnMenu.targetPinned ? I18n.tr( "Switch to Hover Mode" ) : I18n.tr( "Switch to Docked Mode" )
            iconLigature: cornerBtnMenu.targetPinned ? "visibility_off" : "visibility"
            onTriggered: CircuitPanel.setPanelPinned( cornerBtnMenu.targetPanelId, !cornerBtnMenu.targetPinned )
        }
        ContextMenuSeparator {}
        ContextMenuItem {
            text: I18n.tr( "Move to Top Left" )
            checkable: true
            checked: cornerBtnMenu.targetCorner === "tl"
            onTriggered: CircuitPanel.movePanelToCorner( cornerBtnMenu.targetPanelId, "tl" )
        }
        ContextMenuItem {
            text: I18n.tr( "Move to Bottom Left" )
            checkable: true
            checked: cornerBtnMenu.targetCorner === "bl"
            onTriggered: CircuitPanel.movePanelToCorner( cornerBtnMenu.targetPanelId, "bl" )
        }
        ContextMenuItem {
            text: I18n.tr( "Move to Top Right" )
            checkable: true
            checked: cornerBtnMenu.targetCorner === "tr"
            onTriggered: CircuitPanel.movePanelToCorner( cornerBtnMenu.targetPanelId, "tr" )
        }
        ContextMenuItem {
            text: I18n.tr( "Move to Bottom Right" )
            checkable: true
            checked: cornerBtnMenu.targetCorner === "br"
            onTriggered: CircuitPanel.movePanelToCorner( cornerBtnMenu.targetPanelId, "br" )
        }
        ContextMenuSeparator {}
        ContextMenuItem {
            text: I18n.tr( "Reset Overlay Layout" )
            iconLigature: "restart_alt"
            onTriggered: CircuitPanel.resetOverlayLayout()
        }
    }

    // ---- 4-Corner drag drop targets & ghost pill -------------------------
    Repeater {
        model: ["tl", "bl", "tr", "br"]
        delegate: Rectangle {
            id: dropZone
            required property string modelData
            readonly property bool isTarget: root.dragTargetCorner === dropZone.modelData
            readonly property bool isLeft: dropZone.modelData === "tl" || dropZone.modelData === "bl"
            readonly property bool isTop: dropZone.modelData === "tl" || dropZone.modelData === "tr"
            readonly property int pad: 12
            readonly property int topY: root.topInset + pad
            readonly property int zoneW: Math.min( 160, Math.max( 90, root.width * 0.22 ) )
            readonly property int zoneH: Math.min( 110, Math.max( 64, ( root.height - topY - pad ) * 0.26 ) )

            visible: root.dragActive
            z: 20
            x: isLeft ? pad : ( root.width - pad - zoneW )
            y: isTop ? topY : ( root.height - pad - zoneH )
            width: zoneW
            height: zoneH
            radius: 14
            color: isTarget
                   ? Qt.rgba( appTheme.highlight.r, appTheme.highlight.g, appTheme.highlight.b, 0.22 )
                   : Qt.rgba( appTheme.window.r, appTheme.window.g, appTheme.window.b, 0.72 )
            border.width: isTarget ? 2 : 1
            border.color: isTarget ? appTheme.highlight : appTheme.mid

            AppText {
                anchors.centerIn: parent
                text: root.cornerLabel( dropZone.modelData )
                font.pixelSize: App.fontSmall
                font.bold: dropZone.isTarget
                color: dropZone.isTarget ? appTheme.highlight : appTheme.windowText
                opacity: dropZone.isTarget ? 1.0 : 0.75
            }
        }
    }

    AppSurface {
        id: dragGhost
        visible: root.dragActive && root.dragPanelId !== ""
        z: 21
        radius: height / 2
        shadowEnabled: true
        elevation: 1.0
        width: ghostRow.implicitWidth + 20
        height: 30
        x: Math.max( 8, Math.min( root.width - width - 8, root.dragX - width / 2 ) )
        y: Math.max( root.topInset + 8, Math.min( root.height - height - 8, root.dragY - height / 2 ) )

        Row {
            id: ghostRow
            anchors.centerIn: parent
            spacing: 6

            AppIcon {
                anchors.verticalCenter: parent.verticalCenter
                icon: root.panelIcon( root.dragPanelId )
                size: 16
                color: appTheme.highlight
            }
            AppText {
                anchors.verticalCenter: parent.verticalCenter
                text: root.panelLabel( root.dragPanelId )
                font.pixelSize: App.fontSmall
                font.bold: true
                color: appTheme.windowText
            }
        }
    }

    // ---- Overlay toolbar --------------------------------------------------
    // Control, not Frame: Frame on macOS uses the native NSBox style which enforces
    // hardcoded native insets (10px top, 20px bottom) that ignore `padding`.
    // Control respects custom padding and background customization cleanly.
    Control {
        id: toolbar
        x: root.r("toolbar").x; y: root.r("toolbar").y
        // Extra horizontal breathing room inside the pill caps, plus any extra
        // vertical clearance if taller text/dropdown items expand the row height.
        readonly property real endPadding: 5 + Math.max( 0, Math.round( ( toolbarRow.implicitHeight - toolbarGrip.implicitHeight ) / 2 ) )
        topPadding: 4
        bottomPadding: 4
        leftPadding: toolbar.endPadding
        rightPadding: toolbar.endPadding
        // Its x/y come back from updateOverlayLayout(), which needs the size it
        // just measured; this is the adjustSize() the QToolBar used to do.
        onImplicitWidthChanged: CircuitPanel.setToolbarSize( toolbar.implicitWidth, toolbar.implicitHeight )
        onImplicitHeightChanged: CircuitPanel.setToolbarSize( toolbar.implicitWidth, toolbar.implicitHeight )
        Component.onCompleted: CircuitPanel.setToolbarSize( toolbar.implicitWidth, toolbar.implicitHeight )
        z: root.overlayZ( "toolbar" )

        // Click-to-front lives inside the background, not as a sibling of the
        // Row: a Frame measures its implicit size from its one contentItem, and
        // a second loose child anchored to `parent` (whose size depends on that
        // same measurement) is a circular binding that collapsed the toolbar to
        // a near-zero "dot". Inside the background, the MouseArea's `parent` is
        // the already-sized Rectangle, so nothing feeds back into the sizing.
        background: AppSurface {
            radius: height / 2

            // Click-to-front: bring toolbar to front when clicking background/margins
            MouseArea {
                anchors.fill: parent
                onPressed: (m) => { CircuitPanel.overlayClicked( "toolbar" ) }
            }
        }

        contentItem: Row {
            id: toolbarRow
            spacing: 3

            // Drag handle: first item inside the toolbar
            Item {
                id: toolbarGrip
                implicitWidth: 26
                implicitHeight: 26
                anchors.verticalCenter: toolbarRow.verticalCenter

                Rectangle {
                    anchors.fill: parent
                    radius: height / 2
                    color: gripMouse.pressed
                             ? Qt.rgba( appTheme.windowText.r, appTheme.windowText.g, appTheme.windowText.b, 0.22 )
                             : gripMouse.containsMouse
                                 ? Qt.rgba( appTheme.windowText.r, appTheme.windowText.g, appTheme.windowText.b, 0.10 )
                                 : "transparent"
                    Behavior on color { ColorAnimation { duration: 90 } }
                }

                AppIcon {
                    anchors.centerIn: parent
                    icon: "drag_indicator"
                    size: 16
                    color: appTheme.windowText
                }

                MouseArea {
                    id: gripMouse
                    anchors.fill: parent
                    hoverEnabled: true
                    cursorShape: Qt.SizeAllCursor
                    onDoubleClicked: CircuitPanel.resetOverlayPos( "toolbar" )
                    onPressed: (m) => {
                        CircuitPanel.overlayClicked( "toolbar" )
                        var p = gripMouse.mapToItem( root, m.x, m.y )
                        CircuitPanel.gestureBegin( "toolbar", p.x, p.y )
                    }
                    onPositionChanged: (m) => { var p = gripMouse.mapToItem( root, m.x, m.y ); CircuitPanel.gestureMove( p.x, p.y ) }
                    onReleased: CircuitPanel.gestureEnd()
                }

                ToolTip.visible: gripMouse.containsMouse
                ToolTip.text: App.translate( "Drag Toolbar" )
                ToolTip.delay: 600
            }

            // Separator to the right of the drag handle
            Item {
                implicitWidth: 7
                implicitHeight: 18
                anchors.verticalCenter: toolbarRow.verticalCenter
                Rectangle {
                    anchors.centerIn: parent
                    width: 1
                    height: parent.height
                    color: appTheme.mid
                    opacity: 0.6
                }
            }

            // Compact menu button for non-macOS systems when the inline menu bar is not shown
            AppToolButton {
                id: appMenuBtn
                visible: !App.nativeMenus && !root.inlineMenuBarVisible
                anchors.verticalCenter: toolbarRow.verticalCenter
                checked: appMenu.opened
                ToolTip.visible: appMenuBtn.hovered && !appMenu.opened
                ToolTip.text: App.translate( "Menu" )
                onClicked: {
                    if ( appMenu.opened )
                        root.closeAppMenu()
                    else
                        root.openAppMenu()
                }
                contentItem: Item {
                    implicitWidth: 18
                    implicitHeight: 18
                    AppIcon {
                        anchors.centerIn: parent
                        icon: "menu"
                        size: 16
                        color: appMenuBtn.isCheckedActive ? appTheme.highlight : appMenuBtn.textColor
                    }
                }

                AppContextMenu {
                    id: appMenu
                    popupType: Popup.Item
                    focus: true
                    closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutsideParent
                    property bool menuDirty: true
                    onOpenedChanged: {
                        if ( !root.inlineMenuBarVisible )
                            AppMenuBar.setAltMenuOpen( appMenu.opened )
                    }
                    onAboutToShow: {
                        if ( appMenu.menuDirty || appMenu.count === 0 )
                            root.rebuildAppMenu()
                        root.positionAppMenu()
                    }
                }
            }

            // Separator to the right of the menu button
            Item {
                visible: !App.nativeMenus && !root.inlineMenuBarVisible
                implicitWidth: 7
                implicitHeight: 18
                anchors.verticalCenter: toolbarRow.verticalCenter
                Rectangle {
                    anchors.centerIn: parent
                    width: 1
                    height: parent.height
                    color: appTheme.mid
                    opacity: 0.6
                }
            }

            Repeater {
                model: CircuitPanel.toolbarItems
                delegate: Loader {
                    id: tbLoader
                    required property var modelData
                    required property int index
                    visible: tbLoader.modelData.visible
                    // A Row top-aligns children of different heights, so without
                    // this the buttons sat above the taller items beside them.
                    anchors.verticalCenter: toolbarRow.verticalCenter
                    sourceComponent: tbLoader.modelData.kind === "separator" ? separatorItem
                                   : tbLoader.modelData.kind === "widget"    ? widgetItem
                                                                            : actionItem

                    Component {
                        id: separatorItem
                        // Implicit size, not width/height: the Loader above sizes
                        // itself to the item's *implicit* size, so a bare
                        // `width: 1; height: 18` left it 0x0 -- the Row packed the
                        // next button over the line, and with the Loader
                        // vertically centred at zero height the line hung below
                        // the icons instead of beside them.
                        Item {
                            implicitWidth: 7   // 3px of air on either side of the line
                            implicitHeight: 18
                            Rectangle {
                                anchors.centerIn: parent
                                width: 1
                                height: parent.height
                                color: appTheme.mid
                                opacity: 0.6
                            }
                        }
                    }
                    Component {
                        id: actionItem
                        AppToolButton {
                            id: actionBtn
                            enabled: tbLoader.modelData.enabled
                            checkable: tbLoader.modelData.checkable
                            checked: tbLoader.modelData.checked
                            tint: (tbLoader.modelData.tint !== undefined && tbLoader.modelData.tint !== "") ? tbLoader.modelData.tint : appTheme.windowText
                            ToolTip.visible: actionBtn.hovered && tbLoader.modelData.tooltip !== ""
                            ToolTip.text: tbLoader.modelData.tooltip
                            onClicked: {
                                CircuitPanel.overlayClicked( "toolbar" )
                                CircuitPanel.toolbarTriggered( tbLoader.index )
                            }
                            contentItem: Item {
                                implicitWidth: 18
                                implicitHeight: 18
                                AppIcon {
                                    anchors.centerIn: parent
                                    visible: tbLoader.modelData.iconLigature !== undefined && tbLoader.modelData.iconLigature !== ""
                                    icon: tbLoader.modelData.iconLigature || ""
                                    size: 16
                                    color: actionBtn.isCheckedActive ? appTheme.highlight : actionBtn.textColor
                                }
                                Image {
                                    anchors.fill: parent
                                    visible: !tbLoader.modelData.iconLigature || tbLoader.modelData.iconLigature === ""
                                    source: tbLoader.modelData.icon || ""
                                    fillMode: Image.PreserveAspectFit
                                    mipmap: true
                                }
                            }
                        }
                    }
                    Component {
                        id: widgetItem
                        Loader {
                            sourceComponent: tbLoader.modelData.widget === "msg"            ? textItem
                                           : tbLoader.modelData.widget === "compileStatus"  ? compileStatusItem
                                           : tbLoader.modelData.widget === "coords"         ? coordsItem
                                           : tbLoader.modelData.widget === "zoom"           ? zoomItem
                                           : tbLoader.modelData.widget === "warnings"       ? warnItem
                                           : tbLoader.modelData.widget === "canvasOverflow" ? canvasOverflowItem
                                           : tbLoader.modelData.widget === "subcNav"        ? subcNavItem
                                           : tbLoader.modelData.widget === "deviceSel"      ? deviceSelItem
                                                                                            : embeddedItem
                        }
                    }
                }
            }
        }
    }

    Timer {
        id: compileAutoHideTimer
        repeat: false
        onTriggered: EditorPanel.clearCompileStatus()
    }

    Component {
        id: compileStatusItem
        Rectangle {
            id: compilePill
            readonly property var st: CircuitPanel.compileStatus
            readonly property color fgColor: (st && st.fg) ? st.fg : appTheme.windowText
            visible: st ? st.visible === true : false
            implicitWidth: Math.ceil(compileRow.implicitWidth + 14)
            implicitHeight: Math.max(22, Math.ceil((compileRow.implicitHeight + 6) / 2) * 2)
            radius: compilePill.height / 2
            color: (st && st.bg) ? st.bg : "transparent"

            MouseArea {
                id: compilePillMouse
                anchors.fill: parent
                hoverEnabled: true
                cursorShape: Qt.PointingHandCursor
                onClicked: {
                    CircuitPanel.overlayClicked( "toolbar" )
                    CircuitPanel.showCompLog()
                }
            }

            ToolTip.visible: Boolean(compilePillMouse.containsMouse && !compileActMouse.containsMouse
                             && compilePill.st && compilePill.st.tooltip !== "")
            ToolTip.text: (compilePill.st && compilePill.st.tooltip) ? compilePill.st.tooltip : ""
            ToolTip.delay: 400

            Row {
                id: compileRow
                anchors.centerIn: parent
                spacing: 4

                Item {
                    width: 16
                    height: 16
                    anchors.verticalCenter: parent.verticalCenter

                    AppIcon {
                        id: compileIcon
                        anchors.centerIn: parent
                        icon: (compilePill.st && compilePill.st.icon) ? compilePill.st.icon : ""
                        size: 16
                        color: compilePill.fgColor
                        NumberAnimation on rotation {
                            from: 0
                            to: 360
                            duration: 900
                            loops: Animation.Infinite
                            running: Boolean(compilePill.visible && compilePill.st && compilePill.st.spinning === true)
                            onRunningChanged: if ( !running ) compileIcon.rotation = 0
                        }
                    }
                }

                AppText {
                    anchors.verticalCenter: parent.verticalCenter
                    text: (compilePill.st && compilePill.st.text) ? compilePill.st.text : ""
                    color: compilePill.fgColor
                }

                Item {
                    id: compileActBtn
                    visible: Boolean(compilePill.st && (compilePill.st.cancellable === true || compilePill.st.dismissible === true))
                    width: 16
                    height: 16
                    anchors.verticalCenter: parent.verticalCenter

                    Rectangle {
                        anchors.fill: parent
                        radius: height / 2
                        color: compileActMouse.containsMouse
                               ? Qt.alpha(compilePill.fgColor, 0.22)
                               : "transparent"
                    }

                    AppIcon {
                        anchors.centerIn: parent
                        icon: (compilePill.st && compilePill.st.cancellable === true) ? "stop_circle" : "close"
                        size: 16
                        color: compilePill.fgColor
                    }

                    MouseArea {
                        id: compileActMouse
                        anchors.fill: parent
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onClicked: {
                            CircuitPanel.overlayClicked( "toolbar" )
                            if ( compilePill.st && compilePill.st.cancellable === true )
                                EditorPanel.cancelCompile()
                            else
                                EditorPanel.clearCompileStatus()
                        }
                    }

                    ToolTip.visible: compileActMouse.containsMouse
                    ToolTip.text: (compilePill.st && compilePill.st.cancellable === true)
                                  ? App.translate( "Cancel compilation" )
                                  : App.translate( "Dismiss" )
                    ToolTip.delay: 300
                }
            }
        }
    }

    // The run state sits in a coloured pill, as the QLabel did: green running,
    // amber stopped, red on error. CircuitWidget::setMsg picks the pair.
    Component {
        id: textItem
        Rectangle {
            id: msgPill
            implicitWidth: Math.ceil(msgText.implicitWidth + 14)
            implicitHeight: Math.ceil((msgText.implicitHeight + 6) / 2) * 2
            radius: msgPill.height / 2
            color: CircuitPanel.messageBg
            visible: msgText.text !== ""
            AppText {
                id: msgText
                anchors.centerIn: parent
                text: CircuitPanel.messageText
                color: CircuitPanel.messageColor
            }
        }
    }
    /* The cursor position, in a neutral pill matching the run-state one beside
     * it. Each number sits in a box sized from the widest six-character value
     * the font can produce rather than from the text: this updates on every
     * mouse move over the canvas, and a label that grows a digit re-measured
     * the toolbar and slid the whole strip sideways under the pointer.
     */
    Component {
        id: coordsItem
        Rectangle {
            implicitWidth: Math.ceil(coordsRow.implicitWidth + 14)
            implicitHeight: Math.ceil((coordsRow.implicitHeight + 6) / 2) * 2
            radius: height / 2
            color: Qt.rgba( appTheme.windowText.r, appTheme.windowText.g,
                            appTheme.windowText.b, 0.10 )

            // Digits are not the widest glyph in every font and a minus sign
            // costs a slot of its own, so measure the real worst case rather
            // than assuming six zeroes.
            TextMetrics {
                id: valueMetrics
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
                    text: App.translate( "X" )
                    color: Qt.rgba( appTheme.windowText.r, appTheme.windowText.g,
                                    appTheme.windowText.b, 0.6 )
                }
                AppText {
                    anchors.verticalCenter: parent.verticalCenter
                    // Grows past the reserved width only for a coordinate that
                    // genuinely needs more than six characters.
                    width: Math.max( valueMetrics.width, implicitWidth )
                    horizontalAlignment: Text.AlignRight
                    text: CircuitPanel.coordsX
                    color: appTheme.windowText
                }
                AppText {
                    anchors.verticalCenter: parent.verticalCenter
                    leftPadding: 7 // the gap between the two axes, not inside one
                    text: App.translate( "Y" )
                    color: Qt.rgba( appTheme.windowText.r, appTheme.windowText.g,
                                    appTheme.windowText.b, 0.6 )
                }
                AppText {
                    anchors.verticalCenter: parent.verticalCenter
                    width: Math.max( valueMetrics.width, implicitWidth )
                    horizontalAlignment: Text.AlignRight
                    text: CircuitPanel.coordsY
                    color: appTheme.windowText
                }
            }
        }
    }
    // Same compact sizing as the icon buttons: left at the style's 40px floor
    // these two alone made the whole strip that tall.
    //
    // The zoom level opens the zoom menu, as the InstantPopup QToolButton did,
    // so it keeps that button's drop-down arrow and its tooltip -- without them
    // nothing said the percentage was clickable at all.
    Component {
        id: zoomItem
        AppToolButton {
            id: zoomBtn
            leftPadding: 6
            rightPadding: 6
            checked: CircuitPanel.zoomMenuOpen
            ToolTip.visible: zoomBtn.hovered
            ToolTip.text: App.translate( "Zoom Options" )
            onClicked: {
                CircuitPanel.overlayClicked( "toolbar" )
                if ( zoomMenu.visible ) {
                    zoomMenu.close()
                } else {
                    // A closed Menu reports width/height 0 until its rows are measured.
                    zoomMenu.updateGeometry()
                    var menuW = Math.max( zoomMenu.implicitWidth,
                                          zoomMenu.contentImplicitWidth + zoomMenu.padding * 2 )
                    var menuH = Math.max( zoomMenu.implicitHeight,
                                          zoomMenu.contentImplicitHeight + zoomMenu.padding * 2 )
                    root.placeToolbarPopup( zoomMenu, zoomBtn, false, menuW, menuH, false )
                    CircuitPanel.showZoomMenu( 0, 0 )
                    zoomMenu.open()
                }
            }
            contentItem: Row {
                id: zoomRow
                spacing: 1
                AppText {
                    anchors.verticalCenter: parent.verticalCenter
                    text: CircuitPanel.zoomText
                    font: zoomBtn.font
                    color: appTheme.windowText
                }
                AppIcon {
                    anchors.verticalCenter: parent.verticalCenter
                    icon: "arrow_drop_down"
                    size: 16
                    color: appTheme.windowText
                }
            }
        }
    }
    Component {
        id: warnItem
        AppToolButton {
            id: warnBtn
            leftPadding: 6
            rightPadding: 6
            visible: CircuitPanel.warningsText !== ""
            checked: overloadPopup.visible
            onClicked: {
                CircuitPanel.overlayClicked( "toolbar" )
                if ( !overloadPopup.visible )
                    root.placeToolbarPopup( overloadPopup, warnBtn, true, 0, overloadPopup.naturalHeight, true )
                overloadPopup.toggle()
            }
            contentItem: Row {
                spacing: 4
                AppIcon {
                    anchors.verticalCenter: parent.verticalCenter
                    icon: "warning"
                    size: 16
                    color: CircuitPanel.warningsCrashed ? CircuitCanvas.msgErrorBg : CircuitCanvas.msgWarnBg
                }
                AppText {
                    anchors.verticalCenter: parent.verticalCenter
                    text: CircuitPanel.warningsText
                    font: warnBtn.font
                    color: appTheme.windowText
                }
            }
        }
    }
    Component {
        id: canvasOverflowItem
        AppToolButton {
            id: overflowBtn
            leftPadding: 6
            rightPadding: 6
            visible: CircuitPanel.canvasOverflowText !== ""
            checked: canvasOverflowPopup.visible
            onClicked: {
                CircuitPanel.overlayClicked( "toolbar" )
                if ( !canvasOverflowPopup.visible )
                    root.placeToolbarPopup( canvasOverflowPopup, overflowBtn, true, 0, canvasOverflowPopup.naturalHeight, true )
                canvasOverflowPopup.toggle()
            }
            contentItem: Row {
                spacing: 4
                AppIcon {
                    anchors.verticalCenter: parent.verticalCenter
                    icon: "warning"
                    size: 16
                    color: CircuitCanvas.msgWarnBg
                }
                AppText {
                    anchors.verticalCenter: parent.verticalCenter
                    text: CircuitPanel.canvasOverflowText
                    font: overflowBtn.font
                    color: appTheme.windowText
                }
            }
        }
    }
    Component {
        id: subcNavItem
        AppToolButton {
            id: subcBtn
            leftPadding: 6
            rightPadding: 6
            visible: CircuitPanel.subcNav ? CircuitPanel.subcNav.visible : false
            checked: subcPopup.visible
            ToolTip.visible: subcBtn.hovered && CircuitPanel.subcNav && CircuitPanel.subcNav.buttonTooltip !== ""
            ToolTip.text: CircuitPanel.subcNav ? CircuitPanel.subcNav.buttonTooltip : ""
            onClicked: {
                CircuitPanel.overlayClicked( "toolbar" )
                if ( subcPopup.visible ) {
                    subcPopup.close()
                } else {
                    root.placeToolbarPopup( subcPopup, subcBtn, false, 0, subcPopup.naturalHeight, true )
                    subcPopup.open()
                }
            }
            contentItem: Row {
                spacing: 6
                anchors.verticalCenter: parent.verticalCenter
                AppIcon {
                    anchors.verticalCenter: parent.verticalCenter
                    icon: (CircuitPanel.subcNav && CircuitPanel.subcNav.iconLigature) ? CircuitPanel.subcNav.iconLigature : "account_tree"
                    size: 16
                    color: appTheme.windowText
                }
                AppText {
                    anchors.verticalCenter: parent.verticalCenter
                    text: CircuitPanel.subcNav ? CircuitPanel.subcNav.buttonText : ""
                    font: subcBtn.font
                    color: appTheme.windowText
                    elide: Text.ElideMiddle
                    maximumLineCount: 1
                }
                AppIcon {
                    anchors.verticalCenter: parent.verticalCenter
                    icon: "arrow_drop_down"
                    size: 16
                    color: appTheme.windowText
                }
            }
        }
    }
    Component {
        id: deviceSelItem
        AppToolButton {
            id: deviceBtn
            leftPadding: 6
            rightPadding: 6
            visible: CircuitPanel.deviceSel ? CircuitPanel.deviceSel.visible : false
            checked: devicePopup.visible
            ToolTip.visible: deviceBtn.hovered && CircuitPanel.deviceSel && CircuitPanel.deviceSel.buttonTooltip !== ""
            ToolTip.text: CircuitPanel.deviceSel ? CircuitPanel.deviceSel.buttonTooltip : ""
            onClicked: {
                CircuitPanel.overlayClicked( "toolbar" )
                if ( devicePopup.visible ) {
                    devicePopup.close()
                } else {
                    root.placeToolbarPopup( devicePopup, deviceBtn, false, 0, devicePopup.naturalHeight, true )
                    devicePopup.open()
                }
            }
            contentItem: Row {
                spacing: 6
                anchors.verticalCenter: parent.verticalCenter
                AppIcon {
                    anchors.verticalCenter: parent.verticalCenter
                    icon: (CircuitPanel.deviceSel && CircuitPanel.deviceSel.iconLigature) ? CircuitPanel.deviceSel.iconLigature : "memory"
                    size: 16
                    color: appTheme.windowText
                }
                AppText {
                    anchors.verticalCenter: parent.verticalCenter
                    text: CircuitPanel.deviceSel ? CircuitPanel.deviceSel.buttonText : ""
                    font: deviceBtn.font
                    color: appTheme.windowText
                    elide: Text.ElideMiddle
                    maximumLineCount: 1
                }
                AppIcon {
                    anchors.verticalCenter: parent.verticalCenter
                    icon: "arrow_drop_down"
                    size: 16
                    color: appTheme.windowText
                }
            }
        }
    }
    Component { id: embeddedItem; Item { width: 0; height: 0 } }

    // ---- Sim info card ----------------------------------------------------
    Item {
        id: infoCard
        x: root.r("infoCard").x; y: root.r("infoCard").y
        width: root.r("infoCard").width; height: root.r("infoCard").height
        visible: CircuitPanel.infoVisible
        z: root.overlayZ( "infoCard" )

        MouseArea {
            anchors.fill: parent
            onPressed: (m) => { CircuitPanel.overlayClicked( "infoCard" ) }
        }

        // infowidget.qml sizes itself to its content; the card's rect comes back
        // from updateOverlayLayout(), so that size has to go out first.
        Loader {
            id: infoLoader
            anchors.fill: parent
            source: "InfoCard.qml"
            onLoaded: infoCard.publishInfoSize()
        }
        // `as Item` only so the implicit sizes below resolve statically; Loader.item
        // is typed QObject.
        readonly property Item infoItem: infoLoader.item as Item
        function publishInfoSize(): void {
            if ( infoCard.infoItem ) CircuitPanel.setInfoCardSize( infoCard.infoItem.implicitWidth, infoCard.infoItem.implicitHeight )
        }
        Connections {
            target: infoCard.infoItem
            function onImplicitWidthChanged(): void { infoCard.publishInfoSize() }
            function onImplicitHeightChanged(): void { infoCard.publishInfoSize() }
        }

        // Close button
        AppToolButton {
            id: infoCloseBtn
            width: 24; height: 24
            x: parent.width - 28; y: 4
            padding: 0
            ToolTip.visible: infoCloseBtn.hovered
            ToolTip.text: App.translate( "Hide" )
            onClicked: CircuitPanel.hideInfo()
            contentItem: AppIcon {
                icon: "close"
                size: 16
                color: appTheme.windowText
            }
        }
    }

    AppContextMenu {
        id: zoomMenu
        // No submenus, so the window popup is unnecessary. Item fits inside
        // the window the same way the MCU list does; Window fits the screen.
        popupType: Popup.Item
        onClosed: CircuitPanel.zoomMenuClosed()
        ContextMenuItem { text: App.translate( "Zoom to Fit" ); onTriggered: CircuitCanvas.zoomToFit() }
        ContextMenuItem { text: App.translate( "Zoom to Selection" ); onTriggered: CircuitCanvas.zoomSelected() }
        ContextMenuSeparator {}
        ContextMenuItem { text: App.translate( "Reset Zoom" ); onTriggered: CircuitCanvas.zoomOne() }
    }

    AppListPopup {
        id: subcPopup
        rows: CircuitPanel.subcNav ? CircuitPanel.subcNav.rows : []
        onAboutToShow: CircuitPanel.setSubcPopupOpen(true)
        onClosed: CircuitPanel.setSubcPopupOpen(false)
        onActivated: function(index: int): void {
            CircuitPanel.activateSubcNav(index)
        }
    }

    AppListPopup {
        id: devicePopup
        rows: CircuitPanel.deviceSel ? CircuitPanel.deviceSel.rows : []
        onAboutToShow: CircuitPanel.setDevicePopupOpen(true)
        onClosed: CircuitPanel.setDevicePopupOpen(false)
        onActivated: function(index: int): void {
            CircuitPanel.activateDevice(index)
        }
    }

    OverloadPanel {
        id: overloadPopup
    }

    CanvasOverflowPanel {
        id: canvasOverflowPopup
    }

    AppMenuBuilder {
        id: menuBuilder
    }

    function rebuildAppMenu(): void {
        appMenu.populating = true
        menuBuilder.clearMenu( appMenu )
        var menus = AppMenuBar.menus || []
        for ( var i = 0; i < menus.length; ++i ) {
            var m = menus[i]
            if ( !m )
                continue
            var sub = menuBuilder.createSubMenu( appMenu, String( m.title || "" ) )
            if ( !sub )
                continue
            menuBuilder.populateMenu( sub, m.items || [] )
            appMenu.addMenu( sub )
        }
        appMenu.populating = false
        appMenu.menuDirty = false
        appMenu.updateGeometry()
    }

    function altMenuBlocked(): bool {
        return App.nativeMenus || CommandCenter.visible || TabSwitcher.visible
            || UiUtils.hasModalOverlay( Overlay.overlay, appMenu )
    }

    function openAppMenu(): void {
        if ( root.altMenuBlocked() )
            return
        CircuitPanel.overlayClicked( "toolbar" )
        if ( appMenu.menuDirty || appMenu.count === 0 )
            root.rebuildAppMenu()
        root.positionAppMenu()
        appMenu.open()
    }

    function closeAppMenu(): void {
        appMenu.closeTree()
    }

    Connections {
        target: CircuitCanvas
        function onZoomChanged(): void { CircuitPanel.setZoomPercent( CircuitCanvas.zoom ) }
        function onCoordsChanged(): void { CircuitPanel.setCoords( CircuitCanvas.cursorX, CircuitCanvas.cursorY ) }
        function onRootCircuitLoaded(): void {
            CircuitPanel.syncSimState( CircuitCanvas.simRunning, CircuitCanvas.simPaused )
            CircuitPanel.resetSubcircuits( CircuitCanvas.filePath, CircuitCanvas.fileName, CircuitCanvas.subcircuitTree )
            CircuitPanel.updateDevices( CircuitCanvas.programmableDevices )
            if ( !EditorPanel.compiling )
                EditorPanel.clearCompileStatus()
        }
        function onSubcircuitsChanged(): void {
            CircuitPanel.syncSubcircuits( CircuitCanvas.filePath, CircuitCanvas.fileName, CircuitCanvas.subcircuitTree )
        }
        function onFilePathChanged(): void {
            CircuitPanel.updateDevices( CircuitCanvas.programmableDevices )
        }
        function onOpenSubcircuitRequested( path: string, label: string ): void {
            CircuitPanel.navigateIntoSubcircuit( path, label )
            CircuitCanvas.loadSubcircuit( path )
        }
        function onDevicesChanged(): void {
            CircuitPanel.updateDevices( CircuitCanvas.programmableDevices )
        }
        function onWarningsChanged(): void {
            CircuitPanel.setWarnings(CircuitCanvas.warningsText, CircuitCanvas.warningsCrashed)
        }
        function onCanvasOverflowChanged(): void {
            var text = CircuitCanvas.canvasOverflow ? App.translate("Canvas Overflow") : ""
            CircuitPanel.setCanvasOverflow(text)
        }
        function onSimChanged(): void {
            CircuitPanel.syncSimState( CircuitCanvas.simRunning, CircuitCanvas.simPaused )
            if ( CircuitCanvas.simError )
                CircuitPanel.setMessage( App.translate( CircuitCanvas.simError ), CircuitCanvas.msgErrorBg, CircuitCanvas.msgErrorText )
            else if ( CircuitCanvas.simWarning )
                CircuitPanel.setMessage( App.translate( CircuitCanvas.simWarning ), CircuitCanvas.msgWarnBg, CircuitCanvas.msgWarnText )
            else
                CircuitPanel.clearMessage()
        }
    }

    Connections {
        target: App
        function onI18nTickChanged(): void {
            CircuitPanel.reloadI18n()
            if ( CircuitCanvas.simError )
                CircuitPanel.setMessage( App.translate( CircuitCanvas.simError ), CircuitCanvas.msgErrorBg, CircuitCanvas.msgErrorText )
            else if ( CircuitCanvas.simWarning )
                CircuitPanel.setMessage( App.translate( CircuitCanvas.simWarning ), CircuitCanvas.msgWarnBg, CircuitCanvas.msgWarnText )
            else
                CircuitPanel.clearMessage()
        }
        function onThemeChanged(): void {
            CircuitPanel.reloadI18n()
            if ( CircuitCanvas.simError )
                CircuitPanel.setMessage( App.translate( CircuitCanvas.simError ), CircuitCanvas.msgErrorBg, CircuitCanvas.msgErrorText )
            else if ( CircuitCanvas.simWarning )
                CircuitPanel.setMessage( App.translate( CircuitCanvas.simWarning ), CircuitCanvas.msgWarnBg, CircuitCanvas.msgWarnText )
        }
    }

    Connections {
        target: CircuitPanel
        function onLoadCircuitRequested( path: string ): void {
            CircuitCanvas.loadSubcircuit( path )
        }
        function onActiveDeviceChanged( id: string ): void {
            CircuitCanvas.activeDeviceId = id
            var src = CircuitCanvas.activeFirmwareSource()
            if ( src )
                EditorPanel.focusOpenFile( src )
        }
        function onAction( id: string ): void {
            if ( id === "zoomIn" ) CircuitCanvas.zoomIn()
            else if ( id === "zoomOut" ) CircuitCanvas.zoomOut()
            else if ( id === "powerCirc" ) AppMenuBar.triggerAction( "sim.power" )
            else if ( id === "powerOn" ) {
                if ( !CircuitCanvas.simRunning )
                    AppMenuBar.triggerAction( "sim.power" )
            }
            else if ( id === "powerOff" ) CircuitCanvas.powerOff()
            else if ( id === "pauseCirc" || id === "pauseQemu" || id === "resumeQemu" ) CircuitCanvas.pauseCirc()
            else if ( id === "settCircuit" ) App.showCircuitSettings()
            else if ( id === "watch" ) App.showMcu()
            else if ( id === "debug" ) AppMenuBar.triggerAction( "sim.debug" )
            else if ( id === "debugStep" ) AppMenuBar.triggerAction( "sim.step" )
            else if ( id === "debugStepOver" ) AppMenuBar.triggerAction( "sim.stepOver" )
            else if ( id === "debugReset" ) AppMenuBar.triggerAction( "sim.reset" )
            else if ( id === "debugStop" ) AppMenuBar.triggerAction( "sim.stop" )
            else if ( id.startsWith( "openSubcircuit:" ) ) CircuitCanvas.openSubcircuit( id.substring( 15 ) )
        }
    }


    Connections {
        target: EditorPanel
        function onRequestShow(): void { CircuitPanel.showEditor() }
        function onRequestHide(): void { CircuitPanel.hideEditor() }
        function onFocusedChanged(): void {
            if ( EditorPanel.focused ) {
                CircuitPanel.overlayClicked( "editor" )
                if ( root.floatingPanelId === "editor" )
                    CircuitPanel.openPanel( "editor" )
            } else {
                root.scheduleHoverClose()
            }
        }
        function onDebugStateChanged(): void { CircuitPanel.refreshDebugToolbar() }
        function onCompilingChanged(): void {
            CircuitPanel.setCompileStatus( EditorPanel.compileStatus )
            var st = CircuitPanel.compileStatus
            var ms = (st && st.autoHideMs) ? Number(st.autoHideMs) : 0
            if ( ms > 0 ) {
                compileAutoHideTimer.interval = ms
                compileAutoHideTimer.restart()
            } else {
                compileAutoHideTimer.stop()
            }
        }
    }

    Connections {
        target: AppDialog
        function onSettingsReset(): void { CircuitPanel.reloadOverlayLayout() }
    }

    Connections {
        target: FileBrowser
        function onOpenFile( path: string ): void {
            EditorPanel.loadFile( path )
            EditorPanel.focusEditor()
            AppDialog.addRecentFile( path )
            AppMenuBar.rebuild()
        }
        function onOpenCircuit( path: string ): void { CircuitCanvas.loadPath( path ) }
    }

    Connections {
        target: AppMenuBar
        function onMenusChanged(): void {
            appMenu.menuDirty = true
        }
        function onTogglePopupRequested(): void {
            if ( root.inlineMenuBarVisible )
                return
            var wasOpen = appMenu.opened
            if ( root.altMenuBlocked() ) {
                AppMenuBar.noteAltToggled( false )
                return
            }
            if ( wasOpen )
                root.closeAppMenu()
            else
                root.openAppMenu()
            AppMenuBar.noteAltToggled( appMenu.opened !== wasOpen )
        }
    }

}
