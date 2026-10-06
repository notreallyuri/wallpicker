// Text filled with a left-to-right gradient (Tailwind's bg-gradient-to-r +
// bg-clip-text): the real text (so its true weight; a canvas tops out at
// bold), recolored by a small shader that keeps its smooth edges (a mask
// made them jagged). shaders/gradient-text.frag.
import QtQuick
import qs.services

Item {
    id: gradientText

    property alias text: label.text
    property color from: Theme.primary
    property color to: Theme.a(Theme.foreground, 0.8)
    property alias font: label.font

    implicitWidth: label.implicitWidth
    implicitHeight: label.implicitHeight

    Text {
        id: label
        color: "white"
        font.family: Theme.font
        font.pixelSize: 20
        // The app asks for font-black; WebKit draws the variable font's
        // black lighter than Qt draws a fixed one: ExtraBold matches it.
        font.weight: Font.ExtraBold
        font.letterSpacing: -0.5
        layer.enabled: true
        visible: false
    }
    ShaderEffect {
        anchors.fill: label
        property variant source: label
        property color from: gradientText.from
        property color to: gradientText.to
        fragmentShader: Qt.resolvedUrl("../shaders/gradient-text.frag.qsb")
    }
}
