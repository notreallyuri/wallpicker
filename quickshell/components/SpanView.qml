// Monitors on wallpicker's span canvas (Span.js), drawn at `mapping` ({k: px
// per mm, x, y: the mm at the top left}): each screen exactly its physical
// footprint, so what it shows is the part it gets; its frame a thin ring
// behind the screens, so monitors that touch don't cover each other. The
// spanned ones show their part of the picture (mirrored like the span);
// the others are empty. Names on chips; the picture behind the gaps
// dimmed, when asked (the Span dialog).
import QtQuick
import Quickshell.Widgets
import qs.services
import "Span.js" as Span

Item {
    id: view

    /// Every monitor drawn: [{name, x, y, w, h}] (mm).
    property var places: []
    /// The names of the ones spanned.
    property var spanned: []
    property var look: Span.initial()
    /// The picture (a file:// URL) and its size.
    property string source: ""
    property real naturalWidth: 0
    property real naturalHeight: 0
    property var mapping: Span.fit(places, width, height, 0)
    property bool showNames: true
    /// The picture behind the gaps between monitors, dimmed.
    property bool dimBehind: false
    /// A monitor shown picked (being dragged).
    property string active: ""
    /// The frame ring's width (px).
    property real ring: 3

    // Repeaters go by count (a move replaces `places`; remaking every
    // screen would reload its picture): a missing one is this.
    readonly property var none: ({
            name: "",
            x: 0,
            y: 0,
            w: 0,
            h: 0
        })
    readonly property var spannedPlaces: places.filter(p => spanned.indexOf(p.name) >= 0)
    readonly property var box: Span.bounds(spannedPlaces)
    readonly property var pic: Span.picture(look, spannedPlaces, naturalWidth, naturalHeight, view.mapping.k)
    function px(mm, axis) {
        return (mm - (axis === "x" ? mapping.x : mapping.y)) * mapping.k;
    }

    // The picture over the spanned box, mirrored like the span; placed as
    // seen from (ox, oy) in the view.
    component Picture: Item {
        property real ox: 0
        property real oy: 0
        property real dim: 1
        x: view.px(view.box.x, "x") - ox
        y: view.px(view.box.y, "y") - oy
        width: view.box.w * view.mapping.k
        height: view.box.h * view.mapping.k
        transform: Scale {
            origin.x: view.box.w * view.mapping.k / 2
            origin.y: view.box.h * view.mapping.k / 2
            xScale: view.look.flip.h ? -1 : 1
            yScale: view.look.flip.v ? -1 : 1
            Behavior on xScale {
                NumberAnimation {
                    duration: Theme.ms(300)
                    easing.type: Easing.OutCubic
                }
            }
            Behavior on yScale {
                NumberAnimation {
                    duration: Theme.ms(300)
                    easing.type: Easing.OutCubic
                }
            }
        }
        Image {
            source: view.source
            asynchronous: true
            smooth: true
            mipmap: true
            opacity: parent.dim
            x: view.pic.x
            y: view.pic.y
            width: view.pic.width
            height: view.pic.height
        }
    }

    // What the bezels hide, dimmed.
    ClippingRectangle {
        visible: view.dimBehind && view.spannedPlaces.length > 1
        x: view.px(view.box.x, "x")
        y: view.px(view.box.y, "y")
        width: view.box.w * view.mapping.k
        height: view.box.h * view.mapping.k
        color: "transparent"
        Picture {
            ox: parent.x
            oy: parent.y
            dim: 0.22
        }
    }
    // The frames, behind every screen.
    Repeater {
        model: view.places.length
        Rectangle {
            required property int index
            readonly property var modelData: view.places[index] || view.none
            x: view.px(modelData.x, "x") - view.ring
            y: view.px(modelData.y, "y") - view.ring
            width: modelData.w * view.mapping.k + 2 * view.ring
            height: modelData.h * view.mapping.k + 2 * view.ring
            radius: view.ring + 3
            color: view.active === modelData.name ? Theme.primary : "#18181b"
            border.width: 1
            border.color: Qt.rgba(1, 1, 1, 0.08)
        }
    }
    Repeater {
        model: view.places.length
        ClippingRectangle {
            id: screen
            required property int index
            readonly property var modelData: view.places[index] || view.none
            readonly property bool on: view.spanned.indexOf(modelData.name) >= 0
            x: view.px(modelData.x, "x")
            y: view.px(modelData.y, "y")
            width: modelData.w * view.mapping.k
            height: modelData.h * view.mapping.k
            radius: 3
            color: "#27272a"
            Picture {
                visible: screen.on
                ox: screen.x
                oy: screen.y
            }
            // Out of the span: a dim, empty screen.
            Rectangle {
                visible: !screen.on
                anchors.fill: parent
                color: Qt.rgba(0, 0, 0, 0.35)
            }
            // A thin dark edge: where monitors touch, a seam.
            Rectangle {
                anchors.fill: parent
                radius: screen.radius
                color: "transparent"
                border.width: 1
                border.color: Qt.rgba(0, 0, 0, 0.55)
            }
            Rectangle {
                visible: view.showNames
                x: 5
                y: 5
                width: label.implicitWidth + 10
                height: label.implicitHeight + 4
                radius: height / 2
                color: Qt.rgba(0, 0, 0, 0.6)
                Text {
                    id: label
                    anchors.centerIn: parent
                    text: screen.modelData.name
                    color: screen.on ? "white" : Qt.rgba(1, 1, 1, 0.6)
                    font.family: Theme.font
                    font.pixelSize: 10
                    font.weight: Font.Bold
                }
            }
        }
    }
}
