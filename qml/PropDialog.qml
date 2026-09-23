import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import cs_app

/* Property inspector body. Rows come from CircuitCanvas.propGroups JSON. */
Rectangle {
    id: root
    function tr(s) { return App.i18nTick >= 0 ? App.translate(s) : s }
    implicitWidth: mainCol.implicitWidth + 24
    implicitHeight: mainCol.implicitHeight + 24
    color: appTheme.window

    SystemPalette { id: appTheme }

    property string itemUid: ""
    property string currentTypeText: itemUid ? CircuitCanvas.propTypeTextFor(itemUid) : CircuitCanvas.propTypeText
    property string currentDescription: itemUid ? CircuitCanvas.propDescriptionFor(itemUid) : CircuitCanvas.propDescription
    property string currentLabel: itemUid ? CircuitCanvas.propLabelFor(itemUid) : CircuitCanvas.propLabel
    property bool currentShowId: itemUid ? CircuitCanvas.propShowIdFor(itemUid) : CircuitCanvas.propShowId
    property bool currentAnyInfo: itemUid ? CircuitCanvas.propAnyInfoFor(itemUid) : CircuitCanvas.propAnyInfo

    property bool infoVisible: false
    property int currentTabIndex: 0
    property var activeGroups: []
    property var activeRows: []
    property string activeUid: ""
    property int activeTab: -1

    readonly property bool hasVisibleRows: {
        if (!activeRows || activeRows.length === 0) return false
        for (var i = 0; i < activeRows.length; i++) {
            if (activeRows[i] && activeRows[i].rowVisible !== false) return true
        }
        return false
    }

    function sameGroups(a, b) {
        if (!a || !b || a.length !== b.length) return false
        for (var i = 0; i < a.length; i++) {
            if ((a[i].name || "") !== (b[i].name || "")) return false
        }
        return true
    }

    function sameStructure(a, b) {
        if (!a || !b || a.length !== b.length) return false
        if (rowRepeater.count !== b.length) return false
        for (var i = 0; i < a.length; i++) {
            if (!a[i] || !b[i] || a[i].name !== b[i].name || a[i].kind !== b[i].kind) return false
        }
        return true
    }

    function syncProps() {
        var uid = root.itemUid.length > 0 ? root.itemUid : (CircuitCanvas.selectedUid || "")
        var groups = root.itemUid.length > 0 ? CircuitCanvas.propGroupsFor(root.itemUid) : (CircuitCanvas.propGroups || [])

        var typeText = root.itemUid.length > 0 ? CircuitCanvas.propTypeTextFor(root.itemUid) : CircuitCanvas.propTypeText
        var description = root.itemUid.length > 0 ? CircuitCanvas.propDescriptionFor(root.itemUid) : CircuitCanvas.propDescription
        var label = root.itemUid.length > 0 ? CircuitCanvas.propLabelFor(root.itemUid) : CircuitCanvas.propLabel
        var showId = root.itemUid.length > 0 ? CircuitCanvas.propShowIdFor(root.itemUid) : CircuitCanvas.propShowId
        var anyInfo = root.itemUid.length > 0 ? CircuitCanvas.propAnyInfoFor(root.itemUid) : CircuitCanvas.propAnyInfo

        if (root.currentTypeText !== typeText) root.currentTypeText = typeText
        if (root.currentDescription !== description) root.currentDescription = description
        if (root.currentLabel !== label) root.currentLabel = label
        if (root.currentShowId !== showId) root.currentShowId = showId
        if (root.currentAnyInfo !== anyInfo) root.currentAnyInfo = anyInfo

        if (!sameGroups(activeGroups, groups)) {
            activeGroups = groups
        }

        if (activeGroups.length > 0) {
            if (root.currentTabIndex >= activeGroups.length) {
                root.currentTabIndex = 0
            }
        } else {
            root.currentTabIndex = 0
        }

        var tabIdx = Math.max(0, Math.min(root.currentTabIndex, groups.length > 0 ? groups.length - 1 : 0))
        var currentTabRows = (groups.length > 0 && groups[tabIdx]) ? (groups[tabIdx].rows || []) : []

        var needsRebuild = (uid !== activeUid) || (tabIdx !== activeTab) || !sameStructure(activeRows, currentTabRows)
        activeUid = uid
        activeTab = tabIdx

        if (needsRebuild) {
            activeRows = currentTabRows
        } else {
            for (var i = 0; i < currentTabRows.length; i++) {
                var item = rowRepeater.itemAt(i)
                if (item && typeof item.updateRow === "function") {
                    item.updateRow(currentTabRows[i])
                }
            }
        }
    }

    Component.onCompleted: syncProps()

    onItemUidChanged: syncProps()

    onCurrentTabIndexChanged: syncProps()

    Connections {
        target: CircuitCanvas
        function onPropsChanged() {
            root.syncProps()
        }
    }

    component FieldLabel: Text {
        font: Qt.font({ family: App.fontFamily, pixelSize: App.fontSize })
        color: appTheme.windowText
        verticalAlignment: Text.AlignVCenter
        Layout.preferredWidth: 140
        elide: Text.ElideRight
    }

    component ShowValButton: AppToolButton {
        id: valBtn
        property var row
        onRowChanged: if (row) checked = (row.showOnCanvas === true)
        visible: row && row.canShowOnCanvas === true
        checkable: true
        checked: row ? (row.showOnCanvas === true) : false
        implicitWidth: 26; implicitHeight: 26
        contentItem: AppIcon {
            text: "visibility"
            color: valBtn.isCheckedActive ? appTheme.highlight : appTheme.windowText
        }
        ToolTip.text: root.tr("Show property value on the canvas")
        ToolTip.visible: hovered
        onToggled: {
            if (row && checked !== (row.showOnCanvas === true)) {
                if (root.itemUid.length > 0) {
                    CircuitCanvas.setShowPropFor(root.itemUid, row.name, checked)
                } else {
                    CircuitCanvas.setShowProp(row.name, checked)
                }
            }
        }
    }

    ColumnLayout {
        id: mainCol
        anchors.fill: parent
        anchors.margins: 12
        spacing: 10

        RowLayout {
            Layout.fillWidth: true
            Layout.alignment: Qt.AlignTop
            spacing: 6

            Text {
                text: root.currentTypeText
                font.family: App.fontFamily
                font.pixelSize: 15
                font.bold: true
                color: appTheme.windowText
                Layout.fillWidth: true
                elide: Text.ElideRight
            }
            AppToolButton {
                id: infoBtn
                visible: root.currentAnyInfo
                checkable: true
                checked: root.infoVisible
                implicitWidth: 26; implicitHeight: 26
                contentItem: AppIcon {
                    text: "info"
                    color: infoBtn.isCheckedActive ? appTheme.highlight : appTheme.windowText
                }
                ToolTip.text: root.tr("Show info for this component and its properties")
                ToolTip.visible: hovered
                onToggled: root.infoVisible = checked
            }
        }

        Item {
            visible: root.infoVisible && root.currentDescription.length > 0
            Layout.fillWidth: true
            Layout.alignment: Qt.AlignTop
            implicitHeight: descriptionText.implicitHeight

            Rectangle {
                anchors.left: parent.left
                anchors.top: parent.top
                anchors.bottom: parent.bottom
                width: 1
                color: appTheme.mid
            }
            Text {
                id: descriptionText
                anchors.left: parent.left
                anchors.leftMargin: 9
                anchors.right: parent.right
                text: root.currentDescription
                font: Qt.font({ family: App.fontFamily, pixelSize: App.fontSize })
                color: appTheme.windowText
                opacity: 0.65
                wrapMode: Text.WordWrap
            }
        }

        RowLayout {
            Layout.fillWidth: true
            Layout.alignment: Qt.AlignTop
            spacing: 8

            FieldLabel { text: root.tr("Label") }
            AppTextField {
                id: labelField
                text: root.currentLabel
                Layout.fillWidth: true
                Layout.preferredWidth: 200
                onEditingFinished: {
                    if (text !== root.currentLabel) {
                        if (root.itemUid.length > 0) {
                            CircuitCanvas.setPropLabelFor(root.itemUid, text)
                        } else {
                            CircuitCanvas.setPropLabel(text)
                        }
                    }
                }
                Connections {
                    target: root
                    function onCurrentLabelChanged() {
                        if (!labelField.activeFocus) labelField.text = root.currentLabel
                    }
                }
            }
            AppToolButton {
                id: showIdBtn
                checkable: true
                checked: root.currentShowId
                implicitWidth: 26; implicitHeight: 26
                contentItem: AppIcon {
                    text: "visibility"
                    color: showIdBtn.isCheckedActive ? appTheme.highlight : appTheme.windowText
                }
                ToolTip.text: root.tr("Show component label on the canvas")
                ToolTip.visible: hovered
                onToggled: {
                    if (checked !== root.currentShowId) {
                        if (root.itemUid.length > 0) {
                            CircuitCanvas.setPropShowIdFor(root.itemUid, checked)
                        } else {
                            CircuitCanvas.setPropShowId(checked)
                        }
                    }
                }
            }
        }

        // Tab bar when multiple property groups exist
        AppTabBar {
            id: tabBar
            visible: root.activeGroups && root.activeGroups.length > 1
            Layout.fillWidth: true
            Layout.alignment: Qt.AlignTop
            currentIndex: root.currentTabIndex
            onCurrentIndexChanged: {
                if (currentIndex >= 0 && currentIndex !== root.currentTabIndex) {
                    root.currentTabIndex = currentIndex
                }
            }

            Repeater {
                model: root.activeGroups
                AppTabButton {
                    required property var modelData
                    text: modelData.name || ""
                }
            }
        }

        Rectangle {
            visible: root.hasVisibleRows
            Layout.fillWidth: true
            Layout.alignment: Qt.AlignTop
            implicitWidth: groupCol.implicitWidth + 28
            implicitHeight: groupCol.implicitHeight + 28
            radius: 8
            color: appTheme.base
            border.color: appTheme.mid
            border.width: 1

            ColumnLayout {
                id: groupCol
                x: 14
                y: 14
                width: parent.width - 28
                spacing: 10

                Repeater {
                    id: rowRepeater
                    model: root.activeRows

                    ColumnLayout {
                        id: rowItem
                        required property var modelData
                        required property int index
                        property var row: modelData
                        visible: row && row.rowVisible !== false
                        Layout.fillWidth: true
                        spacing: 0

                        function updateRow(newRow) {
                            row = newRow
                            if (rowLoader.item) {
                                rowLoader.item.row = newRow
                            }
                        }

                        Loader {
                            id: rowLoader
                            Layout.fillWidth: true
                            sourceComponent: {
                                if (!rowItem.row) return null
                                switch (rowItem.row.kind) {
                                    case "bool": return boolDelegate
                                    case "enum": return enumDelegate
                                    case "num":
                                    case "number":
                                    case "double":
                                    case "int":
                                    case "uint": return numberDelegate
                                    default: return stringDelegate
                                }
                            }
                            onLoaded: if (item) item.row = rowItem.row
                        }

                        Item {
                            visible: root.infoVisible && rowItem.row && rowItem.row.info && rowItem.row.info.length > 0
                            Layout.fillWidth: true
                            Layout.topMargin: 10
                            implicitHeight: rowInfoText.implicitHeight

                            Rectangle {
                                anchors.left: parent.left
                                anchors.top: parent.top
                                anchors.bottom: parent.bottom
                                width: 1
                                color: appTheme.mid
                            }
                            Text {
                                id: rowInfoText
                                anchors.left: parent.left
                                anchors.leftMargin: 9
                                anchors.right: parent.right
                                text: rowItem.row ? rowItem.row.info : ""
                                font: Qt.font({ family: App.fontFamily, pixelSize: App.fontSize })
                                color: appTheme.windowText
                                opacity: 0.65
                                wrapMode: Text.WordWrap
                            }
                        }
                    }
                }
            }
        }

        Item {
            Layout.fillWidth: true
            Layout.fillHeight: true
        }
    }

    Component {
        id: boolDelegate
        AppCheckBox {
            id: boolBox
            property var row
            onRowChanged: if (row && checked !== row.boolValue) checked = row.boolValue
            text: row ? row.caption : ""
            font: Qt.font({ family: App.fontFamily, pixelSize: App.fontSize })
            enabled: row ? row.enabled : true
            checked: row ? row.boolValue : false
            Layout.fillWidth: true
            onToggled: {
                if (row && checked !== row.boolValue) {
                    if (root.itemUid.length > 0) {
                        CircuitCanvas.setPropBoolFor(root.itemUid, row.name, checked)
                    } else {
                        CircuitCanvas.setPropBool(row.name, checked)
                    }
                }
            }
        }
    }

    Component {
        id: enumDelegate
        RowLayout {
            id: enumRow
            property var row

            function syncEnum() {
                if (row && row.rawOptions && enumBox) {
                    for (var i = 0; i < row.rawOptions.length; i++) {
                        if (row.rawOptions[i] === row.text) {
                            if (enumBox.currentIndex !== i) enumBox.currentIndex = i
                            break
                        }
                    }
                }
            }

            onRowChanged: syncEnum()
            spacing: 8
            enabled: row ? row.enabled : true
            Layout.fillWidth: true
            FieldLabel { text: row ? row.caption : "" }
            AppComboBox {
                id: enumBox
                Layout.fillWidth: true
                Layout.preferredWidth: 200
                model: (row && row.options) ? row.options : []
                currentIndex: {
                    if (!row || !row.rawOptions) return 0
                    for (var i = 0; i < row.rawOptions.length; i++) {
                        if (row.rawOptions[i] === row.text) return i
                    }
                    return 0
                }
                onModelChanged: enumRow.syncEnum()
                Component.onCompleted: enumRow.syncEnum()
                onActivated: function(index) {
                    if (row && row.rawOptions && index >= 0 && index < row.rawOptions.length) {
                        if (row.rawOptions[index] !== row.text) {
                            if (root.itemUid.length > 0) {
                                CircuitCanvas.setPropTextFor(root.itemUid, row.name, row.rawOptions[index])
                            } else {
                                CircuitCanvas.setPropText(row.name, row.rawOptions[index])
                            }
                        }
                    }
                }
            }
            ShowValButton {
                row: parent.row
            }
        }
    }

    Component {
        id: stringDelegate
        RowLayout {
            id: strRow
            property var row
            onRowChanged: if (row && !strField.activeFocus) strField.text = row.text || ""
            spacing: 8
            enabled: row ? row.enabled : true
            Layout.fillWidth: true
            FieldLabel { text: row ? row.caption : "" }
            AppTextField {
                id: strField
                text: row ? row.text : ""
                Layout.fillWidth: true
                Layout.preferredWidth: 200
                onEditingFinished: {
                    if (row && text !== row.text) {
                        if (root.itemUid.length > 0) {
                            CircuitCanvas.setPropTextFor(root.itemUid, row.name, text)
                        } else {
                            CircuitCanvas.setPropText(row.name, text)
                        }
                    }
                }
            }
            ShowValButton {
                row: parent.row
            }
        }
    }

    Component {
        id: numberDelegate
        RowLayout {
            id: numRow
            property var row

            function syncNumber() {
                if (row && !numField.activeFocus) {
                    var t = row.text || ""
                    if (numField.text !== t) numField.text = t
                }
            }

            onRowChanged: syncNumber()
            spacing: 8
            enabled: row ? row.enabled : true
            Layout.fillWidth: true
            FieldLabel { text: row ? row.caption : "" }
            AppTextField {
                id: numField
                text: row ? (row.text || "") : ""
                Layout.fillWidth: true
                Layout.preferredWidth: (row && row.unit && row.unit.length > 0) ? 160 : 200
                onEditingFinished: {
                    if (row && text !== row.text) {
                        var val = text.trim()
                        if (root.itemUid.length > 0) {
                            CircuitCanvas.setPropTextFor(root.itemUid, row.name, val)
                        } else {
                            CircuitCanvas.setPropText(row.name, val)
                        }
                    }
                }
            }
            Text {
                id: unitLabel
                visible: row ? (row.unit !== undefined && row.unit.length > 0) : false
                text: (row && row.unit) ? row.unit : ""
                font: Qt.font({ family: App.fontFamily, pixelSize: App.fontSize })
                color: appTheme.windowText
                opacity: 0.85
                Layout.preferredWidth: (visible && text.length > 0) ? 28 : 0
                verticalAlignment: Text.AlignVCenter
                elide: Text.ElideRight
            }
            ShowValButton {
                row: parent.row
            }
        }
    }
}
