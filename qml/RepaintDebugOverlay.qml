import QtQuick
import cs_app

/* Lightweight debug overlay canvas that floats over CircuitCanvas to visually
 * highlight and differentiate full canvas repaints vs small / dirty region repaints
 * in real-time, complete with a diagnostic telemetry HUD.
 */
Item {
    id: root
    function tr(s) { return App.i18nTick >= 0 ? App.translate(s) : s }
    visible: AppDialog.repaintOverlayEnabled
    enabled: false

    property var recentEvents: []
    property int totalRepaints: 0
    property int fullRepaints: 0
    property int partialRepaints: 0
    property real repaintFps: 0.0
    property string lastRepaintType: "DIRTY REGIONS"
    property real lastAreaPercent: 0.0
    property int lastDirtyCount: 0
    property real lastFpsCalcTime: 0
    property int repaintsInWindow: 0
    property real lastEventTime: 0
    readonly property bool isTelemetryActive: recentEvents.length > 0 && ((Date.now() - lastEventTime) < 400)
    readonly property string monoFamily: App.monoFontFamily || "Menlo"

    function recordRepaint(rects, isFull, subDirtyRects) {
        if (!root.visible || root.width <= 0 || root.height <= 0)
            return

        var now = Date.now()
        lastEventTime = now

        var ev = {
            "rects": rects || [],
            "subDirtyRects": subDirtyRects || [],
            "isFull": !!isFull,
            "timeMs": now
        }

        recentEvents.push(ev)
        while (recentEvents.length > 20)
            recentEvents.shift()

        totalRepaints++
        if (isFull)
            fullRepaints++
        else
            partialRepaints++

        lastRepaintType = isFull ? "FULL REPAINT" : "DIRTY REGIONS"
        lastDirtyCount = isFull ? ((subDirtyRects && subDirtyRects.length > 0) ? subDirtyRects.length : 1)
                                : (rects ? rects.length : 0)

        // Compute covered area %
        var totalArea = 0
        if (rects && rects.length > 0) {
            for (var i = 0; i < rects.length; ++i) {
                var r = rects[i]
                totalArea += (r.width * r.height)
            }
        }
        var canvasArea = root.width * root.height
        lastAreaPercent = (canvasArea > 0) ? Math.min(100.0, (totalArea / canvasArea) * 100.0) : 0.0

        // Repaint FPS calculation (500ms sliding window)
        repaintsInWindow++
        if (now - lastFpsCalcTime >= 500) {
            if (lastFpsCalcTime > 0)
                repaintFps = (repaintsInWindow * 1000.0) / (now - lastFpsCalcTime)
            else
                repaintFps = repaintsInWindow * 2.0
            repaintsInWindow = 0
            lastFpsCalcTime = now
        }

        if (!fadeTimer.running)
            fadeTimer.start()

        canvas.requestPaint()
    }

    onVisibleChanged: {
        if (!visible) {
            recentEvents = []
            fadeTimer.stop()
            if (canvas) canvas.requestPaint()
        }
    }

    Connections {
        target: CircuitCanvas
        function onRepaintRecorded(rects, isFull, subRects) {
            root.recordRepaint(rects, isFull, subRects)
        }
    }

    Timer {
        id: fadeTimer
        interval: 16
        repeat: true
        running: false
        onTriggered: {
            var now = Date.now()
            var filtered = []
            for (var i = 0; i < root.recentEvents.length; ++i) {
                var ev = root.recentEvents[i]
                if (now - ev.timeMs <= 500)
                    filtered.push(ev)
            }
            root.recentEvents = filtered
            if (root.recentEvents.length === 0)
                fadeTimer.stop()
            canvas.requestPaint()
        }
    }

    // 1. Canvas for flashing highlighted repaint regions
    Canvas {
        id: canvas
        anchors.fill: parent
        renderTarget: Canvas.FramebufferObject

        onPaint: {
            var ctx = getContext("2d")
            ctx.clearRect(0, 0, width, height)

            if (!root.visible || root.recentEvents.length === 0)
                return

            var now = Date.now()
            var kFadeDurationMs = 500.0

            for (var e = 0; e < root.recentEvents.length; ++e) {
                var ev = root.recentEvents[e]
                var age = now - ev.timeMs
                if (age > kFadeDurationMs)
                    continue

                var progress = age / kFadeDurationMs
                var alpha = Math.max(0.0, 1.0 - progress)

                if (ev.isFull) {
                    // Full screen flash (Red)
                    ctx.fillStyle = Qt.rgba(1.0, 45 / 255, 45 / 255, (35 / 255) * alpha)
                    ctx.strokeStyle = Qt.rgba(1.0, 30 / 255, 30 / 255, (220 / 255) * alpha)
                    ctx.lineWidth = 3

                    for (var fi = 0; fi < ev.rects.length; ++fi) {
                        var fr = ev.rects[fi]
                        ctx.fillRect(fr.x, fr.y, fr.width, fr.height)
                        ctx.strokeRect(fr.x + 1.5, fr.y + 1.5, Math.max(0, fr.width - 3), Math.max(0, fr.height - 3))
                    }

                    // Draw specific subDirtyRects that caused this full repaint
                    if (ev.subDirtyRects && ev.subDirtyRects.length > 0) {
                        ctx.fillStyle = Qt.rgba(1.0, 230 / 255, 0, (80 / 255) * alpha)
                        ctx.strokeStyle = Qt.rgba(1.0, 240 / 255, 0, (240 / 255) * alpha)
                        ctx.lineWidth = 2
                        for (var si = 0; si < ev.subDirtyRects.length; ++si) {
                            var sdr = ev.subDirtyRects[si]
                            ctx.fillRect(sdr.x, sdr.y, sdr.width, sdr.height)
                            ctx.strokeRect(sdr.x, sdr.y, sdr.width, sdr.height)
                        }
                    }
                } else {
                    // Partial / Small dirty area flash (Green)
                    ctx.fillStyle = Qt.rgba(0, 230 / 255, 130 / 255, (70 / 255) * alpha)
                    ctx.strokeStyle = Qt.rgba(0, 1.0, 130 / 255, (230 / 255) * alpha)
                    ctx.lineWidth = 2

                    for (var pi = 0; pi < ev.rects.length; ++pi) {
                        var pr = ev.rects[pi]
                        ctx.fillRect(pr.x, pr.y, pr.width, pr.height)
                        ctx.strokeRect(pr.x, pr.y, pr.width, pr.height)
                    }
                }
            }
        }
    }

    // 2. HUD Overlay Badge (top-right corner, 270x104)
    Rectangle {
        id: hudCard
        width: 270
        height: 104
        anchors.top: parent.top
        anchors.topMargin: 14
        anchors.right: parent.right
        anchors.rightMargin: 14
        radius: 6
        color: "#e80e1218"
        border.color: "#1cffffff"
        border.width: 1
        clip: true

        // Top accent line
        Rectangle {
            anchors.top: parent.top
            anchors.left: parent.left
            anchors.right: parent.right
            height: 3
            color: "#00a0f0"
        }

        // Header: Title & Status
        Item {
            id: headerRow
            anchors.top: parent.top
            anchors.topMargin: 8
            anchors.left: parent.left
            anchors.leftMargin: 12
            anchors.right: parent.right
            anchors.rightMargin: 12
            height: 16

            Text {
                text: root.tr("REPAINT TELEMETRY")
                anchors.left: parent.left
                anchors.verticalCenter: parent.verticalCenter
                color: "#e1ebf8"
                font.family: root.monoFamily
                font.pixelSize: 10
                font.bold: true
                font.letterSpacing: 0.8
            }

            Row {
                anchors.right: parent.right
                anchors.verticalCenter: parent.verticalCenter
                spacing: 5

                Rectangle {
                    width: 6
                    height: 6
                    radius: 3
                    anchors.verticalCenter: parent.verticalCenter
                    color: root.isTelemetryActive ? "#28dc82" : "#788796"
                }

                Text {
                    text: root.isTelemetryActive ? root.tr("ACTIVE") : root.tr("IDLE")
                    color: root.isTelemetryActive ? "#28dc82" : "#788796"
                    font.family: root.monoFamily
                    font.pixelSize: 9
                    font.bold: false
                    anchors.verticalCenter: parent.verticalCenter
                }
            }
        }

        // Divider
        Rectangle {
            anchors.top: headerRow.bottom
            anchors.topMargin: 4
            anchors.left: parent.left
            anchors.leftMargin: 10
            anchors.right: parent.right
            anchors.rightMargin: 10
            height: 1
            color: "#14ffffff"
        }

        // Mode pill & Repaint Rate
        Item {
            id: middleRow
            anchors.top: parent.top
            anchors.topMargin: 33
            anchors.left: parent.left
            anchors.leftMargin: 12
            anchors.right: parent.right
            anchors.rightMargin: 12
            height: 20

            readonly property bool isFull: root.lastRepaintType === "FULL REPAINT"

            Rectangle {
                id: modePill
                width: 96
                height: 18
                radius: 3
                anchors.left: parent.left
                anchors.verticalCenter: parent.verticalCenter
                color: middleRow.isFull ? "#2eff3232" : "#2400c86e"
                border.color: middleRow.isFull ? "#8cff4646" : "#8c28dc82"
                border.width: 1

                Text {
                    anchors.centerIn: parent
                    text: middleRow.isFull ? root.tr("FULL FRAME") : root.tr("DIRTY REGIONS")
                    color: middleRow.isFull ? "#ff5f5f" : "#32eb91"
                    font.family: root.monoFamily
                    font.pixelSize: 8
                    font.bold: true
                }
            }

            Row {
                anchors.left: modePill.right
                anchors.leftMargin: 10
                anchors.verticalCenter: parent.verticalCenter
                spacing: 4

                Text {
                    text: root.tr("Rate:")
                    color: "#a0afc3"
                    font.family: root.monoFamily
                    font.pixelSize: 10
                }

                Text {
                    text: Math.round(root.repaintFps) + " Hz"
                    color: "#ebf5ff"
                    font.family: root.monoFamily
                    font.pixelSize: 10
                    font.bold: true
                }
            }
        }

        // Telemetry Row: Area, Dirty, Total
        Text {
            id: telemetryRow
            anchors.top: parent.top
            anchors.topMargin: 58
            anchors.left: parent.left
            anchors.leftMargin: 12
            anchors.right: parent.right
            anchors.rightMargin: 12
            text: root.tr("Area: %1%   Dirty: %2   Total: %3")
                .arg(root.lastAreaPercent.toFixed(1))
                .arg(root.lastDirtyCount)
                .arg(root.totalRepaints)
            color: "#a0afc3"
            font.family: root.monoFamily
            font.pixelSize: 10
        }

        // Footer / Legend
        Row {
            anchors.top: parent.top
            anchors.topMargin: 80
            anchors.left: parent.left
            anchors.leftMargin: 12
            anchors.right: parent.right
            anchors.rightMargin: 12
            spacing: 12

            Row {
                spacing: 4
                anchors.verticalCenter: parent.verticalCenter
                Rectangle {
                    width: 6; height: 6; radius: 3; color: "#ff3c3c"
                    anchors.verticalCenter: parent.verticalCenter
                }
                Text {
                    text: root.tr("Full Frame")
                    color: "#8c9baf"
                    font.family: root.monoFamily
                    font.pixelSize: 9
                }
            }

            Row {
                spacing: 4
                anchors.verticalCenter: parent.verticalCenter
                Rectangle {
                    width: 6; height: 6; radius: 3; color: "#00e682"
                    anchors.verticalCenter: parent.verticalCenter
                }
                Text {
                    text: root.tr("Dirty Regions")
                    color: "#8c9baf"
                    font.family: root.monoFamily
                    font.pixelSize: 9
                }
            }

            Row {
                spacing: 4
                anchors.verticalCenter: parent.verticalCenter
                Rectangle {
                    width: 6; height: 6; radius: 3; color: "#ffe600"
                    anchors.verticalCenter: parent.verticalCenter
                }
                Text {
                    text: root.tr("Source Rects")
                    color: "#8c9baf"
                    font.family: root.monoFamily
                    font.pixelSize: 9
                }
            }
        }
    }
}
