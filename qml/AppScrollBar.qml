import QtQuick
import QtQuick.Controls
import QtQuick.Templates as T

/* Template ScrollBar supporting AsNeeded auto-hide, AlwaysOn, and theme coloring.
 */
T.ScrollBar {
    id: control
    SystemPalette { id: appTheme }

    implicitWidth: orientation === Qt.Vertical
                   ? 12
                   : Math.max(100, implicitBackgroundWidth + leftInset + rightInset,
                              implicitContentWidth + leftPadding + rightPadding)
    implicitHeight: orientation === Qt.Horizontal
                    ? 12
                    : Math.max(100, implicitBackgroundHeight + topInset + bottomInset,
                               implicitContentHeight + topPadding + bottomPadding)

    padding: 2
    visible: policy === ScrollBar.AlwaysOn || (size < 1.0 && size > 0.0)
    minimumSize: {
        if ( orientation === Qt.Horizontal )
            return width > 0 ? Math.min( 0.5, ( height / 2 ) / width ) : 0.1
        return height > 0 ? Math.min( 0.5, ( width / 2 ) / height ) : 0.1
    }
    opacity: (policy === ScrollBar.AlwaysOn || active || hovered || pressed) ? 1.0 : 0.0

    Behavior on opacity {
        NumberAnimation { duration: 150 }
    }

    background: Rectangle {
        implicitWidth: control.interactive ? 10 : 6
        implicitHeight: control.interactive ? 10 : 6
        color: "transparent"
        visible: control.size < 1.0 || control.policy === ScrollBar.AlwaysOn
    }

    contentItem: Rectangle {
        implicitWidth: control.interactive ? 8 : 5
        implicitHeight: control.interactive ? 8 : 5
        radius: width / 2
        color: control.pressed ? appTheme.highlight : (control.hovered ? appTheme.mid : appTheme.midlight)
        opacity: control.pressed ? 0.95 : (control.hovered ? 0.85 : 0.65)

        Behavior on color {
            ColorAnimation { duration: 100 }
        }
    }
}
