import QtQuick
import QtQuick.Controls.impl

Item {
    id: root

    property string icon: ""
    property real size: 16
    property color color: "#ffffff"

    // Snap the rasterized SVG to an integer multiple of its 16x16 grid so
    // non-16 container sizes never introduce fractional-pixel blur.
    readonly property int renderSize: Math.max(16, Math.round(size / 16) * 16)

    implicitWidth: size
    implicitHeight: size

    ColorImage {
        anchors.centerIn: parent
        width: root.renderSize
        height: root.renderSize
        source: root.icon !== "" ? ("qrc:/icons/ui/" + root.icon + ".svg") : ""
        sourceSize: Qt.size(root.renderSize, root.renderSize)
        color: root.color
        fillMode: Image.PreserveAspectFit
        smooth: true
    }
}
