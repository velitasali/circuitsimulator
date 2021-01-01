import QtQuick
import QtQuick.Templates as T
import cs_app

// ContextMenuItem delegate for AppContextMenu.
// Renders icons via Material Symbols Rounded ligatures, right-aligned shortcuts,
// and properly collapses (height = 0) when visible is false so no blank gaps occur.
// Template MenuItem, not the platform style: Windows paints accent hover on top.
T.MenuItem {
    id: control
    SystemPalette { id: appTheme }

    // Material Symbols ligature, or "" for a row without an icon.
    property string iconLigature: ""
    // Display text only for shortcut key combinations.
    property string shortcutText: ""
    // Instantiator delegates cannot switch type, so a separator row is this item.
    property bool isSeparator: false
    // Menu strings use &F for Alt+F; native MenuItem hides the ampersand, Text does not.
    readonly property string displayText: {
        var s = String(text)
        var out = ""
        for (var i = 0; i < s.length; ++i) {
            if (s.charAt(i) === "&") {
                if (i + 1 < s.length && s.charAt(i + 1) === "&") {
                    out += "&"
                    i++
                }
            } else {
                out += s.charAt(i)
            }
        }
        return out
    }

    implicitHeight: visible ? (isSeparator ? 9 : 26) : 0
    implicitWidth: visible ? (isSeparator ? 160 : (contentItem.implicitWidth + 8)) : 0
    height: visible ? (isSeparator ? 9 : 26) : 0
    clip: true
    hoverEnabled: !isSeparator
    font: Qt.font({ family: (typeof App !== "undefined" ? App.fontFamily : "Ubuntu"), pixelSize: (typeof App !== "undefined" ? App.fontSize : 12) })
    topInset: 0
    bottomInset: 0
    leftInset: 0
    rightInset: 0
    padding: 0
    topPadding: 0
    bottomPadding: 0


    background: Item {
        implicitWidth: control.visible ? 160 : 0
        implicitHeight: control.visible ? (control.isSeparator ? 9 : 26) : 0
        visible: control.visible

        Rectangle {
            visible: control.isSeparator
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.verticalCenter: parent.verticalCenter
            anchors.leftMargin: 8
            anchors.rightMargin: 8
            height: 1
            color: appTheme.mid
        }
        Rectangle {
            visible: !control.isSeparator
            x: 2
            y: 1
            width: parent.width - 4
            height: parent.height - 2
            radius: 4
            color: control.highlighted ? appTheme.highlight : "transparent"
        }
    }

    arrow: AppIcon {
        x: control.width - width - 8
        y: (control.height - height) / 2
        visible: control.visible && !control.isSeparator && control.subMenu
        text: "chevron_right"
        font.pixelSize: 16
        color: control.highlighted ? appTheme.highlightedText : appTheme.windowText
    }

    contentItem: Item {
        visible: control.visible && !control.isSeparator
        implicitWidth: control.visible && !control.isSeparator ? (6 + 18 + 8 + textLabel.implicitWidth
                       + (control.shortcutText !== "" ? (shortcutLabel.implicitWidth + 20) : 0)
                       + 12 + (control.subMenu ? 20 : 0)) : 0
        implicitHeight: control.visible && !control.isSeparator ? 26 : 0

        AppIcon {
            id: iconLabel
            anchors.left: parent.left
            anchors.leftMargin: 6
            anchors.verticalCenter: parent.verticalCenter
            width: 18
            clip: true
            visible: control.iconLigature !== "" || control.checked
            text: control.iconLigature !== "" ? control.iconLigature : (control.checked ? "check" : "")
            font.pixelSize: 16
            color: control.highlighted ? appTheme.highlightedText : appTheme.windowText
            opacity: control.enabled ? 1.0 : 0.4
        }
        Text {
            id: textLabel
            anchors.left: iconLabel.right
            anchors.leftMargin: 8
            anchors.right: control.shortcutText !== "" ? shortcutLabel.left : parent.right
            anchors.rightMargin: control.shortcutText !== "" ? 16 : (control.subMenu ? 28 : 8)
            anchors.verticalCenter: parent.verticalCenter
            text: control.displayText
            font: control.font
            color: control.highlighted ? appTheme.highlightedText : appTheme.windowText
            opacity: control.enabled ? 1.0 : 0.4
            elide: Text.ElideRight
        }
        Text {
            id: shortcutLabel
            anchors.right: parent.right
            anchors.rightMargin: 8
            anchors.verticalCenter: parent.verticalCenter
            visible: control.shortcutText !== ""
            text: control.shortcutText
            font: control.font
            color: control.highlighted ? appTheme.highlightedText
                                       : Qt.rgba(appTheme.windowText.r, appTheme.windowText.g,
                                                 appTheme.windowText.b, 0.6)
            opacity: control.enabled ? 1.0 : 0.4
        }
    }
}
