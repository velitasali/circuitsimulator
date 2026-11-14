import QtQuick

// `overhang` hangs the grab outside targetItem. The Wayland input mask keeps
// that strip hittable; see apply_linux_shadow_mask in theme.cpp.
Item {
    id: root

    property Window targetWindow: null
    property Item targetItem: null
    property int overhang: 3
    property int handleThickness: 5
    property int cornerSize: 12

    anchors.fill: targetItem ? targetItem : parent
    anchors.margins: targetItem ? -overhang : 0
    visible: targetWindow ? (targetWindow.visibility !== Window.Maximized && targetWindow.visibility !== Window.FullScreen) : true
    z: 9999

    MouseArea {
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        anchors.leftMargin: root.cornerSize
        anchors.rightMargin: root.cornerSize
        height: root.handleThickness
        hoverEnabled: true
        cursorShape: Qt.SizeVerCursor
        acceptedButtons: Qt.LeftButton
        onPressed: (mouse) => { if (root.targetWindow) root.targetWindow.startSystemResize(Qt.TopEdge) }
    }

    MouseArea {
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        anchors.leftMargin: root.cornerSize
        anchors.rightMargin: root.cornerSize
        height: root.handleThickness
        hoverEnabled: true
        cursorShape: Qt.SizeVerCursor
        acceptedButtons: Qt.LeftButton
        onPressed: (mouse) => { if (root.targetWindow) root.targetWindow.startSystemResize(Qt.BottomEdge) }
    }

    MouseArea {
        anchors.top: parent.top
        anchors.bottom: parent.bottom
        anchors.left: parent.left
        anchors.topMargin: root.cornerSize
        anchors.bottomMargin: root.cornerSize
        width: root.handleThickness
        hoverEnabled: true
        cursorShape: Qt.SizeHorCursor
        acceptedButtons: Qt.LeftButton
        onPressed: (mouse) => { if (root.targetWindow) root.targetWindow.startSystemResize(Qt.LeftEdge) }
    }

    MouseArea {
        anchors.top: parent.top
        anchors.bottom: parent.bottom
        anchors.right: parent.right
        anchors.topMargin: root.cornerSize
        anchors.bottomMargin: root.cornerSize
        width: root.handleThickness
        hoverEnabled: true
        cursorShape: Qt.SizeHorCursor
        acceptedButtons: Qt.LeftButton
        onPressed: (mouse) => { if (root.targetWindow) root.targetWindow.startSystemResize(Qt.RightEdge) }
    }

    MouseArea {
        anchors.top: parent.top
        anchors.left: parent.left
        width: root.cornerSize
        height: root.cornerSize
        hoverEnabled: true
        cursorShape: Qt.SizeFDiagCursor
        acceptedButtons: Qt.LeftButton
        onPressed: (mouse) => { if (root.targetWindow) root.targetWindow.startSystemResize(Qt.TopEdge | Qt.LeftEdge) }
    }

    MouseArea {
        anchors.top: parent.top
        anchors.right: parent.right
        width: root.cornerSize
        height: root.cornerSize
        hoverEnabled: true
        cursorShape: Qt.SizeBDiagCursor
        acceptedButtons: Qt.LeftButton
        onPressed: (mouse) => { if (root.targetWindow) root.targetWindow.startSystemResize(Qt.TopEdge | Qt.RightEdge) }
    }

    MouseArea {
        anchors.bottom: parent.bottom
        anchors.left: parent.left
        width: root.cornerSize
        height: root.cornerSize
        hoverEnabled: true
        cursorShape: Qt.SizeBDiagCursor
        acceptedButtons: Qt.LeftButton
        onPressed: (mouse) => { if (root.targetWindow) root.targetWindow.startSystemResize(Qt.BottomEdge | Qt.LeftEdge) }
    }

    MouseArea {
        anchors.bottom: parent.bottom
        anchors.right: parent.right
        width: root.cornerSize
        height: root.cornerSize
        hoverEnabled: true
        cursorShape: Qt.SizeFDiagCursor
        acceptedButtons: Qt.LeftButton
        onPressed: (mouse) => { if (root.targetWindow) root.targetWindow.startSystemResize(Qt.BottomEdge | Qt.RightEdge) }
    }
}
