import QtQuick
import QtQuick.Controls
import QtQuick.Templates as T
import cs_app

// A tab in an AppTabBar (a row of checkable ToolButtons, not a TabBar -- see
// that file for why). Built on QtQuick.Templates T.ToolButton for platform independence.
T.ToolButton {
    id: control
    SystemPalette { id: appTheme }

    implicitWidth: Math.max(implicitBackgroundWidth + leftInset + rightInset,
                            contentItem.implicitWidth + leftPadding + rightPadding)
    implicitHeight: Math.max(implicitBackgroundHeight + topInset + bottomInset,
                             contentItem.implicitHeight + topPadding + bottomPadding)

    checkable: true
    autoExclusive: true // exclusive among siblings under the same parent -- no ButtonGroup needed
    font: Qt.font({ family: App.fontFamily, pixelSize: App.fontSize })

    width: implicitWidth
    leftPadding: 14
    rightPadding: 14
    topPadding: 4
    bottomPadding: 4

    background: Rectangle {
        anchors.fill: parent
        radius: 4
        visible: control.checked || control.hovered
        color: control.checked ? appTheme.highlight : appTheme.midlight
    }

    contentItem: Text {
        text: control.text
        font: control.font
        color: control.checked ? appTheme.highlightedText : appTheme.windowText
        horizontalAlignment: Text.AlignHCenter
        verticalAlignment: Text.AlignVCenter
        elide: Text.ElideRight
    }

    // Reports its own position back to the enclosing AppTabBar's currentIndex;
    // see AppTabBar.tabButtons() for how that position is derived.
    onCheckedChanged: {
        if ( !checked || !parent || !parent.tabButtons ) return
        const index = parent.tabButtons().indexOf( control )
        if ( index >= 0 && parent.currentIndex !== index ) parent.currentIndex = index
    }
}
