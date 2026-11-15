pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import cs_app
import cs_ui

/* Property inspector body. Rows come from CircuitCanvas.propGroups JSON. */
Rectangle {
    id: root
    implicitWidth: mainCol.implicitWidth + 24
    implicitHeight: mainCol.implicitHeight + 24
    color: appTheme.window

    SystemPalette { id: appTheme }
    readonly property SystemPalette appTheme: appTheme

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

    function sameGroups(a: var, b: var): bool {
        if (!a || !b || a.length !== b.length) return false
        for (var i = 0; i < a.length; i++) {
            if ((a[i].name || "") !== (b[i].name || "")) return false
        }
        return true
    }

    function sameStructure(a: var, b: var): bool {
        if (!a || !b || a.length !== b.length) return false
        if (rowRepeater.count !== b.length) return false
        for (var i = 0; i < a.length; i++) {
            if (!a[i] || !b[i] || a[i].name !== b[i].name || a[i].kind !== b[i].kind) return false
        }
        return true
    }

    function syncProps(): void {
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
                const item = rowRepeater.itemAt(i) as PropRow
                if (item) {
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
        function onPropsChanged(): void {
            root.syncProps()
        }
    }

    component FieldLabel: AppText {
        color: root.appTheme.windowText
        verticalAlignment: Text.AlignVCenter
        Layout.preferredWidth: 140
        elide: Text.ElideRight
    }

    component ShowValButton: AppToolButton {
        id: valBtn
        property var row: null
        onRowChanged: if (valBtn.row) valBtn.checked = (valBtn.row.showOnCanvas === true)
        visible: valBtn.row && valBtn.row.canShowOnCanvas === true
        checkable: true
        checked: valBtn.row ? (valBtn.row.showOnCanvas === true) : false
        implicitWidth: 26; implicitHeight: 26
        contentItem: AppIcon {
            icon: "visibility"
            color: valBtn.isCheckedActive ? root.appTheme.highlight : root.appTheme.windowText
        }
        ToolTip.text: I18n.tr("Show property value on the canvas")
        ToolTip.visible: valBtn.hovered
        onToggled: {
            if (valBtn.row && valBtn.checked !== (valBtn.row.showOnCanvas === true)) {
                if (root.itemUid.length > 0) {
                    CircuitCanvas.setShowPropFor(root.itemUid, valBtn.row.name, valBtn.checked)
                } else {
                    CircuitCanvas.setShowProp(valBtn.row.name, valBtn.checked)
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

            AppText {
                text: root.currentTypeText
                font.pixelSize: App.fontTitle
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
                    icon: "info"
                    color: infoBtn.isCheckedActive ? appTheme.highlight : appTheme.windowText
                }
                ToolTip.text: I18n.tr("Show info for this component and its properties")
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
            AppText {
                id: descriptionText
                anchors.left: parent.left
                anchors.leftMargin: 9
                anchors.right: parent.right
                text: root.currentDescription
                font.pixelSize: App.fontSmall
                color: appTheme.windowText
                opacity: 0.65
                wrapMode: Text.WordWrap
            }
        }

        RowLayout {
            Layout.fillWidth: true
            Layout.alignment: Qt.AlignTop
            spacing: 8

            FieldLabel { text: I18n.tr("Label") }
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
                    icon: "visibility"
                    color: showIdBtn.isCheckedActive ? appTheme.highlight : appTheme.windowText
                }
                ToolTip.text: I18n.tr("Show component label on the canvas")
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
                    id: groupTabBtn
                    required property var modelData
                    text: groupTabBtn.modelData.name || ""
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

                component PropRow: ColumnLayout {
                    id: rowItem
                    required property var modelData
                    required property int index
                    property var row: modelData
                    visible: rowItem.row && rowItem.row.rowVisible !== false
                    Layout.fillWidth: true
                    spacing: 0

                    function applyRowToItem(obj: QtObject, newRow: var): void {
                        if (!obj) return
                        const kind = newRow ? newRow.kind : ""
                        switch (kind) {
                            case "bool": (obj as PropBoolRow).row = newRow; break
                            case "enum": (obj as PropEnumRow).row = newRow; break
                            case "num":
                            case "number":
                            case "double":
                            case "int":
                            case "uint": (obj as PropNumRow).row = newRow; break
                            default: (obj as PropStrRow).row = newRow; break
                        }
                    }

                    function updateRow(newRow: var): void {
                        rowItem.row = newRow
                        rowItem.applyRowToItem(rowLoader.item, newRow)
                    }

                    Loader {
                        id: rowLoader
                        Layout.fillWidth: true
                        sourceComponent: {
                            if (!rowItem.row) return null
                            switch (rowItem.row.kind) {
                                case "bool": return root.boolDelegate
                                case "enum": return root.enumDelegate
                                case "num":
                                case "number":
                                case "double":
                                case "int":
                                case "uint": return root.numberDelegate
                                default: return root.stringDelegate
                            }
                        }
                        onLoaded: rowItem.applyRowToItem(rowLoader.item, rowItem.row)
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
                            color: root.appTheme.mid
                        }
                        AppText {
                            id: rowInfoText
                            anchors.left: parent.left
                            anchors.leftMargin: 9
                            anchors.right: parent.right
                            text: rowItem.row ? rowItem.row.info : ""
                            font.pixelSize: App.fontSmall
                            color: root.appTheme.windowText
                            opacity: 0.65
                            wrapMode: Text.WordWrap
                        }
                    }
                }

                Repeater {
                    id: rowRepeater
                    model: root.activeRows
                    delegate: PropRow {}
                }
            }
        }

        Item {
            Layout.fillWidth: true
            Layout.fillHeight: true
        }
    }

    component PropBoolRow: AppCheckBox {
        id: boolBox
        property var row: null
        onRowChanged: if (boolBox.row && boolBox.checked !== boolBox.row.boolValue) boolBox.checked = boolBox.row.boolValue
        text: boolBox.row ? boolBox.row.caption : ""
        enabled: boolBox.row ? boolBox.row.enabled : true
        checked: boolBox.row ? boolBox.row.boolValue : false
        Layout.fillWidth: true
        onToggled: {
            if (boolBox.row && boolBox.checked !== boolBox.row.boolValue) {
                if (root.itemUid.length > 0) {
                    CircuitCanvas.setPropBoolFor(root.itemUid, boolBox.row.name, boolBox.checked)
                } else {
                    CircuitCanvas.setPropBool(boolBox.row.name, boolBox.checked)
                }
            }
        }
    }

    component PropEnumRow: RowLayout {
        id: enumRow
        property var row: null

        function syncEnum(): void {
            if (enumRow.row && enumRow.row.rawOptions && enumBox) {
                for (var i = 0; i < enumRow.row.rawOptions.length; i++) {
                    if (enumRow.row.rawOptions[i] === enumRow.row.text) {
                        if (enumBox.currentIndex !== i) enumBox.currentIndex = i
                        break
                    }
                }
            }
        }

        onRowChanged: enumRow.syncEnum()
        spacing: 8
        enabled: enumRow.row ? enumRow.row.enabled : true
        Layout.fillWidth: true
        FieldLabel { text: enumRow.row ? enumRow.row.caption : "" }
        AppComboBox {
            id: enumBox
            Layout.fillWidth: true
            Layout.preferredWidth: 200
            model: (enumRow.row && enumRow.row.options) ? enumRow.row.options : []
            currentIndex: {
                if (!enumRow.row || !enumRow.row.rawOptions) return 0
                for (var i = 0; i < enumRow.row.rawOptions.length; i++) {
                    if (enumRow.row.rawOptions[i] === enumRow.row.text) return i
                }
                return 0
            }
            onModelChanged: enumRow.syncEnum()
            Component.onCompleted: enumRow.syncEnum()
            onActivated: function(index: int): void {
                if (enumRow.row && enumRow.row.rawOptions && index >= 0 && index < enumRow.row.rawOptions.length) {
                    if (enumRow.row.rawOptions[index] !== enumRow.row.text) {
                        if (root.itemUid.length > 0) {
                            CircuitCanvas.setPropTextFor(root.itemUid, enumRow.row.name, enumRow.row.rawOptions[index])
                        } else {
                            CircuitCanvas.setPropText(enumRow.row.name, enumRow.row.rawOptions[index])
                        }
                    }
                }
            }
        }
        ShowValButton {
            row: enumRow.row
        }
    }

    component PropStrRow: RowLayout {
        id: strRow
        property var row: null
        onRowChanged: if (strRow.row && !strField.activeFocus) strField.text = strRow.row.text || ""
        spacing: 8
        enabled: strRow.row ? strRow.row.enabled : true
        Layout.fillWidth: true
        FieldLabel { text: strRow.row ? strRow.row.caption : "" }
        AppTextField {
            id: strField
            text: strRow.row ? strRow.row.text : ""
            Layout.fillWidth: true
            Layout.preferredWidth: 200
            onEditingFinished: {
                if (strRow.row && strField.text !== strRow.row.text) {
                    if (root.itemUid.length > 0) {
                        CircuitCanvas.setPropTextFor(root.itemUid, strRow.row.name, strField.text)
                    } else {
                        CircuitCanvas.setPropText(strRow.row.name, strField.text)
                    }
                }
            }
        }
        ShowValButton {
            row: strRow.row
        }
    }

    component PropNumRow: RowLayout {
        id: numRow
        property var row: null

        function syncNumber(): void {
            if (numRow.row && !numField.activeFocus) {
                var t = numRow.row.text || ""
                if (numField.text !== t) numField.text = t
            }
        }

        onRowChanged: numRow.syncNumber()
        spacing: 8
        enabled: numRow.row ? numRow.row.enabled : true
        Layout.fillWidth: true
        FieldLabel { text: numRow.row ? numRow.row.caption : "" }
        AppTextField {
            id: numField
            text: numRow.row ? (numRow.row.text || "") : ""
            Layout.fillWidth: true
            Layout.preferredWidth: (numRow.row && numRow.row.unit && numRow.row.unit.length > 0) ? 160 : 200
            onEditingFinished: {
                if (numRow.row && numField.text !== numRow.row.text) {
                    var val = numField.text.trim()
                    if (root.itemUid.length > 0) {
                        CircuitCanvas.setPropTextFor(root.itemUid, numRow.row.name, val)
                    } else {
                        CircuitCanvas.setPropText(numRow.row.name, val)
                    }
                }
            }
        }
        AppText {
            id: unitLabel
            visible: numRow.row ? (numRow.row.unit !== undefined && numRow.row.unit.length > 0) : false
            text: (numRow.row && numRow.row.unit) ? numRow.row.unit : ""
            color: root.appTheme.windowText
            opacity: 0.85
            Layout.preferredWidth: (unitLabel.visible && unitLabel.text.length > 0) ? 28 : 0
            verticalAlignment: Text.AlignVCenter
            elide: Text.ElideRight
        }
        ShowValButton {
            row: numRow.row
        }
    }

    readonly property Component boolDelegate: Component { PropBoolRow {} }
    readonly property Component enumDelegate: Component { PropEnumRow {} }
    readonly property Component stringDelegate: Component { PropStrRow {} }
    readonly property Component numberDelegate: Component { PropNumRow {} }
}
