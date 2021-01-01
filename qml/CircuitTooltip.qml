import QtQuick
import cs_app

Item {
    id: root

    readonly property int maxTextWidth: 344
    readonly property int padX: 8
    readonly property int padY: 6

    SystemPalette { id: appTheme }

    visible: CircuitCanvas.hoverVisible && CircuitCanvas.hoverHtml.length > 0
    x: Math.min(parent.width - width - 10, Math.max(10, CircuitCanvas.hoverX + 14))
    y: Math.min(parent.height - height - 10, Math.max(10, CircuitCanvas.hoverY + 14))
    z: 1000

    implicitWidth: label.implicitWidth + 2 * padX + 2
    implicitHeight: label.implicitHeight + 2 * padY + 2
    width: Math.min(implicitWidth, root.maxTextWidth + 2 * padX + 2)
    height: implicitHeight

    Rectangle {
        anchors.fill: parent
        color: appTheme.base
        opacity: 0.95
        border.color: appTheme.mid
        border.width: 1
        radius: 6
    }

    Text {
        id: label
        x: root.padX
        y: root.padY
        width: Math.min(implicitWidth, root.maxTextWidth)

        text: CircuitCanvas.hoverHtml
        textFormat: Text.RichText
        wrapMode: Text.WordWrap
        font.family: App.fontFamily
        font.pixelSize: 12
        color: appTheme.text
        horizontalAlignment: Text.AlignLeft
        verticalAlignment: Text.AlignTop
    }
}
