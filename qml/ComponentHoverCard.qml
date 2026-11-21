pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import cs_app
import cs_ui

Item {
    id: root

    property var info: null
    property bool active: false
    property real anchorY: 0
    property rect panelRect: Qt.rect(0, 0, 0, 0)
    property bool preferRight: true
    property real topInset: 0

    readonly property int minCardWidth: 210
    readonly property int maxCardWidth: 300
    readonly property int padX: 12
    readonly property int padY: 10
    readonly property int panelGap: 10
    readonly property int edgeMargin: 8

    readonly property string titleText: (root.info && root.info.title) ? ("" + root.info.title) : ""
    readonly property string descText: (root.info && root.info.description) ? ("" + root.info.description) : ""
    readonly property string categoryText: (root.info && root.info.category) ? ("" + root.info.category) : ""
    readonly property string iconSource: (root.info && root.info.iconUri) ? ("" + root.info.iconUri) : ""
    readonly property string shortcutText: (root.info && root.info.shortcut) ? ("" + root.info.shortcut) : ""
    readonly property bool isCustom: Boolean(root.info && root.info.isCustom)

    SystemPalette { id: appTheme }

    enabled: false
    z: 1000
    visible: root.active && (root.titleText.length > 0 || root.descText.length > 0)

    TextMetrics {
        id: titleMetrics
        font.family: App.fontFamily
        font.pixelSize: App.fontNormal
        font.bold: true
        text: root.titleText
    }

    TextMetrics {
        id: categoryMetrics
        font.family: App.fontFamily
        font.pixelSize: App.fontSmall
        text: root.categoryText
    }

    TextMetrics {
        id: descMetrics
        font.family: App.fontFamily
        font.pixelSize: App.fontSmall
        text: root.descText
    }

    readonly property real naturalContentWidth: {
        var iconSlot = root.iconSource.length > 0 ? 42 : 0
        var badgeSlot = (root.isCustom ? (customBadge.implicitWidth + 6) : 0)
            + (root.shortcutText.length > 0 ? (shortcutBadge.implicitWidth + 6) : 0)
        var headerW = iconSlot + Math.max(titleMetrics.advanceWidth + badgeSlot, categoryMetrics.advanceWidth)
        return Math.max(headerW, descMetrics.advanceWidth)
    }

    implicitWidth: Math.min(root.maxCardWidth, Math.max(root.minCardWidth, Math.ceil(root.naturalContentWidth) + 2 * root.padX + 2))
    implicitHeight: Math.ceil(contentCol.implicitHeight) + 2 * root.padY
    width: implicitWidth
    height: implicitHeight

    x: {
        var pw = root.parent ? root.parent.width : 0
        var rightX = root.panelRect.x + root.panelRect.width + root.panelGap
        var leftX = root.panelRect.x - root.width - root.panelGap
        var fitsRight = rightX + root.width <= pw - root.edgeMargin
        var fitsLeft = leftX >= root.edgeMargin
        if (root.preferRight) {
            if (fitsRight) return rightX
            if (fitsLeft) return leftX
        } else {
            if (fitsLeft) return leftX
            if (fitsRight) return rightX
        }
        return Math.max(root.edgeMargin, Math.min(pw - root.width - root.edgeMargin, root.preferRight ? rightX : leftX))
    }

    y: {
        var ph = root.parent ? root.parent.height : 0
        var minY = root.topInset + root.edgeMargin
        var maxY = Math.max(minY, ph - root.height - root.edgeMargin)
        return Math.max(minY, Math.min(maxY, Math.round(root.anchorY - root.height / 2)))
    }

    Rectangle {
        anchors.fill: parent
        color: appTheme.base
        opacity: 0.96
        border.color: appTheme.mid
        border.width: 1
        radius: 10
    }

    ColumnLayout {
        id: contentCol
        x: root.padX
        y: root.padY
        width: root.width - 2 * root.padX
        spacing: 8

        RowLayout {
            Layout.fillWidth: true
            spacing: 10

            Rectangle {
                visible: root.iconSource.length > 0
                Layout.preferredWidth: 32
                Layout.preferredHeight: 32
                Layout.alignment: Qt.AlignVCenter
                radius: 7
                color: Qt.rgba(appTheme.windowText.r, appTheme.windowText.g, appTheme.windowText.b, 0.07)

                ItemDelegate {
                    anchors.centerIn: parent
                    width: 22
                    height: 22
                    padding: 0
                    display: AbstractButton.IconOnly
                    icon.source: root.iconSource
                    icon.width: 22
                    icon.height: 22
                    icon.color: appTheme.windowText
                    background: null
                    hoverEnabled: false
                    enabled: false
                }
            }

            ColumnLayout {
                Layout.fillWidth: true
                Layout.alignment: Qt.AlignVCenter
                spacing: 2

                RowLayout {
                    Layout.fillWidth: true
                    spacing: 6

                    AppText {
                        text: root.titleText
                        font.bold: true
                        color: appTheme.windowText
                        wrapMode: Text.WordWrap
                        Layout.fillWidth: true
                    }

                    Rectangle {
                        id: shortcutBadge
                        visible: root.shortcutText.length > 0
                        Layout.alignment: Qt.AlignVCenter
                        implicitWidth: Math.max(20, shortcutBadgeLabel.implicitWidth + 10)
                        implicitHeight: shortcutBadgeLabel.implicitHeight + 4
                        radius: 4
                        color: Qt.rgba(appTheme.windowText.r, appTheme.windowText.g, appTheme.windowText.b, 0.08)
                        border.color: Qt.rgba(appTheme.windowText.r, appTheme.windowText.g, appTheme.windowText.b, 0.22)
                        border.width: 1

                        AppText {
                            id: shortcutBadgeLabel
                            anchors.centerIn: parent
                            text: root.shortcutText
                            font.pixelSize: App.fontMicro
                            font.bold: true
                            color: appTheme.windowText
                        }
                    }

                    Rectangle {
                        id: customBadge
                        visible: root.isCustom
                        Layout.alignment: Qt.AlignVCenter
                        implicitWidth: customBadgeLabel.implicitWidth + 10
                        implicitHeight: customBadgeLabel.implicitHeight + 4
                        radius: 4
                        color: Qt.rgba(appTheme.highlight.r, appTheme.highlight.g, appTheme.highlight.b, 0.16)
                        border.color: Qt.rgba(appTheme.highlight.r, appTheme.highlight.g, appTheme.highlight.b, 0.35)
                        border.width: 1

                        AppText {
                            id: customBadgeLabel
                            anchors.centerIn: parent
                            text: I18n.tr("Custom")
                            font.pixelSize: App.fontMicro
                            font.bold: true
                            color: appTheme.highlight
                        }
                    }
                }

                AppText {
                    visible: root.categoryText.length > 0
                    text: root.categoryText
                    font.pixelSize: App.fontSmall
                    color: appTheme.windowText
                    opacity: 0.62
                    elide: Text.ElideMiddle
                    Layout.fillWidth: true
                }
            }
        }

        Rectangle {
            visible: root.descText.length > 0
            Layout.fillWidth: true
            Layout.preferredHeight: 1
            color: appTheme.mid
            opacity: 0.35
        }

        AppText {
            visible: root.descText.length > 0
            text: root.descText
            font.pixelSize: App.fontSmall
            color: appTheme.text
            opacity: 0.92
            wrapMode: Text.WordWrap
            Layout.fillWidth: true
        }
    }
}
