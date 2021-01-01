import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import cs_app

/* Tab bar + one editor. C++ had a Repeater of CodeEditor ctx objects; the
 * current buffer is EditorPanel.currentText so typing does not rebuild tabs. */
Item {
    id: root
    SystemPalette { id: appTheme }
    function tr(s) { return App.i18nTick >= 0 ? App.translate(s) : s }
    readonly property var ctx: EditorPanel

    // pollLsp also drains the background compiler. Keep polling while a
    // compile is running even if the editor overlay is hidden, otherwise the
    // job would finish silently until the next time CodeEditor is shown.
    Timer {
        interval: 50
        running: root.ctx && root.ctx.compiling
        repeat: true
        onTriggered: EditorPanel.pollLsp()
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 4
        spacing: 4

        RowLayout {
            Layout.fillWidth: true
            spacing: 4

            AppTabBar {
                id: docTabs
                Layout.fillWidth: true
                Layout.alignment: Qt.AlignVCenter
                Layout.minimumHeight: 24
                Layout.preferredHeight: Math.max( 24, docTabs.implicitHeight )
                onCurrentIndexChanged: if ( root.ctx ) root.ctx.currentDocument = currentIndex

                Repeater {
                    model: root.ctx ? root.ctx.documents : []
                    delegate: AppTabButton {
                        id: tabBtn
                        required property var modelData
                        required property int index
                        text: modelData.title
                        rightPadding: 24 + (modelData.hasSettings ? 18 : 0) + (modelData.hasCompiler ? 18 : 0)

                        readonly property color itemColor: tabBtn.checked ? appTheme.highlightedText : appTheme.windowText

                        AppToolButton {
                            id: uploadRunBtn
                            visible: modelData.hasCompiler === true
                            anchors.right: settingsBtn.left
                            anchors.verticalCenter: parent.verticalCenter
                            anchors.rightMargin: 2
                            width: 16; height: 16
                            padding: 0
                            radius: 3
                            ToolTip.text: EditorPanel.compiling
                                          ? root.tr("Cancel compilation")
                                          : root.tr("Upload and Run on the active device")
                            ToolTip.delay: 500
                            contentItem: AppIcon {
                                text: EditorPanel.compiling ? "stop_circle" : "play_arrow"
                                font.pixelSize: 13
                                color: tabBtn.itemColor
                            }
                            onClicked: {
                                if (EditorPanel.compiling) {
                                    EditorPanel.cancelCompile()
                                } else {
                                    docTabs.currentIndex = index
                                    EditorPanel.uploadRun()
                                }
                            }
                        }
                        AppToolButton {
                            id: settingsBtn
                            visible: modelData.hasSettings === true
                            anchors.right: closeBtn.left
                            anchors.verticalCenter: parent.verticalCenter
                            anchors.rightMargin: 2
                            width: 16; height: 16
                            padding: 0
                            radius: 3
                            ToolTip.text: root.tr("File Settings")
                            ToolTip.delay: 500
                            contentItem: AppIcon {
                                text: "settings"
                                font.pixelSize: 13
                                color: tabBtn.itemColor
                            }
                            onClicked: {
                                docTabs.currentIndex = index
                                EditorPanel.fileProps()
                            }
                        }
                        AppToolButton {
                            id: closeBtn
                            anchors.right: parent.right
                            anchors.verticalCenter: parent.verticalCenter
                            anchors.rightMargin: 2
                            width: 16; height: 16
                            padding: 0
                            radius: 3
                            ToolTip.text: root.tr( "Close" )
                            ToolTip.delay: 500
                            contentItem: Text {
                                text: "×"
                                font.pixelSize: 13
                                font.bold: true
                                color: tabBtn.itemColor
                                horizontalAlignment: Text.AlignHCenter
                                verticalAlignment: Text.AlignVCenter
                            }
                            onClicked: ctx.closeDocument( index )
                        }
                    }
                }
            }

            Rectangle {
                Layout.alignment: Qt.AlignVCenter
                width: 1
                height: 16
                color: appTheme.mid
                opacity: 0.6
            }

            AppToolButton {
                id: newFileBtn
                Layout.alignment: Qt.AlignVCenter
                implicitWidth: 20
                implicitHeight: 20
                width: 20
                height: 20
                padding: 0
                radius: 4
                ToolTip.text: root.tr( "New File" )
                ToolTip.delay: 500
                contentItem: AppIcon {
                    text: "add"
                    font.pixelSize: 14
                    color: newFileBtn.textColor
                }
                onClicked: EditorPanel.newFile()
            }
        }
        Binding {
            target: docTabs
            property: "currentIndex"
            value: root.ctx ? root.ctx.currentDocument : 0
            restoreMode: Binding.RestoreNone
        }

        Rectangle {
            Layout.fillWidth: true
            Layout.fillHeight: true
            radius: 8
            color: appTheme.base
            border.color: appTheme.mid
            border.width: 1
            clip: true
            visible: root.ctx && root.ctx.currentDocument >= 0

            CodeEditor {
                anchors.fill: parent
            }
        }

        Label {
            Layout.fillWidth: true
            Layout.fillHeight: true
            visible: !( root.ctx && root.ctx.currentDocument >= 0 )
            text: root.tr( "Open a file from the Files tab, or click + for a new buffer." )
            color: appTheme.windowText
            wrapMode: Text.WordWrap
            horizontalAlignment: Text.AlignHCenter
            verticalAlignment: Text.AlignVCenter
            opacity: 0.6
        }
    }
}
