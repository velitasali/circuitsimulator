import QtQuick
import QtQuick.Controls
import cs_app

/* Scrollable TextEdit inside a Flickable.
 * Uses plain TextEdit rather than TextArea.flickable because TextArea clips
 * text to its padding in viewport coordinates on all four sides, preventing
 * scrolling into the padded area. TextEdit keeps padding in document coords.
 */
Item {
    id: root
    SystemPalette { id: appTheme }

    property alias text: area.text
    property alias textFormat: area.textFormat
    property alias font: area.font
    property alias color: area.color
    property alias selectionColor: area.selectionColor
    property alias selectedTextColor: area.selectedTextColor
    property alias readOnly: area.readOnly
    property alias selectByMouse: area.selectByMouse
    property alias placeholderTextColor: placeholderLabel.color
    property string placeholderText: ""
    property bool wordWrap: false
    property int wrapMode: wordWrap ? TextEdit.Wrap : TextEdit.NoWrap
    property int contentPadding: 9
    property bool defaultContextMenu: true
    readonly property bool _wrapped: wrapMode !== TextEdit.NoWrap

    readonly property alias selectedText: area.selectedText
    readonly property alias editor: area
    readonly property alias flickable: flick

    default property alias editorChildren: area.data

    function copy(): void {
        area.copy()
    }

    function selectAll(): void {
        area.selectAll()
    }

    function positionAt(x: real, y: real): int {
        return area.positionAt(x, y)
    }

    function _applyTailScroll(): void {
        flick.contentY = Math.max(0, flick.contentHeight - flick.height)
        flick.contentX = 0
    }

    function scrollToTail(): void {
        _applyTailScroll()
        Qt.callLater(_applyTailScroll)
    }

    Flickable {
        id: flick
        anchors.fill: parent
        clip: true
        boundsBehavior: Flickable.StopAtBounds
        contentWidth: root._wrapped ? width : Math.max(width, area.implicitWidth)
        contentHeight: Math.max(height, area.implicitHeight)

        /* TextEdit's large-document support only re-renders the glyphs it
         * thinks are visible, and it recomputes that range off contentX/
         * contentY, not off the Flickable's own width/height -- so widening
         * the panel updates flick.width immediately but the newly exposed
         * strip stays blank until something nudges contentX/Y.
         */
        onWidthChanged: { contentX += 1; contentX -= 1 }
        onHeightChanged: { contentY += 1; contentY -= 1 }
        onContentHeightChanged: {
            if (contentY > contentHeight - height) {
                contentY = Math.max(0, contentHeight - height)
            }
        }

        ScrollBar.vertical: AppScrollBar { policy: ScrollBar.AsNeeded }
        ScrollBar.horizontal: AppScrollBar {
            policy: root._wrapped ? ScrollBar.AlwaysOff : ScrollBar.AsNeeded
        }

        TextEdit {
            id: area
            width: root._wrapped ? flick.width : Math.max(flick.width, implicitWidth)
            height: Math.max(flick.height, implicitHeight)
            wrapMode: root.wrapMode
            topPadding: root.contentPadding
            bottomPadding: root.contentPadding
            leftPadding: root.contentPadding
            rightPadding: root.contentPadding
            font.family: App.fontFamily
            font.pixelSize: App.fontNormal
            selectionColor: appTheme.highlight
            selectedTextColor: appTheme.highlightedText

            onWrapModeChanged: {
                if (root._wrapped) {
                    flick.contentX = 0
                }
            }

            AppText {
                id: placeholderLabel
                x: area.leftPadding
                y: area.topPadding
                width: Math.max(0, flick.width - area.leftPadding - area.rightPadding)
                text: root.placeholderText
                font: area.font
                color: appTheme.placeholderText
                wrapMode: Text.WordWrap
                visible: area.length === 0
            }

            TapHandler {
                enabled: root.defaultContextMenu
                acceptedButtons: Qt.RightButton
                onTapped: defaultMenu.popup()
            }

            AppTextContextMenu {
                id: defaultMenu
                textEdit: area
            }
        }
    }
}
