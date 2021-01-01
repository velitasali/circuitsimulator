import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import cs_app

Rectangle {
    id: root
    function tr(s) { return App.i18nTick >= 0 ? App.translate(s) : s }
    SystemPalette { id: appTheme }
    readonly property font uiFont: Qt.font({ family: App.fontFamily || "Ubuntu", pixelSize: App.fontSize || 13 })
    implicitWidth: Math.max(460, content.implicitWidth + 24)
    implicitHeight: content.implicitHeight + 24
    color: appTheme.window

    ColumnLayout {
        id: content
        anchors.fill: parent
        anchors.margins: 12
        spacing: 8

        Text {
            text: root.tr("Find")
            font.family: root.uiFont.family
            font.pixelSize: root.uiFont.pixelSize
            font.bold: true
            color: appTheme.windowText
        }
        RowLayout {
            Layout.fillWidth: true
            spacing: 6
            AppTextField {
                text: EditorPanel.findText
                font: root.uiFont
                Layout.fillWidth: true
                focus: true
                onTextEdited: EditorPanel.findText = text
                onAccepted: EditorPanel.findNext()
            }
            AppButton { text: root.tr("Previous"); font: root.uiFont; onClicked: EditorPanel.findPrev() }
            AppButton { text: root.tr("Next"); font: root.uiFont; onClicked: EditorPanel.findNext() }
            AppButton { text: root.tr("All"); font: root.uiFont; onClicked: EditorPanel.findAll() }
        }

        Text {
            text: root.tr("Replace")
            font.family: root.uiFont.family
            font.pixelSize: root.uiFont.pixelSize
            font.bold: true
            color: appTheme.windowText
            Layout.topMargin: 4
        }
        RowLayout {
            Layout.fillWidth: true
            spacing: 6
            AppTextField {
                text: EditorPanel.replaceText
                font: root.uiFont
                Layout.fillWidth: true
                onTextEdited: EditorPanel.replaceText = text
            }
            AppButton { text: root.tr("Replace"); font: root.uiFont; onClicked: EditorPanel.replace() }
            AppButton { text: root.tr("Repl/Find"); font: root.uiFont; onClicked: EditorPanel.replaceAndFind() }
            AppButton { text: root.tr("Repl All"); font: root.uiFont; onClicked: EditorPanel.replaceAll() }
        }

        RowLayout {
            Layout.fillWidth: true
            Layout.topMargin: 4
            spacing: 12
            AppCheckBox {
                text: root.tr("Case Sensitive")
                font: root.uiFont
                checked: EditorPanel.findCaseSensitive
                onToggled: EditorPanel.findCaseSensitive = checked
            }
            AppCheckBox {
                text: root.tr("Whole Words")
                font: root.uiFont
                checked: EditorPanel.findWholeWords
                onToggled: EditorPanel.findWholeWords = checked
            }
            AppCheckBox {
                text: root.tr("Regexp")
                font: root.uiFont
                checked: EditorPanel.findRegexp
                onToggled: EditorPanel.findRegexp = checked
            }
            Item { Layout.fillWidth: true }
            AppButton { text: root.tr("Close"); font: root.uiFont; onClicked: EditorPanel.closeFind() }
        }
    }
}
