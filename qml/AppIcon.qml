import QtQuick
import QtQuick.Controls.impl

Item {
    id: root

    property string icon: ""
    property real size: 16
    property color color: "#ffffff"

    implicitWidth: size
    implicitHeight: size

    ColorImage {
        anchors.centerIn: parent
        width: root.size
        height: root.size
        source: root.icon !== "" ? ("qrc:/icons/ui/" + root.icon + ".svg") : ""
        sourceSize: Qt.size(Math.ceil(root.size), Math.ceil(root.size))
        color: root.color
        fillMode: Image.PreserveAspectFit
        smooth: true
    }
}
