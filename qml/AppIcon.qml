import QtQuick

/* Reusable Material Symbols Rounded icon component.
 *
 * Configured with font hinting disabled and high-quality Qt distance field
 * rendering to ensure crisp, clean vector curves on all platforms (especially Windows).
 */
Text {
    id: root

    property alias icon: root.text

    font.family: "Material Symbols Rounded"
    font.pixelSize: 16
    font.hintingPreference: Font.PreferNoHinting

    renderType: Text.QtRendering

    horizontalAlignment: Text.AlignHCenter
    verticalAlignment: Text.AlignVCenter
}
