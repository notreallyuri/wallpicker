// Frosted glass (backdrop-blur-xl): what's under it, blurred, then a tint.
// `source` is the item behind (it has to be a sibling's subtree, not an
// ancestor); `origin` maps this item's corner into it.
import QtQuick
import QtQuick.Effects
import Quickshell.Widgets
import qs.services

ClippingRectangle {
    id: glass

    property Item source
    property point origin: Qt.point(0, 0)
    property color tint: Theme.a(Theme.background, 0.7)

    color: "transparent"

    ShaderEffectSource {
        id: under
        anchors.fill: parent
        sourceItem: glass.source
        sourceRect: Qt.rect(glass.origin.x, glass.origin.y, glass.width, glass.height)
        visible: false
    }
    MultiEffect {
        anchors.fill: parent
        source: under
        blurEnabled: true
        blur: 1
        blurMax: 64
    }
    Rectangle {
        anchors.fill: parent
        color: glass.tint
    }
}
