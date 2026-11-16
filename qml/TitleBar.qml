pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtQuick.Templates as T
import QtQuick.Window
import cs_app
import cs_ui

/* Transparent main-window title strip over the live canvas.
 *
 * Windows: WinChromeNativeFilter returns HTCLIENT on caption MouseAreas
 * (titleBarInteractive), HTTOP along the top resize band, and HTCAPTION on
 * the icon, title, and empty strip so the window drags. The strip draws
 * Minimize / Maximize / Close.
 *
 * macOS: traffic lights stay. Qt.ExpandedClientAreaHint extends the canvas
 * under them. A press on this strip drags; a double-click zooms. The
 * caption buttons stay hidden.
 */
Item {
    id: bar
    objectName: "titleBar"
    SystemPalette { id: appTheme }

    required property Window window

    readonly property bool macOS: Qt.platform.os === "osx"
    // Full left margin. 74 matches the native fallback until the zoom
    // button is measured. 0 means the default 8px margin on Windows.
    property int leadingInset: macOS ? 74 : 0

    function refreshChrome() {
        if (!macOS)
            return
        leadingInset = App.titlebarLeadingInset()
    }

    Connections {
        target: bar.window
        function onVisibilityChanged() { Qt.callLater(bar.refreshChrome) }
    }

    readonly property bool windowActive: window ? window.active : true
    readonly property bool maximized: window ? (window.visibility === Window.Maximized || window.visibility === Window.FullScreen) : false
    readonly property string projectName: FileBrowser.rootPath
        ? (FileBrowser.rootPath.split("/").filter(Boolean).pop() || "")
        : ""
    readonly property string fileLabel: (CircuitCanvas.fileName ? CircuitCanvas.fileName : I18n.tr("Untitled"))
        + (CircuitCanvas.modified ? "*" : "")

    readonly property int leftEdge: bar.leadingInset > 0 ? bar.leadingInset : 8
    readonly property int rightEdge: (captionCluster.visible ? captionCluster.width : 0) + 12
    readonly property int usableWidth: Math.max(0, bar.width - bar.leftEdge - bar.rightEdge)
    readonly property int pillGap: 8
    // icon 16 + row spacing 4 + pill padding 14. The row drops a hidden MenuBar,
    // so the fit check cannot read menuPill.implicitWidth.
    readonly property int menuPillChrome: 34
    readonly property int menuPillWidth: menuBar.implicitWidth + bar.menuPillChrome
    // Floors of the title pill once the menu icon has moved onto the menu pill:
    // sim chip, project badge (48), file label (64), plus the pill's own padding.
    readonly property int titleContentMinWidth: {
        var w = 16 + Math.min(fileLabelText.implicitWidth, 64)
        if (bar.projectName !== "")
            w += Math.min(projText.implicitWidth + 10, 48) + leftGroup.spacing
        if (CircuitCanvas.simRunning)
            w += simStateText.implicitWidth + 12 + leftGroup.spacing
        return w
    }
    readonly property bool menuBarVisible: !App.nativeMenus
        && menuBar.implicitWidth > 0
        && (bar.usableWidth - bar.menuPillWidth - bar.pillGap >= bar.titleContentMinWidth)

    onMenuBarVisibleChanged: {
        if (!menuBarVisible)
            closeAllMenus()
    }

    implicitHeight: 34
    height: visible ? 34 : 0

    AppMenuBuilder {
        id: menuBuilder
    }

    function anyMenuOpen(): bool {
        for (var i = 0; i < menuBar.count; ++i) {
            var m = menuBar.menuAt(i)
            if (m && m.opened)
                return true
        }
        return false
    }

    function syncAltMenuOpen(): void {
        if (bar.menuBarVisible)
            AppMenuBar.setAltMenuOpen(bar.anyMenuOpen())
    }

    function closeAllMenus(): void {
        for (var i = 0; i < menuBar.count; ++i) {
            var m = menuBar.menuAt(i)
            if (m && m.opened)
                m.close()
        }
        if (!App.nativeMenus)
            AppMenuBar.setAltMenuOpen(false)
    }

    Connections {
        target: AppMenuBar
        function onMenusChanged(): void {
            for (var i = 0; i < menuBar.count; ++i) {
                var m = menuBar.menuAt(i) as AppContextMenu
                if (m)
                    m.menuDirty = true
            }
        }
        function onTogglePopupRequested(): void {
            // The bar is already on screen, so this release must not open a menu.
            // Closing is the release half of a press the filter ate while a menu was open.
            if (!bar.visible || !bar.menuBarVisible)
                return
            if (bar.anyMenuOpen()) {
                bar.closeAllMenus()
                AppMenuBar.noteAltToggled(true)
                return
            }
            AppMenuBar.noteAltToggled(false)
        }
    }

    // Linux drag / double-click-maximize background (below interactive pills and caption buttons).
    Item {
        anchors.fill: parent
        anchors.rightMargin: captionCluster.visible ? captionCluster.width : 0
        visible: Qt.platform.os === "linux"

        TapHandler {
            acceptedButtons: Qt.LeftButton
            onDoubleTapped: {
                if (!bar.window)
                    return
                if (bar.maximized)
                    bar.window.showNormal()
                else
                    bar.window.showMaximized()
            }
        }

        DragHandler {
            target: null
            acceptedButtons: Qt.LeftButton
            onActiveChanged: {
                if (active && bar.window)
                    bar.window.startSystemMove()
            }
        }
    }

    // Left (non-macOS when wide enough): app icon + inline menu bar over a translucent pill.
    // The container stays visible so the bar's contentWidth can be measured. The MenuBar
    // itself hides when it does not fit; that width does not collapse with visible.
    Item {
        id: menuContainer
        visible: !App.nativeMenus
        enabled: bar.menuBarVisible
        opacity: bar.menuBarVisible ? 1.0 : 0.0
        anchors.left: parent.left
        anchors.leftMargin: bar.leftEdge
        anchors.verticalCenter: parent.verticalCenter
        height: 26
        width: bar.menuBarVisible ? menuPill.implicitWidth : 0
        clip: true

        Rectangle {
            id: menuPill
            anchors.left: parent.left
            anchors.verticalCenter: parent.verticalCenter
            height: 26
            implicitWidth: menuPillRow.implicitWidth + 14
            width: implicitWidth
            radius: 6
            color: Qt.rgba(appTheme.window.r, appTheme.window.g, appTheme.window.b, 0.72)
            opacity: bar.windowActive ? 1.0 : 0.7

            Row {
                id: menuPillRow
                anchors.left: parent.left
                anchors.leftMargin: 8
                anchors.verticalCenter: parent.verticalCenter
                spacing: 4

                Image {
                    anchors.verticalCenter: parent.verticalCenter
                    source: "qrc:/icons/circuitsimulator.png"
                    width: 16
                    height: 16
                    sourceSize.width: 32
                    sourceSize.height: 32
                    fillMode: Image.PreserveAspectFit
                    smooth: true
                }

                T.MenuBar {
                    id: menuBar
                    // The Alt filter keys off isVisible(), not enabled or opacity.
                    // A collapsed bar would still take the hamburger's Alt tap.
                    visible: bar.menuBarVisible
                    anchors.verticalCenter: parent.verticalCenter
                    implicitWidth: contentWidth + leftPadding + rightPadding
                    implicitHeight: Math.max(22, contentHeight + topPadding + bottomPadding)
                    leftPadding: 0
                    rightPadding: 0
                    topPadding: 0
                    bottomPadding: 0
                    spacing: 1
                    font: Qt.font({ family: App.fontFamily, pixelSize: App.fontSmall })
                    background: null

                    contentItem: Row {
                        spacing: menuBar.spacing
                        Repeater {
                            id: menuBarRepeater
                            model: menuBar.contentModel
                        }
                    }

                    delegate: T.MenuBarItem {
                        id: barItem
                        property bool titleBarInteractive: true
                        enabled: bar.menuBarVisible
                        implicitWidth: contentItem.implicitWidth + leftPadding + rightPadding
                        implicitHeight: 22
                        font: menuBar.font
                        hoverEnabled: true
                        leftPadding: 7
                        rightPadding: 7
                        topPadding: 0
                        bottomPadding: 0

                        readonly property var mnemonic: UiUtils.splitMnemonic(barItem.text)
                        readonly property real labelOpacity: bar.windowActive ? (enabled ? 1.0 : 0.4) : 0.75

                        contentItem: Item {
                            implicitWidth: mnemonicRow.implicitWidth
                            implicitHeight: mnemonicRow.implicitHeight

                            Row {
                                id: mnemonicRow
                                anchors.centerIn: parent
                                spacing: 0

                                AppText {
                                    text: barItem.mnemonic.lead
                                    font.pixelSize: App.fontSmall
                                    color: appTheme.windowText
                                    opacity: barItem.labelOpacity
                                }
                                AppText {
                                    text: barItem.mnemonic.mark
                                    font.pixelSize: App.fontSmall
                                    font.underline: true
                                    visible: text !== ""
                                    color: appTheme.windowText
                                    opacity: barItem.labelOpacity
                                }
                                AppText {
                                    text: barItem.mnemonic.tail
                                    font.pixelSize: App.fontSmall
                                    color: appTheme.windowText
                                    opacity: barItem.labelOpacity
                                }
                            }
                        }

                        background: Rectangle {
                            radius: 4
                            color: (barItem.down || barItem.highlighted)
                                ? Qt.rgba(appTheme.windowText.r, appTheme.windowText.g, appTheme.windowText.b, 0.16)
                                : barItem.hovered
                                    ? Qt.rgba(appTheme.windowText.r, appTheme.windowText.g, appTheme.windowText.b, 0.10)
                                    : "transparent"
                            Behavior on color { ColorAnimation { duration: 80 } }
                        }
                    }

                    Instantiator {
                        model: (!App.nativeMenus && AppMenuBar.menus) ? AppMenuBar.menus.length : 0
                        onObjectAdded: (index, object) => menuBar.insertMenu(index, object)
                        onObjectRemoved: (_, object) => menuBar.removeMenu(object)
                        delegate: AppContextMenu {
                            id: topMenu
                            required property int index
                            property bool menuDirty: true
                            popupType: Popup.Item
                            focus: true
                            title: {
                                var ms = AppMenuBar.menus
                                return (ms && index >= 0 && index < ms.length && ms[index])
                                    ? String(ms[index].title || "")
                                    : ""
                            }
                            onOpenedChanged: bar.syncAltMenuOpen()
                            onAboutToShow: {
                                if (topMenu.menuDirty || topMenu.count === 0) {
                                    var ms = AppMenuBar.menus
                                    var items = (ms && index >= 0 && index < ms.length && ms[index])
                                        ? ms[index].items
                                        : []
                                    menuBuilder.populateMenu(topMenu, items)
                                    topMenu.menuDirty = false
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Active file title pill: live simulation state badge, project badge & active file title
    // (plus app icon when the inline menu bar is not shown). Centers in the titlebar and
    // clamps between the left controls (menu bar / traffic lights) and right caption buttons,
    // expanding to fit directory and file names and truncating with ellipsis only when constrained.
    Rectangle {
        id: leftPill
        anchors.verticalCenter: parent.verticalCenter
        height: 26
        readonly property int minLeftX: bar.menuBarVisible
            ? (bar.leftEdge + menuContainer.width + bar.pillGap)
            : bar.leftEdge
        readonly property int maxRightX: Math.max(minLeftX, bar.width - bar.rightEdge)
        readonly property int maxWidth: Math.max(0, maxRightX - minLeftX)
        width: Math.min(leftGroup.implicitWidth + 16, maxWidth)
        x: Math.max(minLeftX, Math.min(maxRightX - width, Math.round((bar.width - width) / 2)))
        visible: width > 0
        clip: true
        radius: 6
        color: Qt.rgba(appTheme.window.r, appTheme.window.g, appTheme.window.b, 0.72)
        opacity: bar.windowActive ? 1.0 : 0.7

        RowLayout {
            id: leftGroup
            anchors.fill: parent
            anchors.leftMargin: 8
            anchors.rightMargin: 8
            spacing: 8

            Image {
                visible: !bar.menuBarVisible
                source: "qrc:/icons/circuitsimulator.png"
                Layout.preferredWidth: 16
                Layout.preferredHeight: 16
                sourceSize.width: 32
                sourceSize.height: 32
                fillMode: Image.PreserveAspectFit
                smooth: true
            }

            Rectangle {
                visible: CircuitCanvas.simRunning
                radius: height / 2
                color: CircuitCanvas.simPaused ? CircuitCanvas.msgWarnBg : CircuitCanvas.msgOkBg
                implicitWidth: simStateText.implicitWidth + 12
                implicitHeight: simStateText.implicitHeight + 4
                Layout.preferredWidth: implicitWidth
                Layout.minimumWidth: implicitWidth

                AppText {
                    id: simStateText
                    anchors.centerIn: parent
                    font.pixelSize: App.fontSmall
                    text: CircuitCanvas.simPaused ? I18n.tr("Paused") : I18n.tr("Running")
                    color: CircuitCanvas.simPaused ? CircuitCanvas.msgWarnText : CircuitCanvas.msgOkText
                }
            }

            Rectangle {
                visible: bar.projectName !== ""
                radius: 4
                color: Qt.rgba(appTheme.windowText.r, appTheme.windowText.g, appTheme.windowText.b, 0.08)
                readonly property real naturalWidth: projText.implicitWidth + 10
                implicitWidth: naturalWidth
                implicitHeight: projText.implicitHeight + 4
                Layout.fillWidth: true
                Layout.preferredWidth: naturalWidth
                Layout.maximumWidth: naturalWidth
                Layout.minimumWidth: Math.min(naturalWidth, 48)

                AppText {
                    id: projText
                    anchors.centerIn: parent
                    width: Math.min(implicitWidth, Math.max(0, parent.width - 10))
                    font.pixelSize: App.fontSmall
                    text: bar.projectName
                    color: appTheme.windowText
                    elide: Text.ElideRight
                }
            }

            AppText {
                id: fileLabelText
                Layout.fillWidth: true
                Layout.preferredWidth: implicitWidth
                Layout.maximumWidth: implicitWidth
                Layout.minimumWidth: Math.min(implicitWidth, 64)
                font.pixelSize: App.fontSmall
                text: bar.fileLabel
                color: appTheme.windowText
                elide: Text.ElideMiddle
                maximumLineCount: 1
            }
        }

        // Labels accept the left button, so the bar-wide handler under this pill never sees the press.
        Item {
            anchors.fill: parent
            visible: Qt.platform.os === "linux"
            z: 1

            TapHandler {
                acceptedButtons: Qt.LeftButton
                onDoubleTapped: {
                    if (!bar.window)
                        return
                    if (bar.maximized)
                        bar.window.showNormal()
                    else
                        bar.window.showMaximized()
                }
            }

            DragHandler {
                target: null
                acceptedButtons: Qt.LeftButton
                onActiveChanged: {
                    if (active && bar.window)
                        bar.window.startSystemMove()
                }
            }
        }
    }

    // Right: Windows-style Minimize / Maximize-Restore / Close caption buttons
    // over a subtle translucent backing so glyphs remain legible over the canvas.
    // macOS keeps the traffic lights, so this cluster stays hidden.
    Rectangle {
        id: captionCluster
        visible: !bar.macOS
        anchors.top: parent.top
        anchors.right: parent.right
        width: captionButtons.width
        height: parent.height
        color: Qt.rgba(appTheme.window.r, appTheme.window.g, appTheme.window.b, 0.78)
        topRightRadius: (Qt.platform.os === "linux" && !bar.maximized) ? 10 : 0

        Row {
            id: captionButtons
            height: parent.height
            spacing: 0

            Rectangle {
                width: 46
                height: captionButtons.height
                color: minMouse.pressed
                    ? Qt.rgba(appTheme.windowText.r, appTheme.windowText.g, appTheme.windowText.b, 0.16)
                    : minMouse.containsMouse
                        ? Qt.rgba(appTheme.windowText.r, appTheme.windowText.g, appTheme.windowText.b, 0.10)
                        : "transparent"
                Behavior on color { ColorAnimation { duration: 80 } }

                Rectangle {
                    anchors.centerIn: parent
                    width: 10
                    height: 1
                    color: appTheme.windowText
                    opacity: bar.windowActive ? 0.9 : 0.6
                }

                MouseArea {
                    id: minMouse
                    anchors.fill: parent
                    hoverEnabled: true
                    property bool titleBarInteractive: true
                    ToolTip.visible: containsMouse
                    ToolTip.delay: 600
                    ToolTip.text: I18n.tr("Minimize")
                    onClicked: if (bar.window) bar.window.showMinimized()
                }
            }

            Rectangle {
                width: 46
                height: captionButtons.height
                color: maxMouse.pressed
                    ? Qt.rgba(appTheme.windowText.r, appTheme.windowText.g, appTheme.windowText.b, 0.16)
                    : maxMouse.containsMouse
                        ? Qt.rgba(appTheme.windowText.r, appTheme.windowText.g, appTheme.windowText.b, 0.10)
                        : "transparent"
                Behavior on color { ColorAnimation { duration: 80 } }

                Rectangle {
                    visible: !bar.maximized
                    anchors.centerIn: parent
                    width: 10
                    height: 10
                    color: "transparent"
                    border.width: 1
                    border.color: appTheme.windowText
                    opacity: bar.windowActive ? 0.9 : 0.6
                }

                Item {
                    visible: bar.maximized
                    anchors.centerIn: parent
                    width: 10
                    height: 10
                    opacity: bar.windowActive ? 0.9 : 0.6

                    Rectangle {
                        x: 2
                        y: 0
                        width: 8
                        height: 8
                        color: "transparent"
                        border.width: 1
                        border.color: appTheme.windowText
                    }
                    Rectangle {
                        x: 0
                        y: 2
                        width: 8
                        height: 8
                        color: appTheme.window
                        border.width: 1
                        border.color: appTheme.windowText
                    }
                }

                MouseArea {
                    id: maxMouse
                    anchors.fill: parent
                    hoverEnabled: true
                    property bool titleBarInteractive: true
                    ToolTip.visible: containsMouse
                    ToolTip.delay: 600
                    ToolTip.text: bar.maximized ? I18n.tr("Restore") : I18n.tr("Maximize")
                    onClicked: {
                        if (!bar.window)
                            return
                        if (bar.maximized)
                            bar.window.showNormal()
                        else
                            bar.window.showMaximized()
                    }
                }
            }

            Rectangle {
                width: 46
                height: captionButtons.height
                topRightRadius: (Qt.platform.os === "linux" && !bar.maximized) ? 10 : 0
                color: closeMouse.pressed
                    ? Qt.darker(App.closeHoverBg, 1.15)
                    : closeMouse.containsMouse
                        ? App.closeHoverBg
                        : "transparent"
                Behavior on color { ColorAnimation { duration: 80 } }

                AppIcon {
                    anchors.centerIn: parent
                    icon: "close"
                    size: 16
                    color: (closeMouse.containsMouse || closeMouse.pressed)
                        ? App.closeHoverText
                        : appTheme.windowText
                    opacity: (closeMouse.containsMouse || closeMouse.pressed || bar.windowActive) ? 0.9 : 0.6
                }

                MouseArea {
                    id: closeMouse
                    anchors.fill: parent
                    hoverEnabled: true
                    property bool titleBarInteractive: true
                    ToolTip.visible: containsMouse
                    ToolTip.delay: 600
                    ToolTip.text: I18n.tr("Close")
                    onClicked: if (bar.window) bar.window.close()
                }
            }
        }
    }

    // macOS drag handle (inline menu bar is never active on macOS).
    MouseArea {
        anchors.fill: parent
        anchors.rightMargin: captionCluster.visible ? captionCluster.width : 0
        visible: bar.macOS
        // Keep the press here until titlebarMouseDown starts the native drag.
        preventStealing: true
        acceptedButtons: Qt.LeftButton
        onPressed: App.titlebarMouseDown()
    }
}
