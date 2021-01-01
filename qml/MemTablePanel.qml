import QtQuick
import QtQuick.Controls
import QtQuick.Dialogs
import QtQuick.Layouts
import cs_app

// Hex view of a memory block matching C++ MemTablePanel.qml.
Rectangle {
    id: root
    function tr(s) { return App.i18nTick >= 0 ? App.translate(s) : s }
    anchors.fill: parent
    SystemPalette { id: appTheme }
    color: appTheme.window

    readonly property int cellBytes: MemoryTable.cellBytes || 1
    readonly property int cellW: 20 * cellBytes + 4
    readonly property int asciiW: 10 * cellBytes + 4
    readonly property int spacerW: 8
    readonly property int addrW: 68
    readonly property int rowH: 22

    readonly property int totalHexW: 16 * cellW
    readonly property int totalAsciiW: 16 * asciiW
    readonly property int totalContentW: addrW + totalHexW + spacerW + totalAsciiW + 24

    Timer {
        interval: 150
        running: MemoryTable.visible && CircuitCanvas.simRunning
        repeat: true
        onTriggered: {
            if (MemoryTable.activeUid) {
                var bytes = CircuitCanvas.getMemoryBytes(MemoryTable.activeUid)
                if (bytes)
                    MemoryTable.setDataBytes(bytes)
            }
        }
    }

    Connections {
        target: MemoryTable
        function onScrollTo(row) {
            listView.positionViewAtIndex(row, ListView.Contain)
        }
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 4
        spacing: 0

        // Top horizontal header
        Rectangle {
            Layout.fillWidth: true
            height: root.rowH
            color: appTheme.window
            z: 2

            Row {
                anchors.fill: parent
                spacing: 0

                // Address header corner
                Rectangle {
                    width: root.addrW
                    height: root.rowH
                    color: appTheme.window
                    Text {
                        anchors.centerIn: parent
                        text: root.tr("Addr")
                        font.family: "Ubuntu Mono"
                        font.pixelSize: 11
                        font.bold: true
                        color: appTheme.text
                        opacity: 0.7
                    }
                }

                // 16 Hex column headers
                Repeater {
                    model: 16
                    Rectangle {
                        required property int index
                        width: root.cellW
                        height: root.rowH
                        color: appTheme.window
                        Text {
                            anchors.centerIn: parent
                            text: index.toString(16).toUpperCase()
                            font.family: "Ubuntu Mono"
                            font.pixelSize: 11
                            font.bold: true
                            color: appTheme.text
                            opacity: 0.7
                        }
                    }
                }

                // Spacer header
                Rectangle {
                    width: root.spacerW
                    height: root.rowH
                    color: appTheme.window
                }

                // 16 ASCII column headers
                Repeater {
                    model: 16
                    Rectangle {
                        required property int index
                        width: root.asciiW
                        height: root.rowH
                        color: appTheme.window
                        Text {
                            anchors.centerIn: parent
                            text: index.toString(16).toUpperCase()
                            font.family: "Ubuntu Mono"
                            font.pixelSize: 11
                            font.bold: true
                            color: appTheme.text
                            opacity: 0.7
                        }
                    }
                }
            }

            Rectangle {
                anchors.left: parent.left
                anchors.right: parent.right
                anchors.bottom: parent.bottom
                height: 1
                color: appTheme.mid
            }
        }

        // Table Body
        Rectangle {
            Layout.fillWidth: true
            Layout.fillHeight: true
            color: appTheme.base
            border.color: appTheme.mid
            border.width: 1
            radius: 2
            clip: true

            ListView {
                id: listView
                anchors.fill: parent
                anchors.margins: 1
                clip: true
                boundsBehavior: Flickable.StopAtBounds
                model: MemoryTable.rowCount
                ScrollBar.vertical: AppScrollBar { policy: ScrollBar.AsNeeded }
                ScrollBar.horizontal: AppScrollBar { policy: ScrollBar.AsNeeded }

                delegate: Rectangle {
                    id: rowDelegate
                    required property int index
                    width: Math.max(listView.width, root.totalContentW)
                    height: root.rowH
                    color: index % 2 ? appTheme.alternateBase : "transparent"

                    readonly property var rowData: MemoryTable.rowData(rowDelegate.index, MemoryTable.revision)

                    Row {
                        anchors.fill: parent
                        spacing: 0

                        // Row Address label
                        Rectangle {
                            width: root.addrW
                            height: root.rowH
                            color: "transparent"

                            Text {
                                anchors.centerIn: parent
                                text: (rowDelegate.rowData && rowDelegate.rowData.address) ? rowDelegate.rowData.address : MemoryTable.rowAddress(rowDelegate.index)
                                font.family: "Ubuntu Mono"
                                font.pixelSize: 11
                                font.bold: true
                                color: appTheme.text
                                opacity: 0.7
                            }
                        }

                        // 16 Hex Cells
                        Repeater {
                            model: 16
                            Rectangle {
                                id: hexCell
                                required property int index
                                width: root.cellW
                                height: root.rowH

                                readonly property int cellAddr: rowDelegate.index * 16 + hexCell.index
                                readonly property bool isSelected: MemoryTable.selectedAddress === cellAddr
                                readonly property string cellHex: (rowDelegate.rowData && rowDelegate.rowData.bytes && hexCell.index < rowDelegate.rowData.bytes.length)
                                                                  ? rowDelegate.rowData.bytes[hexCell.index] : "--"
                                readonly property bool isDimmed: cellHex === "00" || cellHex === "FF" || cellHex === "--"

                                color: isSelected ? appTheme.highlight : (hexMouse.containsMouse ? appTheme.midlight : "transparent")

                                TextInput {
                                    id: hexInput
                                    anchors.fill: parent
                                    text: hexCell.cellHex
                                    font.family: "Ubuntu Mono"
                                    font.pixelSize: 12
                                    font.weight: Font.DemiBold
                                    horizontalAlignment: TextInput.AlignHCenter
                                    verticalAlignment: TextInput.AlignVCenter
                                    selectByMouse: true
                                    color: hexCell.isSelected ? appTheme.highlightedText
                                                              : (hexCell.isDimmed ? appTheme.mid : appTheme.windowText)

                                    onActiveFocusChanged: {
                                        if (activeFocus)
                                            MemoryTable.cellClicked(rowDelegate.index, hexCell.index)
                                    }

                                    onEditingFinished: {
                                        var val = MemoryTable.cellEdited(rowDelegate.index, hexCell.index, text)
                                        if (val >= 0 && MemoryTable.activeUid)
                                            CircuitCanvas.setMemoryByte(MemoryTable.activeUid, hexCell.cellAddr, val)
                                    }
                                }

                                MouseArea {
                                    id: hexMouse
                                    anchors.fill: parent
                                    acceptedButtons: Qt.RightButton
                                    hoverEnabled: true
                                    cursorShape: Qt.IBeamCursor

                                    onClicked: contextMenu.popup()
                                    onEntered: hexTip.text = MemoryTable.hoverText(rowDelegate.index, hexCell.index)

                                    ToolTip {
                                        id: hexTip
                                        visible: text.length > 0 && hexMouse.containsMouse
                                        delay: 350
                                    }
                                }
                            }
                        }

                        // Spacer separator
                        Rectangle {
                            width: root.spacerW
                            height: root.rowH
                            color: appTheme.window
                        }

                        // 16 ASCII Cells
                        Repeater {
                            model: 16
                            Rectangle {
                                id: asciiCell
                                required property int index
                                width: root.asciiW
                                height: root.rowH

                                readonly property int cellAddr: rowDelegate.index * 16 + asciiCell.index
                                readonly property bool isSelected: MemoryTable.selectedAddress === cellAddr
                                readonly property string cellChar: (rowDelegate.rowData && rowDelegate.rowData.ascii && asciiCell.index < rowDelegate.rowData.ascii.length)
                                                                   ? rowDelegate.rowData.ascii[asciiCell.index] : " "

                                color: isSelected ? appTheme.highlight : (asciiMouse.containsMouse ? appTheme.midlight : "transparent")

                                TextInput {
                                    id: asciiInput
                                    anchors.fill: parent
                                    text: asciiCell.cellChar
                                    font.family: "Ubuntu Mono"
                                    font.pixelSize: 12
                                    font.weight: Font.Normal
                                    horizontalAlignment: TextInput.AlignHCenter
                                    verticalAlignment: TextInput.AlignVCenter
                                    selectByMouse: true
                                    color: asciiCell.isSelected ? appTheme.highlightedText : appTheme.windowText

                                    onActiveFocusChanged: {
                                        if (activeFocus)
                                            MemoryTable.cellClicked(rowDelegate.index, asciiCell.index + 17)
                                    }

                                    onEditingFinished: {
                                        var val = MemoryTable.cellEdited(rowDelegate.index, asciiCell.index + 17, text)
                                        if (val >= 0 && MemoryTable.activeUid)
                                            CircuitCanvas.setMemoryByte(MemoryTable.activeUid, asciiCell.cellAddr, val)
                                    }
                                }

                                MouseArea {
                                    id: asciiMouse
                                    anchors.fill: parent
                                    acceptedButtons: Qt.RightButton
                                    hoverEnabled: true
                                    cursorShape: Qt.IBeamCursor

                                    onClicked: contextMenu.popup()
                                    onEntered: asciiTip.text = MemoryTable.hoverText(rowDelegate.index, asciiCell.index + 17)

                                    ToolTip {
                                        id: asciiTip
                                        visible: text.length > 0 && asciiMouse.containsMouse
                                        delay: 350
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    AppContextMenu {
        id: contextMenu
        ContextMenuItem {
            iconLigature: "save"
            text: root.tr("Save Memory Table")
            onTriggered: saveMemDialog.open()
        }
        ContextMenuItem {
            iconLigature: "folder_open"
            text: root.tr("Load Memory Table")
            onTriggered: loadMemDialog.open()
        }
    }

    FileDialog {
        id: saveMemDialog
        title: root.tr("Save Memory Table")
        fileMode: FileDialog.SaveFile
        currentFolder: App.suggestFolderUrl(CircuitCanvas.filePath)
        nameFilters: [
            root.tr("Data files (*.data)"),
            root.tr("Binary files (*.bin)"),
            root.tr("All files (*)")
        ]
        onAccepted: {
            if (selectedFile) {
                MemoryTable.saveToFile("" + selectedFile)
            }
        }
    }

    FileDialog {
        id: loadMemDialog
        title: root.tr("Load Memory Table")
        fileMode: FileDialog.OpenFile
        currentFolder: App.suggestFolderUrl(CircuitCanvas.filePath)
        nameFilters: [
            root.tr("All files (*.*)"),
            root.tr("Data files (*.data)"),
            root.tr("Hex files (*.hex *.ihx)"),
            root.tr("Binary files (*.bin)")
        ]
        onAccepted: {
            if (selectedFile) {
                if (MemoryTable.loadFromFile("" + selectedFile) && MemoryTable.activeUid) {
                    var bytes = CircuitCanvas.getMemoryBytes(MemoryTable.activeUid)
                    // If file loaded, write new bytes to canvas/sim
                    CircuitCanvas.loadItemData(MemoryTable.activeUid, "" + selectedFile)
                }
            }
        }
    }
}
