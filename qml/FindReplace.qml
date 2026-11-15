import QtQuick
import QtQuick.Layouts
import cs_app
import cs_ui

Rectangle {
    id: root
    SystemPalette { id: appTheme }
    readonly property font uiFont: Qt.font({ family: App.fontFamily, pixelSize: App.fontNormal })
    implicitWidth: Math.max(460, content.implicitWidth + 24)
    implicitHeight: content.implicitHeight + 24
    color: appTheme.window

    ColumnLayout {
        id: content
        anchors.fill: parent
        anchors.margins: 12
        spacing: 8

        AppText {
            text: I18n.tr("Find")
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
            AppButton { text: I18n.tr("Previous"); font: root.uiFont; onClicked: EditorPanel.findPrev() }
            AppButton { text: I18n.tr("Next"); font: root.uiFont; onClicked: EditorPanel.findNext() }
            AppButton { text: I18n.tr("All"); font: root.uiFont; onClicked: EditorPanel.findAll() }
        }

        AppText {
            text: I18n.tr("Replace")
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
            AppButton { text: I18n.tr("Replace"); font: root.uiFont; onClicked: EditorPanel.replace() }
            AppButton { text: I18n.tr("Repl/Find"); font: root.uiFont; onClicked: EditorPanel.replaceAndFind() }
            AppButton { text: I18n.tr("Repl All"); font: root.uiFont; onClicked: EditorPanel.replaceAll() }
        }

        RowLayout {
            Layout.fillWidth: true
            Layout.topMargin: 4
            spacing: 12
            AppCheckBox {
                text: I18n.tr("Case Sensitive")
                font: root.uiFont
                checked: EditorPanel.findCaseSensitive
                onToggled: EditorPanel.findCaseSensitive = checked
            }
            AppCheckBox {
                text: I18n.tr("Whole Words")
                font: root.uiFont
                checked: EditorPanel.findWholeWords
                onToggled: EditorPanel.findWholeWords = checked
            }
            AppCheckBox {
                text: I18n.tr("Regexp")
                font: root.uiFont
                checked: EditorPanel.findRegexp
                onToggled: EditorPanel.findRegexp = checked
            }
            Item { Layout.fillWidth: true }
            AppButton { text: I18n.tr("Close"); font: root.uiFont; onClicked: EditorPanel.closeFind() }
        }
    }
}
