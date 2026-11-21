pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls
import QtQuick.Effects
import QtQuick.Layouts
import QtQuick.Window
import cs_app

/* The window every ported panel lives in.
 *
 * Panels were written as Items to be loaded into a QQuickWidget, so they stay
 * Items: this supplies the Window around them rather than each panel growing
 * its own. That is what lets a panel be shown either on its own or embedded,
 * and it keeps the port from touching 19 QML files just to change their root.
 *
 * Opening is staged via `openRequested` + `panelSource`: content is loaded,
 * sized, and positioned while native `visible` is still false so the OS window
 * appears once at its final geometry without flickering or repositioning.
 */
ApplicationWindow {
    id: appWindow
    SystemPalette { id: appTheme }

    property string panelSource: ""
    property var panelProperties: undefined
    property bool openRequested: false

    // Floor minimum dimensions preserved across adoptContentSize() updates.
    property int contentMinWidth: 0
    property int contentMinHeight: 0

    property alias source: loader.source
    property alias panel: loader.item
    readonly property Item panelItem: loader.item as Item

    // Mirrors QQuickWidget's SizeViewToRootObject plus QLayout::SetFixedSize:
    // the window takes the panel's own size and refuses to be resized. Without
    // it the window drives the panel, as SizeRootObjectToView did.
    property bool sizeToContent: false
    property bool resizable: !sizeToContent
    property bool maximizable: resizable

    // Remembers user-dragged position across close/reopen in the same session.
    property bool hasSavedPosition: false
    property int savedX: 0
    property int savedY: 0

    // Internal staging state.
    property bool _completed: false
    property bool _preparingOpen: false
    property bool _chromeSizeApplied: false
    property int _openedX: 0
    property int _openedY: 0

    // Linux custom CSD chrome matching Main.qml (frameless window, rounded frame,
    // client-side drop shadow, compact titlebar, and resize handles).
    readonly property bool linuxCustomChrome: Qt.platform.os === "linux" && App.customTitlebar
    readonly property bool linuxMaximized: appWindow.visibility === Window.Maximized || appWindow.visibility === Window.FullScreen
    property int linuxBaseShadowMargin: linuxCustomChrome ? 20 : 0
    readonly property int linuxShadowMargin: (linuxCustomChrome && !linuxMaximized) ? linuxBaseShadowMargin : 0
    readonly property bool linuxMarginsIncluded: linuxCustomChrome
    readonly property int linuxResizeOverhang: resizable ? 3 : 0
    readonly property int linuxResizeThickness: 5
    readonly property int linuxTitleBarHeight: linuxCustomChrome ? 34 : 0

    signal aboutToShow()

    visible: false

    flags: appWindow.linuxCustomChrome
           ? ((appWindow.maximizable ? Qt.Window : Qt.Dialog) | Qt.FramelessWindowHint)
           : appWindow.maximizable
           ? Qt.Window
           : (Qt.Dialog | Qt.CustomizeWindowHint | Qt.WindowTitleHint | Qt.WindowCloseButtonHint)

    topPadding: 0
    leftPadding: 0
    rightPadding: 0
    bottomPadding: 0

    // The declaring ApplicationWindow tracks us and closes us with itself.
    property bool closeWithOwner: true
    property Window ownerWindow: null
    property bool isQuitting: false

    // Closes the window on Escape (standard dialog behavior).
    property bool closeOnEscape: true

    font: Qt.font({ family: App.fontFamily, pixelSize: App.fontSize })
    color: appWindow.linuxCustomChrome ? "transparent" : appTheme.window
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
    transientParent: ownerWindow

    Binding {
        target: appWindow
        property: "minimumWidth"
        when: !appWindow.sizeToContent
              && (!appWindow.linuxCustomChrome || appWindow._chromeSizeApplied)
              && (appWindow.contentMinWidth > 0 || appWindow.linuxCustomChrome)
        value: (appWindow.contentMinWidth > 0 ? appWindow.contentMinWidth : 200) + appWindow.linuxShadowMargin * 2
    }

    Binding {
        target: appWindow
        property: "minimumHeight"
        when: !appWindow.sizeToContent
              && (!appWindow.linuxCustomChrome || appWindow._chromeSizeApplied)
              && (appWindow.contentMinHeight > 0 || appWindow.linuxCustomChrome)
        value: (appWindow.contentMinHeight > 0 ? appWindow.contentMinHeight : 80) + appWindow.linuxTitleBarHeight + appWindow.linuxShadowMargin * 2
    }

    Item {
        id: windowMask
        anchors.fill: windowFrame
        visible: false
        layer.enabled: appWindow.linuxShadowMargin > 0

        Rectangle {
            anchors.fill: parent
            radius: 14
        }
    }

    RectangularShadow {
        anchors.fill: windowFrame
        visible: appWindow.linuxShadowMargin > 0
        radius: 14
        blur: 14
        offset: Qt.vector2d(0, 3)
        color: appWindow.active ? App.windowShadow : App.windowShadowInactive
    }

    Rectangle {
        id: windowFrame
        anchors.fill: parent
        anchors.margins: appWindow.linuxShadowMargin
        radius: appWindow.linuxShadowMargin > 0 ? 14 : 0
        color: appTheme.window
        layer.enabled: appWindow.linuxShadowMargin > 0
        layer.effect: MultiEffect {
            maskEnabled: true
            maskSource: windowMask
        }

        Item {
            id: childTitleBar
            visible: appWindow.linuxCustomChrome
            anchors.top: parent.top
            anchors.left: parent.left
            anchors.right: parent.right
            height: visible ? appWindow.linuxTitleBarHeight : 0
            z: 10

            Item {
                anchors.fill: parent
                anchors.rightMargin: captionButtons.width

                TapHandler {
                    acceptedButtons: Qt.LeftButton
                    enabled: appWindow.maximizable
                    onDoubleTapped: {
                        if (appWindow.linuxMaximized)
                            appWindow.showNormal()
                        else
                            appWindow.showMaximized()
                    }
                }

                DragHandler {
                    target: null
                    acceptedButtons: Qt.LeftButton
                    onActiveChanged: {
                        if (active)
                            appWindow.startSystemMove()
                    }
                }
            }

            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: 12
                anchors.rightMargin: captionButtons.width + 8
                spacing: 8

                Image {
                    source: "qrc:/icons/circuitsimulator.png"
                    Layout.preferredWidth: 16
                    Layout.preferredHeight: 16
                    Layout.alignment: Qt.AlignVCenter
                    sourceSize.width: 32
                    sourceSize.height: 32
                    fillMode: Image.PreserveAspectFit
                    smooth: true
                    opacity: appWindow.active ? 1.0 : 0.65
                }

                AppText {
                    text: appWindow.title
                    font.pixelSize: App.fontSmall
                    color: appTheme.windowText
                    opacity: appWindow.active ? 1.0 : 0.65
                    elide: Text.ElideRight
                    maximumLineCount: 1
                    verticalAlignment: Text.AlignVCenter
                    Layout.fillWidth: true
                    Layout.alignment: Qt.AlignVCenter
                }
            }

            WindowCaptionButtons {
                id: captionButtons
                anchors.top: parent.top
                anchors.right: parent.right
                height: parent.height
                window: appWindow
                showMinimize: appWindow.maximizable
                showMaximize: appWindow.maximizable
                windowActive: appWindow.active
                topRightRadius: appWindow.linuxShadowMargin > 0 ? 14 : 0
            }

            Rectangle {
                anchors.left: parent.left
                anchors.right: parent.right
                anchors.bottom: parent.bottom
                height: 1
                color: appTheme.mid
                opacity: 0.4
            }
        }

        Loader {
            id: loader
            anchors.top: childTitleBar.bottom
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.bottom: parent.bottom
            focus: true
            onLoaded: if (appWindow.sizeToContent) appWindow.adoptContentSize()
        }
    }

    Rectangle {
        anchors.fill: windowFrame
        z: 20
        visible: appWindow.linuxShadowMargin > 0
        radius: 14
        color: "transparent"
        border.color: appTheme.mid
        border.width: 1
    }

    WindowResizeHandles {
        targetWindow: appWindow
        targetItem: windowFrame
        overhang: appWindow.linuxResizeOverhang
        handleThickness: appWindow.linuxResizeThickness
        visible: appWindow.linuxShadowMargin > 0 && appWindow.resizable
    }

    Connections {
        target: (appWindow.sizeToContent && loader.item) ? loader.item : null
        function onImplicitWidthChanged() { appWindow.adoptContentSize() }
        function onImplicitHeightChanged() { appWindow.adoptContentSize() }
    }

    function adoptContentSize(): void {
        const item = loader.item as Item
        if (!item) return
        const rawW = item.implicitWidth > 0 ? item.implicitWidth : item.width
        const rawH = item.implicitHeight > 0 ? item.implicitHeight : item.height
        if (rawW <= 0 || rawH <= 0) return
        const shadowPad = appWindow.linuxShadowMargin * 2
        const w = Math.max(appWindow.contentMinWidth, Math.ceil(rawW)) + shadowPad
        const h = Math.max(appWindow.contentMinHeight, Math.ceil(rawH)) + appWindow.linuxTitleBarHeight + shadowPad
        const targetMaxW = appWindow.resizable ? 16777215 : w
        const targetMaxH = appWindow.resizable ? 16777215 : h
        if (appWindow.width === w
            && appWindow.height === h
            && appWindow.minimumWidth === w
            && appWindow.minimumHeight === h
            && appWindow.maximumWidth === targetMaxW
            && appWindow.maximumHeight === targetMaxH) {
            return
        }
        if (w > appWindow.maximumWidth)
            appWindow.maximumWidth = 16777215
        if (h > appWindow.maximumHeight)
            appWindow.maximumHeight = 16777215
        if (w < appWindow.minimumWidth)
            appWindow.minimumWidth = w
        if (h < appWindow.minimumHeight)
            appWindow.minimumHeight = h
        appWindow.width = w
        appWindow.height = h
        appWindow.minimumWidth = w
        appWindow.minimumHeight = h
        appWindow.maximumWidth = targetMaxW
        appWindow.maximumHeight = targetMaxH
    }

    function _screenForPoint(px: int, py: int, w: int, h: int): var {
        const cx = px + Math.round(w / 2)
        const cy = py + Math.round(h / 2)
        const screens = Application.screens
        if (screens) {
            for (let i = 0; i < screens.length; i++) {
                const s = screens[i]
                if (s && cx >= s.virtualX && cx < s.virtualX + s.width
                    && cy >= s.virtualY && cy < s.virtualY + s.height) {
                    return s
                }
            }
        }
        return appWindow.screen || (ownerWindow ? ownerWindow.screen : null)
    }

    function _clampToScreen(px: int, py: int, w: int, h: int, scr: var): var {
        if (!scr || scr.width <= 0 || scr.height <= 0)
            return { x: px, y: py }
        const margin = appWindow.linuxShadowMargin
        const minX = scr.virtualX - margin
        const maxX = Math.max(minX, scr.virtualX + scr.width - w + margin)
        // Reserve top clearance on native chrome so the OS title bar stays reachable.
        const topInset = appWindow.linuxCustomChrome ? 0 : 32
        const minY = scr.virtualY + topInset - margin
        const maxY = Math.max(minY, scr.virtualY + scr.height - h + margin)
        return {
            x: Math.max(minX, Math.min(px, maxX)),
            y: Math.max(minY, Math.min(py, maxY))
        }
    }

    function _positionWindow(): void {
        const shadowPad = appWindow.linuxShadowMargin * 2
        const w = appWindow.width > 0
            ? appWindow.width
            : Math.max(appWindow.minimumWidth, appWindow.contentMinWidth + shadowPad)
        const h = appWindow.height > 0
            ? appWindow.height
            : Math.max(appWindow.minimumHeight, appWindow.contentMinHeight + appWindow.linuxTitleBarHeight + shadowPad)
        let targetX = appWindow.x
        let targetY = appWindow.y
        let scr = ownerWindow ? ownerWindow.screen : appWindow.screen

        if (appWindow.hasSavedPosition) {
            targetX = appWindow.savedX
            targetY = appWindow.savedY
            scr = appWindow._screenForPoint(targetX, targetY, w, h)
        } else if (ownerWindow && ownerWindow.width > 0 && ownerWindow.height > 0) {
            targetX = ownerWindow.x + Math.round((ownerWindow.width - w) / 2)
            targetY = ownerWindow.y + Math.round((ownerWindow.height - h) / 2)
        }

        const clamped = appWindow._clampToScreen(targetX, targetY, w, h, scr)
        appWindow.x = clamped.x
        appWindow.y = clamped.y
        appWindow._openedX = clamped.x
        appWindow._openedY = clamped.y
    }

    function _recordUserPosition(): void {
        if (!appWindow.visible || appWindow._preparingOpen || appWindow.visibility !== Window.Windowed)
            return
        if (!appWindow.hasSavedPosition) {
            if (Math.abs(appWindow.x - appWindow._openedX) <= 8
                && Math.abs(appWindow.y - appWindow._openedY) <= 8) {
                return
            }
            appWindow.hasSavedPosition = true
        }
        appWindow.savedX = appWindow.x
        appWindow.savedY = appWindow.y
    }

    function _loadPanel(): void {
        if (appWindow.panelSource.length === 0)
            return
        if (appWindow.panelProperties !== undefined) {
            loader.setSource(appWindow.panelSource, appWindow.panelProperties)
        } else {
            loader.source = appWindow.panelSource
        }
    }

    function _beginOpen(): void {
        if (appWindow._preparingOpen)
            return
        if (appWindow.visible) {
            appWindow.bringToFront()
            return
        }
        appWindow._preparingOpen = true
        if (!appWindow.sizeToContent && appWindow.linuxCustomChrome && !appWindow._chromeSizeApplied) {
            if (appWindow.width > 0)
                appWindow.width += appWindow.linuxBaseShadowMargin * 2
            if (appWindow.height > 0)
                appWindow.height += appWindow.linuxTitleBarHeight + appWindow.linuxBaseShadowMargin * 2
            appWindow._chromeSizeApplied = true
        }
        appWindow._loadPanel()
        if (appWindow.sizeToContent) {
            appWindow.adoptContentSize()
        } else {
            if (appWindow.minimumWidth > 0 && appWindow.width < appWindow.minimumWidth)
                appWindow.width = appWindow.minimumWidth
            if (appWindow.minimumHeight > 0 && appWindow.height < appWindow.minimumHeight)
                appWindow.height = appWindow.minimumHeight
        }
        appWindow.aboutToShow()
        appWindow._positionWindow()
        appWindow._preparingOpen = false
        appWindow.visible = true
        appWindow.raise()
        appWindow.requestActivate()
    }

    function _endClose(): void {
        if (appWindow.activeFocusItem)
            appWindow.activeFocusItem.focus = false
        if (appWindow.visible)
            appWindow.visible = false
        if (loader.source !== "")
            loader.source = ""
    }

    function loadSource(src: string, initialProperties: var): void {
        if (initialProperties !== undefined) {
            loader.setSource(src, initialProperties)
        } else {
            loader.source = src
        }
    }

    function open(): void {
        if (!appWindow.openRequested) {
            appWindow.openRequested = true
        } else {
            appWindow.bringToFront()
        }
    }

    function bringToFront(): void {
        if (appWindow._preparingOpen)
            return
        if (!appWindow.openRequested) {
            appWindow.openRequested = true
            return
        }
        if (!appWindow.visible)
            return
        if (appWindow.visibility === Window.Minimized) {
            appWindow.showNormal()
        }
        appWindow.raise()
        appWindow.requestActivate()
    }

    Component.onCompleted: {
        appWindow._completed = true
        if (appWindow.openRequested)
            appWindow._beginOpen()
    }

    onOpenRequestedChanged: {
        if (!appWindow._completed)
            return
        if (appWindow.openRequested)
            appWindow._beginOpen()
        else
            appWindow._endClose()
    }

    onXChanged: _recordUserPosition()
    onYChanged: _recordUserPosition()

    onVisibleChanged: {
        if (!visible && activeFocusItem) {
            activeFocusItem.focus = false
        }
    }

    onClosing: (close) => {
        if (activeFocusItem) {
            activeFocusItem.focus = false
        }
    }

    // Instance onClosing handlers hide via source properties (App.oscVisible,
    // etc.) so the openRequested: binding stays intact.
    function dismiss(): void {
        if (activeFocusItem) {
            activeFocusItem.focus = false
        }
        if (visible) {
            close()
        } else if (openRequested) {
            openRequested = false
        }
    }

    Shortcut {
        sequence: "Escape"
        enabled: appWindow.closeOnEscape && appWindow.visible
        onActivated: appWindow.dismiss()
    }
}
