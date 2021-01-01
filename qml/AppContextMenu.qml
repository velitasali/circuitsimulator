import QtQuick
import QtQuick.Controls
import QtQuick.Templates as T

// Template Menu, not the platform style: Windows Quick Controls draws a light
// popup and accent-colored hover regardless of the app palette / dark theme.
T.Menu {
    id: control
    SystemPalette { id: appTheme }

    // Copied onto the parent row that addMenu/insertMenu builds from `delegate`.
    property string iconLigature: ""

    // macOS style defaults to Popup.Native, which ignores this delegate and
    // treats an empty submenu as a disabled row. Window matches src/gui and
    // keeps hover-cascading submenus.
    implicitWidth: Math.max(implicitBackgroundWidth + leftInset + rightInset,
                            contentWidth + leftPadding + rightPadding)
    implicitHeight: Math.max(implicitBackgroundHeight + topInset + bottomInset,
                             contentHeight + topPadding + bottomPadding)

    popupType: Popup.Window
    cascade: true
    margins: 0
    topInset: 0
    bottomInset: 0
    leftInset: 0
    rightInset: 0
    padding: 4
    overlap: 1

    palette.window: appTheme.window
    palette.windowText: appTheme.windowText
    palette.base: appTheme.base
    palette.text: appTheme.windowText
    palette.button: appTheme.button
    palette.buttonText: appTheme.buttonText
    palette.highlight: appTheme.highlight
    palette.highlightedText: appTheme.highlightedText
    palette.mid: appTheme.mid
    palette.midlight: appTheme.midlight

    delegate: ContextMenuItem {
        iconLigature: subMenu && subMenu.iconLigature ? subMenu.iconLigature : ""
    }

    background: Rectangle {
        implicitWidth: 200
        implicitHeight: 20
        color: appTheme.window
        border.color: appTheme.mid
        border.width: 1
        radius: 6
    }

    property real contentImplicitWidth: 180
    property real contentImplicitHeight: 0

    function updateGeometry() {
        var maxW = 180
        var totalH = 0
        for (var i = 0; i < count; ++i) {
            var it = itemAt(i)
            if (it && it.visible) {
                if (it.implicitWidth > maxW)
                    maxW = it.implicitWidth
                totalH += (it.implicitHeight > 0 ? it.implicitHeight : it.height)
            }
        }
        contentImplicitWidth = Math.min(maxW, 520)
        contentImplicitHeight = totalH
    }

    Connections {
        target: control
        function onAboutToShow() {
            control.updateGeometry()
        }
    }
    onCountChanged: updateGeometry()

    // macOS Menu.qml uses spacing: 2, so every hidden row still adds a gap.
    contentItem: ListView {
        id: listView
        implicitHeight: control.contentImplicitHeight > 0 ? control.contentImplicitHeight : contentHeight
        implicitWidth: control.contentImplicitWidth
        model: control.contentModel
        interactive: false
        clip: true
        spacing: 0
        currentIndex: control.currentIndex
    }
}
