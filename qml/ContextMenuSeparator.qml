import QtQuick
import QtQuick.Templates as T

T.MenuSeparator {
    id: control
    SystemPalette { id: appTheme }

    implicitHeight: visible ? 9 : 0
    height: visible ? 9 : 0
    clip: true
    topPadding: visible ? 4 : 0
    bottomPadding: visible ? 4 : 0
    topInset: 0
    bottomInset: 0
    leftInset: 0
    rightInset: 0
    padding: visible ? 4 : 0

    contentItem: Rectangle {
        visible: control.visible
        implicitHeight: control.visible ? 1 : 0
        color: appTheme.mid
    }
}
