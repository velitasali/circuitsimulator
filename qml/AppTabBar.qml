import QtQuick

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
    }

    onContentWidthChanged: {
        if ( contentWidth <= width ) contentX = 0
        else if ( contentX > contentWidth - width ) contentX = contentWidth - width
    }

    // Centers the tabs when they fit within the available width;
    // aligns them to the left (x = 0) when they overflow so scrolling starts at the beginning.
    Row {
        id: row
        anchors.verticalCenter: parent.verticalCenter
        x: (control.centered && width < control.width)
           ? Math.round( (control.width - width) / 2 )
           : 0

        function tabButtons() {
            return control.tabButtons()
        }

        property alias currentIndex: control.currentIndex

        onChildrenChanged: Qt.callLater( control.syncTabs )
    }

    // Duck-typed rather than an instanceof check: children also contains any
    // Repeater used to generate buttons, which has no `checkable` property.
    function tabButtons() {
        const result = []
        for ( let i = 0; i < row.children.length; i++ ) {
            if ( row.children[i].checkable !== undefined ) result.push( row.children[i] )
        }
        return result
    }

    function ensureVisible( item ) {
        if ( !item || contentWidth <= width ) return
        const itemLeft = row.x + item.x
        const itemRight = itemLeft + item.width
        if ( itemLeft < contentX ) {
            contentX = Math.max( 0, itemLeft )
        } else if ( itemRight > contentX + width ) {
            contentX = Math.min( contentWidth - width, itemRight - width )
        }
    }

    // Synchronizes checked state of tab buttons with currentIndex whenever tabs
    // are added, removed, or refreshed (e.g. dynamic Repeater in editorpanel.qml).
    // Using callLater prevents premature index resets while delegates are
    // incrementally created.
    function syncTabs() {
        const buttons = tabButtons()
        if ( buttons.length === 0 ) {
            if ( currentIndex !== -1 ) currentIndex = -1
            return
        }

        let targetIndex = currentIndex
        if ( targetIndex < 0 || targetIndex >= buttons.length ) {
            targetIndex = Math.max( 0, Math.min( targetIndex, buttons.length - 1 ) )
        }

        if ( currentIndex !== targetIndex ) {
            currentIndex = targetIndex
        }

        for ( let i = 0; i < buttons.length; i++ ) {
            buttons[i].checked = ( i === targetIndex )
        }
        ensureVisible( buttons[targetIndex] )
    }

    onCurrentIndexChanged: {
        if (Window.window && Window.window.activeFocusItem) {
            Window.window.activeFocusItem.focus = false
        }
        const buttons = tabButtons()
        for ( let i = 0; i < buttons.length; i++ ) {
            buttons[i].checked = ( i === currentIndex )
        }
        if ( currentIndex >= 0 && currentIndex < buttons.length ) {
            ensureVisible( buttons[currentIndex] )
        }
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
