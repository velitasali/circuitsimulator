import QtQuick
import QtQuick.Controls
import cs_app

// Body of list popup matching C++ QmlListPopup. Rows are flat; "depth" drives indentation.
Popup {
    id: root

    property var rows: []
    property int preferredWidth: 256
    property int minWidth: 240
    property int maxWidth: 520

    signal activated(int index)

    padding: 0
    margins: 0
    modal: false
    focus: true
    closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutside

    SystemPalette { id: appTheme }

    width: {
        var len = rows ? rows.length : 0
        var maxChars = 0
        for (var i = 0; i < len; ++i) {
            var r = rows[i]
            var t = r ? (r.text || "") : ""
            var d = (r && r.depth) ? r.depth : 0
            maxChars = Math.max(maxChars, t.length + d * 2)
        }
        var computedW = Math.max(minWidth, Math.min(maxWidth, maxChars * 8 + 48))
        return Math.max(preferredWidth, computedW)
    }

    height: Math.min(360, Math.max(60, (rows ? rows.length : 0) * 26 + 16))

    background: Rectangle {
        color: appTheme.window
        border.color: appTheme.mid
        border.width: 1
        radius: 12
    }

    contentItem: Item {
        anchors.fill: parent
        anchors.margins: 8

        ListView {
            id: list
            anchors.fill: parent
            clip: true
            model: root.rows
            boundsBehavior: Flickable.StopAtBounds
            ScrollBar.vertical: AppScrollBar {
                policy: ScrollBar.AsNeeded
            }

            delegate: Rectangle {
                id: rowRect
                required property var modelData
                required property int index

                width: list.width
                height: 24
                radius: 4
                color: modelData.current
                       ? appTheme.alternateBase
                       : (hover.hovered ? appTheme.highlight : "transparent")

                Text {
                    anchors.fill: parent
                    anchors.leftMargin: 6 + (modelData.depth || 0) * 14
                    anchors.rightMargin: 6
                    text: modelData.text || ""
                    font: Qt.font({
                        family: App.fontFamily,
                        pixelSize: App.fontSize,
                        bold: Boolean(rowRect.modelData.current)
                    })
                    color: hover.hovered && !rowRect.modelData.current
                           ? appTheme.highlightedText
                           : appTheme.windowText
                    elide: Text.ElideRight
                    verticalAlignment: Text.AlignVCenter
                }

                HoverHandler { id: hover }
                TapHandler {
                    onTapped: {
                        root.activated(rowRect.index)
                        root.close()
                    }
                }
            }
        }
    }
}
