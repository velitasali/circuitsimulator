pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtQuick.Templates as T
import cs_app
import cs_ui

/* The circuit area: the canvas, and the overlays floating over it.
 *
 * The side panel, the editor panel, the toolbar and the sim-info card are not
 * siblings of the canvas -- they float above it and may overlap each other
 * freely -- which is why they are laid out here rather than in main.qml.
 *
 * None of the placement logic lives in this file. CircuitWidget still computes
 * every rect from its stored drag/resize/snap state and publishes them as
 * `overlayRects`; this only binds to them and sends gestures back. That is what
 * kept the port from having to re-derive any of that behaviour.
 */
Item {
    id: root
    objectName: "circuitPanelRoot"
    SystemPalette { id: appTheme }
    readonly property var rects: CircuitPanel.overlayRects

    // Overlay rects are JSON {x,y,w,h}, not QRectF.
    function r( key: string ): rect {
        var v = rects[key]
        if ( v === undefined || v === null )
            return Qt.rect( 0, 0, 0, 0 )
        var w = ( v.w !== undefined ) ? v.w : v.width
        var h = ( v.h !== undefined ) ? v.h : v.height
        return Qt.rect( v.x, v.y, w, h )
    }
    function overlayZ( key: string ): real { return rects[key + "Z"] !== undefined ? rects[key + "Z"] : 1 }

    function toolbarOpensUpward(): bool {
        return toolbar.y + toolbar.height / 2 > ( root.topInset + root.height ) / 2
    }

    // Free gap on the side the popup opens into, inside the 8px screen inset.
    function toolbarPopupRoom(): real {
        var inset = 8
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
        popup.x = Math.max( 8, Math.min( root.width - w - 8, pt.x ) )
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
        var targetX = Math.max( 8, Math.min( root.width - menuW - 8, anchorPt.x ) )
        var targetY = root.toolbarPopupY( shownH )
        var localPt = root.mapToItem( appMenuBtn, targetX, targetY )
        appMenu.x = localPt.x
        appMenu.y = localPt.y
    }

    // The overlays float over the canvas and are translucent: the QSS painted
    // them rgba(window, 0.85) so the canvas shows through.
    readonly property color overlayBg: Qt.rgba( appTheme.window.r,
                                                appTheme.window.g,
                                                appTheme.window.b, 0.85 )

    property int topInset: 0
    onTopInsetChanged: CircuitPanel.setTopInset( topInset )

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
    onWidthChanged: root.publishViewSize()
    onHeightChanged: root.publishViewSize()
    Component.onCompleted: {
        CircuitPanel.setTopInset( root.topInset )
        root.publishViewSize()
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

    // ---- Side panel ------------------------------------------------------
    Rectangle {
        id: sidePanel
        // MainWindow walks the QML active-focus chain for these names to work
        // out which panel the keyboard is in; see MainWindow::qmlFocusInside.
        objectName: "sidePanel"
        visible: root.rects.sidePanelVisible === true
        x: root.r("sidePanel").x; y: root.r("sidePanel").y
        width: root.r("sidePanel").width; height: root.r("sidePanel").height
        color: appTheme.window
        radius: 14
        border.color: appTheme.mid
        border.width: 1
        z: root.overlayZ( "sidePanel" )

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

        // Absorbs unhandled pointer events (clicks on empty space, scrolling over margins)
        // and prevents any pointer events from leaking to the canvas below.
        MouseArea {
            anchors.fill: parent
            hoverEnabled: true
            acceptedButtons: Qt.AllButtons
            onPressed: (m) => { CircuitPanel.overlayClicked( "sidePanel" ) }
            onWheel: (wheel) => { wheel.accepted = true }
        }

        WheelHandler {
            target: null
            acceptedDevices: PointerDevice.Mouse | PointerDevice.TouchPad
            orientation: Qt.Horizontal | Qt.Vertical
        }

        HoverHandler {}

        DropArea {
            anchors.fill: parent
            onEntered: (drag) => { drag.accepted = true }
            onDropped: (drop) => { drop.accept() }
        }

        ColumnLayout {
            anchors.fill: parent
            anchors.margins: 6
            spacing: 4

            // The tab strip is the app's QSS QTabBar look, not the platform
            // QQC2 TabBar (a segmented control on macOS). The QSS rules that
            // used to style #sidePanel QTabBar no longer reach it.
            AppTabBar {
                id: sideTabs
                Layout.fillWidth: true
                centered: true

                // Material Symbols glyphs, as the QSS gave this tab bar. Four
                // word labels do not fit a 250px panel -- they overflowed it and
                // spilled onto the canvas -- and the names live in the tooltips.
                Repeater {
                    model: [ { glyph: "developer_board", label: "Components" },
                             { glyph: "folder_open",     label: "Files" },
                             { glyph: "terminal",        label: "Simulator Messages" },
                             { glyph: "bug_report",      label: "Compiler Messages" } ]
                    delegate: AppTabButton {
                        id: sideTab
                        required property var modelData
                        text: I18n.tr(sideTab.modelData.label)
                        leftPadding: 12
                        rightPadding: 12
                        ToolTip.visible: sideTab.hovered
                        ToolTip.text: I18n.tr(sideTab.modelData.label)
                        onClicked: CircuitPanel.overlayClicked( "sidePanel" )
                        contentItem: AppIcon {
                            icon: sideTab.modelData.glyph
                            size: 16
                            color: sideTab.checked ? appTheme.highlightedText : appTheme.windowText
                        }
                    }
                }

                // Two-way with MainWindow.sidePanelTab: clicking a tab pushes
                // out, and showSidePanelTab() (Focus Files, Show Simulator
                // Messages, the command palette) pushes back in. A plain
                // `currentIndex: app.sidePanelTab` would not survive the first
                // click, which breaks the binding; RestoreNone re-asserts it.
                onCurrentIndexChanged: {
                    sidePanel.clearComponentHover()
                    CircuitPanel.overlayClicked( "sidePanel" )
                    App.sidePanelTab = sideTabs.currentIndex
                }
            }
            Binding {
                target: sideTabs
                property: "currentIndex"
                value: App.sidePanelTab
                restoreMode: Binding.RestoreNone
            }

            StackLayout {
                Layout.fillWidth: true
                Layout.fillHeight: true
                currentIndex: sideTabs.currentIndex

                ColumnLayout {
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    spacing: 4
                    AppTextField {
                        id: componentSearch
                        objectName: "componentSearch"
                        Layout.fillWidth: true
                        placeholderText: App.searchPlaceholder
                        onPressed: CircuitPanel.overlayClicked( "sidePanel" )
                        onTextChanged: {
                            App.setSearchFilter( componentSearch.text )
                            ComponentList.search( componentSearch.text )
                        }

                        Connections {
                            target: App
                            function onFocusSearchRequested(): void {
                                componentSearch.forceActiveFocus()
                                componentSearch.selectAll()
                            }
                        }
                    }
                    // Container card: the same framed treatment PropDialog gives the
                    // area below its tab bar. Only the listing itself sits inside it --
                    // the search field above stays outside, as it is not part of the list.
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

                Loader {
                    objectName: "fileBrowser"
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    source: "FileBrowserView.qml"
                }

                // Two log panes, so the backend comes in as `ctx` rather than a
                // context property -- one engine can hold only one `outPanel`.
                // Framed like the other tabs' listings; a log has no toolbar to
                // keep outside it, so the card just wraps the whole pane.
                Item {
                    Rectangle {
                        anchors.fill: parent
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
                Item {
                    Rectangle {
                        anchors.fill: parent
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
        }
    }

    ComponentHoverCard {
        id: componentHoverCard
        readonly property ComponentListView listView: componentListLoader.item as ComponentListView
        active: sidePanel.visible && sideTabs.currentIndex === 0 && listView !== null && listView.hoverCardVisible
        info: listView ? listView.hoveredItemInfo : null
        anchorY: {
            var _panelY = sidePanel.y
            return ( listView && listView.hoverCardVisible )
                ? componentListLoader.mapToItem( root, 0, listView.hoveredRowCenterY ).y
                : 0
        }
        panelRect: Qt.rect( sidePanel.x, sidePanel.y, sidePanel.width, sidePanel.height )
        preferRight: root.rects.sidePanelClusterRight !== false
        topInset: root.topInset
    }

    // ---- Editor panel ----------------------------------------------------
    Rectangle {
        // NOT `id: editorPanel`: that shadowed the `editorPanel` context property
        // for everything this Loader creates, so editorpanel.qml's `ctx` resolved
        // to this Rectangle instead of EditorWidget and every binding on it read
        // as undefined. The objectName is what the focus tests match on.
        id: editorPanelOverlay
        objectName: "editorPanel"
        visible: root.rects.editorPanelVisible === true
        x: root.r("editorPanel").x; y: root.r("editorPanel").y
        width: root.r("editorPanel").width; height: root.r("editorPanel").height
        color: root.overlayBg
        radius: 14
        border.color: appTheme.mid
        border.width: 1
        z: root.overlayZ( "editorPanel" )

        // Absorbs unhandled pointer events (clicks on empty space, scrolling over margins)
        // and prevents any pointer events from leaking to the canvas below.
        MouseArea {
            anchors.fill: parent
            hoverEnabled: true
            acceptedButtons: Qt.AllButtons
            onPressed: (m) => { CircuitPanel.overlayClicked( "editorPanel" ) }
            onWheel: (wheel) => { wheel.accepted = true }
        }

        WheelHandler {
            target: null
            acceptedDevices: PointerDevice.Mouse | PointerDevice.TouchPad
            orientation: Qt.Horizontal | Qt.Vertical
        }

        HoverHandler {}

        DropArea {
            anchors.fill: parent
            onEntered: (drag) => { drag.accepted = true }
            onDropped: (drop) => { drop.accept() }
        }

        Loader { anchors.fill: parent; anchors.margins: 1; source: "EditorPanelView.qml" }
    }

    // ---- Resize / drag handles -------------------------------------------
    // Invisible strips on the panel borders, exactly where the QWidget handles
    // were. Each forwards to the same CircuitWidget gesture handlers.
    Repeater {
        model: [
            { key: "spResizeRight",  cursor: Qt.SizeHorCursor, id: "sidePanelRight",   panel: "sidePanel"   },
            { key: "spResizeBottom", cursor: Qt.SizeVerCursor, id: "sidePanelBottom",  panel: "sidePanel"   },
            { key: "spResizeLeft",   cursor: Qt.SizeHorCursor, id: "sidePanelLeft",    panel: "sidePanel"   },
            { key: "spResizeTop",    cursor: Qt.SizeVerCursor, id: "sidePanelTop",     panel: "sidePanel"   },
            { key: "epResizeLeft",   cursor: Qt.SizeHorCursor, id: "editorPanelLeft",  panel: "editorPanel" },
            { key: "epResizeBottom", cursor: Qt.SizeVerCursor, id: "editorPanelBottom",panel: "editorPanel" },
            { key: "epResizeRight",  cursor: Qt.SizeHorCursor, id: "editorPanelRight", panel: "editorPanel" },
            { key: "epResizeTop",    cursor: Qt.SizeVerCursor, id: "editorPanelTop",   panel: "editorPanel" },
            { key: "spResizeTL", cursor: Qt.SizeFDiagCursor, id: "sidePanelTL",   panel: "sidePanel"   },
            { key: "spResizeTR", cursor: Qt.SizeBDiagCursor, id: "sidePanelTR",   panel: "sidePanel"   },
            { key: "spResizeBL", cursor: Qt.SizeBDiagCursor, id: "sidePanelBL",   panel: "sidePanel"   },
            { key: "spResizeBR", cursor: Qt.SizeFDiagCursor, id: "sidePanelBR",   panel: "sidePanel"   },
            { key: "epResizeTL", cursor: Qt.SizeFDiagCursor, id: "editorPanelTL", panel: "editorPanel" },
            { key: "epResizeTR", cursor: Qt.SizeBDiagCursor, id: "editorPanelTR", panel: "editorPanel" },
            { key: "epResizeBL", cursor: Qt.SizeBDiagCursor, id: "editorPanelBL", panel: "editorPanel" },
            { key: "epResizeBR", cursor: Qt.SizeFDiagCursor, id: "editorPanelBR", panel: "editorPanel" }
        ]
        delegate: MouseArea {
            id: resizeHandle
            required property var modelData
            // A hidden panel keeps publishing its rects, so without the second
            // half these strips stayed live over bare canvas.
            visible: root.r( resizeHandle.modelData.key ).width > 0
                     && root.rects[ resizeHandle.modelData.panel + "Visible" ] === true
            x: root.r( resizeHandle.modelData.key ).x;     y: root.r( resizeHandle.modelData.key ).y
            width: root.r( resizeHandle.modelData.key ).width
            height: root.r( resizeHandle.modelData.key ).height
            cursorShape: resizeHandle.modelData.cursor
            // Just above the panel they belong to, so they keep their place in
            // the overlay stack. At z 0 the panel covered them: a corner sits
            // 7px inside it, leaving only a 3px sliver of it grabbable -- and
            // over a panel's own controls, nothing of it at all.
            z: root.overlayZ( resizeHandle.modelData.panel ) + 0.5
            onPressed: (m) => {
                CircuitPanel.overlayClicked( resizeHandle.modelData.panel )
                var p = resizeHandle.mapToItem( root, m.x, m.y )
                CircuitPanel.gestureBegin( resizeHandle.modelData.id, p.x, p.y )
            }
            onPositionChanged: (m) => { var p = resizeHandle.mapToItem( root, m.x, m.y ); CircuitPanel.gestureMove( p.x, p.y ) }
            onReleased: CircuitPanel.gestureEnd()
            // Back to the default size, as double-clicking a splitter did.
            onDoubleClicked: CircuitPanel.resetOverlaySize( resizeHandle.modelData.id.startsWith( "sidePanel" )
                                                   ? "sidePanel" : "editorPanel" )
        }
    }

    /* ---- Panel grips and show/hide pills ---------------------------------
     * One pair per panel, sitting on the side that faces the canvas.
     * CircuitWidget already handled the "sidePanel"/"editorPanel" drag gestures;
     * nothing in the QML had ever started one, so the panels could not be moved
     * at all, and there was no way to hide one but the View menu.
     */
    Repeater {
        model: [
            { key: "spDragHandle", id: "sidePanel",   toggle: false },
            { key: "spToggle",     id: "sidePanel",   toggle: true  },
            { key: "epDragHandle", id: "editorPanel", toggle: false },
            { key: "epToggle",     id: "editorPanel", toggle: true  }
        ]
        delegate: Rectangle {
            id: handle
            required property var modelData
            readonly property rect box: root.r( handle.modelData.key )
            readonly property bool panelVisible: root.rects[ handle.modelData.id + "Visible" ] === true
            // A toggle stays put when its panel is hidden -- it is what brings it
            // back; a grip has nothing to drag then.
            visible: handle.box.width > 0 && ( handle.modelData.toggle || handle.panelVisible )
            x: handle.box.x; y: handle.box.y
            width: handle.box.width; height: handle.box.height
            radius: handle.height / 2
            color: pointer.containsMouse ? appTheme.midlight
                                         : ( handle.modelData.id === "sidePanel" ? appTheme.window : root.overlayBg )
            border.color: appTheme.mid
            border.width: 1
            z: root.overlayZ( handle.modelData.id ) + 0.5

            AppIcon {
                anchors.centerIn: parent
                // A drag handle always shows its grip dots. A toggle points
                // towards the panel it will act on: towards where the panel
                // currently sits while visible (the direction hiding it will
                // travel), and towards where it will reappear once hidden --
                // the opposite direction, since the pill has jumped to the far
                // margin by then (see CircuitWidget::updateOverlayLayout).
                icon: !handle.modelData.toggle ? "drag_indicator"
                    : ( root.rects[ handle.modelData.id + "ClusterRight" ] === handle.panelVisible )
                      ? "chevron_left" : "chevron_right"
                size: 16
                color: appTheme.windowText
            }

            MouseArea {
                id: pointer
                anchors.fill: parent
                hoverEnabled: true
                cursorShape: handle.modelData.toggle ? Qt.PointingHandCursor : Qt.SizeAllCursor
                onClicked: if ( handle.modelData.toggle ) {
                    CircuitPanel.overlayClicked( handle.modelData.id )
                    CircuitPanel.togglePanel( handle.modelData.id )
                }
                onDoubleClicked: if ( !handle.modelData.toggle ) CircuitPanel.resetOverlayPos( handle.modelData.id )
                onPressed: (m) => {
                    CircuitPanel.overlayClicked( handle.modelData.id )
                    if ( !handle.modelData.toggle ) {
                        var p = pointer.mapToItem( root, m.x, m.y )
                        CircuitPanel.gestureBegin( handle.modelData.id, p.x, p.y )
                    }
                }
                onPositionChanged: (m) => {
                    if ( !handle.modelData.toggle && pointer.pressed ) {
                        var p = pointer.mapToItem( root, m.x, m.y )
                        CircuitPanel.gestureMove( p.x, p.y )
                    }
                }
                onReleased: if ( !handle.modelData.toggle ) CircuitPanel.gestureEnd()
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
        padding: 4
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
        background: Rectangle {
            color: root.overlayBg
            radius: height / 2
            border.color: appTheme.mid
            border.width: 1

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
                    CircuitPanel.showSidePanel()
                    App.sidePanelTab = 3
                    if ( compilePill.st && compilePill.st.state === "error" )
                        CircuitPanel.showEditor()
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
            var starting = !CircuitPanel.simRunning && CircuitCanvas.simRunning
            CircuitPanel.syncSimState( CircuitCanvas.simRunning, CircuitCanvas.simPaused )
            if ( starting ) {
                CircuitPanel.showSidePanel()
                App.sidePanelTab = 2
            }
            if ( CircuitCanvas.simError )
                CircuitPanel.setMessage( App.translate( CircuitCanvas.simError ), CircuitCanvas.msgErrorBg, CircuitCanvas.msgErrorText )
            else if ( CircuitCanvas.simWarning )
                CircuitPanel.setMessage( App.translate( CircuitCanvas.simWarning ), CircuitCanvas.msgWarnBg, CircuitCanvas.msgWarnText )
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
        }
        function onThemeChanged(): void {
            CircuitPanel.reloadI18n()
        }
    }

    Connections {
        target: CircuitPanel
        function onLoadCircuitRequested( path: string ): void {
            CircuitCanvas.loadSubcircuit( path )
        }
        function onActiveDeviceChanged( id: string ): void {
            CircuitCanvas.activeDeviceId = id
        }
        function onAction( id: string ): void {
            if ( id === "zoomIn" ) CircuitCanvas.zoomIn()
            else if ( id === "zoomOut" ) CircuitCanvas.zoomOut()
            else if ( id === "powerCirc" || id === "powerOn" ) CircuitCanvas.powerCirc()
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
        function onFocusedChanged(): void { if ( EditorPanel.focused ) CircuitPanel.overlayClicked( "editorPanel" ) }
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
