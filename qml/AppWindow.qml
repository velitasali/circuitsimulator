import QtQuick
import QtQuick.Window
import QtQuick.Controls
import cs_app

/* The window every ported panel lives in.
 *
 * Panels were written as Items to be loaded into a QQuickWidget, so they stay
 * Items: this supplies the Window around them rather than each panel growing
 * its own. That is what lets a panel be shown either on its own or embedded,
 * and it keeps the port from touching 19 QML files just to change their root.
 */
ApplicationWindow {
    id: appWindow
    SystemPalette { id: appTheme }

    property alias source: loader.source
    property alias panel: loader.item

    // Mirrors QQuickWidget's SizeViewToRootObject plus QLayout::SetFixedSize:
    // the window takes the panel's own size and refuses to be resized. Without
    // it the window drives the panel, as SizeRootObjectToView did.
    property bool sizeToContent: false

    // The declaring ApplicationWindow tracks us and closes us with itself.
    property bool closeWithOwner: true
    property Window ownerWindow: (typeof win !== "undefined" && win !== appWindow) ? win : null
    property bool isQuitting: false

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
    transientParent: ownerWindow

    Loader {
        id: loader
        anchors.fill: parent
        focus: true
        onLoaded: if ( appWindow.sizeToContent ) appWindow.adoptContentSize()
    }

    Connections {
        target: (appWindow.sizeToContent && loader.item) ? loader.item : null
        function onImplicitWidthChanged() { appWindow.adoptContentSize() }
        function onImplicitHeightChanged() { appWindow.adoptContentSize() }
    }

    function adoptContentSize() {
        if ( !loader.item ) return
        var w = loader.item.implicitWidth > 0 ? loader.item.implicitWidth : loader.item.width
        var h = loader.item.implicitHeight > 0 ? loader.item.implicitHeight : loader.item.height
        if ( w <= 0 || h <= 0 ) return
        appWindow.width = w
        appWindow.height = h
        appWindow.minimumWidth = w
        appWindow.minimumHeight = h
    }

    function loadSource(src, initialProperties) {
        if (initialProperties !== undefined) {
            loader.setSource(src, initialProperties)
        } else {
            loader.source = src
        }
    }

    function bringToFront() {
        if (!appWindow.visible) {
            appWindow.visible = true
        }
        if (appWindow.visibility === Window.Minimized) {
            appWindow.showNormal()
        }
        appWindow.show()
        appWindow.raise()
        appWindow.requestActivate()
    }

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
    // etc.) so the visible: binding stays intact. Do not assign visible here.
    function dismiss() {
        if (activeFocusItem) {
            activeFocusItem.focus = false
        }
        if (visible)
            close()
    }

    Component.onCompleted: {
        if (ownerWindow && typeof ownerWindow.registerOwnedWindow === "function")
            ownerWindow.registerOwnedWindow(appWindow)
    }

    Component.onDestruction: {
        if (ownerWindow && typeof ownerWindow.unregisterOwnedWindow === "function")
            ownerWindow.unregisterOwnedWindow(appWindow)
    }
}
