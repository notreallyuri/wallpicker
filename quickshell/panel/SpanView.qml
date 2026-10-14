// The control panel on the span shown: the picture across its monitors as
// they stand (dimmed: what the bezels hide), framed here (drag to move,
// scroll to zoom, the sliders, mirroring); each change goes on the
// monitors once it's been still for a moment. Arranging them (which ones,
// where, the bezels) is the Span dialog's.
import QtQuick
import QtQuick.Layouts
import qs.components
import qs.dialogs
import qs.services
import "../components/Span.js" as SpanLayout

ColumnLayout {
    id: view

    readonly property string path: Monitors.spanState ? Monitors.spanState.path : ""
    readonly property var names: Monitors.spanNames
    /// Every monitor as arranged: [{name, x, y, w, h}] (mm), and the bezels.
    property var places: []
    property real bezel: 0
    property var look: SpanLayout.initial()
    // The picture as previewed (a fitted copy) and its size.
    property string file: ""
    property real naturalWidth: 0
    property real naturalHeight: 0
    property int pending: 0
    readonly property bool busy: pending > 0
    readonly property string busyText: "Showing it on the monitors…"
    readonly property bool adjusted: JSON.stringify(look) !== JSON.stringify(SpanLayout.initial())
    readonly property var image: Library.images.find(i => i.path === path) || null

    function load() {
        if (path === "")
            return;
        look = JSON.parse(JSON.stringify(Monitors.spanState.look));
        file = "";
        naturalWidth = 0;
        Backend.request("wallpapers.span_layout", {
            fresh: false,
            bezel: null
        }, l => {
            view.places = l.monitors.map(m => ({
                        name: m.name,
                        x: m.x,
                        y: m.y,
                        w: m.w,
                        h: m.h
                    }));
            view.bezel = l.bezel;
        }, e => Toasts.error("Could not read the monitors", e));
        Backend.request("wallpapers.framing_image", {
            path: path
        }, f => view.file = f, e => Toasts.error("Could not load the picture", e));
    }
    onPathChanged: load()
    Component.onCompleted: load()

    Timer {
        id: settle
        interval: 350
        onTriggered: {
            view.pending++;
            Span.update({
                look: view.look
            }, () => view.pending--);
        }
    }
    function setLook(fields) {
        look = Object.assign({}, look, fields);
        settle.restart();
    }
    function reset() {
        if (!adjusted)
            return;
        look = SpanLayout.initial();
        settle.restart();
    }

    spacing: 0

    ColumnLayout {
        Layout.fillWidth: true
        Layout.bottomMargin: 14
        spacing: 6
        SpanCanvas {
            Layout.fillWidth: true
            Layout.preferredHeight: width * 0.5
            places: view.places
            chosen: view.names
            look: view.look
            arranging: false
            bezel: view.bezel
            source: view.file ? "file://" + view.file : ""
            naturalWidth: view.naturalWidth
            naturalHeight: view.naturalHeight
            onLookMoved: l => view.look = l
            onSettled: settle.restart()
        }
        // Its size: the fitted copy has the picture's proportions.
        Image {
            visible: false
            source: view.file ? "file://" + view.file : ""
            asynchronous: true
            onStatusChanged: if (status === Image.Ready) {
                view.naturalWidth = implicitWidth;
                view.naturalHeight = implicitHeight;
            }
        }
        Text {
            Layout.fillWidth: true
            horizontalAlignment: Text.AlignHCenter
            elide: Text.ElideMiddle
            text: view.path.split("/").pop()
            color: Theme.foreground
            font.family: Theme.font
            font.pixelSize: 13
            font.weight: Font.Bold
        }
        Text {
            Layout.fillWidth: true
            horizontalAlignment: Text.AlignHCenter
            wrapMode: Text.Wrap
            text: "Drag the picture to move it across them, scroll to zoom. Dimmed: what the bezels hide."
            color: Theme.mutedForeground
            font.family: Theme.font
            font.pixelSize: 12
        }
    }

    PanelSection {
        Layout.fillWidth: true
        title: "Arrange"
        hint: "Bezels " + Math.round(view.bezel) + " mm"
        Text {
            Layout.fillWidth: true
            wrapMode: Text.Wrap
            text: "Which monitors, where they stand on your desk and the bezels between them."
            color: Theme.mutedForeground
            font.family: Theme.font
            font.pixelSize: 13
        }
        Btn {
            small: true
            icon: "monitor"
            text: "Arrange monitors…"
            onClicked: Dialogs.arrangeSpan()
        }
    }

    PanelSection {
        Layout.fillWidth: true
        title: "Frame"
        hint: "Zoom " + view.look.zoom + "%"
        SpanLookControls {
            Layout.fillWidth: true
            look: view.look
            onSlid: (field, value) => {
                const f = {};
                f[field] = value;
                view.setLook(f);
            }
            onMirrored: axis => {
                const flip = Object.assign({}, view.look.flip);
                flip[axis] = !flip[axis];
                view.setLook({
                    flip: flip
                });
            }
        }
    }

    PanelSection {
        Layout.fillWidth: true
        title: "About"
        InfoRow {
            Layout.fillWidth: true
            label: "Picture"
            value: view.image && view.image.width > 0 ? view.image.width + " × " + view.image.height : ""
        }
        InfoRow {
            Layout.fillWidth: true
            label: "Across"
            value: view.names.map(n => {
                const m = Monitors.list.find(x => x.name === n);
                return m ? n + " (" + m.width + "×" + m.height + ")" : n;
            }).join(", ")
        }
        InfoRow {
            Layout.fillWidth: true
            label: "Artists"
            value: Library.artists(view.path).join(", ")
        }
    }

    Text {
        Layout.fillWidth: true
        Layout.topMargin: 4
        wrapMode: Text.Wrap
        text: "The span ends when another wallpaper goes on one of them."
        color: Theme.mutedForeground
        font.family: Theme.font
        font.pixelSize: 12
    }
}
