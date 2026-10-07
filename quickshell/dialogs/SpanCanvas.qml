// The Span dialog's canvas: every monitor at its place and physical size
// (SpanView), the picture across the ones spanned. Arranging: click a
// monitor to put it in or out of the span, drag it to move it (it snaps to
// the others: against their edges a bezel apart, or lined up with them).
// Framing: drag to move the picture, scroll to zoom. `settled` after each
// change.
import QtQuick
import qs.components
import qs.services
import "../components/Frame.js" as Frame
import "../components/Span.js" as SpanLayout

Item {
    id: canvas

    /// Every monitor: [{name, x, y, w, h}] (mm).
    property var places: []
    property var chosen: []
    property var look: SpanLayout.initial()
    property bool arranging: true
    /// What monitors snap apart at (mm).
    property real bezel: 0
    property string source: ""
    property real naturalWidth: 0
    property real naturalHeight: 0

    signal placesMoved(var places)
    signal toggled(string name)
    signal lookMoved(var look)
    signal settled

    readonly property var spannedPlaces: places.filter(p => chosen.indexOf(p.name) >= 0)
    // The scale stays put while a monitor is dragged (the canvas would
    // shift under the pointer as the arrangement grows).
    property var held: null
    readonly property var mapping: held || SpanLayout.fit(places, width, height, 28)

    implicitWidth: 528
    implicitHeight: 260

    Rectangle {
        anchors.fill: parent
        radius: 14
        color: Theme.a(Theme.background, 0.5)
        border.width: 1
        border.color: Theme.a(Theme.border, 0.6)
    }
    SpanView {
        id: view
        anchors.fill: parent
        places: canvas.places
        spanned: canvas.chosen
        look: canvas.look
        source: canvas.source
        naturalWidth: canvas.naturalWidth
        naturalHeight: canvas.naturalHeight
        mapping: canvas.mapping
        dimBehind: true
    }

    function framed(f) {
        return Object.assign({}, look, {
            zoom: f.zoom,
            x: f.x,
            y: f.y
        });
    }

    // Framing: the picture moves under the pointer.
    MouseArea {
        visible: !canvas.arranging
        anchors.fill: parent
        cursorShape: pressed ? Qt.ClosedHandCursor : Qt.OpenHandCursor
        property point last
        readonly property real boxWidth: SpanLayout.bounds(canvas.spannedPlaces).w * canvas.mapping.k
        readonly property bool ready: canvas.naturalWidth > 0 && canvas.spannedPlaces.length > 0
        onPressed: mouse => last = Qt.point(mouse.x, mouse.y)
        onPositionChanged: mouse => {
            if (!ready)
                return;
            const f = Frame.dragged(SpanLayout.frameOf(canvas.look, canvas.spannedPlaces), canvas.naturalWidth, canvas.naturalHeight, boxWidth, mouse.x - last.x, mouse.y - last.y, canvas.look.flip);
            canvas.lookMoved(canvas.framed(f));
            last = Qt.point(mouse.x, mouse.y);
        }
        onReleased: canvas.settled()
        // Scroll to zoom: a notch is 10%.
        onWheel: wheel => {
            if (!ready)
                return;
            const zoomed = Frame.zoomed(SpanLayout.frameOf(canvas.look, canvas.spannedPlaces), 1 + wheel.angleDelta.y / 1200);
            canvas.lookMoved(canvas.framed(Frame.dragged(zoomed, canvas.naturalWidth, canvas.naturalHeight, boxWidth, 0, 0, canvas.look.flip)));
            canvas.settled();
        }
    }

    // Arranging: each monitor takes clicks and drags. By count, not by the
    // places themselves: a move replaces them, which would remake the one
    // being dragged.
    Repeater {
        model: canvas.arranging ? canvas.places.length : 0
        MouseArea {
            required property int index
            readonly property var modelData: canvas.places[index] || {
                name: "",
                x: 0,
                y: 0,
                w: 0,
                h: 0
            }
            x: (modelData.x - canvas.mapping.x) * canvas.mapping.k
            y: (modelData.y - canvas.mapping.y) * canvas.mapping.k
            width: modelData.w * canvas.mapping.k
            height: modelData.h * canvas.mapping.k
            hoverEnabled: true
            cursorShape: pressed ? Qt.ClosedHandCursor : Qt.PointingHandCursor
            property point start
            property var from
            property bool moved: false
            onPressed: mouse => {
                start = mapToItem(canvas, mouse.x, mouse.y);
                from = Object.assign({}, modelData);
                moved = false;
                canvas.held = canvas.mapping;
                view.active = modelData.name;
            }
            onPositionChanged: mouse => {
                if (!pressed)
                    return;
                const at = mapToItem(canvas, mouse.x, mouse.y);
                const dx = at.x - start.x, dy = at.y - start.y;
                if (!moved && Math.abs(dx) + Math.abs(dy) < 4)
                    return;
                moved = true;
                const k = canvas.held.k;
                const free = Object.assign({}, from, {
                    x: from.x + dx / k,
                    y: from.y + dy / k
                });
                // Snaps within 10 px on screen.
                const snapped = SpanLayout.snap(free, canvas.places.filter(p => p.name !== from.name), canvas.bezel, 10 / k);
                canvas.placesMoved(canvas.places.map(p => p.name === from.name ? snapped : p));
            }
            onReleased: {
                canvas.held = null;
                view.active = "";
                if (moved)
                    canvas.settled();
                else
                    canvas.toggled(modelData.name);
            }
        }
    }
}
