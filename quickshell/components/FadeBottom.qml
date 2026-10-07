// Holds a scrolling view and fades its bottom edge out (a mask-image
// gradient), so what scrolls in from below comes in gradually instead of
// from a hard cut. The fade shrinks as the view nears its end: the last row
// isn't left dimmed. Put the view inside, filling it, and set `flickable`.
// shaders/fade-bottom.frag.
import QtQuick

Item {
    id: fade

    property Flickable flickable
    /// How tall the fade is, in pixels.
    property real size: 56

    // What's left to scroll below the view's bottom edge.
    readonly property real below: flickable ? Math.max(0, flickable.originY + flickable.contentHeight + flickable.bottomMargin - flickable.height - flickable.contentY) : 0

    layer.enabled: true
    layer.effect: ShaderEffect {
        property real edge: fade.height > 0 ? Math.min(fade.size, fade.below) / fade.height : 0
        fragmentShader: Qt.resolvedUrl("../shaders/fade-bottom.frag.qsb")
    }
}
