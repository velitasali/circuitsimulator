import QtQuick
import QtQuick.Controls
import QtQuick.Templates as T
import cs_app

/* The app's flat icon button.
 *
 * Built on QtQuick.Templates T.ToolButton for complete independence from platform styles.
 */
T.ToolButton {
    id: control
    SystemPalette { id: appTheme }

    implicitWidth: Math.max(implicitBackgroundWidth + leftInset + rightInset,
                            contentItem.implicitWidth + leftPadding + rightPadding)
    implicitHeight: Math.max(implicitBackgroundHeight + topInset + bottomInset,
                             contentItem.implicitHeight + topPadding + bottomPadding)

    font: Qt.font({ family: App.fontFamily, pixelSize: App.fontSize })
    padding: 4
    opacity: control.enabled ? 1.0 : 0.4

    // The tint the hover/press fills are mixed from. Kept as a property so a
    // button on a coloured surface can lift it off that instead.
    property color tint: appTheme.windowText
    property color textColor: control.isCheckedActive ? appTheme.highlight : control.tint
    property real radius: 5

    // True only when the button is actively checked AND enabled.
    readonly property bool isCheckedActive: control.checked && control.enabled

    ToolTip.delay: 600

    background: Rectangle {
        // The square floor the icon buttons want; the width still grows with
        // the content, which is what the text ones (zoom, warnings) need.
        implicitWidth: 26
        implicitHeight: 26
        radius: control.radius

        // When active (checked and enabled), use a highlight tint matching the border.
        // When disabled or unchecked, do not show highlight decoration.
        color: control.down
                 ? ( control.isCheckedActive ? Qt.rgba( appTheme.highlight.r, appTheme.highlight.g, appTheme.highlight.b, 0.28 )
                                             : Qt.rgba( control.tint.r, control.tint.g, control.tint.b, 0.22 ) )
             : control.isCheckedActive && control.hovered
                 ? Qt.rgba( appTheme.highlight.r, appTheme.highlight.g, appTheme.highlight.b, 0.22 )
             : control.isCheckedActive
                 ? Qt.rgba( appTheme.highlight.r, appTheme.highlight.g, appTheme.highlight.b, 0.15 )
             : ( control.hovered && control.enabled )
                 ? Qt.rgba( control.tint.r, control.tint.g, control.tint.b, 0.10 )
                 : "transparent"

        border.width: ( control.isCheckedActive || ( control.visualFocus && control.enabled ) ) ? 1 : 0
        border.color: Qt.rgba( appTheme.highlight.r, appTheme.highlight.g,
                               appTheme.highlight.b, 0.8 )

        Behavior on color { ColorAnimation { duration: 90 } }
    }

    contentItem: Text {
        text: control.text
        font: control.font
        color: control.textColor
        horizontalAlignment: Text.AlignHCenter
        verticalAlignment: Text.AlignVCenter
        elide: Text.ElideRight
    }

    Behavior on opacity { NumberAnimation { duration: 90 } }
}
