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
    property int leftContentPadding: contentPadding
    property bool defaultContextMenu: true
    property bool followTail: false
    readonly property bool _wrapped: wrapMode !== TextEdit.NoWrap

    property bool _pinnedToTail: true
    property int _programmaticDepth: 0
    property real _lastContentY: 0

    readonly property alias selectedText: area.selectedText
    readonly property alias hoveredLink: area.hoveredLink
    readonly property alias editor: area
    readonly property alias flickable: flick

    signal linkActivated(string link)

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

    function _maxContentY(): real {
        return Math.max(0, flick.contentHeight - flick.height)
    }

    function _runProgrammatic(fn): void {
        _programmaticDepth += 1
        try {
            fn()
        } finally {
            _programmaticDepth -= 1
            _lastContentY = flick.contentY
        }
    }

    function _applyTailScroll(): void {
        _runProgrammatic(function() {
            flick.contentY = root._maxContentY()
            flick.contentX = 0
        })
    }

    function _applyTailScrollIfPinned(): void {
        if (_pinnedToTail && !flick.dragging && !vbar.pressed) {
            _applyTailScroll()
        }
    }

    function _syncViewportBounds(): void {
        var maxY = _maxContentY()
        if (followTail) {
            if (maxY <= 0) {
                _pinnedToTail = true
            }
            if (_pinnedToTail && !flick.dragging && !vbar.pressed) {
                _applyTailScroll()
            } else if (flick.contentY > maxY) {
                _runProgrammatic(function() {
                    flick.contentY = maxY
                })
                _pinnedToTail = true
            }
        } else if (flick.contentY > maxY) {
            _runProgrammatic(function() {
                flick.contentY = maxY
            })
        }
    }

    function scrollToTail(): void {
        _pinnedToTail = true
        _applyTailScroll()
        Qt.callLater(_applyTailScrollIfPinned)
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
        onHeightChanged: {
            root._runProgrammatic(function() {
                flick.contentY += 1
                flick.contentY -= 1
                root._syncViewportBounds()
            })
        }
        onContentHeightChanged: root._syncViewportBounds()
        onContentYChanged: {
            if (root._programmaticDepth > 0) {
                return
            }
            var maxY = root._maxContentY()
            if (maxY <= 0) {
                root._pinnedToTail = true
            } else if (flick.contentY < root._lastContentY) {
                if (flick.contentY < maxY) {
                    root._pinnedToTail = false
                }
            } else if (flick.atYEnd || flick.contentY >= maxY - 2) {
                root._pinnedToTail = true
            } else {
                root._pinnedToTail = false
            }
            root._lastContentY = flick.contentY
        }
        onDraggingChanged: {
            if (!dragging && root.followTail) {
                root._applyTailScrollIfPinned()
            }
        }

        ScrollBar.vertical: AppScrollBar {
            id: vbar
            policy: ScrollBar.AsNeeded
            onPressedChanged: {
                if (!pressed && root.followTail) {
                    root._applyTailScrollIfPinned()
                }
            }
        }
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
            leftPadding: root.leftContentPadding
            rightPadding: root.contentPadding
            font.family: App.fontFamily
            font.pixelSize: App.fontNormal
            selectionColor: appTheme.highlight
            selectedTextColor: appTheme.highlightedText

            onLinkActivated: (link) => root.linkActivated(link)

            onTextChanged: {
                if (!root.followTail) {
                    return
                }
                if (area.length === 0) {
                    root._pinnedToTail = true
                }
                if (root._pinnedToTail && !flick.dragging && !vbar.pressed) {
                    root._applyTailScroll()
                    Qt.callLater(root._applyTailScrollIfPinned)
                }
            }

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

            property AppTextContextMenu _defaultMenu: null
            Component {
                id: defaultMenuComp
                AppTextContextMenu {}
            }

            TapHandler {
                enabled: root.defaultContextMenu
                acceptedButtons: Qt.RightButton
                onTapped: {
                    if (!area._defaultMenu)
                        area._defaultMenu = defaultMenuComp.createObject(area, { textEdit: area }) as AppTextContextMenu
                    if (area._defaultMenu)
                        area._defaultMenu.popup()
                }
            }
        }
    }
}
