import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtQuick.Window
import cs_app

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

    function tr(s) { return App.i18nTick >= 0 ? App.translate(s) : s }

    readonly property bool windowActive: window ? window.active : true
    readonly property bool maximized: window ? (window.visibility === Window.Maximized || window.visibility === Window.FullScreen) : false
    readonly property string projectName: FileBrowser.rootPath
        ? (FileBrowser.rootPath.split("/").filter(Boolean).pop() || "")
        : ""
    readonly property string fileLabel: (CircuitCanvas.fileName ? CircuitCanvas.fileName : tr("Untitled"))
        + (CircuitCanvas.modified ? "*" : "")

    implicitHeight: 34
    height: visible ? 34 : 0

    // Left: app icon, live simulation state badge, project badge & active file title
    // over a subtle translucent pill so text stays legible over the live canvas.
    Rectangle {
        id: leftPill
        anchors.left: parent.left
        anchors.leftMargin: bar.leadingInset > 0 ? bar.leadingInset : 8
        anchors.verticalCenter: parent.verticalCenter
        height: 26
        width: Math.min(leftGroup.implicitWidth + 16, Math.max(160, Math.round(bar.width * 0.45)))
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

                AppText {
                    id: simStateText
                    anchors.centerIn: parent
                    font.pixelSize: App.fontSmall
                    text: CircuitCanvas.simPaused ? bar.tr("Paused") : bar.tr("Running")
                    color: CircuitCanvas.simPaused ? CircuitCanvas.msgWarnText : CircuitCanvas.msgOkText
                }
            }

            Rectangle {
                visible: bar.projectName !== ""
                radius: 4
                color: Qt.rgba(appTheme.windowText.r, appTheme.windowText.g, appTheme.windowText.b, 0.08)
                implicitWidth: Math.min(120, projText.implicitWidth + 10)
                implicitHeight: projText.implicitHeight + 4

                AppText {
                    id: projText
                    anchors.centerIn: parent
                    width: Math.min(implicitWidth, parent.width - 10)
                    font.pixelSize: App.fontSmall
                    text: bar.projectName
                    color: appTheme.windowText
                    elide: Text.ElideRight
                }
            }

            AppText {
                Layout.fillWidth: true
                font.pixelSize: App.fontSmall
                text: bar.fileLabel
                color: appTheme.windowText
                elide: Text.ElideMiddle
                maximumLineCount: 1
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
                    ToolTip.text: bar.tr("Minimize")
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
                    ToolTip.text: bar.maximized ? bar.tr("Restore") : bar.tr("Maximize")
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
                    ToolTip.text: bar.tr("Close")
                    onClicked: if (bar.window) bar.window.close()
                }
            }
        }
    }

    // Drag handle: active on macOS and Linux. Absent on Windows so it cannot steal HTCAPTION.
    // Right margin stops before the caption buttons so it never intercepts minimize/maximize/close.
    MouseArea {
        anchors.fill: parent
        anchors.rightMargin: captionCluster.visible ? captionCluster.width : 0
        visible: bar.macOS
        // Keep the press here until titlebarMouseDown starts the native drag.
        preventStealing: true
        acceptedButtons: Qt.LeftButton
        onPressed: App.titlebarMouseDown()
    }

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
}
