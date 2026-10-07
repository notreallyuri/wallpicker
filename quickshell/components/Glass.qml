// Frosted glass (backdrop-blur-xl): everything behind it, blurred, then a
// tint. Behind is the panel's color, the wallpaper over it (`backdrop`),
// and the content (`source`); each has to be a sibling's subtree, not an
// ancestor. They're taken `pad` wider than the glass all round and blurred
// before it's cut out: blurring only what's inside left its edges lighter,
// a rim that read as a pane of glass in front, not a blur.
import QtQuick
import QtQuick.Effects
import Quickshell.Widgets
import qs.services

ClippingRectangle {
    id: glass

    /// The content behind (the grid).
    property Item source
    /// What's behind that (the panel's wallpaper), and the color under it.
    property Item backdrop
    property color base: Theme.background
    /// Anything whose change moves the glass relative to them (it's
    /// measured again then): positions in other items don't notify.
    property var follow
    /// Frosted (Settings → Performance): off, a solid tint and no blur,
    /// which is most of the CPU while scrolling.
    readonly property bool frosted: Prefs.glass
    property color tint: Theme.a(Theme.background, frosted ? 0.7 : 0.94)
    property real blurMax: 64

    readonly property real pad: blurMax
    // The captured area's corner in `item`.
    function corner(item) {
        return item ? glass.mapToItem(item, -pad, -pad) : Qt.point(0, 0);
    }
    readonly property point sourceCorner: {
        glass.follow;
        glass.x;
        glass.y;
        return glass.corner(glass.source);
    }
    readonly property point backdropCorner: {
        glass.follow;
        glass.x;
        glass.y;
        return glass.corner(glass.backdrop);
    }

    color: "transparent"

    // Everything behind, a margin wider than the glass.
    Item {
        id: behind
        x: -glass.pad
        y: -glass.pad
        width: glass.width + 2 * glass.pad
        height: glass.height + 2 * glass.pad
        layer.enabled: glass.frosted
        visible: false
        Rectangle {
            anchors.fill: parent
            color: glass.base
        }
        ShaderEffectSource {
            visible: glass.backdrop !== null
            live: glass.frosted
            anchors.fill: parent
            sourceItem: glass.backdrop
            sourceRect: Qt.rect(glass.backdropCorner.x, glass.backdropCorner.y, behind.width, behind.height)
        }
        ShaderEffectSource {
            live: glass.frosted
            anchors.fill: parent
            sourceItem: glass.source
            sourceRect: Qt.rect(glass.sourceCorner.x, glass.sourceCorner.y, behind.width, behind.height)
        }
    }
    MultiEffect {
        visible: glass.frosted
        x: behind.x
        y: behind.y
        width: behind.width
        height: behind.height
        source: behind
        blurEnabled: true
        blur: 1
        blurMax: glass.blurMax
    }
    Rectangle {
        anchors.fill: parent
        color: glass.tint
    }
}
