import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import cs_app

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
    function tr(s) { return App.i18nTick >= 0 ? App.translate(s) : s }
    readonly property var ctx: CircuitPanel
    readonly property var rects: ctx ? ctx.overlayRects : ({})

    // Overlay rects are JSON {x,y,w,h}, not QRectF.
    function r( key ) {
        var v = rects[key]
        if ( v === undefined || v === null )
            return Qt.rect( 0, 0, 0, 0 )
        var w = ( v.w !== undefined ) ? v.w : v.width
        var h = ( v.h !== undefined ) ? v.h : v.height
        return Qt.rect( v.x, v.y, w, h )
    }
    function overlayZ( key ) { return rects[key + "Z"] !== undefined ? rects[key + "Z"] : 1 }

    // The overlays float over the canvas and are translucent: the QSS painted
    // them rgba(window, 0.85) so the canvas shows through.
    readonly property color overlayBg: Qt.rgba( appTheme.window.r,
                                                appTheme.window.g,
                                                appTheme.window.b, 0.85 )

    // CircuitWidget lays every overlay out against the canvas area's size, which
    // it used to take from resizeEvent(). Nothing reported it after the port, so
    // m_viewSize stayed (-1,-1) and every panel came out with a negative height,
    // which is why the side and editor panels never appeared.
    function publishViewSize() { if ( ctx ) ctx.setViewSize( width, height ) }
    onWidthChanged: publishViewSize()
    onHeightChanged: publishViewSize()
    Component.onCompleted: {
        publishViewSize()
        if ( ctx ) {
            ctx.setZoomPercent( CircuitCanvas.zoom )
            ctx.resetSubcircuits( CircuitCanvas.filePath, CircuitCanvas.fileName, CircuitCanvas.subcircuitTree )
            ctx.updateDevices( CircuitCanvas.programmableDevices )
            ctx.setCanvasOverflow( CircuitCanvas.canvasOverflow ? App.translate("Canvas Overflow") : "" )
        }
    }



    // ---- Canvas ----------------------------------------------------------
    Loader {
        id: canvasLoader
        anchors.fill: parent
        source: "CircuitView.qml"
    }

    // ---- Side panel ------------------------------------------------------
    Rectangle {
        id: sidePanel
        // MainWindow walks the QML active-focus chain for these names to work
        // out which panel the keyboard is in; see MainWindow::qmlFocusInside.
        objectName: "sidePanel"
        visible: rects.sidePanelVisible === true
        x: r("sidePanel").x; y: r("sidePanel").y
        width: r("sidePanel").width; height: r("sidePanel").height
        color: appTheme.window
        radius: 6
        border.color: appTheme.mid
        border.width: 1
        z: root.overlayZ( "sidePanel" )

        // Absorbs unhandled pointer events (clicks on empty space, scrolling over margins)
        // and prevents any pointer events from leaking to the canvas below.
        MouseArea {
            anchors.fill: parent
            hoverEnabled: true
            acceptedButtons: Qt.AllButtons
            onPressed: (m) => { if ( ctx ) ctx.overlayClicked( "sidePanel" ) }
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
                        text: root.tr(modelData.label)
                        leftPadding: 12
                        rightPadding: 12
                        ToolTip.visible: hovered
                        ToolTip.text: root.tr(modelData.label)
                        onClicked: if ( ctx ) ctx.overlayClicked( "sidePanel" )
                        contentItem: AppIcon {
                            text: sideTab.modelData.glyph
                            font.pixelSize: 18
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
                    if ( ctx ) ctx.overlayClicked( "sidePanel" )
                    App.sidePanelTab = currentIndex
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
                        font: Qt.font({ family: App.fontFamily, pixelSize: App.fontSize })
                        placeholderText: App.searchPlaceholder
                        onPressed: if ( ctx ) ctx.overlayClicked( "sidePanel" )
                        onTextChanged: {
                            App.setSearchFilter( text )
                            ComponentList.search( text )
                        }

                        Connections {
                            target: App
                            function onFocusSearchRequested() {
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
                        radius: 8
                        color: appTheme.base
                        border.color: appTheme.mid
                        border.width: 1
                        clip: true

                        Loader {
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
                        radius: 8
                        color: appTheme.base
                        border.color: appTheme.mid
                        border.width: 1
                        clip: true

                        Loader {
                            anchors.fill: parent
                            source: "OutPanelText.qml"
                            onLoaded: item.ctx = SimulatorLog
                        }
                    }
                }
                Item {
                    Rectangle {
                        anchors.fill: parent
                        radius: 8
                        color: appTheme.base
                        border.color: appTheme.mid
                        border.width: 1
                        clip: true

                        Loader {
                            anchors.fill: parent
                            source: "OutPanelText.qml"
                            onLoaded: item.ctx = CompilerLog
                        }
                    }
                }
            }
        }
    }

    // ---- Editor panel ----------------------------------------------------
    Rectangle {
        // NOT `id: editorPanel`: that shadowed the `editorPanel` context property
        // for everything this Loader creates, so editorpanel.qml's `ctx` resolved
        // to this Rectangle instead of EditorWidget and every binding on it read
        // as undefined. The objectName is what the focus tests match on.
        id: editorPanelOverlay
        objectName: "editorPanel"
        visible: rects.editorPanelVisible === true
        x: r("editorPanel").x; y: r("editorPanel").y
        width: r("editorPanel").width; height: r("editorPanel").height
        color: root.overlayBg
        radius: 6
        border.color: appTheme.mid
        border.width: 1
        z: root.overlayZ( "editorPanel" )

        // Absorbs unhandled pointer events (clicks on empty space, scrolling over margins)
        // and prevents any pointer events from leaking to the canvas below.
        MouseArea {
            anchors.fill: parent
            hoverEnabled: true
            acceptedButtons: Qt.AllButtons
            onPressed: (m) => { if ( ctx ) ctx.overlayClicked( "editorPanel" ) }
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

        Loader { anchors.fill: parent; anchors.margins: 1; source: "EditorPanel.qml" }
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
            required property var modelData
            // A hidden panel keeps publishing its rects, so without the second
            // half these strips stayed live over bare canvas.
            visible: root.r( modelData.key ).width > 0
                     && root.rects[ modelData.panel + "Visible" ] === true
            x: root.r( modelData.key ).x;     y: root.r( modelData.key ).y
            width: root.r( modelData.key ).width
            height: root.r( modelData.key ).height
            cursorShape: modelData.cursor
            // Just above the panel they belong to, so they keep their place in
            // the overlay stack. At z 0 the panel covered them: a corner sits
            // 7px inside it, leaving only a 3px sliver of it grabbable -- and
            // over a panel's own controls, nothing of it at all.
            z: root.overlayZ( modelData.panel ) + 0.5
            onPressed: (m) => {
                if ( ctx ) ctx.overlayClicked( modelData.panel )
                var p = mapToItem( root, m.x, m.y )
                ctx.gestureBegin( modelData.id, p.x, p.y )
            }
            onPositionChanged: (m) => { var p = mapToItem( root, m.x, m.y ); ctx.gestureMove( p.x, p.y ) }
            onReleased: ctx.gestureEnd()
            // Back to the default size, as double-clicking a splitter did.
            onDoubleClicked: ctx.resetOverlaySize( modelData.id.startsWith( "sidePanel" )
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
            readonly property rect box: root.r( modelData.key )
            readonly property bool panelVisible: root.rects[ modelData.id + "Visible" ] === true
            // A toggle stays put when its panel is hidden -- it is what brings it
            // back; a grip has nothing to drag then.
            visible: box.width > 0 && ( modelData.toggle || panelVisible )
            x: box.x; y: box.y
            width: box.width; height: box.height
            radius: height / 2
            color: pointer.containsMouse ? appTheme.midlight
                                         : ( modelData.id === "sidePanel" ? appTheme.window : root.overlayBg )
            border.color: appTheme.mid
            border.width: 1
            z: root.overlayZ( modelData.id ) + 0.5

            AppIcon {
                anchors.centerIn: parent
                // A drag handle always shows its grip dots. A toggle points
                // towards the panel it will act on: towards where the panel
                // currently sits while visible (the direction hiding it will
                // travel), and towards where it will reappear once hidden --
                // the opposite direction, since the pill has jumped to the far
                // margin by then (see CircuitWidget::updateOverlayLayout).
                text: !handle.modelData.toggle ? "drag_indicator"
                    : ( root.rects[ handle.modelData.id + "ClusterRight" ] === handle.panelVisible )
                      ? "chevron_left" : "chevron_right"
                font.pixelSize: 16
                color: appTheme.windowText
            }

            MouseArea {
                id: pointer
                anchors.fill: parent
                hoverEnabled: true
                cursorShape: handle.modelData.toggle ? Qt.PointingHandCursor : Qt.SizeAllCursor
                onClicked: if ( handle.modelData.toggle ) {
                    if ( ctx ) ctx.overlayClicked( handle.modelData.id )
                    ctx.togglePanel( handle.modelData.id )
                }
                onDoubleClicked: if ( !handle.modelData.toggle ) ctx.resetOverlayPos( handle.modelData.id )
                onPressed: (m) => {
                    if ( ctx ) ctx.overlayClicked( handle.modelData.id )
                    if ( !handle.modelData.toggle ) {
                        var p = mapToItem( root, m.x, m.y )
                        ctx.gestureBegin( handle.modelData.id, p.x, p.y )
                    }
                }
                onPositionChanged: (m) => {
                    if ( !handle.modelData.toggle && pressed ) {
                        var p = mapToItem( root, m.x, m.y )
                        ctx.gestureMove( p.x, p.y )
                    }
                }
                onReleased: if ( !handle.modelData.toggle ) ctx.gestureEnd()
            }
        }
    }

    // ---- Overlay toolbar --------------------------------------------------
    // Control, not Frame: Frame on macOS uses the native NSBox style which enforces
    // hardcoded native insets (10px top, 20px bottom) that ignore `padding`.
    // Control respects custom padding and background customization cleanly.
    Control {
        id: toolbar
        x: r("toolbar").x; y: r("toolbar").y
        padding: 4
        // Its x/y come back from updateOverlayLayout(), which needs the size it
        // just measured; this is the adjustSize() the QToolBar used to do.
        onImplicitWidthChanged: if ( ctx ) ctx.setToolbarSize( implicitWidth, implicitHeight )
        onImplicitHeightChanged: if ( ctx ) ctx.setToolbarSize( implicitWidth, implicitHeight )
        Component.onCompleted: if ( ctx ) ctx.setToolbarSize( implicitWidth, implicitHeight )
        z: root.overlayZ( "toolbar" )

        // Click-to-front lives inside the background, not as a sibling of the
        // Row: a Frame measures its implicit size from its one contentItem, and
        // a second loose child anchored to `parent` (whose size depends on that
        // same measurement) is a circular binding that collapsed the toolbar to
        // a near-zero "dot". Inside the background, the MouseArea's `parent` is
        // the already-sized Rectangle, so nothing feeds back into the sizing.
        background: Rectangle {
            color: root.overlayBg
            radius: 6
            border.color: appTheme.mid
            border.width: 1

            // Click-to-front: bring toolbar to front when clicking background/margins
            MouseArea {
                anchors.fill: parent
                onPressed: (m) => { if ( ctx ) ctx.overlayClicked( "toolbar" ) }
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
                    radius: 5
                    color: gripMouse.pressed
                             ? Qt.rgba( appTheme.windowText.r, appTheme.windowText.g, appTheme.windowText.b, 0.22 )
                             : gripMouse.containsMouse
                                 ? Qt.rgba( appTheme.windowText.r, appTheme.windowText.g, appTheme.windowText.b, 0.10 )
                                 : "transparent"
                    Behavior on color { ColorAnimation { duration: 90 } }
                }

                AppIcon {
                    anchors.centerIn: parent
                    text: "drag_indicator"
                    font.pixelSize: 16
                    color: appTheme.windowText
                }

                MouseArea {
                    id: gripMouse
                    anchors.fill: parent
                    hoverEnabled: true
                    cursorShape: Qt.SizeAllCursor
                    onDoubleClicked: ctx.resetOverlayPos( "toolbar" )
                    onPressed: (m) => {
                        if ( ctx ) ctx.overlayClicked( "toolbar" )
                        var p = mapToItem( root, m.x, m.y )
                        ctx.gestureBegin( "toolbar", p.x, p.y )
                    }
                    onPositionChanged: (m) => { var p = mapToItem( root, m.x, m.y ); ctx.gestureMove( p.x, p.y ) }
                    onReleased: ctx.gestureEnd()
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

            Repeater {
                model: ctx ? ctx.toolbarItems : []
                delegate: Loader {
                    required property var modelData
                    required property int index
                    visible: modelData.visible
                    // A Row top-aligns children of different heights, so without
                    // this the buttons sat above the taller items beside them.
                    anchors.verticalCenter: toolbarRow.verticalCenter
                    sourceComponent: modelData.kind === "separator" ? separatorItem
                                   : modelData.kind === "widget"    ? widgetItem
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
                            enabled: modelData.enabled
                            checkable: modelData.checkable
                            checked: modelData.checked
                            tint: (modelData.tint !== undefined && modelData.tint !== "") ? modelData.tint : appTheme.windowText
                            ToolTip.visible: hovered && modelData.tooltip !== ""
                            ToolTip.text: modelData.tooltip
                            onClicked: {
                                if ( ctx ) ctx.overlayClicked( "toolbar" )
                                ctx.toolbarTriggered( index )
                            }
                            contentItem: Item {
                                implicitWidth: 18
                                implicitHeight: 18
                                AppIcon {
                                    anchors.centerIn: parent
                                    visible: modelData.iconLigature !== undefined && modelData.iconLigature !== ""
                                    text: modelData.iconLigature || ""
                                    font.pixelSize: 16
                                    color: actionBtn.isCheckedActive ? appTheme.highlight : actionBtn.textColor
                                }
                                Image {
                                    anchors.fill: parent
                                    visible: !modelData.iconLigature || modelData.iconLigature === ""
                                    source: modelData.icon || ""
                                    fillMode: Image.PreserveAspectFit
                                    mipmap: true
                                }
                            }
                        }
                    }
                    Component {
                        id: widgetItem
                        Loader {
                            sourceComponent: modelData.widget === "msg"            ? textItem
                                           : modelData.widget === "coords"         ? coordsItem
                                           : modelData.widget === "zoom"           ? zoomItem
                                           : modelData.widget === "warnings"       ? warnItem
                                           : modelData.widget === "canvasOverflow" ? canvasOverflowItem
                                           : modelData.widget === "subcNav"        ? subcNavItem
                                           : modelData.widget === "deviceSel"      ? deviceSelItem
                                                                                   : embeddedItem
                        }
                    }
                }
            }
        }
    }

    // The run state sits in a coloured pill, as the QLabel did: green running,
    // amber stopped, red on error. CircuitWidget::setMsg picks the pair.
    Component {
        id: textItem
        Rectangle {
            implicitWidth: msgText.implicitWidth + 14
            implicitHeight: msgText.implicitHeight + 6
            radius: height / 2
            color: ctx ? ctx.messageBg : "transparent"
            visible: msgText.text !== ""
            Text {
                id: msgText
                anchors.centerIn: parent
                text: ctx ? ctx.messageText : ""
                font: Qt.font({ family: App.fontFamily, pixelSize: App.fontSize })
                color: ctx ? ctx.messageColor : appTheme.windowText
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
            implicitWidth: coordsRow.implicitWidth + 14
            implicitHeight: coordsRow.implicitHeight + 6
            radius: height / 2
            color: Qt.rgba( appTheme.windowText.r, appTheme.windowText.g,
                            appTheme.windowText.b, 0.10 )

            // Digits are not the widest glyph in every font and a minus sign
            // costs a slot of its own, so measure the real worst case rather
            // than assuming six zeroes.
            TextMetrics {
                id: valueMetrics
                font: Qt.font({ family: App.fontFamily, pixelSize: App.fontSize })
                text: "-99999"
            }
            Row {
                id: coordsRow
                anchors.centerIn: parent
                spacing: 3

                Text {
                    anchors.verticalCenter: parent.verticalCenter
                    text: App.translate( "X" )
                    font: Qt.font({ family: App.fontFamily, pixelSize: App.fontSize })
                    color: Qt.rgba( appTheme.windowText.r, appTheme.windowText.g,
                                    appTheme.windowText.b, 0.6 )
                }
                Text {
                    anchors.verticalCenter: parent.verticalCenter
                    // Grows past the reserved width only for a coordinate that
                    // genuinely needs more than six characters.
                    width: Math.max( valueMetrics.width, implicitWidth )
                    horizontalAlignment: Text.AlignRight
                    text: ctx ? ctx.coordsX : 0
                    font: Qt.font({ family: App.fontFamily, pixelSize: App.fontSize })
                    color: appTheme.windowText
                }
                Text {
                    anchors.verticalCenter: parent.verticalCenter
                    leftPadding: 7 // the gap between the two axes, not inside one
                    text: App.translate( "Y" )
                    font: Qt.font({ family: App.fontFamily, pixelSize: App.fontSize })
                    color: Qt.rgba( appTheme.windowText.r, appTheme.windowText.g,
                                    appTheme.windowText.b, 0.6 )
                }
                Text {
                    anchors.verticalCenter: parent.verticalCenter
                    width: Math.max( valueMetrics.width, implicitWidth )
                    horizontalAlignment: Text.AlignRight
                    text: ctx ? ctx.coordsY : 0
                    font: Qt.font({ family: App.fontFamily, pixelSize: App.fontSize })
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
            checked: ctx && ctx.zoomMenuOpen
            ToolTip.visible: hovered
            ToolTip.text: App.translate( "Zoom Options" )
            onClicked: {
                if ( ctx ) ctx.overlayClicked( "toolbar" )
                var leftX = zoomBtn.mapToGlobal( 0, 0 ).x
                var bottomY = toolbar.mapToGlobal( 0, toolbar.height ).y + 4
                ctx.showZoomMenu( 0, 0 )
                zoomMenu.popup()
            }
            contentItem: Row {
                id: zoomRow
                spacing: 1
                Text {
                    anchors.verticalCenter: parent.verticalCenter
                    text: ctx ? ctx.zoomText : ""
                    font: zoomBtn.font
                    color: appTheme.windowText
                }
                AppIcon {
                    anchors.verticalCenter: parent.verticalCenter
                    text: "arrow_drop_down"
                    font.pixelSize: 16
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
            visible: ctx && ctx.warningsText !== ""
            checked: overloadPopup.visible
            onClicked: {
                if ( ctx ) ctx.overlayClicked( "toolbar" )
                // Align right edge of dialog with right edge of warnBtn,
                // and place top of dialog 4px below overlay toolbar (matching infoCard spacing).
                var pt = warnBtn.mapToItem( root, warnBtn.width - overloadPopup.width, warnBtn.height + 4 )
                overloadPopup.toggle( pt )
            }
            contentItem: Row {
                spacing: 4
                AppIcon {
                    anchors.verticalCenter: parent.verticalCenter
                    text: "warning"
                    font.pixelSize: 16
                    color: ctx && ctx.warningsCrashed ? CircuitCanvas.msgErrorBg : CircuitCanvas.msgWarnBg
                }
                Text {
                    anchors.verticalCenter: parent.verticalCenter
                    text: ctx ? ctx.warningsText : ""
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
            visible: ctx && ctx.canvasOverflowText !== ""
            checked: canvasOverflowPopup.visible
            onClicked: {
                if ( ctx ) ctx.overlayClicked( "toolbar" )
                var pt = overflowBtn.mapToItem( root, overflowBtn.width - canvasOverflowPopup.width, overflowBtn.height + 4 )
                canvasOverflowPopup.toggle( pt )
            }
            contentItem: Row {
                spacing: 4
                AppIcon {
                    anchors.verticalCenter: parent.verticalCenter
                    text: "warning"
                    font.pixelSize: 16
                    color: CircuitCanvas.msgWarnBg
                }
                Text {
                    anchors.verticalCenter: parent.verticalCenter
                    text: ctx ? ctx.canvasOverflowText : ""
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
            visible: ctx && ctx.subcNav ? ctx.subcNav.visible : false
            checked: subcPopup.visible
            ToolTip.visible: hovered && ctx && ctx.subcNav && ctx.subcNav.buttonTooltip !== ""
            ToolTip.text: ctx && ctx.subcNav ? ctx.subcNav.buttonTooltip : ""
            onClicked: {
                if ( ctx ) ctx.overlayClicked( "toolbar" )
                if (subcPopup.visible) {
                    subcPopup.close()
                } else {
                    var pt = subcBtn.mapToItem(root, 0, subcBtn.height + 4)
                    subcPopup.x = Math.max(8, Math.min(root.width - subcPopup.width - 8, pt.x))
                    subcPopup.y = pt.y
                    subcPopup.open()
                }
            }
            contentItem: Row {
                spacing: 6
                anchors.verticalCenter: parent.verticalCenter
                AppIcon {
                    anchors.verticalCenter: parent.verticalCenter
                    text: (ctx && ctx.subcNav && ctx.subcNav.iconLigature) ? ctx.subcNav.iconLigature : "account_tree"
                    font.pixelSize: 18
                    color: appTheme.windowText
                }
                Text {
                    anchors.verticalCenter: parent.verticalCenter
                    text: ctx && ctx.subcNav ? ctx.subcNav.buttonText : ""
                    font: subcBtn.font
                    color: appTheme.windowText
                    elide: Text.ElideMiddle
                    maximumLineCount: 1
                }
                AppIcon {
                    anchors.verticalCenter: parent.verticalCenter
                    text: "arrow_drop_down"
                    font.pixelSize: 16
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
            visible: ctx && ctx.deviceSel ? ctx.deviceSel.visible : false
            checked: devicePopup.visible
            ToolTip.visible: hovered && ctx && ctx.deviceSel && ctx.deviceSel.buttonTooltip !== ""
            ToolTip.text: ctx && ctx.deviceSel ? ctx.deviceSel.buttonTooltip : ""
            onClicked: {
                if ( ctx ) ctx.overlayClicked( "toolbar" )
                if (devicePopup.visible) {
                    devicePopup.close()
                } else {
                    var pt = deviceBtn.mapToItem(root, 0, deviceBtn.height + 4)
                    devicePopup.x = Math.max(8, Math.min(root.width - devicePopup.width - 8, pt.x))
                    devicePopup.y = pt.y
                    devicePopup.open()
                }
            }
            contentItem: Row {
                spacing: 6
                anchors.verticalCenter: parent.verticalCenter
                AppIcon {
                    anchors.verticalCenter: parent.verticalCenter
                    text: (ctx && ctx.deviceSel && ctx.deviceSel.iconLigature) ? ctx.deviceSel.iconLigature : "memory"
                    font.pixelSize: 18
                    color: appTheme.windowText
                }
                Text {
                    anchors.verticalCenter: parent.verticalCenter
                    text: ctx && ctx.deviceSel ? ctx.deviceSel.buttonText : ""
                    font: deviceBtn.font
                    color: appTheme.windowText
                    elide: Text.ElideMiddle
                    maximumLineCount: 1
                }
                AppIcon {
                    anchors.verticalCenter: parent.verticalCenter
                    text: "arrow_drop_down"
                    font.pixelSize: 16
                    color: appTheme.windowText
                }
            }
        }
    }
    Component { id: embeddedItem; Item { width: 0; height: 0 } }

    // ---- Sim info card ----------------------------------------------------
    Item {
        id: infoCard
        x: r("infoCard").x; y: r("infoCard").y
        width: r("infoCard").width; height: r("infoCard").height
        visible: ctx ? ctx.infoVisible : false
        z: root.overlayZ( "infoCard" )

        MouseArea {
            anchors.fill: parent
            onPressed: (m) => { if ( ctx ) ctx.overlayClicked( "infoCard" ) }
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
        function publishInfoSize() {
            if ( ctx && infoItem ) ctx.setInfoCardSize( infoItem.implicitWidth, infoItem.implicitHeight )
        }
        Connections {
            target: infoCard.infoItem
            function onImplicitWidthChanged() { infoCard.publishInfoSize() }
            function onImplicitHeightChanged() { infoCard.publishInfoSize() }
        }

        // Close button
        AppToolButton {
            width: 24; height: 24
            x: parent.width - 28; y: 4
            padding: 0
            ToolTip.visible: hovered
            ToolTip.text: App.translate( "Hide" )
            onClicked: ctx.hideInfo()
            contentItem: AppIcon {
                text: "close"
                font.pixelSize: 16
                color: appTheme.windowText
            }
        }
    }

    AppContextMenu {
        id: zoomMenu
        onClosed: ctx.zoomMenuClosed()
        ContextMenuItem { text: App.translate( "Zoom to Fit" ); onTriggered: CircuitCanvas.zoomToFit() }
        ContextMenuItem { text: App.translate( "Zoom to Selection" ); onTriggered: CircuitCanvas.zoomSelected() }
        ContextMenuSeparator {}
        ContextMenuItem { text: App.translate( "Reset Zoom" ); onTriggered: CircuitCanvas.zoomOne() }
    }

    AppListPopup {
        id: subcPopup
        rows: ctx && ctx.subcNav ? ctx.subcNav.rows : []
        onAboutToShow: if (ctx) ctx.setSubcPopupOpen(true)
        onClosed: if (ctx) ctx.setSubcPopupOpen(false)
        onActivated: function(index) {
            if (ctx) ctx.activateSubcNav(index)
        }
    }

    AppListPopup {
        id: devicePopup
        rows: ctx && ctx.deviceSel ? ctx.deviceSel.rows : []
        onAboutToShow: if (ctx) ctx.setDevicePopupOpen(true)
        onClosed: if (ctx) ctx.setDevicePopupOpen(false)
        onActivated: function(index) {
            if (ctx) ctx.activateDevice(index)
        }
    }

    OverloadPanel {
        id: overloadPopup
    }

    CanvasOverflowPanel {
        id: canvasOverflowPopup
    }

    Connections {
        target: CircuitCanvas
        function onZoomChanged() { ctx.setZoomPercent( CircuitCanvas.zoom ) }
        function onCoordsChanged() { ctx.setCoords( CircuitCanvas.cursorX, CircuitCanvas.cursorY ) }
        function onRootCircuitLoaded() {
            ctx.resetSubcircuits( CircuitCanvas.filePath, CircuitCanvas.fileName, CircuitCanvas.subcircuitTree )
            ctx.updateDevices( CircuitCanvas.programmableDevices )
        }
        function onFilePathChanged() {
            ctx.updateDevices( CircuitCanvas.programmableDevices )
        }
        function onOpenSubcircuitRequested( path, label ) {
            ctx.navigateIntoSubcircuit( path, label )
            CircuitCanvas.loadSubcircuit( path )
        }
        function onDevicesChanged() {
            ctx.updateDevices( CircuitCanvas.programmableDevices )
        }
        function onWarningsChanged() {
            if (ctx) ctx.setWarnings(CircuitCanvas.warningsText, CircuitCanvas.warningsCrashed)
        }
        function onCanvasOverflowChanged() {
            if (ctx) {
                var text = CircuitCanvas.canvasOverflow ? App.translate("Canvas Overflow") : ""
                ctx.setCanvasOverflow(text)
            }
        }
        function onSimChanged() {
            if ( CircuitCanvas.simError )
                ctx.setMessage( App.translate( CircuitCanvas.simError ), CircuitCanvas.msgErrorBg, CircuitCanvas.msgErrorText )
            else if ( CircuitCanvas.simWarning )
                ctx.setMessage( App.translate( CircuitCanvas.simWarning ), CircuitCanvas.msgWarnBg, CircuitCanvas.msgWarnText )
            else if ( CircuitPanel.simRunning ) {
                if ( CircuitPanel.simPaused )
                    ctx.setMessage( App.translate( "Paused" ), CircuitCanvas.msgWarnBg, CircuitCanvas.msgWarnText )
                else
                    ctx.setMessage( App.translate( "Running" ), CircuitCanvas.msgOkBg, CircuitCanvas.msgOkText )
            }
        }
    }

    Connections {
        target: App
        function onI18nTickChanged() {
            if ( ctx ) ctx.reloadI18n()
            if ( CircuitCanvas.simError )
                ctx.setMessage( App.translate( CircuitCanvas.simError ), CircuitCanvas.msgErrorBg, CircuitCanvas.msgErrorText )
            else if ( CircuitCanvas.simWarning )
                ctx.setMessage( App.translate( CircuitCanvas.simWarning ), CircuitCanvas.msgWarnBg, CircuitCanvas.msgWarnText )
        }
        function onThemeChanged() {
            if ( ctx ) ctx.reloadI18n()
        }
    }

    Connections {
        target: CircuitPanel
        function onLoadCircuitRequested( path ) {
            CircuitCanvas.loadSubcircuit( path )
        }
        function onActiveDeviceChanged( id ) {
            CircuitCanvas.activeDeviceId = id
        }
        function onAction( id ) {
            if ( id === "zoomIn" ) CircuitCanvas.zoomIn()
            else if ( id === "zoomOut" ) CircuitCanvas.zoomOut()
            else if ( id === "powerOn" ) {
                CircuitPanel.showSidePanel()
                App.sidePanelTab = 2
                CircuitCanvas.powerOn()
            }
            else if ( id === "powerOff" ) CircuitCanvas.powerOff()
            else if ( id === "pauseQemu" ) CircuitCanvas.pauseQemu()
            else if ( id === "resumeQemu" ) CircuitCanvas.resumeQemu()
            else if ( id === "settCircuit" ) App.showCircuitSettings()
            else if ( id.startsWith( "openSubcircuit:" ) ) CircuitCanvas.openSubcircuit( id.substring( 15 ) )
        }
    }


    Connections {
        target: EditorPanel
        function onRequestShow() { CircuitPanel.showEditor() }
        function onRequestHide() { CircuitPanel.hideEditor() }
        function onFocusedChanged() { if ( EditorPanel.focused && ctx ) ctx.overlayClicked( "editorPanel" ) }
    }

    Connections {
        target: FileBrowser
        function onOpenFile( path ) {
            EditorPanel.loadFile( path )
            AppDialog.addRecentFile( path )
            AppMenuBar.rebuild()
        }
        function onOpenCircuit( path ) { CircuitCanvas.loadPath( path ) }
    }

}
