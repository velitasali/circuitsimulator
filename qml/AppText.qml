import QtQuick
import cs_app

Text {
    id: root
    SystemPalette { id: appTheme }

    color: appTheme.windowText
    font.family: App.fontFamily
    font.pixelSize: App.fontNormal
}
