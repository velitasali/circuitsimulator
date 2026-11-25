pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Dialogs
import QtQuick.Layouts
import cs_app
import cs_ui

// DC Parameter Sweep and Transistor Curve Tracer.
// Sweeps independent voltage/current sources, resistors, or potentiometers and
// plots node/differential voltages, pin currents, or dissipated power.
Rectangle {
    id: root

    color: pal.window
    SystemPalette { id: pal; colorGroup: SystemPalette.Active }

    // Result from dcSweepMulti: { primary_source, secondary_source, ref_pin, series: [{ probe_pin, curves: [{ secondary_val, points }] }] }
    property var result: null
    // Flattened trace list for plotting: [{ label, secondaryVal, probePin, points }]
    property var flatTraces: []
    property var extraProbes: []
    property var sourceMeta: ({})

    property real xMin: 0
    property real xMax: 5
    property real yMin: 0
    property real yMax: 0.02
    property real xFullMin: 0
    property real xFullMax: 5
    property real yFullMin: 0
    property real yFullMax: 0.02
    property bool isZoomed: false

    property var hoverInfo: null
    property var pinnedInfo: null
    property bool draggingZoom: false
    property real dragStartX: 0
    property real dragStartY: 0
    property real dragCurrX: 0
    property real dragCurrY: 0

    property string summaryText: I18n.tr("Configure sources and probe, then click Run Sweep.")
    property string statusText: summaryText

    readonly property int plotLeft: 78
    readonly property int plotRight: 20
    readonly property int plotTop: 16
    readonly property int plotBottom: 46
    readonly property real logFloor: 1e-15

    component ControlCard: Rectangle {
        id: card
        default property alias content: cardLayout.data

        Layout.fillWidth: true
        implicitHeight: cardLayout.implicitHeight + 16
        radius: 10
        color: Qt.rgba(pal.base.r, pal.base.g, pal.base.b, 0.45)
        border.color: Qt.rgba(pal.mid.r, pal.mid.g, pal.mid.b, 0.55)
        border.width: 1

        ColumnLayout {
            id: cardLayout
            anchors.fill: parent
            anchors.margins: 8
            spacing: 8
        }
    }

    function getTraceColor(idx: int): color {
        var colors = [
            CircuitCanvas.laChannel6Color,
            CircuitCanvas.laChannel1Color,
            CircuitCanvas.laChannel2Color,
            CircuitCanvas.laChannel4Color,
            CircuitCanvas.laChannel7Color,
            CircuitCanvas.laChannel5Color,
            CircuitCanvas.laChannel3Color,
            CircuitCanvas.scopeChannel4Color
        ]
        var c = colors[idx % colors.length]
        if (!App.darkTheme && UiUtils.isLightColor(c))
            return Qt.darker(c, 1.35)
        return c
    }

    function unitForSource(id: string): string {
        if (id && root.sourceMeta && root.sourceMeta[id] && root.sourceMeta[id].unit !== undefined)
            return root.sourceMeta[id].unit
        return "V"
    }

    function isYCurrent(): bool {
        return yAxisBox.currentIndex === 0
    }

    function isYVoltage(): bool {
        return yAxisBox.currentIndex === 1
    }

    function isYPower(): bool {
        return yAxisBox.currentIndex === 2
    }

    function isXProbeVolt(): bool {
        return xAxisBox.currentIndex === 1
    }

    function xUnit(): string {
        if (root.isXProbeVolt())
            return "V"
        var src = root.result ? root.result.primary_source : primSourceBox.currentText
        return root.unitForSource(src)
    }

    function yUnit(): string {
        if (root.isYCurrent()) return "A"
        if (root.isYVoltage()) return "V"
        return "W"
    }

    function getRawX(pt: var): real {
        return root.isXProbeVolt() ? Number(pt.v_probe) : Number(pt.primary_val)
    }

    function getRawY(pt: var): real {
        if (root.isYCurrent()) return Number(pt.i_probe)
        if (root.isYVoltage()) return Number(pt.v_probe)
        return Number(pt.v_probe) * Number(pt.i_probe)
    }

    function xToPlot(v: real): real {
        if (logXCheck.checked)
            return Math.log10(Math.max(root.logFloor, Math.abs(v)))
        return v
    }

    function yToPlot(v: real): real {
        if (logYCheck.checked)
            return Math.log10(Math.max(root.logFloor, Math.abs(v)))
        return v
    }

    function activeProbeList(): var {
        var out = []
        var cur = probeBox.currentText
        if (cur) out.push(cur)
        for (var i = 0; i < root.extraProbes.length; i++) {
            var p = root.extraProbes[i]
            if (p && out.indexOf(p) < 0)
                out.push(p)
        }
        return out
    }

    function addExtraProbe(): void {
        var cur = probeBox.currentText
        if (!cur || root.extraProbes.indexOf(cur) >= 0) return
        var copy = root.extraProbes.slice()
        copy.push(cur)
        root.extraProbes = copy
    }

    function removeExtraProbe(pin: string): void {
        var copy = []
        for (var i = 0; i < root.extraProbes.length; i++) {
            if (root.extraProbes[i] !== pin)
                copy.push(root.extraProbes[i])
        }
        root.extraProbes = copy
    }

    function selectedRefPin(): string {
        if (refBox.currentIndex <= 0)
            return ""
        return refBox.currentText
    }

    function rebuildFlatTraces(): void {
        var out = []
        if (!root.result || !root.result.series) {
            root.flatTraces = out
            return
        }
        var multiProbe = root.result.series.length > 1
        var secName = root.result.secondary_source || ""
        var secUnit = secName ? root.unitForSource(secName) : ""
        for (var s = 0; s < root.result.series.length; s++) {
            var ser = root.result.series[s]
            var curves = ser.curves || []
            for (var c = 0; c < curves.length; c++) {
                var cv = curves[c]
                var lbl = ""
                if (multiProbe && secName)
                    lbl = ser.probe_pin + " (" + secName + "=" + root.fmtUnit(cv.secondary_val, secUnit) + ")"
                else if (secName)
                    lbl = secName + " = " + root.fmtUnit(cv.secondary_val, secUnit)
                else
                    lbl = ser.probe_pin
                out.push({
                    label: lbl,
                    probePin: ser.probe_pin,
                    secondaryVal: cv.secondary_val,
                    points: cv.points || []
                })
            }
        }
        root.flatTraces = out
    }

    function reloadSourcesAndProbes(): void {
        var rawSources = CircuitCanvas.dcSources()
        var probes = CircuitCanvas.dcProbes()
        var probeArr = probes ? probes : []
        var sourceIds = []
        var meta = {}
        if (rawSources && Array.isArray(rawSources)) {
            for (var i = 0; i < rawSources.length; i++) {
                sourceIds.push(rawSources[i].id)
                meta[rawSources[i].id] = rawSources[i]
            }
        }
        root.sourceMeta = meta

        var keepPrim = primSourceBox.currentText
        var keepSec = secSourceBox.currentText
        var keepProbe = probeBox.currentText
        var keepRef = root.selectedRefPin()

        primSourceBox.model = sourceIds
        secSourceBox.model = sourceIds
        probeBox.model = probeArr
        var refModel = [I18n.tr("GND")].concat(probeArr)
        refBox.model = refModel

        var pi = sourceIds.indexOf(keepPrim)
        primSourceBox.currentIndex = pi >= 0 ? pi : (sourceIds.length > 0 ? 0 : -1)

        var si = sourceIds.indexOf(keepSec)
        if (si >= 0) secSourceBox.currentIndex = si
        else if (sourceIds.length > 1) secSourceBox.currentIndex = 1
        else if (sourceIds.length > 0) secSourceBox.currentIndex = 0

        var pri = probeArr.indexOf(keepProbe)
        probeBox.currentIndex = pri >= 0 ? pri : (probeArr.length > 0 ? 0 : -1)

        var ri = keepRef ? refModel.indexOf(keepRef) : 0
        refBox.currentIndex = ri >= 0 ? ri : 0

        var kept = []
        for (var j = 0; j < root.extraProbes.length; j++) {
            if (probeArr.indexOf(root.extraProbes[j]) >= 0)
                kept.push(root.extraProbes[j])
        }
        root.extraProbes = kept

        if (root.result) {
            root.result = null
            root.flatTraces = []
            root.hoverInfo = null
            root.pinnedInfo = null
            root.isZoomed = false
            root.summaryText = I18n.tr("Circuit changed. Run the sweep again.")
            root.statusText = root.summaryText
            plot.requestPaint()
        }
    }

    function runSweep(): void {
        var primSource = primSourceBox.currentText
        var probes = root.activeProbeList()
        if (!primSource || probes.length === 0) {
            root.summaryText = I18n.tr("Select a primary source and a probe pin.")
            root.statusText = root.summaryText
            return
        }
        var pStart = UiUtils.readNumber(primStartField.text, 0.0)
        var pStop = UiUtils.readNumber(primStopField.text, 5.0)
        var pStep = UiUtils.readNumber(primStepField.text, 0.1)

        var secSource = secEnableCheck.checked ? secSourceBox.currentText : ""
        if (secSource && secSource === primSource) {
            root.summaryText = I18n.tr("Primary and secondary sources must be different components.")
            root.statusText = root.summaryText
            return
        }
        var sStart = UiUtils.readNumber(secStartField.text, 0.0)
        var sStop = UiUtils.readNumber(secStopField.text, 0.0)
        var sStep = UiUtils.readNumber(secStepField.text, 1.0)

        var res = CircuitCanvas.dcSweepMulti(
            primSource, pStart, pStop, pStep,
            secSource, sStart, sStop, sStep,
            JSON.stringify(probes),
            root.selectedRefPin()
        )
        if (!res || res.error) {
            root.result = null
            root.flatTraces = []
            root.hoverInfo = null
            root.pinnedInfo = null
            root.isZoomed = false
            root.summaryText = res && res.error ? res.error : I18n.tr("DC sweep failed.")
            root.statusText = root.summaryText
            plot.requestPaint()
            return
        }
        root.result = res
        root.rebuildFlatTraces()
        root.hoverInfo = null
        root.pinnedInfo = null
        root.isZoomed = false
        root.fitBounds()
        var nTraces = root.flatTraces.length
        var nPts = (nTraces > 0 && root.flatTraces[0].points) ? root.flatTraces[0].points.length : 0
        root.summaryText = (nTraces > 1
            ? (I18n.tr("Sweep completed: ") + nTraces + I18n.tr(" traces (") + nPts + I18n.tr(" pts). "))
            : (I18n.tr("Sweep completed (") + nPts + I18n.tr(" pts). ")))
            + I18n.tr("Click to pin C1 cursor, drag to zoom, right-click to clear C1.")
        root.statusText = root.summaryText
        plot.requestPaint()
    }

    function fitBounds(): void {
        if (!root.flatTraces || root.flatTraces.length === 0) {
            root.xMin = logXCheck.checked ? -3 : 0
            root.xMax = logXCheck.checked ? 1 : 5
            root.yMin = logYCheck.checked ? -6 : 0
            root.yMax = logYCheck.checked ? -1 : 0.02
            root.xFullMin = root.xMin; root.xFullMax = root.xMax
            root.yFullMin = root.yMin; root.yFullMax = root.yMax
            root.isZoomed = false
            plot.requestPaint()
            return
        }
        var minX = Infinity, maxX = -Infinity
        var minY = Infinity, maxY = -Infinity
        for (var c = 0; c < root.flatTraces.length; c++) {
            var pts = root.flatTraces[c].points
            for (var i = 0; i < pts.length; i++) {
                var rx = root.getRawX(pts[i])
                var ry = root.getRawY(pts[i])
                if (logXCheck.checked && Math.abs(rx) < root.logFloor) continue
                if (logYCheck.checked && Math.abs(ry) < root.logFloor) continue
                var x = root.xToPlot(rx)
                var y = root.yToPlot(ry)
                if (x < minX) minX = x
                if (x > maxX) maxX = x
                if (y < minY) minY = y
                if (y > maxY) maxY = y
            }
        }
        if (!isFinite(minX) || !isFinite(maxX) || minX === maxX) {
            minX = isFinite(minX) ? minX - 1 : 0
            maxX = isFinite(maxX) ? maxX + 1 : 5
        }
        if (!isFinite(minY) || !isFinite(maxY) || minY === maxY) {
            minY = isFinite(minY) ? minY - (logYCheck.checked ? 1 : 0.001) : 0
            maxY = isFinite(maxY) ? maxY + (logYCheck.checked ? 1 : 0.001) : 0.02
        }
        var xPad = Math.abs(maxX - minX) * 0.05
        var yPad = Math.abs(maxY - minY) * 0.08
        root.xMin = minX - xPad
        root.xMax = maxX + xPad
        root.yMin = minY - yPad
        root.yMax = maxY + yPad
        root.xFullMin = root.xMin
        root.xFullMax = root.xMax
        root.yFullMin = root.yMin
        root.yFullMax = root.yMax
        root.isZoomed = false
        plot.requestPaint()
    }

    function resetZoom(): void {
        if (!root.result) return
        root.xMin = root.xFullMin
        root.xMax = root.xFullMax
        root.yMin = root.yFullMin
        root.yMax = root.yFullMax
        root.isZoomed = false
        plot.requestPaint()
    }

    function zoomAt(factor: real, px: real, py: real): void {
        if (!root.result) return
        var plotW = Math.max(1, plot.width - root.plotLeft - root.plotRight)
        var plotH = Math.max(1, plot.height - root.plotTop - root.plotBottom)
        var fx = Math.max(0.0, Math.min(1.0, (px - root.plotLeft) / plotW))
        var fy = Math.max(0.0, Math.min(1.0, (root.plotTop + plotH - py) / plotH))

        var xSpan = root.xMax - root.xMin
        var ySpan = root.yMax - root.yMin
        var xFull = root.xFullMax - root.xFullMin
        var yFull = root.yFullMax - root.yFullMin
        var newX = Math.max(xFull * 1e-4, Math.min(xFull, xSpan * factor))
        var newY = Math.max(yFull * 1e-4, Math.min(yFull, ySpan * factor))

        var ax = root.xMin + fx * xSpan
        var ay = root.yMin + fy * ySpan
        var nx0 = ax - fx * newX
        var nx1 = nx0 + newX
        if (nx0 < root.xFullMin) { nx0 = root.xFullMin; nx1 = Math.min(root.xFullMax, nx0 + newX) }
        if (nx1 > root.xFullMax) { nx1 = root.xFullMax; nx0 = Math.max(root.xFullMin, nx1 - newX) }

        var ny0 = ay - fy * newY
        var ny1 = ny0 + newY
        if (ny0 < root.yFullMin) { ny0 = root.yFullMin; ny1 = Math.min(root.yFullMax, ny0 + newY) }
        if (ny1 > root.yFullMax) { ny1 = root.yFullMax; ny0 = Math.max(root.yFullMin, ny1 - newY) }

        root.xMin = nx0; root.xMax = nx1
        root.yMin = ny0; root.yMax = ny1
        root.isZoomed = (newX < xFull * 0.999 || newY < yFull * 0.999)
        plot.requestPaint()
    }

    function mapX(xPlot: real): real {
        var w = plot.width - root.plotLeft - root.plotRight
        var span = root.xMax - root.xMin
        return root.plotLeft + (span > 0 ? (xPlot - root.xMin) / span : 0.5) * w
    }

    function mapY(yPlot: real): real {
        var h = plot.height - root.plotTop - root.plotBottom
        var span = root.yMax - root.yMin
        return root.plotTop + h - (span > 0 ? (yPlot - root.yMin) / span : 0.5) * h
    }

    function plotXOfPx(px: real): real {
        var w = Math.max(1, plot.width - root.plotLeft - root.plotRight)
        var t = Math.max(0.0, Math.min(1.0, (px - root.plotLeft) / w))
        return root.xMin + t * (root.xMax - root.xMin)
    }

    function plotYOfPy(py: real): real {
        var h = Math.max(1, plot.height - root.plotTop - root.plotBottom)
        var t = Math.max(0.0, Math.min(1.0, (root.plotTop + h - py) / h))
        return root.yMin + t * (root.yMax - root.yMin)
    }

    function fmtUnit(v: real, unit: string): string {
        if (!unit) {
            return Math.abs(v) < 1e-12 ? "0" : v.toFixed(3)
        }
        if (Math.abs(v) < 1e-12) return "0 " + unit
        var av = Math.abs(v)
        if (av >= 1e9) return (v / 1e9).toFixed(2) + " G" + unit
        if (av >= 1e6) return (v / 1e6).toFixed(2) + " M" + unit
        if (av >= 1e3) return (v / 1e3).toFixed(2) + " k" + unit
        if (av >= 1.0) return v.toFixed(unit === "A" ? 3 : 2) + " " + unit
        if (av >= 1e-3) return (v * 1e3).toFixed(unit === "A" ? 2 : 1) + " m" + unit
        if (av >= 1e-6) return (v * 1e6).toFixed(unit === "A" ? 2 : 1) + " \u00B5" + unit
        if (av >= 1e-9) return (v * 1e9).toFixed(2) + " n" + unit
        return (v * 1e12).toFixed(1) + " p" + unit
    }

    function findNearest(mx: real, my: real): var {
        if (!root.flatTraces || root.flatTraces.length === 0) return null
        var bestDist = 28 * 28
        var best = null
        for (var c = 0; c < root.flatTraces.length; c++) {
            var pts = root.flatTraces[c].points
            for (var i = 0; i < pts.length; i++) {
                var pt = pts[i]
                var rx = root.getRawX(pt)
                var ry = root.getRawY(pt)
                if (logXCheck.checked && Math.abs(rx) < root.logFloor) continue
                if (logYCheck.checked && Math.abs(ry) < root.logFloor) continue
                var px = root.mapX(root.xToPlot(rx))
                var py = root.mapY(root.yToPlot(ry))
                var dx = mx - px
                var dy = my - py
                var d2 = dx * dx + dy * dy
                if (d2 < bestDist) {
                    bestDist = d2
                    best = { traceIdx: c, pointIdx: i, point: pt }
                }
            }
        }
        return best
    }

    function updateReadout(info: var): void {
        if (!info || !root.result) {
            root.statusText = root.summaryText
            return
        }
        var xU = root.xUnit()
        var yU = root.yUnit()
        if (root.pinnedInfo && (root.pinnedInfo.traceIdx !== info.traceIdx || root.pinnedInfo.pointIdx !== info.pointIdx)) {
            var x1 = root.getRawX(root.pinnedInfo.point)
            var y1 = root.getRawY(root.pinnedInfo.point)
            var x2 = root.getRawX(info.point)
            var y2 = root.getRawY(info.point)
            var dx = x2 - x1
            var dy = y2 - y1
            var out = "C1\u2192C2:   \u0394X = " + root.fmtUnit(dx, xU)
                + "   |   \u0394Y = " + root.fmtUnit(dy, yU)
            if (Math.abs(dx) > 1e-15) {
                var slope = dy / dx
                if (xU === "V" && yU === "A") {
                    out += "   |   gm = " + root.fmtUnit(slope, "S")
                    if (Math.abs(dy) > 1e-15)
                        out += "   |   r = \u0394V/\u0394I = " + root.fmtUnit(dx / dy, "\u03A9")
                } else if (xU === "A" && yU === "V") {
                    out += "   |   r = \u0394V/\u0394I = " + root.fmtUnit(slope, "\u03A9")
                } else {
                    out += "   |   \u0394Y/\u0394X = " + slope.toPrecision(4)
                }
            }
            root.statusText = out
            return
        }

        var tr = root.flatTraces[info.traceIdx]
        var sText = tr && tr.label ? (tr.label + "   |   ") : ""
        var xName = root.isXProbeVolt() ? "Vprobe" : root.result.primary_source
        var yName = root.isYCurrent() ? "I" : (root.isYVoltage() ? "V" : "P")
        sText += "X (" + xName + ") = " + root.fmtUnit(root.getRawX(info.point), xU)
        sText += "   |   Y (" + yName + ") = " + root.fmtUnit(root.getRawY(info.point), yU)
        sText += "   [V: " + root.fmtUnit(info.point.v_probe, "V")
            + ", I: " + root.fmtUnit(info.point.i_probe, "A")
            + ", P: " + root.fmtUnit(info.point.v_probe * info.point.i_probe, "W") + "]"
        root.statusText = sText
    }

    function buildCsv(): string {
        if (!root.result || !root.result.series || root.result.series.length === 0)
            return ""
        var lines = ["probe_pin,secondary_val,primary_val,v_probe_v,i_probe_a,power_w"]
        for (var s = 0; s < root.result.series.length; s++) {
            var ser = root.result.series[s]
            var curves = ser.curves || []
            for (var c = 0; c < curves.length; c++) {
                var pts = curves[c].points || []
                for (var i = 0; i < pts.length; i++) {
                    var p = pts[i]
                    lines.push([
                        ser.probe_pin,
                        p.secondary_val,
                        p.primary_val,
                        p.v_probe,
                        p.i_probe,
                        p.v_probe * p.i_probe
                    ].join(","))
                }
            }
        }
        return lines.join("\n") + "\n"
    }

    FileDialog {
        id: exportCsvDialog
        title: I18n.tr("Export DC Sweep CSV")
        fileMode: FileDialog.SaveFile
        currentFolder: App.suggestFolderUrl(CircuitCanvas.filePath)
        nameFilters: [I18n.tr("CSV files (*.csv)"), I18n.tr("All files (*)")]
        onAccepted: {
            if (exportCsvDialog.selectedFile) {
                var res = CircuitCanvas.saveTextFile("" + exportCsvDialog.selectedFile, root.buildCsv())
                if (res && res.ok)
                    root.statusText = I18n.tr("Exported CSV: ") + res.path
                else
                    root.statusText = (res && res.error) ? res.error : I18n.tr("Failed to export CSV.")
            }
        }
    }

    FileDialog {
        id: exportPngDialog
        title: I18n.tr("Save DC Sweep Plot Image")
        fileMode: FileDialog.SaveFile
        currentFolder: App.suggestFolderUrl(CircuitCanvas.filePath)
        nameFilters: [I18n.tr("PNG images (*.png)"), I18n.tr("All files (*)")]
        onAccepted: {
            if (exportPngDialog.selectedFile) {
                var localPath = CircuitCanvas.localPathFromUrl("" + exportPngDialog.selectedFile)
                if (localPath) {
                    plotFrame.grabToImage(function(result) {
                        if (result && result.saveToFile(localPath))
                            root.statusText = I18n.tr("Saved plot image: ") + localPath
                        else
                            root.statusText = I18n.tr("Failed to save plot image.")
                    })
                }
            }
        }
    }

    Component.onCompleted: root.reloadSourcesAndProbes()

    Connections {
        target: CircuitCanvas
        function onItemsChanged() { root.reloadSourcesAndProbes() }
        function onWiresChanged() { root.reloadSourcesAndProbes() }
        function onRootCircuitLoaded() { root.reloadSourcesAndProbes() }
        function onAppearanceChanged() { plot.requestPaint() }
    }

    Connections {
        target: App
        function onThemeChanged() { plot.requestPaint() }
        function onFontChanged() { plot.requestPaint() }
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 12
        spacing: 8

        ControlCard {
            // Row 1: Primary Source
            RowLayout {
                spacing: 8
                Layout.fillWidth: true

                AppText {
                    text: I18n.tr("Primary Source:")
                    font.pixelSize: App.fontSmall
                    Layout.preferredWidth: 158
                }
                AppComboBox {
                    id: primSourceBox
                    Layout.preferredWidth: 140
                }
                AppText {
                    text: I18n.tr("Start:")
                    font.pixelSize: App.fontSmall
                }
                AppTextField {
                    id: primStartField
                    Layout.preferredWidth: 68
                    text: "0.0"
                    onAccepted: root.runSweep()
                }
                AppText {
                    text: I18n.tr("Stop:")
                    font.pixelSize: App.fontSmall
                }
                AppTextField {
                    id: primStopField
                    Layout.preferredWidth: 68
                    text: "5.0"
                    onAccepted: root.runSweep()
                }
                AppText {
                    text: I18n.tr("Step:")
                    font.pixelSize: App.fontSmall
                }
                AppTextField {
                    id: primStepField
                    Layout.preferredWidth: 68
                    text: "0.1"
                    onAccepted: root.runSweep()
                }
                AppText {
                    readonly property string u: root.unitForSource(primSourceBox.currentText)
                    text: u ? ("(" + u + ")") : ""
                    opacity: 0.7
                    font.pixelSize: App.fontSmall
                    Layout.preferredWidth: 28
                }
                Item { Layout.fillWidth: true }
                AppButton {
                    text: I18n.tr("Run Sweep")
                    highlighted: true
                    onClicked: root.runSweep()
                }
            }

            // Row 2: Secondary Source (Family of Curves) + Export
            RowLayout {
                spacing: 8
                Layout.fillWidth: true

                AppCheckBox {
                    id: secEnableCheck
                    text: I18n.tr("Secondary (Family):")
                    checked: false
                    Layout.preferredWidth: 158
                    onCheckedChanged: if (!checked) {
                        root.hoverInfo = null
                        plot.requestPaint()
                    }
                }
                AppComboBox {
                    id: secSourceBox
                    enabled: secEnableCheck.checked
                    Layout.preferredWidth: 140
                }
                AppText {
                    text: I18n.tr("Start:")
                    font.pixelSize: App.fontSmall
                    opacity: secEnableCheck.checked ? 1.0 : 0.5
                }
                AppTextField {
                    id: secStartField
                    enabled: secEnableCheck.checked
                    Layout.preferredWidth: 68
                    text: "10u"
                    onAccepted: root.runSweep()
                }
                AppText {
                    text: I18n.tr("Stop:")
                    font.pixelSize: App.fontSmall
                    opacity: secEnableCheck.checked ? 1.0 : 0.5
                }
                AppTextField {
                    id: secStopField
                    enabled: secEnableCheck.checked
                    Layout.preferredWidth: 68
                    text: "50u"
                    onAccepted: root.runSweep()
                }
                AppText {
                    text: I18n.tr("Step:")
                    font.pixelSize: App.fontSmall
                    opacity: secEnableCheck.checked ? 1.0 : 0.5
                }
                AppTextField {
                    id: secStepField
                    enabled: secEnableCheck.checked
                    Layout.preferredWidth: 68
                    text: "10u"
                    onAccepted: root.runSweep()
                }
                AppText {
                    readonly property string u: root.unitForSource(secSourceBox.currentText)
                    text: u ? ("(" + u + ")") : ""
                    opacity: secEnableCheck.checked ? 0.7 : 0.35
                    font.pixelSize: App.fontSmall
                    Layout.preferredWidth: 28
                }
                Item { Layout.fillWidth: true }
                AppButton {
                    text: I18n.tr("CSV")
                    enabled: root.flatTraces.length > 0
                    onClicked: exportCsvDialog.open()
                }
                AppButton {
                    text: I18n.tr("PNG")
                    enabled: root.flatTraces.length > 0
                    onClicked: exportPngDialog.open()
                }
            }

            // Row 3: Probe Pin (+ multi-probe), Ref (differential), Axes, Log toggles, Zoom reset
            RowLayout {
                spacing: 8
                Layout.fillWidth: true

                AppText {
                    text: I18n.tr("Probe:")
                    font.pixelSize: App.fontSmall
                }
                AppComboBox {
                    id: probeBox
                    Layout.fillWidth: true
                    Layout.minimumWidth: 130
                    Layout.maximumWidth: 210
                }
                AppButton {
                    text: "+"
                    leftPadding: 8
                    rightPadding: 8
                    enabled: probeBox.currentText.length > 0 && root.extraProbes.indexOf(probeBox.currentText) < 0
                    onClicked: root.addExtraProbe()
                }
                AppText {
                    text: I18n.tr("Ref:")
                    font.pixelSize: App.fontSmall
                }
                AppComboBox {
                    id: refBox
                    Layout.preferredWidth: 118
                    model: [I18n.tr("GND")]
                }
                AppText {
                    text: I18n.tr("Y:")
                    font.pixelSize: App.fontSmall
                }
                AppComboBox {
                    id: yAxisBox
                    Layout.preferredWidth: 114
                    model: [I18n.tr("Current (I)"), I18n.tr("Voltage (V)"), I18n.tr("Power (W)")]
                    onCurrentIndexChanged: root.fitBounds()
                }
                AppText {
                    text: I18n.tr("X:")
                    font.pixelSize: App.fontSmall
                }
                AppComboBox {
                    id: xAxisBox
                    Layout.preferredWidth: 138
                    model: [I18n.tr("Source Value"), I18n.tr("Probe Voltage (V)")]
                    onCurrentIndexChanged: root.fitBounds()
                }
                AppCheckBox {
                    id: logXCheck
                    text: I18n.tr("Log X")
                    onToggled: root.fitBounds()
                }
                AppCheckBox {
                    id: logYCheck
                    text: I18n.tr("Log Y")
                    onToggled: root.fitBounds()
                }
                Item { Layout.fillWidth: true }
                AppButton {
                    text: I18n.tr("Reset Zoom")
                    enabled: root.isZoomed
                    onClicked: root.resetZoom()
                }
            }

            // Optional Row 4: Pinned comparison probes
            Flow {
                Layout.fillWidth: true
                spacing: 6
                visible: root.extraProbes.length > 0

                AppText {
                    text: I18n.tr("Pinned probes:")
                    font.pixelSize: App.fontMicro
                    opacity: 0.75
                    height: 22
                    verticalAlignment: Text.AlignVCenter
                }
                Repeater {
                    model: root.extraProbes
                    delegate: Rectangle {
                        id: chip
                        required property string modelData
                        height: 22
                        width: chipRow.implicitWidth + 14
                        radius: 11
                        color: Qt.rgba(pal.highlight.r, pal.highlight.g, pal.highlight.b, 0.16)
                        border.color: Qt.rgba(pal.highlight.r, pal.highlight.g, pal.highlight.b, 0.5)
                        border.width: 1

                        Row {
                            id: chipRow
                            anchors.centerIn: parent
                            spacing: 6
                            AppText {
                                text: chip.modelData
                                font.pixelSize: App.fontMicro
                            }
                            AppText {
                                text: "\u00D7"
                                font.pixelSize: App.fontMicro
                                font.bold: true
                                MouseArea {
                                    anchors.fill: parent
                                    cursorShape: Qt.PointingHandCursor
                                    onClicked: root.removeExtraProbe(chip.modelData)
                                }
                            }
                        }
                    }
                }
            }
        }

        // Trace / Curve Family Legend (shown when > 1 trace)
        Flow {
            Layout.fillWidth: true
            Layout.leftMargin: 2
            Layout.rightMargin: 2
            spacing: 12
            visible: root.flatTraces.length > 1

            Repeater {
                model: Math.min(root.flatTraces.length, 16)
                delegate: Row {
                    id: legendItem
                    required property int index
                    spacing: 5

                    Rectangle {
                        width: 12
                        height: 3
                        radius: 1.5
                        anchors.verticalCenter: parent.verticalCenter
                        color: root.getTraceColor(legendItem.index)
                    }
                    AppText {
                        text: root.flatTraces[legendItem.index] ? root.flatTraces[legendItem.index].label : ""
                        font.pixelSize: App.fontMicro
                    }
                }
            }

            AppText {
                visible: root.flatTraces.length > 16
                text: "+" + (root.flatTraces.length - 16) + " " + I18n.tr("more")
                opacity: 0.7
                font.pixelSize: App.fontMicro
            }
        }

        // Status / Measurement Info Bar
        AppText {
            Layout.fillWidth: true
            Layout.leftMargin: 2
            Layout.rightMargin: 2
            text: root.statusText
            font.pixelSize: App.fontSmall
            elide: Text.ElideRight
        }

        // Rounded Graph Container
        Rectangle {
            id: plotFrame
            Layout.fillWidth: true
            Layout.fillHeight: true
            radius: 10
            color: pal.base
            border.color: Qt.rgba(pal.mid.r, pal.mid.g, pal.mid.b, 0.75)
            border.width: 1
            clip: true

            Canvas {
                id: plot
                anchors.fill: parent
                anchors.margins: 1
                contextType: "2d"

                onWidthChanged: requestPaint()
                onHeightChanged: requestPaint()

                onPaint: {
                    var ctx = getContext("2d")
                    if (!ctx) return

                    ctx.clearRect(0, 0, width, height)
                    ctx.globalAlpha = 1

                    var plotW = width - root.plotLeft - root.plotRight
                    var plotH = height - root.plotTop - root.plotBottom
                    if (plotW <= 0 || plotH <= 0) return

                    // Plot bounding box
                    ctx.strokeStyle = pal.mid
                    ctx.lineWidth = 1
                    ctx.strokeRect(root.plotLeft, root.plotTop, plotW, plotH)

                    var xUnitStr = root.xUnit()
                    var yUnitStr = root.yUnit()

                    ctx.fillStyle = pal.windowText
                    ctx.font = App.fontSmall + "px \"" + App.fontFamily + "\""

                    // Horizontal grid lines (Y axis)
                    ctx.textAlign = "right"
                    ctx.textBaseline = "middle"
                    if (logYCheck.checked) {
                        var yExpLo = Math.ceil(root.yMin - 1e-6)
                        var yExpHi = Math.floor(root.yMax + 1e-6)
                        var yDecStep = Math.max(1, Math.ceil((yExpHi - yExpLo) / 6))
                        for (var ye = yExpLo; ye <= yExpHi; ye += yDecStep) {
                            var pyL = root.mapY(ye)
                            if (pyL < root.plotTop - 1 || pyL > root.plotTop + plotH + 1) continue
                            ctx.strokeStyle = pal.mid
                            ctx.globalAlpha = 0.28
                            ctx.beginPath()
                            ctx.moveTo(root.plotLeft, pyL)
                            ctx.lineTo(root.plotLeft + plotW, pyL)
                            ctx.stroke()
                            ctx.globalAlpha = 1
                            ctx.fillText(root.fmtUnit(Math.pow(10, ye), yUnitStr), root.plotLeft - 8, pyL)
                        }
                    } else {
                        var yTicks = 5
                        for (var j = 0; j <= yTicks; j++) {
                            var fracY = j / yTicks
                            var yVal = root.yMin + fracY * (root.yMax - root.yMin)
                            var py = root.plotTop + plotH - fracY * plotH

                            ctx.strokeStyle = pal.mid
                            ctx.globalAlpha = 0.28
                            ctx.beginPath()
                            ctx.moveTo(root.plotLeft, py)
                            ctx.lineTo(root.plotLeft + plotW, py)
                            ctx.stroke()

                            ctx.globalAlpha = 1
                            ctx.fillText(root.fmtUnit(yVal, yUnitStr), root.plotLeft - 8, py)
                        }
                    }

                    // Vertical grid lines (X axis)
                    ctx.textAlign = "center"
                    ctx.textBaseline = "top"
                    if (logXCheck.checked) {
                        var xExpLo = Math.ceil(root.xMin - 1e-6)
                        var xExpHi = Math.floor(root.xMax + 1e-6)
                        var xDecStep = Math.max(1, Math.ceil((xExpHi - xExpLo) / 6))
                        for (var xe = xExpLo; xe <= xExpHi; xe += xDecStep) {
                            var pxL = root.mapX(xe)
                            if (pxL < root.plotLeft - 1 || pxL > root.plotLeft + plotW + 1) continue
                            ctx.strokeStyle = pal.mid
                            ctx.globalAlpha = 0.28
                            ctx.beginPath()
                            ctx.moveTo(pxL, root.plotTop)
                            ctx.lineTo(pxL, root.plotTop + plotH)
                            ctx.stroke()
                            ctx.globalAlpha = 1
                            ctx.fillText(root.fmtUnit(Math.pow(10, xe), xUnitStr), pxL, root.plotTop + plotH + 6)
                        }
                    } else {
                        var xTicks = 5
                        for (var k = 0; k <= xTicks; k++) {
                            var fracX = k / xTicks
                            var xVal = root.xMin + fracX * (root.xMax - root.xMin)
                            var px = root.plotLeft + fracX * plotW

                            ctx.strokeStyle = pal.mid
                            ctx.globalAlpha = 0.28
                            ctx.beginPath()
                            ctx.moveTo(px, root.plotTop)
                            ctx.lineTo(px, root.plotTop + plotH)
                            ctx.stroke()

                            ctx.globalAlpha = 1
                            ctx.fillText(root.fmtUnit(xVal, xUnitStr), px, root.plotTop + plotH + 6)
                        }
                    }

                    // Subtle zero-axis reference lines (linear mode only)
                    ctx.save()
                    ctx.strokeStyle = pal.windowText
                    ctx.globalAlpha = 0.22
                    ctx.setLineDash([4, 4])
                    if (!logYCheck.checked && root.yMin < 0 && root.yMax > 0) {
                        var y0 = root.mapY(0)
                        ctx.beginPath()
                        ctx.moveTo(root.plotLeft, y0)
                        ctx.lineTo(root.plotLeft + plotW, y0)
                        ctx.stroke()
                    }
                    if (!logXCheck.checked && root.xMin < 0 && root.xMax > 0) {
                        var x0 = root.mapX(0)
                        ctx.beginPath()
                        ctx.moveTo(x0, root.plotTop)
                        ctx.lineTo(x0, root.plotTop + plotH)
                        ctx.stroke()
                    }
                    ctx.restore()

                    // Axis titles
                    var probeLabel = (root.result && root.result.series && root.result.series.length === 1)
                        ? root.result.series[0].probe_pin
                        : (probeBox.currentText || "probe")
                    var refLabel = (root.result && root.result.ref_pin) ? (" \u2212 " + root.result.ref_pin) : ""
                    var xCore = root.isXProbeVolt()
                        ? ("V(" + probeLabel + refLabel + ")")
                        : (root.result ? root.result.primary_source : primSourceBox.currentText)
                    var xTitle = (logXCheck.checked ? ("|" + xCore + "| (log)") : xCore) + (xUnitStr ? (" [" + xUnitStr + "]") : "")

                    var yPrefix = root.isYCurrent() ? "I(" : (root.isYVoltage() ? "V(" : "P(")
                    var yCore = yPrefix + probeLabel + (root.isYCurrent() ? "" : refLabel) + ")"
                    var yTitle = (logYCheck.checked ? ("|" + yCore + "| (log)") : yCore) + " [" + yUnitStr + "]"

                    ctx.fillStyle = pal.windowText
                    ctx.globalAlpha = 0.75
                    ctx.textAlign = "center"
                    ctx.textBaseline = "bottom"
                    ctx.fillText(xTitle, root.plotLeft + plotW / 2, height - 4)

                    ctx.textAlign = "left"
                    ctx.textBaseline = "top"
                    ctx.fillText(yTitle, root.plotLeft + 8, root.plotTop + 6)
                    ctx.globalAlpha = 1

                    // Draw traces clipped to plot rect
                    if (root.flatTraces && root.flatTraces.length > 0) {
                        ctx.save()
                        ctx.beginPath()
                        ctx.rect(root.plotLeft, root.plotTop, plotW, plotH)
                        ctx.clip()

                        ctx.lineWidth = 2
                        ctx.lineCap = "round"
                        ctx.lineJoin = "round"

                        for (var c = 0; c < root.flatTraces.length; c++) {
                            var pts = root.flatTraces[c].points
                            if (!pts || pts.length === 0) continue

                            ctx.strokeStyle = root.getTraceColor(c)
                            ctx.beginPath()
                            var started = false

                            for (var p = 0; p < pts.length; p++) {
                                var rx = root.getRawX(pts[p])
                                var ry = root.getRawY(pts[p])
                                if ((logXCheck.checked && Math.abs(rx) < root.logFloor)
                                    || (logYCheck.checked && Math.abs(ry) < root.logFloor)) {
                                    started = false
                                    continue
                                }
                                var cx = root.mapX(root.xToPlot(rx))
                                var cy = root.mapY(root.yToPlot(ry))
                                if (!started) {
                                    ctx.moveTo(cx, cy)
                                    started = true
                                } else {
                                    ctx.lineTo(cx, cy)
                                }
                            }
                            ctx.stroke()
                        }
                        ctx.restore()
                    }

                    // Pinned C1 cursor
                    if (root.pinnedInfo) {
                        root.paintMarker(ctx, root.pinnedInfo, plotW, plotH, false, "C1")
                    }
                    // Hover C2 cursor
                    if (root.hoverInfo && !root.draggingZoom) {
                        root.paintMarker(ctx, root.hoverInfo, plotW, plotH, true, root.pinnedInfo ? "C2" : "")
                    }
                    // Drag-zoom selection rectangle
                    if (root.draggingZoom) {
                        var zw = Math.abs(root.dragCurrX - root.dragStartX)
                        var zh = Math.abs(root.dragCurrY - root.dragStartY)
                        if (zw > 10 && zh > 10) {
                            var zx = Math.min(root.dragStartX, root.dragCurrX)
                            var zy = Math.min(root.dragStartY, root.dragCurrY)
                            ctx.save()
                            ctx.fillStyle = pal.highlight
                            ctx.globalAlpha = 0.14
                            ctx.fillRect(zx, zy, zw, zh)
                            ctx.strokeStyle = pal.highlight
                            ctx.globalAlpha = 0.75
                            ctx.lineWidth = 1
                            ctx.strokeRect(zx, zy, zw, zh)
                            ctx.restore()
                        }
                    }
                }

                MouseArea {
                    anchors.fill: parent
                    hoverEnabled: true
                    acceptedButtons: Qt.LeftButton | Qt.RightButton
                    cursorShape: Qt.CrossCursor

                    onPressed: function(mouse) {
                        if (mouse.button === Qt.RightButton) {
                            root.pinnedInfo = null
                            root.updateReadout(root.hoverInfo)
                            plot.requestPaint()
                            return
                        }
                        if (!root.result) return
                        if (mouse.x < root.plotLeft || mouse.x > plot.width - root.plotRight
                            || mouse.y < root.plotTop || mouse.y > plot.height - root.plotBottom)
                            return
                        root.dragStartX = mouse.x
                        root.dragStartY = mouse.y
                        root.dragCurrX = mouse.x
                        root.dragCurrY = mouse.y
                        root.draggingZoom = true
                    }

                    onPositionChanged: function(mouse) {
                        if (root.draggingZoom) {
                            root.dragCurrX = Math.max(root.plotLeft, Math.min(plot.width - root.plotRight, mouse.x))
                            root.dragCurrY = Math.max(root.plotTop, Math.min(plot.height - root.plotBottom, mouse.y))
                            plot.requestPaint()
                            return
                        }
                        var nearest = root.findNearest(mouse.x, mouse.y)
                        root.hoverInfo = nearest
                        root.updateReadout(nearest)
                        plot.requestPaint()
                    }

                    onReleased: function(mouse) {
                        if (mouse.button !== Qt.LeftButton || !root.draggingZoom)
                            return
                        root.draggingZoom = false
                        var zw = Math.abs(root.dragCurrX - root.dragStartX)
                        var zh = Math.abs(root.dragCurrY - root.dragStartY)
                        if (zw > 10 && zh > 10) {
                            var x0 = Math.min(root.dragStartX, root.dragCurrX)
                            var x1 = Math.max(root.dragStartX, root.dragCurrX)
                            var y0 = Math.min(root.dragStartY, root.dragCurrY)
                            var y1 = Math.max(root.dragStartY, root.dragCurrY)
                            var nxMin = root.plotXOfPx(x0)
                            var nxMax = root.plotXOfPx(x1)
                            var nyMax = root.plotYOfPy(y0)
                            var nyMin = root.plotYOfPy(y1)
                            if (nxMax > nxMin && nyMax > nyMin) {
                                root.xMin = nxMin
                                root.xMax = nxMax
                                root.yMin = nyMin
                                root.yMax = nyMax
                                root.isZoomed = true
                            }
                        } else {
                            var nearest = root.findNearest(mouse.x, mouse.y)
                            if (nearest) {
                                var same = root.pinnedInfo
                                    && root.pinnedInfo.traceIdx === nearest.traceIdx
                                    && root.pinnedInfo.pointIdx === nearest.pointIdx
                                root.pinnedInfo = same ? null : nearest
                                root.updateReadout(nearest)
                            }
                        }
                        plot.requestPaint()
                    }

                    onDoubleClicked: root.resetZoom()

                    onWheel: function(wheel) {
                        if (!root.result) return
                        var dy = wheel.angleDelta.y !== 0 ? wheel.angleDelta.y : wheel.pixelDelta.y
                        if (dy === 0) return
                        var factor = dy > 0 ? 0.8 : 1.25
                        root.zoomAt(factor, wheel.x, wheel.y)
                    }

                    onExited: {
                        if (!root.draggingZoom) {
                            root.hoverInfo = null
                            root.statusText = root.summaryText
                            plot.requestPaint()
                        }
                    }
                }
            }
        }
    }

    function paintMarker(ctx: var, info: var, plotW: real, plotH: real, dashed: bool, label: string): void {
        if (!info || !info.point) return
        var rx = root.getRawX(info.point)
        var ry = root.getRawY(info.point)
        if ((logXCheck.checked && Math.abs(rx) < root.logFloor)
            || (logYCheck.checked && Math.abs(ry) < root.logFloor))
            return
        var px = root.mapX(root.xToPlot(rx))
        var py = root.mapY(root.yToPlot(ry))
        if (px < root.plotLeft || px > root.plotLeft + plotW || py < root.plotTop || py > root.plotTop + plotH)
            return

        ctx.save()
        ctx.strokeStyle = dashed ? pal.highlight : CircuitCanvas.laChannel2Color
        ctx.lineWidth = 1
        ctx.globalAlpha = dashed ? 0.65 : 0.85
        if (dashed) ctx.setLineDash([4, 4])

        ctx.beginPath()
        ctx.moveTo(root.plotLeft, py)
        ctx.lineTo(root.plotLeft + plotW, py)
        ctx.moveTo(px, root.plotTop)
        ctx.lineTo(px, root.plotTop + plotH)
        ctx.stroke()
        ctx.setLineDash([])
        ctx.globalAlpha = 1

        ctx.fillStyle = root.getTraceColor(info.traceIdx)
        ctx.beginPath()
        ctx.arc(px, py, 5, 0, 2 * Math.PI)
        ctx.fill()
        ctx.strokeStyle = pal.base
        ctx.lineWidth = 1.5
        ctx.stroke()

        if (label) {
            ctx.fillStyle = dashed ? pal.highlight : CircuitCanvas.laChannel2Color
            ctx.font = App.fontMicro + "px \"" + App.fontFamily + "\""
            ctx.textAlign = "left"
            ctx.textBaseline = "bottom"
            ctx.fillText(label, px + 7, py - 5)
        }
        ctx.restore()
    }
}
