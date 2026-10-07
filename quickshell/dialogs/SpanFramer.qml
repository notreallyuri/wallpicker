// The span's Adjust preview: the monitors laid out as they are (positions,
// sizes, the bezel gap), each showing its part of the picture, mirrored
// like the span; what falls between them (behind the bezels) dimmed.
// Drag to move the picture, scroll to zoom; `settled` after each move.
import QtQuick
import Quickshell.Widgets
import qs.services
import "Frame.js" as Frame
import "Span.js" as Span

Item {
    id: framer

    property string path
    /// {zoom, x, y, flip: {h, v}}
    property var look: Span.initial()
    /// The look moved (each step of a drag, each notch of a zoom).
    signal moved(var look)
    /// A drag or a zoom ended: time to show it on the monitors.
    signal settled

    readonly property var layout: Span.layout(Monitors.list, Settings.data.span_gap || 0)
    /// Layout pixels to preview pixels: the layout fitted in 528×220.
    readonly property real k: Math.min(528 / Math.max(layout.width, 1), 220 / Math.max(layout.height, 1))
    readonly property real boxWidth: layout.width * k
    readonly property real boxHeight: layout.height * k

    // The picture's size: the original's, else the framing copy's.
    readonly property var original: Library.images.find(i => i.path === path)
    property real loadedWidth: 0
    property real loadedHeight: 0
    readonly property real naturalWidth: original && original.width > 0 ? original.width : loadedWidth
    readonly property real naturalHeight: original && original.height > 0 ? original.height : loadedHeight
    property string file: ""

    /// The look as a frame over the whole layout.
    function frame() {
        return {
            zoom: look.zoom,
            x: look.x,
            y: look.y,
            width: Math.max(1, Math.round(layout.width)),
            height: Math.max(1, Math.round(layout.height))
        };
    }
    /// The picture's place in the box: {x, y, width, height}.
    readonly property var place: {
        if (naturalWidth === 0)
            return {
                x: 0,
                y: 0,
                width: boxWidth,
                height: boxHeight
            };
        const r = Frame.rect(frame(), naturalWidth, naturalHeight);
        const s = boxWidth / r.width;
        return {
            x: -r.left * s,
            y: -r.top * s,
            width: naturalWidth * s,
            height: naturalHeight * s
        };
    }
    function withFrame(f) {
        return Object.assign({}, look, {
            zoom: f.zoom,
            x: f.x,
            y: f.y
        });
    }

    implicitWidth: boxWidth + 16
    implicitHeight: boxHeight + 16

    onPathChanged: load()
    Component.onCompleted: load()
    function load() {
        file = "";
        loadedWidth = 0;
        loadedHeight = 0;
        if (path === "")
            return;
        const wanted = path;
        Backend.request("wallpapers.framing_image", {
            path: path
        }, f => {
            if (framer.path === wanted)
                framer.file = f;
        }, e => Toasts.error("Could not load the picture", e));
    }

    // The whole picture over the layout, mirrored like the span; `dim`
    // for what the bezels hide.
    component Picture: Item {
        property real dim: 1
        width: framer.boxWidth
        height: framer.boxHeight
        transform: Scale {
            origin.x: framer.boxWidth / 2
            origin.y: framer.boxHeight / 2
            xScale: framer.look.flip.h ? -1 : 1
            yScale: framer.look.flip.v ? -1 : 1
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
            source: framer.file ? "file://" + framer.file : ""
            asynchronous: true
            smooth: true
            mipmap: true
            opacity: parent.dim
            x: framer.place.x
            y: framer.place.y
            width: framer.place.width
            height: framer.place.height
            onStatusChanged: if (status === Image.Ready) {
                framer.loadedWidth = implicitWidth;
                framer.loadedHeight = implicitHeight;
            }
        }
    }

    Item {
        id: box
        x: 8
        y: 8
        width: framer.boxWidth
        height: framer.boxHeight

        ClippingRectangle {
            anchors.fill: parent
            radius: 6
            color: "#18181b"
            Picture {
                dim: 0.25
            }
        }
        Repeater {
            model: framer.layout.monitors
            // A monitor: a thin bezel around its screen, which shows its part.
            Rectangle {
                required property var modelData
                x: modelData.x * framer.k - 3
                y: modelData.y * framer.k - 3
                width: modelData.w * framer.k + 6
                height: modelData.h * framer.k + 6
                radius: 6
                color: "#18181b"
                border.width: 1
                border.color: Qt.rgba(1, 1, 1, 0.12)
                ClippingRectangle {
                    x: 3
                    y: 3
                    width: parent.width - 6
                    height: parent.height - 6
                    radius: 4
                    color: "#27272a"
                    Picture {
                        x: -modelData.x * framer.k
                        y: -modelData.y * framer.k
                    }
                }
                Text {
                    anchors.left: parent.left
                    anchors.bottom: parent.bottom
                    anchors.margins: 8
                    text: modelData.name
                    color: "white"
                    style: Text.Outline
                    styleColor: Qt.rgba(0, 0, 0, 0.6)
                    font.family: Theme.font
                    font.pixelSize: 11
                    font.weight: Font.Bold
                }
            }
        }
        MouseArea {
            anchors.fill: parent
            cursorShape: pressed ? Qt.ClosedHandCursor : Qt.OpenHandCursor
            property point last
            onPressed: mouse => last = Qt.point(mouse.x, mouse.y)
            onPositionChanged: mouse => {
                if (framer.naturalWidth === 0)
                    return;
                const f = Frame.dragged(framer.frame(), framer.naturalWidth, framer.naturalHeight, framer.boxWidth, mouse.x - last.x, mouse.y - last.y, framer.look.flip);
                framer.moved(framer.withFrame(f));
                last = Qt.point(mouse.x, mouse.y);
            }
            onReleased: framer.settled()
            // Scroll to zoom: a notch is 10%.
            onWheel: wheel => {
                if (framer.naturalWidth === 0)
                    return;
                const zoomed = Frame.zoomed(framer.frame(), 1 + wheel.angleDelta.y / 1200);
                framer.moved(framer.withFrame(Frame.dragged(zoomed, framer.naturalWidth, framer.naturalHeight, framer.boxWidth, 0, 0, framer.look.flip)));
                framer.settled();
            }
        }
    }
}
