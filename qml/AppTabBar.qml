import QtQuick
import QtQuick.Templates

// A lightweight, scrollable tab bar component.
//
// Uses AppTabButton children (declared directly or via a Repeater) inside an
// inner Row wrapped by a Flickable. This preserves custom styling and
// auto-exclusive button grouping while providing smooth horizontal scrolling
// when tabs overflow and optional centering when tabs fit within available space.
Flickable {
    id: control

    property int currentIndex: -1
    property bool centered: false

    default property alias contentData: row.data

    implicitWidth: row.implicitWidth
    implicitHeight: row.implicitHeight
    contentWidth: Math.max( width, row.width )
    contentHeight: height
    clip: true
    flickableDirection: Flickable.HorizontalFlick
    boundsBehavior: Flickable.StopAtBounds
    interactive: contentWidth > width

    onWidthChanged: {
        if ( contentWidth <= width ) contentX = 0
        else if ( contentX > contentWidth - width ) contentX = contentWidth - width
        Qt.callLater( ensureVisibleCurrent )
    }

    onContentWidthChanged: {
        if ( contentWidth <= width ) contentX = 0
        else if ( contentX > contentWidth - width ) contentX = contentWidth - width
        Qt.callLater( ensureVisibleCurrent )
    }

    // Centers the tabs when they fit within the available width;
    // aligns them to the left (x = 0) when they overflow so scrolling starts at the beginning.
    Row {
        id: row
        anchors.verticalCenter: parent.verticalCenter
        x: (control.centered && width < control.width)
           ? Math.round( (control.width - width) / 2 )
           : 0

        function ensureVisible(item: Item): void {
            control.ensureVisible(item)
        }

        property alias currentIndex: control.currentIndex

        onChildrenChanged: Qt.callLater( control.syncTabs )
    }

    function buttonCount(): int {
        let count = 0
        for ( let i = 0; i < row.children.length; i++ ) {
            const btn = row.children[i] as AbstractButton
            if ( btn && btn.checkable ) count++
        }
        return count
    }

    function buttonAt( idx: int ): AbstractButton {
        let pos = 0
        for ( let i = 0; i < row.children.length; i++ ) {
            const btn = row.children[i] as AbstractButton
            if ( btn && btn.checkable ) {
                if ( pos === idx ) return btn
                pos++
            }
        }
        return null
    }

    function indexOfButton( target: AbstractButton ): int {
        let pos = 0
        for ( let i = 0; i < row.children.length; i++ ) {
            const btn = row.children[i] as AbstractButton
            if ( btn && btn.checkable ) {
                if ( btn === target ) return pos
                pos++
            }
        }
        return -1
    }

    function ensureVisibleCurrent(): void {
        const btn = buttonAt( currentIndex )
        if ( btn ) {
            ensureVisible( btn )
        }
    }

    function ensureVisible( item: Item ): void {
        if ( !item ) return
        const idx = indexOfButton( item as AbstractButton )
        if ( idx > 0 && item.x === 0 ) {
            // Item has not been positioned by Row layout yet; defer until layout pass
            Qt.callLater( ensureVisibleCurrent )
            return
        }
        const maxContentWidth = Math.max( contentWidth, row.width, row.x + item.x + item.width )
        if ( maxContentWidth <= width ) {
            contentX = 0
            return
        }
        const margin = 4
        const itemLeft = row.x + item.x - margin
        const itemRight = row.x + item.x + item.width + margin
        if ( item.width + margin * 2 >= width ) {
            contentX = Math.max( 0, itemLeft )
        } else if ( itemLeft < contentX ) {
            contentX = Math.max( 0, itemLeft )
        } else if ( itemRight > contentX + width ) {
            contentX = Math.max( 0, Math.min( maxContentWidth - width, itemRight - width ) )
        }
    }

    // Synchronizes checked state of tab buttons with currentIndex whenever tabs
    // are added, removed, or refreshed (e.g. dynamic Repeater in editorpanel.qml).
    // Using callLater prevents premature index resets while delegates are
    // incrementally created.
    function syncTabs(): void {
        const count = buttonCount()
        if ( count === 0 ) {
            if ( currentIndex !== -1 ) currentIndex = -1
            return
        }

        let targetIndex = currentIndex
        if ( targetIndex < 0 || targetIndex >= count ) {
            targetIndex = Math.max( 0, Math.min( targetIndex, count - 1 ) )
        }

        if ( currentIndex !== targetIndex ) {
            currentIndex = targetIndex
        }

        for ( let i = 0; i < count; i++ ) {
            const btn = buttonAt( i )
            if ( btn ) btn.checked = ( i === targetIndex )
        }
        const activeBtn = buttonAt( targetIndex )
        if ( activeBtn ) ensureVisible( activeBtn )
        Qt.callLater( ensureVisibleCurrent )
    }

    onCurrentIndexChanged: {
        const focusItem = Window.window ? Window.window.activeFocusItem : null
        if (focusItem) {
            focusItem.focus = false
        }
        const count = buttonCount()
        for ( let i = 0; i < count; i++ ) {
            const btn = buttonAt( i )
            if ( btn ) btn.checked = ( i === currentIndex )
        }
        const activeBtn = buttonAt( currentIndex )
        if ( activeBtn ) {
            ensureVisible( activeBtn )
        }
        Qt.callLater( ensureVisibleCurrent )
    }

    Component.onCompleted: Qt.callLater( syncTabs )

    // Convert mouse wheel events to horizontal scroll when tabs overflow
    WheelHandler {
        acceptedDevices: PointerDevice.Mouse
        onWheel: (event) => {
            if ( control.contentWidth <= control.width ) return
            let delta = 0
            if ( event.angleDelta.x !== 0 ) {
                delta = event.angleDelta.x
            } else if ( event.angleDelta.y !== 0 ) {
                delta = event.angleDelta.y
            }
            if ( delta !== 0 ) {
                control.contentX = Math.max( 0, Math.min( control.contentWidth - control.width, control.contentX - delta ) )
                event.accepted = true
            }
        }
    }
}
