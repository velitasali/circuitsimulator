pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls
import QtQuick.Window
import cs_app
import cs_ui

Row {
    id: root

    required property Window window
    property bool showMinimize: true
    property bool showMaximize: true
    property bool windowActive: window ? window.active : true
    property real topRightRadius: 0

    readonly property bool maximized: window
        ? (window.visibility === Window.Maximized || window.visibility === Window.FullScreen)
        : false

    SystemPalette { id: appTheme }

    spacing: 0

    Rectangle {
        visible: root.showMinimize
        width: 46
        height: root.height
        color: minMouse.pressed
            ? Qt.rgba(appTheme.windowText.r, appTheme.windowText.g, appTheme.windowText.b, 0.16)
            : minMouse.containsMouse
                ? Qt.rgba(appTheme.windowText.r, appTheme.windowText.g, appTheme.windowText.b, 0.10)
                : "transparent"
        Behavior on color { ColorAnimation { duration: 80 } }

        Rectangle {
            anchors.centerIn: parent
            width: 10
            height: 1
            color: appTheme.windowText
            opacity: root.windowActive ? 0.9 : 0.6
        }

        MouseArea {
            id: minMouse
            anchors.fill: parent
            hoverEnabled: true
            property bool titleBarInteractive: true
            ToolTip.visible: containsMouse
            ToolTip.delay: 600
            ToolTip.text: I18n.tr("Minimize")
            onClicked: if (root.window) root.window.showMinimized()
        }
    }

    Rectangle {
        visible: root.showMaximize
        width: 46
        height: root.height
        color: maxMouse.pressed
            ? Qt.rgba(appTheme.windowText.r, appTheme.windowText.g, appTheme.windowText.b, 0.16)
            : maxMouse.containsMouse
                ? Qt.rgba(appTheme.windowText.r, appTheme.windowText.g, appTheme.windowText.b, 0.10)
                : "transparent"
        Behavior on color { ColorAnimation { duration: 80 } }

        Rectangle {
            visible: !root.maximized
            anchors.centerIn: parent
            width: 10
            height: 10
            color: "transparent"
            border.width: 1
            border.color: appTheme.windowText
            opacity: root.windowActive ? 0.9 : 0.6
        }

        Item {
            visible: root.maximized
            anchors.centerIn: parent
            width: 10
            height: 10
            opacity: root.windowActive ? 0.9 : 0.6

            Rectangle {
                x: 2
                y: 0
                width: 8
                height: 8
                color: "transparent"
                border.width: 1
                border.color: appTheme.windowText
            }
            Rectangle {
                x: 0
                y: 2
                width: 8
                height: 8
                color: appTheme.window
                border.width: 1
                border.color: appTheme.windowText
            }
        }

        MouseArea {
            id: maxMouse
            anchors.fill: parent
            hoverEnabled: true
            property bool titleBarInteractive: true
            ToolTip.visible: containsMouse
            ToolTip.delay: 600
            ToolTip.text: root.maximized ? I18n.tr("Restore") : I18n.tr("Maximize")
            onClicked: {
                if (!root.window)
                    return
                if (root.maximized)
                    root.window.showNormal()
                else
                    root.window.showMaximized()
            }
        }
    }

    Rectangle {
        width: 46
        height: root.height
        topRightRadius: root.topRightRadius
        color: closeMouse.pressed
            ? Qt.darker(App.closeHoverBg, 1.15)
            : closeMouse.containsMouse
                ? App.closeHoverBg
                : "transparent"
        Behavior on color { ColorAnimation { duration: 80 } }

        AppIcon {
            anchors.centerIn: parent
            icon: "close"
            size: 16
            color: (closeMouse.containsMouse || closeMouse.pressed)
                ? App.closeHoverText
                : appTheme.windowText
            opacity: (closeMouse.containsMouse || closeMouse.pressed || root.windowActive) ? 0.9 : 0.6
        }

        MouseArea {
            id: closeMouse
            anchors.fill: parent
            hoverEnabled: true
            property bool titleBarInteractive: true
            ToolTip.visible: containsMouse
            ToolTip.delay: 600
            ToolTip.text: I18n.tr("Close")
            onClicked: if (root.window) root.window.close()
        }
    }
}
