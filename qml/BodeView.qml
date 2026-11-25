pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Dialogs
import QtQuick.Layouts
import cs_app
import cs_ui

// Small-signal magnitude and phase Bode plot. Runs on a copy of the schematic,
// including DC-bias small-signal linearization for active semiconductor parts.
Rectangle {
    id: root

    color: pal.window
    SystemPalette { id: pal; colorGroup: SystemPalette.Active }

    // Array of { probe: string, refPin: string, points: [{ hz, mag, phase, phaseUnwrapped, gain, db }] }
    property var traces: []
    property var extraProbes: []
    property bool swept: false
    property real fLo: 10
    property real fHi: 100000
    property real fFullLo: 10
    property real fFullHi: 100000
    property real dbMin: -40
    property real dbMax: 10
    property real dbFullMin: -40
    property real dbFullMax: 10
    property real phMin: -180
    property real phMax: 180
    property bool isZoomed: false

    property real hoverX: -1
    property int pinnedIdx: -1
    property bool draggingZoom: false
    property real dragStartX: 0
    property real dragStartY: 0
    property real dragCurrX: 0
    property real dragCurrY: 0

    property string summaryText: I18n.tr("Choose a voltage source and probe pin, then click Run Sweep.")
    property string statusText: summaryText

    readonly property int plotLeft: 64
    readonly property int plotRight: 18
    readonly property int plotTop: 14
    readonly property int plotBottom: 30
    readonly property int plotGap: 18

    readonly property color magColor: pal.highlight
    readonly property color phaseColor: {
        var c = CircuitCanvas.laChannel2Color
        return (!App.darkTheme && UiUtils.isLightColor(c)) ? Qt.darker(c, 1.35) : c
    }

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
            pal.highlight,
            CircuitCanvas.laChannel1Color,
            CircuitCanvas.laChannel4Color,
            CircuitCanvas.laChannel2Color,
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

    function traceLabel(t: var): string {
        if (!t) return ""
        return t.refPin ? (t.probe + " \u2212 " + t.refPin) : t.probe
    }

    function activeProbeList(): var {
        var out = []
        var cur = probeBox.currentText
        if (cur)
            out.push(cur)
        for (var i = 0; i < root.extraProbes.length; i++) {
            var p = root.extraProbes[i]
            if (p && out.indexOf(p) < 0)
                out.push(p)
        }
        return out
    }

    function addExtraProbe(): void {
        var cur = probeBox.currentText
        if (!cur) return
        if (root.extraProbes.indexOf(cur) >= 0) return
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

    function reloadEnds(): void {
        var sources = CircuitCanvas.acSources()
        var probes = CircuitCanvas.acProbes()
        var probeArr = probes ? probes : []
        var keepS = sourceBox.currentText
        var keepP = probeBox.currentText
        var keepR = root.selectedRefPin()

        sourceBox.model = sources ? sources : []
        probeBox.model = probeArr
        var refModel = [I18n.tr("GND")].concat(probeArr)
        refBox.model = refModel

        var si = root.indexOfText(sources, keepS)
        sourceBox.currentIndex = si >= 0 ? si : (sources && sources.length > 0 ? 0 : -1)

        var pi = root.indexOfText(probeArr, keepP)
        probeBox.currentIndex = pi >= 0 ? pi : (probeArr.length > 0 ? 0 : -1)

        var ri = keepR ? root.indexOfText(refModel, keepR) : 0
        refBox.currentIndex = ri >= 0 ? ri : 0

        // Prune removed pins from extraProbes
        var kept = []
        for (var i = 0; i < root.extraProbes.length; i++) {
            if (probeArr.indexOf(root.extraProbes[i]) >= 0)
                kept.push(root.extraProbes[i])
        }
        root.extraProbes = kept

        if (root.swept) {
            root.swept = false
            root.traces = []
            root.hoverX = -1
            root.pinnedIdx = -1
            root.isZoomed = false
            root.summaryText = I18n.tr("Circuit changed. Run the sweep again.")
            root.statusText = root.summaryText
            plot.requestPaint()
        }
    }

    function indexOfText(model: var, text: string): int {
        if (!model || !text)
            return -1
        for (var i = 0; i < model.length; i++) {
            if (model[i] === text)
                return i
        }
        return -1
    }

    function unwrapTracePhases(pts: var): void {
        if (!pts || pts.length === 0) return
        var offset = 0
        pts[0].phaseUnwrapped = pts[0].phase
        for (var i = 1; i < pts.length; i++) {
            var d = pts[i].phase - pts[i - 1].phase
            if (d > 180) offset -= 360
            else if (d < -180) offset += 360
            pts[i].phaseUnwrapped = pts[i].phase + offset
        }
    }

    function phaseKey(): string {
        return unwrapCheck.checked ? "phaseUnwrapped" : "phase"
    }

    function fitPhase(): void {
        if (!unwrapCheck.checked || !root.traces || root.traces.length === 0) {
            root.phMin = -180
            root.phMax = 180
            plot.requestPaint()
            return
        }
        var lo = Infinity
        var hi = -Infinity
        for (var t = 0; t < root.traces.length; t++) {
            var pts = root.traces[t].points
            for (var i = 0; i < pts.length; i++) {
                var ph = pts[i].phaseUnwrapped
                if (ph < lo) lo = ph
                if (ph > hi) hi = ph
            }
        }
        if (!isFinite(lo) || !isFinite(hi)) {
            root.phMin = -180
            root.phMax = 180
        } else {
            var lo90 = Math.floor(lo / 90) * 90
            var hi90 = Math.ceil(hi / 90) * 90
            if (hi90 - lo90 < 90) {
                lo90 -= 90
                hi90 += 90
            }
            root.phMin = lo90
            root.phMax = hi90
        }
        plot.requestPaint()
    }

    function buildSummary(): string {
        if (!root.traces || root.traces.length === 0 || root.traces[0].points.length === 0)
            return I18n.tr("The sweep returned no points.")
        var pts = root.traces[0].points
        var peakIdx = 0
        for (var i = 1; i < pts.length; i++) {
            if (pts[i].db > pts[peakIdx].db)
                peakIdx = i
        }
        var peak = pts[peakIdx]
        var target = peak.db - 3.0
        var cutoffs = []
        for (var j = 1; j < pts.length; j++) {
            var a = pts[j - 1]
            var b = pts[j]
            if ((a.db >= target && b.db < target) || (a.db < target && b.db >= target)) {
                var frac = (target - a.db) / (b.db - a.db)
                var logF = Math.log10(a.hz) + frac * (Math.log10(b.hz) - Math.log10(a.hz))
                cutoffs.push(root.fmtHz(Math.pow(10, logF)))
                if (cutoffs.length >= 2)
                    break
            }
        }
        var prefix = root.traces.length > 1 ? (root.traceLabel(root.traces[0]) + " — ") : ""
        var txt = prefix + I18n.tr("Peak: ") + Number(peak.db).toFixed(2) + " dB @ " + root.fmtHz(peak.hz)
        if (cutoffs.length > 0)
            txt += "   |   -3 dB: " + cutoffs.join(", ")
        txt += "   |   " + I18n.tr("Click to pin C1 cursor, drag to zoom, right-click to clear C1.")
        return txt
    }

    function runSweep(): void {
        var source = sourceBox.currentText
        var probes = root.activeProbeList()
        if (!source || probes.length === 0) {
            root.summaryText = I18n.tr("Choose a voltage source and a probe pin.")
            root.statusText = root.summaryText
            return
        }
        var f0 = UiUtils.readNumber(startField.text, 10)
        var f1 = UiUtils.readNumber(stopField.text, 100000)
        var ppd = Math.round(UiUtils.readNumber(ppdField.text, 20))
        var res = CircuitCanvas.acSweepMulti(source, JSON.stringify(probes), root.selectedRefPin(), f0, f1, ppd)
        if (!res || res.error) {
            root.traces = []
            root.swept = false
            root.hoverX = -1
            root.pinnedIdx = -1
            root.isZoomed = false
            root.summaryText = res && res.error ? res.error : I18n.tr("AC sweep failed.")
            root.statusText = root.summaryText
            plot.requestPaint()
            return
        }
        var list = res.traces ? res.traces : []
        for (var t = 0; t < list.length; t++) {
            root.unwrapTracePhases(list[t].points)
        }
        root.traces = list
        root.swept = list.length > 0 && list[0].points && list[0].points.length > 0
        root.pinnedIdx = -1
        root.isZoomed = false
        if (root.swept) {
            var firstPts = list[0].points
            root.fFullLo = firstPts[0].hz
            root.fFullHi = firstPts[firstPts.length - 1].hz
            root.fLo = root.fFullLo
            root.fHi = root.fFullHi
            root.fitDb()
            root.dbFullMin = root.dbMin
            root.dbFullMax = root.dbMax
            root.fitPhase()
            root.summaryText = root.buildSummary()
        } else {
            root.summaryText = I18n.tr("The sweep returned no points.")
        }
        root.statusText = root.summaryText
        plot.requestPaint()
    }

    function fitDb(): void {
        var lo = Infinity
        var hi = -Infinity
        for (var t = 0; t < root.traces.length; t++) {
            var pts = root.traces[t].points
            for (var i = 0; i < pts.length; i++) {
                var db = pts[i].db
                if (db < lo) lo = db
                if (db > hi) hi = db
            }
        }
        if (!isFinite(lo) || !isFinite(hi)) {
            root.dbMin = -40
            root.dbMax = 10
            return
        }
        if (hi - lo > 80)
            lo = hi - 80
        if (hi - lo < 6) {
            var mid = (hi + lo) / 2
            lo = mid - 3
            hi = mid + 3
        }
        var pad = (hi - lo) * 0.08
        root.dbMin = lo - pad
        root.dbMax = hi + pad
    }

    function resetZoom(): void {
        if (!root.swept) return
        root.fLo = root.fFullLo
        root.fHi = root.fFullHi
        root.dbMin = root.dbFullMin
        root.dbMax = root.dbFullMax
        root.isZoomed = false
        plot.requestPaint()
    }

    function zoomFreqAt(factor: real, anchorPx: real): void {
        if (!root.swept) return
        var w = root.innerW()
        var frac = Math.max(0.0, Math.min(1.0, (anchorPx - root.plotLeft) / w))
        var log0 = Math.log10(root.fLo)
        var log1 = Math.log10(root.fHi)
        var full0 = Math.log10(root.fFullLo)
        var full1 = Math.log10(root.fFullHi)
        var span = log1 - log0
        if (span <= 0) return
        var newSpan = Math.max(0.02, Math.min(full1 - full0, span * factor))
        var anchorLog = log0 + frac * span
        var n0 = anchorLog - frac * newSpan
        var n1 = n0 + newSpan
        if (n0 < full0) { n0 = full0; n1 = Math.min(full1, full0 + newSpan) }
        if (n1 > full1) { n1 = full1; n0 = Math.max(full0, full1 - newSpan) }
        root.fLo = Math.pow(10, n0)
        root.fHi = Math.pow(10, n1)
        root.isZoomed = (root.fLo > root.fFullLo * 1.001 || root.fHi < root.fFullHi * 0.999
                         || root.dbMin > root.dbFullMin + 0.01 || root.dbMax < root.dbFullMax - 0.01)
        plot.requestPaint()
    }

    function innerW(): real {
        return Math.max(1, plot.width - root.plotLeft - root.plotRight)
    }

    function paneH(): real {
        var h = plot.height - root.plotTop - root.plotBottom - root.plotGap
        return Math.max(1, h / 2)
    }

    function xOf(hz: real): real {
        var log0 = Math.log10(root.fLo)
        var log1 = Math.log10(root.fHi)
        var span = log1 - log0
        var t = span > 0 ? (Math.log10(hz) - log0) / span : 0
        return root.plotLeft + t * root.innerW()
    }

    function hzOfX(px: real): real {
        var log0 = Math.log10(root.fLo)
        var log1 = Math.log10(root.fHi)
        var t = Math.max(0.0, Math.min(1.0, (px - root.plotLeft) / root.innerW()))
        return Math.pow(10, log0 + t * (log1 - log0))
    }

    function yOf(v: real, top: real, h: real, vmin: real, vmax: real): real {
        var span = vmax - vmin
        var t = span !== 0 ? (v - vmin) / span : 0.5
        return top + h - t * h
    }

    function valOfY(py: real, top: real, h: real, vmin: real, vmax: real): real {
        var t = Math.max(0.0, Math.min(1.0, (top + h - py) / Math.max(1, h)))
        return vmin + t * (vmax - vmin)
    }

    function fmtHz(hz: real): string {
        var a = Math.abs(hz)
        if (a >= 1e6)
            return (hz / 1e6).toFixed(a >= 1e7 ? 0 : 2) + " MHz"
        if (a >= 1e3)
            return (hz / 1e3).toFixed(a >= 1e4 ? 0 : 2) + " kHz"
        if (a >= 10)
            return hz.toFixed(1) + " Hz"
        return hz.toFixed(2) + " Hz"
    }

    function nearest(x: real): int {
        if (!root.traces || root.traces.length === 0)
            return -1
        var pts = root.traces[0].points
        if (!pts || pts.length === 0)
            return -1
        var best = 0
        var bestD = 1e9
        for (var i = 0; i < pts.length; i++) {
            var d = Math.abs(root.xOf(pts[i].hz) - x)
            if (d < bestD) {
                bestD = d
                best = i
            }
        }
        return best
    }

    function updateHoverStatus(idx: int): void {
        if (idx < 0 || !root.traces || root.traces.length === 0) {
            root.statusText = root.summaryText
            return
        }
        var pKey = root.phaseKey()
        var p = root.traces[0].points[idx]
        if (root.pinnedIdx >= 0 && root.pinnedIdx < root.traces[0].points.length && root.pinnedIdx !== idx) {
            var p1 = root.traces[0].points[root.pinnedIdx]
            var df = p.hz - p1.hz
            var dec = Math.log10(p.hz / p1.hz)
            var dDb = p.db - p1.db
            var dPh = p[pKey] - p1[pKey]
            var slope = Math.abs(dec) > 1e-6 ? (dDb / dec) : 0
            root.statusText = "C1 (" + root.fmtHz(p1.hz) + ") \u2192 C2 (" + root.fmtHz(p.hz) + ")"
                + "   |   \u0394f = " + root.fmtHz(df) + " (" + dec.toFixed(2) + " dec)"
                + "   |   \u0394Mag = " + (dDb >= 0 ? "+" : "") + dDb.toFixed(2) + " dB (" + slope.toFixed(1) + " dB/dec)"
                + "   |   \u0394Ph = " + (dPh >= 0 ? "+" : "") + dPh.toFixed(1) + "\u00B0"
            return
        }
        if (root.traces.length === 1) {
            root.statusText = root.fmtHz(p.hz)
                + "   |   " + Number(p.db).toFixed(2) + " dB"
                + "   |   " + Number(p[pKey]).toFixed(1) + "\u00B0"
                + "   |   |V| = " + Number(p.mag).toFixed(4) + " V"
        } else {
            var parts = [root.fmtHz(p.hz)]
            for (var t = 0; t < root.traces.length; t++) {
                var tp = root.traces[t].points[idx]
                parts.push(root.traces[t].probe + ": " + Number(tp.db).toFixed(2) + " dB / " + Number(tp[pKey]).toFixed(1) + "\u00B0")
            }
            root.statusText = parts.join("   |   ")
        }
    }

    function buildCsv(): string {
        if (!root.swept || root.traces.length === 0)
            return ""
        var pKey = root.phaseKey()
        var header = ["freq_hz"]
        for (var t = 0; t < root.traces.length; t++) {
            var lbl = root.traceLabel(root.traces[t]).replace(/,/g, "_")
            header.push(lbl + "_db")
            header.push(lbl + "_phase_deg")
            header.push(lbl + "_mag_v")
            header.push(lbl + "_gain")
        }
        var lines = [header.join(",")]
        var n = root.traces[0].points.length
        for (var i = 0; i < n; i++) {
            var row = [String(root.traces[0].points[i].hz)]
            for (var k = 0; k < root.traces.length; k++) {
                var pt = root.traces[k].points[i]
                row.push(String(pt.db))
                row.push(String(pt[pKey]))
                row.push(String(pt.mag))
                row.push(String(pt.gain))
            }
            lines.push(row.join(","))
        }
        return lines.join("\n") + "\n"
    }

    FileDialog {
        id: exportCsvDialog
        title: I18n.tr("Export AC Sweep CSV")
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
        title: I18n.tr("Save AC Sweep Plot Image")
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

    Component.onCompleted: root.reloadEnds()

    Connections {
        target: CircuitCanvas
        function onItemsChanged() { root.reloadEnds() }
        function onWiresChanged() { root.reloadEnds() }
        function onRootCircuitLoaded() { root.reloadEnds() }
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
            // Row 1: Source, Probe (+ multi-probe), Ref (differential), Run
            RowLayout {
                Layout.fillWidth: true
                spacing: 8

                AppText {
                    text: I18n.tr("Source:")
                    font.pixelSize: App.fontSmall
                }
                AppComboBox {
                    id: sourceBox
                    Layout.preferredWidth: 150
                    model: []
                }
                AppText {
                    text: I18n.tr("Probe:")
                    font.pixelSize: App.fontSmall
                }
                AppComboBox {
                    id: probeBox
                    Layout.fillWidth: true
                    Layout.minimumWidth: 140
                    model: []
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
                    Layout.preferredWidth: 140
                    model: [I18n.tr("GND")]
                }
                AppButton {
                    text: I18n.tr("Run Sweep")
                    highlighted: true
                    onClicked: root.runSweep()
                }
            }

            // Row 2: Frequency range, Unwrap phase, Zoom reset, Export CSV / PNG
            RowLayout {
                Layout.fillWidth: true
                spacing: 8

                AppText {
                    text: I18n.tr("Start (Hz):")
                    font.pixelSize: App.fontSmall
                }
                AppTextField {
                    id: startField
                    Layout.preferredWidth: 78
                    text: "10"
                    onAccepted: root.runSweep()
                }
                AppText {
                    text: I18n.tr("Stop (Hz):")
                    font.pixelSize: App.fontSmall
                }
                AppTextField {
                    id: stopField
                    Layout.preferredWidth: 84
                    text: "100k"
                    onAccepted: root.runSweep()
                }
                AppText {
                    text: I18n.tr("Pts/dec:")
                    font.pixelSize: App.fontSmall
                }
                AppTextField {
                    id: ppdField
                    Layout.preferredWidth: 50
                    text: "20"
                    onAccepted: root.runSweep()
                }
                AppCheckBox {
                    id: unwrapCheck
                    text: I18n.tr("Unwrap phase")
                    checked: false
                    onToggled: root.fitPhase()
                }
                Item { Layout.fillWidth: true }
                AppButton {
                    text: I18n.tr("Reset Zoom")
                    enabled: root.isZoomed
                    onClicked: root.resetZoom()
                }
                AppButton {
                    text: I18n.tr("CSV")
                    enabled: root.swept
                    onClicked: exportCsvDialog.open()
                }
                AppButton {
                    text: I18n.tr("PNG")
                    enabled: root.swept
                    onClicked: exportPngDialog.open()
                }
            }

            // Optional Row 3: Pinned comparison probes (multi-probe chips)
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

        // Legend bar
        Flow {
            Layout.fillWidth: true
            Layout.leftMargin: 2
            Layout.rightMargin: 2
            spacing: 12

            Row {
                spacing: 5
                visible: root.traces.length <= 1
                Rectangle {
                    width: 14; height: 3; radius: 1.5
                    anchors.verticalCenter: parent.verticalCenter
                    color: root.magColor
                }
                AppText {
                    text: I18n.tr("Magnitude")
                    font.pixelSize: App.fontSmall
                }
            }
            Row {
                spacing: 5
                visible: root.traces.length <= 1
                Rectangle {
                    width: 14; height: 3; radius: 1.5
                    anchors.verticalCenter: parent.verticalCenter
                    color: root.phaseColor
                }
                AppText {
                    text: I18n.tr("Phase")
                    font.pixelSize: App.fontSmall
                }
            }
            Repeater {
                model: root.traces.length > 1 ? root.traces.length : 0
                delegate: Row {
                    id: tLeg
                    required property int index
                    spacing: 5
                    Rectangle {
                        width: 14; height: 3; radius: 1.5
                        anchors.verticalCenter: parent.verticalCenter
                        color: root.getTraceColor(tLeg.index)
                    }
                    AppText {
                        text: root.traceLabel(root.traces[tLeg.index])
                        font.pixelSize: App.fontMicro
                    }
                }
            }
        }

        // Status / readout bar
        AppText {
            Layout.fillWidth: true
            Layout.leftMargin: 2
            Layout.rightMargin: 2
            text: root.statusText
            font.pixelSize: App.fontSmall
            elide: Text.ElideRight
        }

        // Rounded graph container
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
                    if (!ctx)
                        return
                    ctx.clearRect(0, 0, width, height)
                    ctx.globalAlpha = 1

                    var topH = root.paneH()
                    var magTop = root.plotTop
                    var phTop = magTop + topH + root.plotGap
                    var phTitle = unwrapCheck.checked ? I18n.tr("Phase (deg, unwrapped)") : I18n.tr("Phase (deg)")
                    root.paintPane(ctx, magTop, topH, root.dbMin, root.dbMax, "db", I18n.tr("Magnitude (dB)"))
                    root.paintPane(ctx, phTop, topH, root.phMin, root.phMax, root.phaseKey(), phTitle)
                    root.paintFreqLabels(ctx, phTop + topH)
                    root.paintCursors(ctx, magTop, topH, phTop, topH)
                    root.paintZoomBox(ctx, magTop, topH, phTop, topH)
                }

                MouseArea {
                    anchors.fill: parent
                    hoverEnabled: true
                    acceptedButtons: Qt.LeftButton | Qt.RightButton
                    cursorShape: Qt.CrossCursor

                    onPressed: function(mouse) {
                        if (mouse.button === Qt.RightButton) {
                            root.pinnedIdx = -1
                            if (root.hoverX >= 0)
                                root.updateHoverStatus(root.nearest(root.hoverX))
                            else
                                root.statusText = root.summaryText
                            plot.requestPaint()
                            return
                        }
                        if (!root.swept) return
                        if (mouse.x < root.plotLeft || mouse.x > plot.width - root.plotRight) return
                        root.dragStartX = mouse.x
                        root.dragStartY = mouse.y
                        root.dragCurrX = mouse.x
                        root.dragCurrY = mouse.y
                        root.draggingZoom = true
                    }

                    onPositionChanged: function(mouse) {
                        if (root.draggingZoom) {
                            root.dragCurrX = Math.max(root.plotLeft, Math.min(plot.width - root.plotRight, mouse.x))
                            root.dragCurrY = mouse.y
                            plot.requestPaint()
                            return
                        }
                        if (mouse.x < root.plotLeft || mouse.x > plot.width - root.plotRight) {
                            if (root.hoverX >= 0) {
                                root.hoverX = -1
                                root.statusText = root.summaryText
                                plot.requestPaint()
                            }
                            return
                        }
                        root.hoverX = mouse.x
                        root.updateHoverStatus(root.nearest(mouse.x))
                        plot.requestPaint()
                    }

                    onReleased: function(mouse) {
                        if (mouse.button !== Qt.LeftButton || !root.draggingZoom)
                            return
                        root.draggingZoom = false
                        var dx = Math.abs(root.dragCurrX - root.dragStartX)
                        var dy = Math.abs(root.dragCurrY - root.dragStartY)
                        if (dx > 10) {
                            var x0 = Math.min(root.dragStartX, root.dragCurrX)
                            var x1 = Math.max(root.dragStartX, root.dragCurrX)
                            var newLo = root.hzOfX(x0)
                            var newHi = root.hzOfX(x1)
                            if (newHi > newLo * 1.001) {
                                root.fLo = newLo
                                root.fHi = newHi
                                // If dragged vertically inside the magnitude pane, also zoom dB range
                                var topH = root.paneH()
                                var magTop = root.plotTop
                                if (dy > 14 && root.dragStartY >= magTop && root.dragStartY <= magTop + topH) {
                                    var y0 = Math.max(magTop, Math.min(root.dragStartY, root.dragCurrY))
                                    var y1 = Math.min(magTop + topH, Math.max(root.dragStartY, root.dragCurrY))
                                    var hiDb = root.valOfY(y0, magTop, topH, root.dbMin, root.dbMax)
                                    var loDb = root.valOfY(y1, magTop, topH, root.dbMin, root.dbMax)
                                    if (hiDb - loDb >= 1.0) {
                                        root.dbMin = loDb
                                        root.dbMax = hiDb
                                    }
                                }
                                root.isZoomed = true
                            }
                        } else {
                            // Short click: pin/toggle Reference Cursor C1
                            var idx = root.nearest(mouse.x)
                            root.pinnedIdx = (root.pinnedIdx === idx) ? -1 : idx
                            root.updateHoverStatus(idx)
                        }
                        plot.requestPaint()
                    }

                    onDoubleClicked: root.resetZoom()

                    onWheel: function(wheel) {
                        if (!root.swept) return
                        var dy = wheel.angleDelta.y !== 0 ? wheel.angleDelta.y : wheel.pixelDelta.y
                        if (dy === 0) return
                        var factor = dy > 0 ? 0.78 : 1.28
                        root.zoomFreqAt(factor, wheel.x)
                    }

                    onExited: {
                        if (!root.draggingZoom) {
                            root.hoverX = -1
                            root.statusText = root.summaryText
                            plot.requestPaint()
                        }
                    }
                }
            }
        }
    }

    function paintPane(ctx: var, top: real, h: real, vmin: real, vmax: real, key: string, unit: string): void {
        var innerW = root.innerW()
        ctx.strokeStyle = pal.mid
        ctx.lineWidth = 1
        ctx.strokeRect(root.plotLeft, top, innerW, h)

        // Horizontal gridlines & Y ticks
        ctx.fillStyle = pal.windowText
        ctx.font = App.fontSmall + "px \"" + App.fontFamily + "\""
        ctx.textAlign = "right"
        ctx.textBaseline = "middle"
        var isPhase = (key === "phase" || key === "phaseUnwrapped")
        var span = vmax - vmin
        var step = isPhase ? Math.max(45, Math.ceil((span / 4) / 45) * 45) : (span / 4)
        for (var v = vmin; v <= vmax + Math.abs(step) * 0.01; v += step) {
            if (step === 0)
                break
            var y = root.yOf(v, top, h, vmin, vmax)
            if (y < top - 1 || y > top + h + 1)
                continue
            ctx.strokeStyle = pal.mid
            ctx.globalAlpha = 0.32
            ctx.beginPath()
            ctx.moveTo(root.plotLeft, y)
            ctx.lineTo(root.plotLeft + innerW, y)
            ctx.stroke()
            ctx.globalAlpha = 1
            ctx.fillText(Math.abs(v) < 0.05 ? "0" : v.toFixed(0), root.plotLeft - 8, y)
            if (!isPhase && v > vmin + span)
                break
        }

        // Subtle 0 dB / 0 deg reference line
        if (vmin < 0 && vmax > 0) {
            var yZero = root.yOf(0, top, h, vmin, vmax)
            ctx.save()
            ctx.strokeStyle = pal.windowText
            ctx.globalAlpha = 0.25
            ctx.setLineDash([4, 4])
            ctx.beginPath()
            ctx.moveTo(root.plotLeft, yZero)
            ctx.lineTo(root.plotLeft + innerW, yZero)
            ctx.stroke()
            ctx.restore()
        }

        // Minor log sub-decade gridlines
        if (root.fHi > root.fLo && root.fLo > 0) {
            var expStart = Math.floor(Math.log10(root.fLo))
            var expEnd = Math.ceil(Math.log10(root.fHi))
            if (expEnd - expStart <= 12) {
                ctx.strokeStyle = pal.mid
                ctx.globalAlpha = 0.14
                for (var e = expStart; e <= expEnd; e++) {
                    var decBase = Math.pow(10, e)
                    for (var m = 2; m <= 9; m++) {
                        var mf = decBase * m
                        if (mf <= root.fLo || mf >= root.fHi)
                            continue
                        var mx = root.xOf(mf)
                        ctx.beginPath()
                        ctx.moveTo(mx, top)
                        ctx.lineTo(mx, top + h)
                        ctx.stroke()
                    }
                }
            }
        }

        // Major decade gridlines
        var decades = root.decadeList()
        ctx.strokeStyle = pal.mid
        ctx.globalAlpha = 0.35
        for (var d = 0; d < decades.length; d++) {
            var x = root.xOf(decades[d])
            ctx.beginPath()
            ctx.moveTo(x, top)
            ctx.lineTo(x, top + h)
            ctx.stroke()
        }
        ctx.globalAlpha = 1

        // Pane title
        ctx.fillStyle = pal.windowText
        ctx.globalAlpha = 0.75
        ctx.textAlign = "left"
        ctx.textBaseline = "top"
        ctx.fillText(unit, root.plotLeft + 8, top + 6)
        ctx.globalAlpha = 1

        if (!root.traces || root.traces.length === 0)
            return

        // Clipped trace rendering
        ctx.save()
        ctx.beginPath()
        ctx.rect(root.plotLeft, top, innerW, h)
        ctx.clip()

        for (var t = 0; t < root.traces.length; t++) {
            var pts = root.traces[t].points
            if (!pts || pts.length === 0) continue
            var traceCol = root.traces.length === 1
                ? (isPhase ? root.phaseColor : root.magColor)
                : root.getTraceColor(t)
            ctx.beginPath()
            var prevPhase = 0
            for (var i = 0; i < pts.length; i++) {
                var p = pts[i]
                var px = root.xOf(p.hz)
                var py = root.yOf(p[key], top, h, vmin, vmax)
                if (i === 0 || (key === "phase" && Math.abs(p.phase - prevPhase) > 180))
                    ctx.moveTo(px, py)
                else
                    ctx.lineTo(px, py)
                prevPhase = p.phase
            }
            ctx.strokeStyle = traceCol
            ctx.lineWidth = 2
            ctx.lineCap = "round"
            ctx.lineJoin = "round"
            ctx.stroke()
        }
        ctx.restore()
        ctx.lineWidth = 1
    }

    function decadeList(): var {
        if (!(root.fHi > root.fLo) || root.fLo <= 0)
            return []
        var a = Math.ceil(Math.log10(root.fLo) - 1e-9)
        var b = Math.floor(Math.log10(root.fHi) + 1e-9)
        var out = []
        for (var e = a; e <= b && out.length < 12; e++)
            out.push(Math.pow(10, e))
        return out
    }

    function paintFreqLabels(ctx: var, axisY: real): void {
        ctx.fillStyle = pal.windowText
        ctx.font = App.fontSmall + "px \"" + App.fontFamily + "\""
        ctx.textAlign = "center"
        ctx.textBaseline = "top"
        var decades = root.decadeList()
        if (decades.length > 0) {
            for (var i = 0; i < decades.length; i++) {
                var x = root.xOf(decades[i])
                ctx.fillText(root.fmtHz(decades[i]), x, axisY + 6)
            }
        } else if (root.fHi > root.fLo) {
            // Zoomed inside less than a decade: show start, mid, end labels
            var fMid = Math.sqrt(root.fLo * root.fHi)
            ctx.fillText(root.fmtHz(root.fLo), root.plotLeft, axisY + 6)
            ctx.fillText(root.fmtHz(fMid), root.plotLeft + root.innerW() / 2, axisY + 6)
            ctx.fillText(root.fmtHz(root.fHi), root.plotLeft + root.innerW(), axisY + 6)
        }
    }

    function paintCursorLine(ctx: var, idx: int, magTop: real, magH: real, phTop: real, phH: real, dashed: bool, label: string): void {
        if (idx < 0 || !root.traces || root.traces.length === 0)
            return
        var pts0 = root.traces[0].points
        if (!pts0 || idx >= pts0.length) return
        var hz = pts0[idx].hz
        if (hz < root.fLo || hz > root.fHi) return
        var x = root.xOf(hz)
        var pKey = root.phaseKey()

        ctx.save()
        ctx.strokeStyle = dashed ? pal.highlight : root.phaseColor
        ctx.globalAlpha = dashed ? 0.6 : 0.85
        if (dashed) ctx.setLineDash([4, 4])
        ctx.beginPath()
        ctx.moveTo(x, magTop)
        ctx.lineTo(x, magTop + magH)
        ctx.moveTo(x, phTop)
        ctx.lineTo(x, phTop + phH)
        ctx.stroke()
        ctx.setLineDash([])
        ctx.globalAlpha = 1

        if (label) {
            ctx.fillStyle = dashed ? pal.highlight : root.phaseColor
            ctx.font = App.fontMicro + "px \"" + App.fontFamily + "\""
            ctx.textAlign = "left"
            ctx.textBaseline = "top"
            ctx.fillText(label, x + 4, magTop + 4)
        }

        for (var t = 0; t < root.traces.length; t++) {
            var p = root.traces[t].points[idx]
            var mCol = root.traces.length === 1 ? root.magColor : root.getTraceColor(t)
            var pCol = root.traces.length === 1 ? root.phaseColor : root.getTraceColor(t)

            var magY = root.yOf(p.db, magTop, magH, root.dbMin, root.dbMax)
            if (magY >= magTop && magY <= magTop + magH) {
                ctx.fillStyle = mCol
                ctx.beginPath()
                ctx.arc(x, magY, 4.5, 0, 2 * Math.PI)
                ctx.fill()
                ctx.strokeStyle = pal.base
                ctx.lineWidth = 1.5
                ctx.stroke()
            }

            var phY = root.yOf(p[pKey], phTop, phH, root.phMin, root.phMax)
            if (phY >= phTop && phY <= phTop + phH) {
                ctx.fillStyle = pCol
                ctx.beginPath()
                ctx.arc(x, phY, 4.5, 0, 2 * Math.PI)
                ctx.fill()
                ctx.strokeStyle = pal.base
                ctx.lineWidth = 1.5
                ctx.stroke()
            }
        }
        ctx.restore()
    }

    function paintCursors(ctx: var, magTop: real, magH: real, phTop: real, phH: real): void {
        if (root.pinnedIdx >= 0) {
            root.paintCursorLine(ctx, root.pinnedIdx, magTop, magH, phTop, phH, false, "C1")
        }
        if (root.hoverX >= 0 && !root.draggingZoom) {
            var hi = root.nearest(root.hoverX)
            if (hi >= 0)
                root.paintCursorLine(ctx, hi, magTop, magH, phTop, phH, true, root.pinnedIdx >= 0 ? "C2" : "")
        }
    }

    function paintZoomBox(ctx: var, magTop: real, magH: real, phTop: real, phH: real): void {
        if (!root.draggingZoom) return
        var dx = Math.abs(root.dragCurrX - root.dragStartX)
        if (dx <= 10) return
        var x0 = Math.min(root.dragStartX, root.dragCurrX)
        var w = dx
        ctx.save()
        ctx.fillStyle = pal.highlight
        ctx.globalAlpha = 0.14
        ctx.fillRect(x0, magTop, w, magH)
        ctx.fillRect(x0, phTop, w, phH)
        ctx.strokeStyle = pal.highlight
        ctx.globalAlpha = 0.7
        ctx.lineWidth = 1
        ctx.strokeRect(x0, magTop, w, magH)
        ctx.strokeRect(x0, phTop, w, phH)
        ctx.restore()
    }
}
