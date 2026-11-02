import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import cs_app

// DC Parameter Sweep and Transistor Curve Tracer.
// Sweeps an independent voltage/current source and plots dependent node voltages or pin currents.
Rectangle {
    id: root

    function tr(s) { return App.i18nTick >= 0 ? App.translate(s) : s }

    color: pal.window
    SystemPalette { id: pal; colorGroup: SystemPalette.Active }

    property var result: null
    property real xMin: 0
    property real xMax: 5
    property real yMin: 0
    property real yMax: 0.02
    property var hoverInfo: null
    property string statusText: root.tr("Configure sources and probe, then click Run Sweep.")

    readonly property int plotLeft: 68
    readonly property int plotRight: 16
    readonly property int plotTop: 16
    readonly property int plotBottom: 32

    function getTraceColor(idx) {
        var colors = [
            CircuitCanvas.scopeChannel1Color,
            CircuitCanvas.scopeChannel2Color,
            CircuitCanvas.scopeChannel3Color,
            CircuitCanvas.scopeChannel4Color,
            CircuitCanvas.laChannel1Color,
            CircuitCanvas.laChannel2Color,
            CircuitCanvas.laChannel3Color,
            CircuitCanvas.laChannel4Color,
            CircuitCanvas.laChannel5Color,
            CircuitCanvas.laChannel6Color
        ]
        return colors[idx % colors.length]
    }

    function isYCurrent() {
        return yAxisBox.currentIndex === 0
    }

    function isXProbeVolt() {
        return xAxisBox.currentIndex === 1
    }

    function getXVal(pt) {
        return root.isXProbeVolt() ? pt.v_probe : pt.primary_val
    }

    function getYVal(pt) {
        return root.isYCurrent() ? pt.i_probe : pt.v_probe
    }

    function readNumber(field, fallback) {
        var v = Number(field.text)
        return isFinite(v) ? v : fallback
    }

    function reloadSourcesAndProbes() {
        var rawSources = CircuitCanvas.dcSources()
        var probes = CircuitCanvas.dcProbes()
        var sourceIds = []
        if (rawSources && Array.isArray(rawSources)) {
            for (var i = 0; i < rawSources.length; i++) {
                sourceIds.push(rawSources[i].id)
            }
        }
        var keepPrim = primSourceBox.currentText
        var keepSec = secSourceBox.currentText
        var keepProbe = probeBox.currentText

        primSourceBox.model = sourceIds
        secSourceBox.model = sourceIds
        probeBox.model = probes ? probes : []

        var pi = sourceIds.indexOf(keepPrim)
        if (pi >= 0)
            primSourceBox.currentIndex = pi
        else if (sourceIds.length > 0)
            primSourceBox.currentIndex = 0

        var si = sourceIds.indexOf(keepSec)
        if (si >= 0)
            secSourceBox.currentIndex = si
        else if (sourceIds.length > 1)
            secSourceBox.currentIndex = 1

        var pri = (probes ? probes : []).indexOf(keepProbe)
        if (pri >= 0)
            probeBox.currentIndex = pri
        else if (probes && probes.length > 0)
            probeBox.currentIndex = 0
    }

    function runSweep() {
        var primSource = primSourceBox.currentText
        var probePin = probeBox.currentText
        if (!primSource || !probePin) {
            root.statusText = root.tr("Select a primary source and a probe pin.")
            return
        }
        var pStart = root.readNumber(primStartField, 0.0)
        var pStop = root.readNumber(primStopField, 5.0)
        var pStep = root.readNumber(primStepField, 0.1)

        var secSource = secEnableCheck.checked ? secSourceBox.currentText : ""
        var sStart = root.readNumber(secStartField, 0.0)
        var sStop = root.readNumber(secStopField, 0.0)
        var sStep = root.readNumber(secStepField, 1.0)

        var res = CircuitCanvas.dcSweep(
            primSource, pStart, pStop, pStep,
            secSource, sStart, sStop, sStep,
            probePin
        )
        if (!res || res.error) {
            root.result = null
            root.statusText = res && res.error ? res.error : root.tr("DC sweep failed.")
            plot.requestPaint()
            return
        }
        root.result = res
        root.fitBounds()
        root.statusText = root.tr("Sweep completed. Move cursor over traces to inspect operating points.")
        plot.requestPaint()
    }

    function fitBounds() {
        if (!root.result || !root.result.curves || root.result.curves.length === 0) {
            root.xMin = 0
            root.xMax = 5
            root.yMin = 0
            root.yMax = 0.02
            plot.requestPaint()
            return
        }
        var minX = Infinity, maxX = -Infinity
        var minY = Infinity, maxY = -Infinity
        for (var c = 0; c < root.result.curves.length; c++) {
            var pts = root.result.curves[c].points
            for (var i = 0; i < pts.length; i++) {
                var x = root.getXVal(pts[i])
                var y = root.getYVal(pts[i])
                if (x < minX) minX = x
                if (x > maxX) maxX = x
                if (y < minY) minY = y
                if (y > maxY) maxY = y
            }
        }
        if (!isFinite(minX) || !isFinite(maxX) || minX === maxX) {
            minX -= 1
            maxX += 1
        }
        if (!isFinite(minY) || !isFinite(maxY) || minY === maxY) {
            minY -= 0.001
            maxY += 0.001
        }
        var xPad = Math.abs(maxX - minX) * 0.05
        var yPad = Math.abs(maxY - minY) * 0.08
        root.xMin = minX - xPad
        root.xMax = maxX + xPad
        root.yMin = minY - yPad
        root.yMax = maxY + yPad
        plot.requestPaint()
    }

    function mapX(x) {
        var w = plot.width - root.plotLeft - root.plotRight
        var span = root.xMax - root.xMin
        return root.plotLeft + (span > 0 ? (x - root.xMin) / span : 0.5) * w
    }

    function mapY(y) {
        var h = plot.height - root.plotTop - root.plotBottom
        var span = root.yMax - root.yMin
        return root.plotTop + h - (span > 0 ? (y - root.yMin) / span : 0.5) * h
    }

    function fmtVal(v, isCurrent) {
        if (Math.abs(v) < 1e-12) return "0"
        var av = Math.abs(v)
        if (isCurrent) {
            if (av >= 1.0) return v.toFixed(3) + " A"
            if (av >= 1e-3) return (v * 1e3).toFixed(2) + " mA"
            if (av >= 1e-6) return (v * 1e6).toFixed(2) + " \u00B5A"
            if (av >= 1e-9) return (v * 1e9).toFixed(2) + " nA"
            return (v * 1e12).toFixed(1) + " pA"
        } else {
            if (av >= 1.0) return v.toFixed(2) + " V"
            if (av >= 1e-3) return (v * 1e3).toFixed(1) + " mV"
            if (av >= 1e-6) return (v * 1e6).toFixed(1) + " \u00B5V"
            return v.toFixed(3) + " V"
        }
    }

    function findNearest(mx, my) {
        if (!root.result || !root.result.curves) return null
        var bestDist = 28 * 28
        var best = null
        for (var c = 0; c < root.result.curves.length; c++) {
            var curve = root.result.curves[c]
            for (var i = 0; i < curve.points.length; i++) {
                var pt = curve.points[i]
                var px = root.mapX(root.getXVal(pt))
                var py = root.mapY(root.getYVal(pt))
                var dx = mx - px
                var dy = my - py
                var d2 = dx * dx + dy * dy
                if (d2 < bestDist) {
                    bestDist = d2
                    best = { curveIdx: c, pointIdx: i, point: pt, px: px, py: py }
                }
            }
        }
        return best
    }

    Component.onCompleted: root.reloadSourcesAndProbes()

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 12
        spacing: 8

        // Row 1: Primary Source
        RowLayout {
            spacing: 8
            Layout.fillWidth: true

            AppText {
                text: root.tr("Primary Source:")
                font.pixelSize: App.fontSmall
            }
            AppComboBox {
                id: primSourceBox
                Layout.preferredWidth: 130
            }
            AppText {
                text: root.tr("Start:")
                font.pixelSize: App.fontSmall
            }
            AppTextField {
                id: primStartField
                Layout.preferredWidth: 64
                text: "0.0"
            }
            AppText {
                text: root.tr("Stop:")
                font.pixelSize: App.fontSmall
            }
            AppTextField {
                id: primStopField
                Layout.preferredWidth: 64
                text: "5.0"
            }
            AppText {
                text: root.tr("Step:")
                font.pixelSize: App.fontSmall
            }
            AppTextField {
                id: primStepField
                Layout.preferredWidth: 64
                text: "0.1"
            }
            Item { Layout.fillWidth: true }
            AppButton {
                text: root.tr("Run Sweep")
                onClicked: root.runSweep()
            }
        }

        // Row 2: Secondary Source (Family of Curves)
        RowLayout {
            spacing: 8
            Layout.fillWidth: true

            AppCheckBox {
                id: secEnableCheck
                text: root.tr("Secondary Source (Family):")
                checked: false
                onCheckedChanged: if (!checked) root.hoverInfo = null
            }
            AppComboBox {
                id: secSourceBox
                enabled: secEnableCheck.checked
                Layout.preferredWidth: 130
            }
            AppText {
                text: root.tr("Start:")
                font.pixelSize: App.fontSmall
                opacity: secEnableCheck.checked ? 1.0 : 0.5
            }
            AppTextField {
                id: secStartField
                enabled: secEnableCheck.checked
                Layout.preferredWidth: 64
                text: "10e-6"
            }
            AppText {
                text: root.tr("Stop:")
                font.pixelSize: App.fontSmall
                opacity: secEnableCheck.checked ? 1.0 : 0.5
            }
            AppTextField {
                id: secStopField
                enabled: secEnableCheck.checked
                Layout.preferredWidth: 64
                text: "50e-6"
            }
            AppText {
                text: root.tr("Step:")
                font.pixelSize: App.fontSmall
                opacity: secEnableCheck.checked ? 1.0 : 0.5
            }
            AppTextField {
                id: secStepField
                enabled: secEnableCheck.checked
                Layout.preferredWidth: 64
                text: "10e-6"
            }
            Item { Layout.fillWidth: true }
        }

        // Row 3: Measurement Probes & Plot Axes
        RowLayout {
            spacing: 8
            Layout.fillWidth: true

            AppText {
                text: root.tr("Probe Pin:")
                font.pixelSize: App.fontSmall
            }
            AppComboBox {
                id: probeBox
                Layout.preferredWidth: 160
            }
            AppText {
                text: root.tr("Y-Axis:")
                font.pixelSize: App.fontSmall
            }
            AppComboBox {
                id: yAxisBox
                Layout.preferredWidth: 120
                model: [root.tr("Current (I)"), root.tr("Voltage (V)")]
                onCurrentIndexChanged: root.fitBounds()
            }
            AppText {
                text: root.tr("X-Axis:")
                font.pixelSize: App.fontSmall
            }
            AppComboBox {
                id: xAxisBox
                Layout.preferredWidth: 140
                model: [root.tr("Source Value"), root.tr("Probe Voltage (V)")]
                onCurrentIndexChanged: root.fitBounds()
            }
            Item { Layout.fillWidth: true }
        }

        // Status / Measurement Info Bar
        RowLayout {
            spacing: 8
            Layout.fillWidth: true

            AppText {
                Layout.fillWidth: true
                text: root.statusText
                font.pixelSize: App.fontSmall
                color: pal.windowText
                elide: Text.ElideRight
            }
        }

        // Canvas Area
        Canvas {
            id: plot
            Layout.fillWidth: true
            Layout.fillHeight: true
            contextType: "2d"

            onWidthChanged: requestPaint()
            onHeightChanged: requestPaint()

            onPaint: {
                var ctx = getContext("2d")
                if (!ctx) return

                ctx.globalAlpha = 1
                ctx.fillStyle = pal.base
                ctx.fillRect(0, 0, width, height)

                var plotW = width - root.plotLeft - root.plotRight
                var plotH = height - root.plotTop - root.plotBottom
                if (plotW <= 0 || plotH <= 0) return

                // Grid background and bounding box
                ctx.strokeStyle = pal.mid
                ctx.lineWidth = 1
                ctx.strokeRect(root.plotLeft, root.plotTop, plotW, plotH)

                // Grid divisions
                var xTicks = 5
                var yTicks = 5

                ctx.fillStyle = pal.windowText
                ctx.font = App.fontSmall + "px \"" + App.fontFamily + "\""
                ctx.textAlign = "right"
                ctx.textBaseline = "middle"

                // Horizontal grid lines (Y axis)
                for (var j = 0; j <= yTicks; j++) {
                    var fracY = j / yTicks
                    var yVal = root.yMin + fracY * (root.yMax - root.yMin)
                    var py = root.plotTop + plotH - fracY * plotH

                    ctx.globalAlpha = 0.25
                    ctx.beginPath()
                    ctx.moveTo(root.plotLeft, py)
                    ctx.lineTo(root.plotLeft + plotW, py)
                    ctx.stroke()

                    ctx.globalAlpha = 1
                    ctx.fillText(root.fmtVal(yVal, root.isYCurrent()), root.plotLeft - 6, py)
                }

                // Vertical grid lines (X axis)
                ctx.textAlign = "center"
                ctx.textBaseline = "top"
                for (var k = 0; k <= xTicks; k++) {
                    var fracX = k / xTicks
                    var xVal = root.xMin + fracX * (root.xMax - root.xMin)
                    var px = root.plotLeft + fracX * plotW

                    ctx.globalAlpha = 0.25
                    ctx.beginPath()
                    ctx.moveTo(px, root.plotTop)
                    ctx.lineTo(px, root.plotTop + plotH)
                    ctx.stroke()

                    ctx.globalAlpha = 1
                    ctx.fillText(root.fmtVal(xVal, false), px, root.plotTop + plotH + 6)
                }

                // Draw curves
                if (root.result && root.result.curves) {
                    ctx.lineWidth = 2
                    ctx.lineCap = "round"
                    ctx.lineJoin = "round"

                    for (var c = 0; c < root.result.curves.length; c++) {
                        var curve = root.result.curves[c]
                        var pts = curve.points
                        if (pts.length === 0) continue

                        ctx.strokeStyle = root.getTraceColor(c)
                        ctx.beginPath()

                        for (var p = 0; p < pts.length; p++) {
                            var cx = root.mapX(root.getXVal(pts[p]))
                            var cy = root.mapY(root.getYVal(pts[p]))
                            if (p === 0)
                                ctx.moveTo(cx, cy)
                            else
                                ctx.lineTo(cx, cy)
                        }
                        ctx.stroke()
                    }
                }

                // Draw crosshair hover marker
                if (root.hoverInfo) {
                    var h = root.hoverInfo
                    ctx.strokeStyle = pal.highlight
                    ctx.lineWidth = 1
                    ctx.setLineDash([4, 4])

                    ctx.beginPath()
                    ctx.moveTo(root.plotLeft, h.py)
                    ctx.lineTo(root.plotLeft + plotW, h.py)
                    ctx.moveTo(h.px, root.plotTop)
                    ctx.lineTo(h.px, root.plotTop + plotH)
                    ctx.stroke()
                    ctx.setLineDash([])

                    ctx.fillStyle = root.getTraceColor(h.curveIdx)
                    ctx.beginPath()
                    ctx.arc(h.px, h.py, 5, 0, 2 * Math.PI)
                    ctx.fill()
                    ctx.strokeStyle = pal.windowText
                    ctx.stroke()
                }
            }

            MouseArea {
                anchors.fill: parent
                hoverEnabled: true

                onPositionChanged: function(mouse) {
                    var nearest = root.findNearest(mouse.x, mouse.y)
                    root.hoverInfo = nearest
                    if (nearest && root.result) {
                        var curve = root.result.curves[nearest.curveIdx]
                        var sText = ""
                        if (root.result.secondary_source) {
                            sText += root.result.secondary_source + " = " + root.fmtVal(curve.secondary_val, false) + "   |   "
                        }
                        sText += "X (" + (root.isXProbeVolt() ? "Vprobe" : root.result.primary_source) + ") = " + root.fmtVal(root.getXVal(nearest.point), false)
                        sText += "   |   "
                        sText += "Y (" + (root.isYCurrent() ? "Iprobe" : "Vprobe") + ") = " + root.fmtVal(root.getYVal(nearest.point), root.isYCurrent())
                        sText += "  [V: " + root.fmtVal(nearest.point.v_probe, false) + ", I: " + root.fmtVal(nearest.point.i_probe, true) + "]"
                        root.statusText = sText
                    }
                    plot.requestPaint()
                }

                onExited: {
                    root.hoverInfo = null
                    if (root.result && root.result.curves && root.result.curves.length > 0)
                        root.statusText = root.tr("Move cursor over traces to inspect operating points.")
                    plot.requestPaint()
                }
            }
        }
    }
}
