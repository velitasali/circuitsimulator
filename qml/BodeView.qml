import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import cs_app

// Small-signal magnitude and phase. The sweep runs on a copy of the schematic.
Rectangle {
    id: root
    function tr(s) { return App.i18nTick >= 0 ? App.translate(s) : s }

    color: pal.window
    SystemPalette { id: pal; colorGroup: SystemPalette.Active }

    property var points: []
    property bool swept: false
    property real fLo: 10
    property real fHi: 100000
    property real dbMin: -40
    property real dbMax: 10
    property real hoverFrac: -1
    property string statusText: ""

    readonly property int plotLeft: 56
    readonly property int plotRight: 12
    readonly property int plotTop: 10
    readonly property int plotBottom: 26
    readonly property int plotGap: 16

    function reloadEnds() {
        var sources = CircuitCanvas.acSources()
        var probes = CircuitCanvas.acProbes()
        var keepS = sourceBox.currentText
        var keepP = probeBox.currentText
        sourceBox.model = sources ? sources : []
        probeBox.model = probes ? probes : []
        var si = root.indexOfText(sources, keepS)
        var pi = root.indexOfText(probes, keepP)
        if (si >= 0)
            sourceBox.currentIndex = si
        if (pi >= 0)
            probeBox.currentIndex = pi
        if (root.swept) {
            root.swept = false
            root.points = []
            root.statusText = root.tr("Circuit changed. Run the sweep again.")
            plot.requestPaint()
        }
    }

    function indexOfText(model, text) {
        if (!model || !text)
            return -1
        for (var i = 0; i < model.length; i++) {
            if (model[i] === text)
                return i
        }
        return -1
    }

    function readNumber(field, fallback) {
        var v = Number(field.text)
        return isFinite(v) ? v : fallback
    }

    function runSweep() {
        var source = sourceBox.currentText
        var probe = probeBox.currentText
        if (!source || !probe) {
            root.statusText = root.tr("Choose a voltage source and a probe pin.")
            return
        }
        var f0 = root.readNumber(startField, 10)
        var f1 = root.readNumber(stopField, 100000)
        var ppd = Math.round(root.readNumber(ppdField, 20))
        var res = CircuitCanvas.acSweep(source, probe, f0, f1, ppd)
        if (!res || res.error) {
            root.points = []
            root.swept = false
            root.statusText = res && res.error ? res.error : root.tr("AC sweep failed.")
            plot.requestPaint()
            return
        }
        root.points = res.points ? res.points : []
        root.swept = root.points.length > 0
        if (root.points.length > 0) {
            root.fLo = root.points[0].hz
            root.fHi = root.points[root.points.length - 1].hz
            root.fitDb()
            root.statusText = root.tr("Move across the plot to read a point.")
        } else {
            root.statusText = root.tr("The sweep returned no points.")
        }
        plot.requestPaint()
    }

    function fitDb() {
        var lo = Infinity
        var hi = -Infinity
        for (var i = 0; i < root.points.length; i++) {
            var db = root.points[i].db
            if (db < lo) lo = db
            if (db > hi) hi = db
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

    function innerW() {
        return Math.max(1, plot.width - root.plotLeft - root.plotRight)
    }

    function paneH() {
        var h = plot.height - root.plotTop - root.plotBottom - root.plotGap
        return Math.max(1, h / 2)
    }

    function xOf(hz) {
        var log0 = Math.log10(root.fLo)
        var log1 = Math.log10(root.fHi)
        var span = log1 - log0
        var t = span > 0 ? (Math.log10(hz) - log0) / span : 0
        return root.plotLeft + t * root.innerW()
    }

    function yOf(v, top, h, vmin, vmax) {
        var span = vmax - vmin
        var t = span !== 0 ? (v - vmin) / span : 0.5
        return top + h - t * h
    }

    function fmtHz(hz) {
        if (hz >= 1e6)
            return (hz / 1e6).toFixed(hz >= 1e7 ? 0 : 2) + " MHz"
        if (hz >= 1e3)
            return (hz / 1e3).toFixed(hz >= 1e4 ? 0 : 2) + " kHz"
        if (hz >= 10)
            return hz.toFixed(1) + " Hz"
        return hz.toFixed(2) + " Hz"
    }

    function nearest(x) {
        if (!root.points || root.points.length === 0)
            return -1
        var best = 0
        var bestD = 1e9
        for (var i = 0; i < root.points.length; i++) {
            var d = Math.abs(root.xOf(root.points[i].hz) - x)
            if (d < bestD) {
                bestD = d
                best = i
            }
        }
        return best
    }

    Component.onCompleted: root.reloadEnds()

    Connections {
        target: CircuitCanvas
        function onItemsChanged() { root.reloadEnds() }
        function onWiresChanged() { root.reloadEnds() }
        function onRootCircuitLoaded() { root.reloadEnds() }
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 12
        spacing: 8

        RowLayout {
            spacing: 8

            AppText {
                text: root.tr("Source")
                color: pal.windowText
                font.pixelSize: App.fontSmall
            }
            AppComboBox {
                id: sourceBox
                Layout.preferredWidth: 140
                model: []
            }
            AppText {
                text: root.tr("Probe")
                color: pal.windowText
                font.pixelSize: App.fontSmall
            }
            AppComboBox {
                id: probeBox
                Layout.fillWidth: true
                Layout.minimumWidth: 140
                model: []
            }
            AppText {
                text: root.tr("Start Hz")
                color: pal.windowText
                font.pixelSize: App.fontSmall
            }
            AppTextField {
                id: startField
                Layout.preferredWidth: 72
                text: "10"
            }
            AppText {
                text: root.tr("Stop Hz")
                color: pal.windowText
                font.pixelSize: App.fontSmall
            }
            AppTextField {
                id: stopField
                Layout.preferredWidth: 80
                text: "100000"
            }
            AppText {
                text: root.tr("Per decade")
                color: pal.windowText
                font.pixelSize: App.fontSmall
            }
            AppTextField {
                id: ppdField
                Layout.preferredWidth: 40
                text: "20"
            }
            AppButton {
                text: root.tr("Run")
                onClicked: root.runSweep()
            }
        }

        RowLayout {
            spacing: 12

            Rectangle {
                width: 14
                height: 3
                color: pal.highlight
            }
            AppText {
                text: root.tr("Magnitude")
                color: pal.windowText
                font.pixelSize: App.fontSmall
            }
            Rectangle {
                width: 14
                height: 3
                color: pal.windowText
            }
            AppText {
                text: root.tr("Phase")
                color: pal.windowText
                font.pixelSize: App.fontSmall
            }
            AppText {
                Layout.fillWidth: true
                text: root.statusText
                color: pal.windowText
                font.pixelSize: App.fontSmall
                elide: Text.ElideRight
            }
        }

        Canvas {
            id: plot
            Layout.fillWidth: true
            Layout.fillHeight: true
            contextType: "2d"

            onWidthChanged: requestPaint()
            onHeightChanged: requestPaint()
            onPaint: {
                var ctx = getContext("2d")
                if (!ctx)
                    return
                ctx.globalAlpha = 1
                ctx.fillStyle = pal.base
                ctx.fillRect(0, 0, width, height)

                var topH = root.paneH()
                var magTop = root.plotTop
                var phTop = magTop + topH + root.plotGap
                root.paintPane(ctx, magTop, topH, root.dbMin, root.dbMax, "db", pal.highlight, "dB")
                root.paintPane(ctx, phTop, topH, -180, 180, "phase", pal.windowText, "deg")
                root.paintFreqLabels(ctx, phTop + topH)
                root.paintHover(ctx, magTop, topH, phTop, topH)
            }

            MouseArea {
                anchors.fill: parent
                hoverEnabled: true
                onPositionChanged: function(mouse) {
                    root.hoverFrac = mouse.x
                    var i = root.nearest(mouse.x)
                    if (i >= 0) {
                        var p = root.points[i]
                        root.statusText = root.fmtHz(p.hz)
                            + "    " + Number(p.db).toFixed(2) + " dB"
                            + "    " + Number(p.phase).toFixed(1) + "\u00B0"
                            + "    " + Number(p.mag).toFixed(4) + " V"
                    }
                    plot.requestPaint()
                }
                onExited: {
                    root.hoverFrac = -1
                    if (root.swept)
                        root.statusText = root.tr("Move across the plot to read a point.")
                    plot.requestPaint()
                }
            }
        }
    }

    function paintPane(ctx, top, h, vmin, vmax, key, color, unit) {
        ctx.strokeStyle = pal.mid
        ctx.lineWidth = 1
        ctx.strokeRect(root.plotLeft, top, root.innerW(), h)
        ctx.fillStyle = pal.windowText
        ctx.font = App.fontSmall + "px \"" + App.fontFamily + "\""
        ctx.textAlign = "right"
        ctx.textBaseline = "middle"
        var stepCount = 4
        var span = vmax - vmin
        var step = span / stepCount
        if (key === "phase")
            step = 90
        for (var v = vmin; v <= vmax + Math.abs(step) * 0.01; v += step) {
            if (step === 0)
                break
            var y = root.yOf(v, top, h, vmin, vmax)
            if (y < top - 1 || y > top + h + 1)
                continue
            ctx.globalAlpha = 0.35
            ctx.beginPath()
            ctx.moveTo(root.plotLeft, y)
            ctx.lineTo(root.plotLeft + root.innerW(), y)
            ctx.stroke()
            ctx.globalAlpha = 1
            ctx.fillText(Math.abs(v) < 0.05 ? "0" : v.toFixed(0), root.plotLeft - 6, y)
            if (key !== "phase" && v > vmin + span)
                break
        }
        ctx.textAlign = "left"
        ctx.textBaseline = "top"
        ctx.fillText(unit, root.plotLeft + 4, top + 2)

        var decades = root.decadeList()
        ctx.globalAlpha = 0.35
        for (var d = 0; d < decades.length; d++) {
            var x = root.xOf(decades[d])
            ctx.beginPath()
            ctx.moveTo(x, top)
            ctx.lineTo(x, top + h)
            ctx.stroke()
        }
        ctx.globalAlpha = 1

        if (!root.points || root.points.length === 0)
            return
        ctx.beginPath()
        for (var i = 0; i < root.points.length; i++) {
            var p = root.points[i]
            var px = root.xOf(p.hz)
            var py = root.yOf(p[key], top, h, vmin, vmax)
            if (i === 0)
                ctx.moveTo(px, py)
            else
                ctx.lineTo(px, py)
        }
        ctx.strokeStyle = color
        ctx.lineWidth = 1.75
        ctx.stroke()
        ctx.lineWidth = 1
    }

    function decadeList() {
        if (!(root.fHi > root.fLo) || root.fLo <= 0)
            return []
        var a = Math.ceil(Math.log10(root.fLo) - 1e-9)
        var b = Math.floor(Math.log10(root.fHi) + 1e-9)
        var out = []
        for (var e = a; e <= b && out.length < 12; e++)
            out.push(Math.pow(10, e))
        return out
    }

    function paintFreqLabels(ctx, axisY) {
        ctx.fillStyle = pal.windowText
        ctx.font = App.fontSmall + "px \"" + App.fontFamily + "\""
        ctx.textAlign = "center"
        ctx.textBaseline = "top"
        var decades = root.decadeList()
        for (var i = 0; i < decades.length; i++) {
            var x = root.xOf(decades[i])
            ctx.fillText(root.fmtHz(decades[i]), x, axisY + 4)
        }
    }

    function paintHover(ctx, magTop, magH, phTop, phH) {
        if (root.hoverFrac < 0 || !root.points || root.points.length === 0)
            return
        var i = root.nearest(root.hoverFrac)
        if (i < 0)
            return
        var x = root.xOf(root.points[i].hz)
        ctx.strokeStyle = pal.highlight
        ctx.globalAlpha = 0.55
        ctx.beginPath()
        ctx.moveTo(x, magTop)
        ctx.lineTo(x, magTop + magH)
        ctx.moveTo(x, phTop)
        ctx.lineTo(x, phTop + phH)
        ctx.stroke()
        ctx.globalAlpha = 1
    }
}
