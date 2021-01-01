import QtQuick
import cs_app

/* QML Canvas stand-in for PlotDisplay (QQuickPaintedItem). Supports multi-track oscilloscope, resolution scaling, and expanded measurements. */
Item {
    id: root
    property var traces: []
    property var hidden: []
    property bool digital: false
    property color paper: "#000000"
    property real radius: 4.0
    property int tracks: 1
    property var chVoltDiv: [ 1.0, 1.0, 1.0, 1.0 ]
    property var chVoltPos: [ 0.0, 0.0, 0.0, 0.0 ]
    property var chTimePos: [ 0.0, 0.0, 0.0, 0.0 ]
    property real voltDiv: 1.0
    property real voltPos: 0.0
    property real timePos: 0.0
    property real zoomFactor: 1.0
    property bool expanded: false
    property real cursorX: -1
    property real cursorY: -1
    property real timeDiv: 1e-3

    readonly property real effZoom: Math.min(8.0, Math.max(1.0, zoomFactor))
    readonly property color gridColor: "#3c444d"
    readonly property color accentGridColor: "#78828e"
    readonly property color cursorColor: "#c8c8c8"
    readonly property bool cursorVisible: expanded && !digital && cursorX >= 0 && cursorX <= width && cursorY >= 0 && cursorY <= height

    function requestPaint() {
        if (canvas) canvas.requestPaint()
    }

    function formatEngTime(val) {
        var sign = val < 0 ? "-" : "+"
        var abs = Math.abs(val)
        if (abs < 1e-9) return "0.00 s"
        if (abs < 1e-6) return sign + (abs * 1e9).toFixed(1) + " ns"
        if (abs < 1e-3) return sign + (abs * 1e6).toFixed(1) + " µs"
        if (abs < 1.0) return sign + (abs * 1e3).toFixed(2) + " ms"
        return sign + abs.toFixed(2) + " s"
    }

    function cursorDeltaTime() {
        var w = width
        if (w <= 0)
            return 0
        var timeSpan = (timeDiv || 1e-3) * 10.0
        return ((cursorX - w / 2) / w) * timeSpan
    }

    function cursorChannelVoltage(chIdx) {
        if (hidden && hidden[chIdx] === true)
            return null
        if (!traces || !traces[chIdx])
            return null
        var ch = traces[chIdx]
        if (!ch || ch.connected === false)
            return null
        var samples = ch.samples
        if (!samples || samples.length < 1)
            return null
        var sIdx = Math.round((cursorX / Math.max(width, 1)) * (samples.length - 1))
        if (sIdx < 0 || sIdx >= samples.length)
            return null
        return samples[sIdx]
    }

    function channelColor(chIdx) {
        if (!traces || !traces[chIdx] || !traces[chIdx].color)
            return "#ffffff"
        return traces[chIdx].color
    }

    Connections {
        target: CircuitCanvas
        function onAppearanceChanged() { root.requestPaint() }
    }

    onTracesChanged: requestPaint()
    onHiddenChanged: requestPaint()
    onTracksChanged: requestPaint()
    onChVoltDivChanged: requestPaint()
    onChVoltPosChanged: requestPaint()
    onChTimePosChanged: requestPaint()
    onVoltDivChanged: requestPaint()
    onVoltPosChanged: requestPaint()
    onTimePosChanged: requestPaint()
    onPaperChanged: requestPaint()
    onRadiusChanged: requestPaint()
    onEffZoomChanged: requestPaint()
    onExpandedChanged: requestPaint()
    onTimeDivChanged: requestPaint()

    Canvas {
        id: canvas
        width: Math.max(1, Math.ceil(root.width * root.effZoom))
        height: Math.max(1, Math.ceil(root.height * root.effZoom))
        scale: 1.0 / root.effZoom
        transformOrigin: Item.TopLeft
        renderTarget: Canvas.Image
        renderStrategy: Canvas.Immediate

        onWidthChanged: requestPaint()
        onHeightChanged: requestPaint()
        onAvailableChanged: { if (available) requestPaint() }
        Component.onCompleted: requestPaint()

        function drawRoundedRect(ctx, x, y, w, h, r) {
            if (r <= 0) {
                ctx.rect(x, y, w, h)
                return
            }
            ctx.beginPath()
            ctx.moveTo(x + r, y)
            ctx.lineTo(x + w - r, y)
            ctx.arcTo(x + w, y, x + w, y + r, r)
            ctx.lineTo(x + w, y + h - r)
            ctx.arcTo(x + w, y + h, x + w - r, y + h, r)
            ctx.lineTo(x + r, y + h)
            ctx.arcTo(x, y + h, x, y + h - r, r)
            ctx.lineTo(x, y + r)
            ctx.arcTo(x, y, x + r, y, r)
            ctx.closePath()
        }

        function strokeHLines(ctx, ys, x0, x1) {
            if (!ys.length)
                return
            ctx.beginPath()
            for (var i = 0; i < ys.length; ++i) {
                ctx.moveTo(x0, ys[i])
                ctx.lineTo(x1, ys[i])
            }
            ctx.stroke()
        }

        function strokeVLines(ctx, xs, y0, y1) {
            if (!xs.length)
                return
            ctx.beginPath()
            for (var i = 0; i < xs.length; ++i) {
                ctx.moveTo(xs[i], y0)
                ctx.lineTo(xs[i], y1)
            }
            ctx.stroke()
        }

        onPaint: {
            var ctx = getContext( "2d" )
            ctx.resetTransform()
            ctx.scale( root.effZoom, root.effZoom )

            var w = root.width
            var h = root.height
            if ( w <= 0 || h <= 0 )
                return

            ctx.fillStyle = root.paper
            drawRoundedRect(ctx, 0, 0, w, h, root.radius)
            ctx.fill()

            ctx.save()
            if (root.radius > 0) {
                drawRoundedRect(ctx, 0, 0, w, h, root.radius)
                ctx.clip()
            }

            if ( root.digital ) {
                ctx.strokeStyle = root.gridColor
                ctx.lineWidth = 1
                var hy = []
                for ( var g = 1; g < 8; ++g )
                    hy.push( h * g / 8 )
                strokeHLines( ctx, hy, 0, w )
                var vx = []
                for ( var gx = 1; gx < 10; ++gx )
                    vx.push( w * gx / 10 )
                strokeVLines( ctx, vx, 0, h )

                if ( root.traces && root.traces.length > 0 ) {
                    for ( var c = 0; c < root.traces.length; ++c ) {
                        if ( root.hidden && root.hidden[c] === true )
                            continue
                        var ch = root.traces[c]
                        if ( !ch || ch.connected === false )
                            continue
                        var samples = ch.samples
                        if ( !samples || samples.length < 2 )
                            continue
                        var rowH = h / 8
                        var rowY = c * rowH
                        ctx.strokeStyle = ch.color
                        ctx.lineWidth = 1.5
                        ctx.beginPath()
                        var last = samples.length - 1
                        for ( var i = 0; i < samples.length; ++i ) {
                            var px = ( i / last ) * w
                            var v = samples[i]
                            var py = rowY + rowH - 3 - v * ( rowH - 6 )
                            if ( i === 0 ) ctx.moveTo( px, py )
                            else ctx.lineTo( px, py )
                        }
                        ctx.stroke()
                    }
                }
            } else {
                var numTracks = Math.max( 1, Math.min( 4, root.tracks || 1 ) )
                var vDivs = 10 * numTracks
                var hCenter = w / 2

                var trackCenters = []
                for ( var t = 0; t < numTracks; ++t )
                    trackCenters.push( ( t + 0.5 ) * h / numTracks )

                var gridH = []
                var accentH = []
                for ( var gy = 1; gy < vDivs; ++gy ) {
                    var yPos = h * gy / vDivs
                    var isTrackCenter = false
                    for ( var tc = 0; tc < trackCenters.length; ++tc ) {
                        if ( Math.abs( yPos - trackCenters[tc] ) < 0.5 ) {
                            isTrackCenter = true
                            break
                        }
                    }
                    if ( isTrackCenter )
                        accentH.push( yPos )
                    else
                        gridH.push( yPos )
                }
                ctx.strokeStyle = root.gridColor
                ctx.lineWidth = 0.75
                strokeHLines( ctx, gridH, 0, w )
                ctx.strokeStyle = root.accentGridColor
                ctx.lineWidth = 1.2
                strokeHLines( ctx, accentH, 0, w )

                var gridV = []
                var accentV = []
                for ( var gx = 1; gx < 10; ++gx ) {
                    var xPos = w * gx / 10
                    if ( Math.abs( xPos - hCenter ) < 0.5 )
                        accentV.push( xPos )
                    else
                        gridV.push( xPos )
                }
                ctx.strokeStyle = root.gridColor
                ctx.lineWidth = 0.75
                strokeVLines( ctx, gridV, 0, h )
                ctx.strokeStyle = root.accentGridColor
                ctx.lineWidth = 1.2
                strokeVLines( ctx, accentV, 0, h )

                ctx.strokeStyle = root.accentGridColor
                ctx.lineWidth = 0.75
                ctx.beginPath()
                for ( var t = 0; t < numTracks; ++t ) {
                    var cy = trackCenters[t]
                    for ( var tx = 0; tx <= 50; ++tx ) {
                        var tickX = w * tx / 50
                        ctx.moveTo( tickX, cy - 1.5 )
                        ctx.lineTo( tickX, cy + 1.5 )
                    }
                }
                var ym = numTracks === 1 ? 50 : ( numTracks === 2 ? 40 : 20 )
                for ( var ty = 0; ty <= ym; ++ty ) {
                    var tickY = h * ty / ym
                    ctx.moveTo( hCenter - 1.5, tickY )
                    ctx.lineTo( hCenter + 1.5, tickY )
                }
                ctx.stroke()

                if ( root.expanded ) {
                    ctx.strokeStyle = root.cursorColor
                    ctx.lineWidth = 1.5
                    ctx.beginPath()
                    ctx.moveTo( hCenter, 0 )
                    ctx.lineTo( hCenter, h )
                    ctx.stroke()
                }

                if ( root.traces && root.traces.length > 0 ) {
                    for ( var c = 0; c < root.traces.length; ++c ) {
                        if ( root.hidden && root.hidden[c] === true )
                            continue
                        var ch = root.traces[c]
                        if ( !ch || ch.connected === false )
                            continue
                        var samples = ch.samples
                        if ( !samples || samples.length < 2 )
                            continue

                        var trackIdx = c % numTracks
                        var trackH = h / numTracks
                        var trackCenterY = ( trackIdx + 0.5 ) * trackH

                        var vd = ( root.chVoltDiv && root.chVoltDiv[c] !== undefined ) ? root.chVoltDiv[c] : ( root.voltDiv || 1.0 )
                        var vp = ( root.chVoltPos && root.chVoltPos[c] !== undefined ) ? root.chVoltPos[c] : ( root.voltPos || 0.0 )
                        var span = Math.max( vd * 10, 1e-12 )
                        var scaleY = trackH / span

                        var minV = 1e12
                        var maxV = -1e12

                        ctx.strokeStyle = ch.color
                        ctx.lineWidth = 1.5
                        ctx.beginPath()
                        var last = samples.length - 1
                        for ( var i = 0; i < samples.length; ++i ) {
                            var px = ( i / last ) * w
                            var v = samples[i]
                            if ( v < minV ) minV = v
                            if ( v > maxV ) maxV = v
                            var py = trackCenterY - ( v - vp ) * scaleY
                            if ( i === 0 ) ctx.moveTo( px, py )
                            else ctx.lineTo( px, py )
                        }
                        ctx.stroke()

                        if ( root.expanded && minV <= maxV ) {
                            var vMaxY = trackCenterY - ( maxV - vp ) * scaleY
                            var vMinY = trackCenterY - ( minV - vp ) * scaleY

                            ctx.strokeStyle = ch.color
                            ctx.lineWidth = 0.5
                            if ( ctx.setLineDash ) ctx.setLineDash( [ 4, 4 ] )
                            ctx.beginPath()
                            if ( vMaxY >= 0 && vMaxY <= h ) {
                                ctx.moveTo( 0, vMaxY )
                                ctx.lineTo( w, vMaxY )
                            }
                            if ( vMinY >= 0 && vMinY <= h ) {
                                ctx.moveTo( 0, vMinY )
                                ctx.lineTo( w, vMinY )
                            }
                            ctx.stroke()
                            if ( ctx.setLineDash ) ctx.setLineDash( [] )

                            ctx.font = "bold 9px sans-serif"
                            ctx.fillStyle = ch.color
                            var xPos = ( c % 2 === 0 ) ? 4 : ( w - 50 )
                            if ( vMaxY >= 0 && vMaxY <= h )
                                ctx.fillText( maxV.toFixed( 2 ) + "V", xPos, Math.max( 10, vMaxY - 3 ) )
                            if ( vMinY >= 0 && vMinY <= h )
                                ctx.fillText( minV.toFixed( 2 ) + "V", xPos, Math.min( h - 4, vMinY + 11 ) )
                        }
                    }
                }
            }

            ctx.restore()
        }
    }

    Item {
        id: cursorOverlay
        anchors.fill: parent
        visible: root.cursorVisible
        z: 1

        Rectangle {
            width: 1
            height: parent.height
            x: Math.round(root.cursorX)
            color: root.cursorColor
            opacity: 0.85
        }
        Rectangle {
            width: parent.width
            height: 1
            y: Math.round(root.cursorY)
            color: root.cursorColor
            opacity: 0.85
        }

        Rectangle {
            id: measureBox
            x: Math.min(parent.width - width - 6, Math.max(6, root.cursorX + 8))
            y: Math.min(parent.height - height - 6, Math.max(6, root.cursorY - 8))
            width: measureCol.implicitWidth + 12
            height: measureCol.implicitHeight + 8
            radius: 3
            color: Qt.rgba(10 / 255, 14 / 255, 18 / 255, 0.85)
            border.color: "#404850"
            border.width: 1

            Column {
                id: measureCol
                anchors.centerIn: parent
                spacing: 1

                Text {
                    text: "Δt: " + root.formatEngTime(root.cursorDeltaTime())
                    color: "#ffffff"
                    font.bold: true
                    font.pixelSize: 9
                }
                Text {
                    visible: root.cursorChannelVoltage(0) !== null
                    text: "Ch 1: " + Number(root.cursorChannelVoltage(0) || 0).toFixed(2) + "V"
                    color: root.channelColor(0)
                    font.bold: true
                    font.pixelSize: 9
                }
                Text {
                    visible: root.cursorChannelVoltage(1) !== null
                    text: "Ch 2: " + Number(root.cursorChannelVoltage(1) || 0).toFixed(2) + "V"
                    color: root.channelColor(1)
                    font.bold: true
                    font.pixelSize: 9
                }
                Text {
                    visible: root.cursorChannelVoltage(2) !== null
                    text: "Ch 3: " + Number(root.cursorChannelVoltage(2) || 0).toFixed(2) + "V"
                    color: root.channelColor(2)
                    font.bold: true
                    font.pixelSize: 9
                }
                Text {
                    visible: root.cursorChannelVoltage(3) !== null
                    text: "Ch 4: " + Number(root.cursorChannelVoltage(3) || 0).toFixed(2) + "V"
                    color: root.channelColor(3)
                    font.bold: true
                    font.pixelSize: 9
                }
            }
        }
    }
}
