import QtQuick
import QtQuick.Templates as T

/* Template Slider, not the platform style.
 *
 * Ensures consistent track and thumb rendering across all operating systems
 * and respects the active application theme.
 */
T.Slider {
    id: control
    SystemPalette { id: appTheme }

    // Optional default marker & magnetic temporary lock-in (detent).
    // Set defaultValue to a finite number within [from, to] to enable.
    property real defaultValue: NaN
    property real defaultCapturePx: 6
    property real defaultReleasePx: 12
    property real defaultRearmPx: 18

    readonly property bool hasDefault: !isNaN(defaultValue)
        && to !== from
        && defaultValue >= Math.min(from, to)
        && defaultValue <= Math.max(from, to)
    readonly property real _defaultEpsilon: Math.max(1e-6, Math.abs(to - from) * 1e-6)
    readonly property bool atDefault: hasDefault && Math.abs(value - defaultValue) <= _defaultEpsilon
    readonly property real defaultPosition: hasDefault
        ? (defaultValue - from) / (to - from)
        : 0
    readonly property real visualDefaultPosition: horizontal
        ? (mirrored ? 1.0 - defaultPosition : defaultPosition)
        : (mirrored ? defaultPosition : 1.0 - defaultPosition)

    readonly property real _handleExtent: horizontal
        ? (handle ? handle.width : 16)
        : (handle ? handle.height : 16)
    readonly property real _travelExtent: Math.max(
        1,
        (horizontal ? availableWidth : availableHeight) - _handleExtent
    )
    readonly property real defaultGuideX: horizontal
        ? leftPadding + _handleExtent / 2 + visualDefaultPosition * _travelExtent
        : leftPadding + availableWidth / 2
    readonly property real defaultGuideY: horizontal
        ? topPadding + availableHeight / 2
        : topPadding + _handleExtent / 2 + visualDefaultPosition * _travelExtent

    property bool _detentSnapping: false
    property bool _detentLocked: false
    property bool _detentArmed: true
    property real _detentLastRaw: NaN

    function _pxFromDefault(v) {
        if (!hasDefault)
            return Infinity
        return Math.abs(v - defaultValue) / Math.abs(to - from) * _travelExtent
    }

    onPressedChanged: {
        if (pressed) {
            _detentLocked = atDefault
            _detentArmed = !atDefault
            _detentLastRaw = value
        } else {
            _detentLocked = false
            _detentArmed = true
            _detentLastRaw = NaN
        }
    }

    onValueChanged: {
        if (!hasDefault || !pressed || _detentSnapping)
            return

        const raw = value
        const distPx = _pxFromDefault(raw)
        const prev = _detentLastRaw
        const crossed = !isNaN(prev) && (
            (prev < defaultValue && raw >= defaultValue) ||
            (prev > defaultValue && raw <= defaultValue)
        )
        _detentLastRaw = raw

        if (_detentLocked) {
            if (distPx <= defaultReleasePx) {
                _detentSnapping = true
                value = defaultValue
                _detentSnapping = false
                return
            }
            _detentLocked = false
            _detentArmed = distPx >= defaultRearmPx
            return
        }

        if ((_detentArmed && distPx <= defaultCapturePx)
                || (crossed && distPx <= defaultReleasePx)) {
            _detentLocked = true
            _detentSnapping = true
            value = defaultValue
            _detentSnapping = false
            return
        }

        if (!_detentArmed && distPx >= defaultRearmPx)
            _detentArmed = true
    }

    implicitWidth: Math.max(implicitBackgroundWidth + leftInset + rightInset,
                            implicitHandleWidth + leftPadding + rightPadding)
    implicitHeight: Math.max(implicitBackgroundHeight + topInset + bottomInset,
                             implicitHandleHeight + topPadding + bottomPadding)

    padding: 6
    hoverEnabled: true
    opacity: control.enabled ? 1.0 : 0.4

    palette.window: appTheme.window
    palette.windowText: appTheme.windowText
    palette.base: appTheme.base
    palette.text: appTheme.text
    palette.highlight: appTheme.highlight
    palette.highlightedText: appTheme.highlightedText
    palette.mid: appTheme.mid
    palette.midlight: appTheme.midlight
    palette.button: appTheme.button
    palette.buttonText: appTheme.buttonText

    background: Rectangle {
        x: control.leftPadding + (control.horizontal ? 0 : (control.availableWidth - width) / 2)
        y: control.topPadding + (control.horizontal ? (control.availableHeight - height) / 2 : 0)
        implicitWidth: control.horizontal ? 200 : 4
        implicitHeight: control.horizontal ? 4 : 200
        width: control.horizontal ? control.availableWidth : implicitWidth
        height: control.horizontal ? implicitHeight : control.availableHeight
        radius: 2
        color: appTheme.midlight

        Rectangle {
            x: control.horizontal ? (control.mirrored ? parent.width - width : 0) : 0
            y: control.horizontal ? 0 : (control.mirrored ? 0 : parent.height - height)
            width: control.horizontal ? control.position * parent.width : parent.width
            height: control.horizontal ? parent.height : control.position * parent.height
            radius: 2
            color: control.enabled ? appTheme.highlight : appTheme.mid
        }

        Rectangle {
            readonly property real dotSize: 3

            visible: control.hasDefault
            width: dotSize
            height: dotSize
            radius: dotSize / 2
            antialiasing: true
            x: control.horizontal
               ? control.defaultGuideX - control.leftPadding - width / 2
               : parent.width / 2 + control._handleExtent / 2 + 1.5
            y: control.horizontal
               ? parent.height / 2 + control._handleExtent / 2 + 1.5
               : control.defaultGuideY - control.topPadding - height / 2
            color: control.atDefault
                   ? (control.enabled ? appTheme.highlight : appTheme.mid)
                   : appTheme.windowText
            opacity: control.atDefault ? 1.0 : 0.45

            Behavior on opacity { NumberAnimation { duration: 90 } }
            Behavior on color { ColorAnimation { duration: 90 } }
        }
    }

    handle: Rectangle {
        x: control.leftPadding + (control.horizontal ? control.visualPosition * (control.availableWidth - width) : (control.availableWidth - width) / 2)
        y: control.topPadding + (control.horizontal ? (control.availableHeight - height) / 2 : control.visualPosition * (control.availableHeight - height))
        implicitWidth: 16
        implicitHeight: 16
        radius: 8
        color: control.pressed ? appTheme.highlight
             : (control.hovered ? Qt.lighter(appTheme.highlight, 1.1) : appTheme.base)
        border.width: 2
        border.color: control.enabled ? appTheme.highlight : appTheme.mid

        Behavior on color { ColorAnimation { duration: 90 } }
    }
}
