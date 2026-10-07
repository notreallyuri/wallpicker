// The Adjust dialog's preview, and where framing happens: a big monitor
// showing exactly the framed part of the picture (mirrored like it will
// be). Drag to move the frame, scroll to zoom; `settled` after each move.
// Without a frame it shows the fit in effect there.
import QtQuick
import qs.components
import qs.services
import "../components/Frame.js" as Frame

MonitorFigure {
    id: framer

    /// The frame being shown (null: none, the fit shows).
    property var frame: null
    /// The frame moved (each step of a drag, each notch of a zoom).
    signal moved(var frame)
    /// A drag or a zoom ended: time to show it on the monitor.
    signal settled

    readonly property var adjusted: Monitors.adjustOf(monitor)
    readonly property string fit: Settings.fitFor(monitor, adjusted.fit) || "cover"
    // The picture's size: the original's (the frame is only proportions
    // and thousandths, so any size of it works), else the framing copy's
    // as loaded. Not the copy's always: tiled, it's decoded at another
    // size.
    readonly property var original: Library.images.find(i => i.path === path)
    property real loadedWidth: 0
    property real loadedHeight: 0
    readonly property real naturalWidth: original && original.width > 0 ? original.width : loadedWidth
    readonly property real naturalHeight: original && original.height > 0 ? original.height : loadedHeight
    property string file: ""

    size: "big"
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

    function current() {
        return frame || Frame.initial(monitorWidth, monitorHeight);
    }

    // The picture's place for the frame: {x, y, width, height} on the screen
    // (the same for any size of it: the scale makes up the difference).
    function placed() {
        const r = Frame.rect(frame, naturalWidth, naturalHeight);
        const s = screenWidth / r.width;
        return {
            x: -r.left * s,
            y: -r.top * s,
            width: naturalWidth * s,
            height: naturalHeight * s
        };
    }

    Item {
        id: stage
        anchors.fill: parent
        transform: Scale {
            origin.x: stage.width / 2
            origin.y: stage.height / 2
            xScale: framer.adjusted.flip.h ? -1 : 1
            yScale: framer.adjusted.flip.v ? -1 : 1
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
            id: picture
            readonly property bool framed: framer.frame !== null && framer.naturalWidth > 0
            readonly property var place: framed ? framer.placed() : null
            // Tiles (or, with awww, one centered) at the original's size
            // on this monitor.
            readonly property bool tiled: !framed && framer.fit === "tile"
            readonly property real tileScale: framer.screenWidth / Math.max(framer.monitorWidth, 1)

            source: framer.file ? "file://" + framer.file : ""
            asynchronous: true
            smooth: true
            mipmap: true
            x: framed ? place.x : 0
            y: framed ? place.y : 0
            width: framed ? place.width : stage.width
            height: framed ? place.height : stage.height
            fillMode: framed ? Image.Stretch : tiled ? (Settings.backend === "awww" ? Image.Pad : Image.Tile) : framer.fit === "contain" ? Image.PreserveAspectFit : framer.fit === "fill" ? Image.Stretch : Image.PreserveAspectCrop
            sourceSize: tiled && framer.naturalWidth > 0 ? Qt.size(framer.naturalWidth * tileScale, framer.naturalHeight * tileScale) : undefined
            onStatusChanged: if (status === Image.Ready && !tiled) {
                framer.loadedWidth = implicitWidth;
                framer.loadedHeight = implicitHeight;
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
            framer.moved(Frame.dragged(framer.current(), framer.naturalWidth, framer.naturalHeight, framer.screenWidth, mouse.x - last.x, mouse.y - last.y, framer.adjusted.flip));
            last = Qt.point(mouse.x, mouse.y);
        }
        onReleased: framer.settled()
        // Scroll to zoom: a notch is 10%.
        onWheel: wheel => {
            if (framer.naturalWidth === 0)
                return;
            const zoomed = Frame.zoomed(framer.current(), 1 + wheel.angleDelta.y / 1200);
            // Its center kept valid at the new size.
            framer.moved(Frame.dragged(zoomed, framer.naturalWidth, framer.naturalHeight, framer.screenWidth, 0, 0, framer.adjusted.flip));
            framer.settled();
        }
    }
}
