// Span a picture across the monitors you choose, arranged as they stand
// on your desk: Arrange (click a monitor in or out, drag it; it snaps to
// the others, a bezel apart) and Frame (zoom, move and mirror the picture
// over them all). For a new span (a tile's "Span across…") nothing changes
// until Span; for the span shown (the Spanned card's Adjust, a right-click
// on its pill) each change goes on the monitors once it's been still for a
// moment (Dialogs.spanPath, spanShown).
import QtQuick
import QtQuick.Layouts
import qs.components
import qs.services
import "../components/Span.js" as SpanLayout

Dialog {
    id: dialog

    readonly property string path: Dialogs.spanPath
    readonly property bool shown: Dialogs.spanShown
    /// Every monitor on the canvas: [{name, x, y, w, h}] (mm).
    property var places: []
    property var chosen: []
    property real bezel: 0
    property var look: SpanLayout.initial()
    property bool arranging: true
    // The picture as previewed (a fitted copy) and its size.
    property string file: ""
    property real naturalWidth: 0
    property real naturalHeight: 0
    // Changes on their way to the monitors (each cuts every part again).
    property int pending: 0
    readonly property bool busy: pending > 0
    readonly property bool enough: chosen.length > 1

    title: shown ? "Adjust the span" : "Span across monitors"
    description: path.split("/").pop()
    preferredWidth: 600
    onClosed: Dialogs.spanPath = ""

    onPathChanged: if (path !== "") {
        arranging = !shown;
        look = shown && Monitors.spanState ? JSON.parse(JSON.stringify(Monitors.spanState.look)) : SpanLayout.initial();
        file = "";
        naturalWidth = 0;
        load(false);
        Backend.request("wallpapers.framing_image", {
            path: path
        }, f => dialog.file = f, e => Toasts.error("Could not load the picture", e));
        open();
    }
    /// The canvas: as arranged (or `fresh` from Hyprland's layout).
    function load(fresh) {
        Backend.request("wallpapers.span_layout", {
            fresh: fresh,
            bezel: fresh ? Math.round(bezel) : null
        }, l => {
            dialog.places = l.monitors.map(m => ({
                        name: m.name,
                        x: m.x,
                        y: m.y,
                        w: m.w,
                        h: m.h
                    }));
            if (!fresh) {
                dialog.chosen = dialog.shown ? Monitors.spanNames.slice() : l.chosen;
                dialog.bezel = l.bezel;
            }
            if (fresh)
                dialog.changed();
        }, e => Toasts.error("Could not read the monitors", e));
    }
    // The span ended meanwhile (another wallpaper on one of its monitors).
    readonly property bool gone: shown && !Monitors.spanned
    onGoneChanged: if (gone && visible)
        close()

    /// What goes to the backend: the monitors chosen, their places, the look.
    function layout() {
        const places = {};
        for (const p of dialog.places)
            places[p.name] = {
                x: p.x,
                y: p.y
            };
        return {
            places: places,
            bezel: Math.round(bezel)
        };
    }
    /// Something changed: the span shown follows once it's still.
    function changed() {
        if (shown)
            settle.restart();
    }
    Timer {
        id: settle
        interval: 350
        onTriggered: {
            if (!dialog.enough)
                return;
            dialog.pending++;
            Span.update({
                look: dialog.look,
                monitors: dialog.chosen,
                layout: dialog.layout()
            }, () => dialog.pending--);
        }
    }
    function toggle(name) {
        chosen = chosen.indexOf(name) >= 0 ? chosen.filter(n => n !== name) : chosen.concat([name]);
        changed();
    }
    function setLook(fields) {
        look = Object.assign({}, look, fields);
        changed();
    }

    // Arrange or Frame.
    RowLayout {
        spacing: 8
        Btn {
            icon: "monitor"
            text: "Arrange monitors"
            implicitHeight: 32
            active: dialog.arranging
            onClicked: dialog.arranging = true
        }
        Btn {
            icon: "sliders-horizontal"
            text: "Frame the picture"
            implicitHeight: 32
            active: !dialog.arranging
            onClicked: dialog.arranging = false
        }
    }

    SpanCanvas {
        Layout.alignment: Qt.AlignHCenter
        places: dialog.places
        chosen: dialog.chosen
        look: dialog.look
        arranging: dialog.arranging
        bezel: dialog.bezel
        source: dialog.file ? "file://" + dialog.file : ""
        naturalWidth: dialog.naturalWidth
        naturalHeight: dialog.naturalHeight
        onPlacesMoved: p => dialog.places = p
        onToggled: name => dialog.toggle(name)
        onLookMoved: l => dialog.look = l
        onSettled: dialog.changed()
    }
    // Its size: the fitted copy has the picture's proportions.
    Image {
        visible: false
        source: dialog.file ? "file://" + dialog.file : ""
        asynchronous: true
        onStatusChanged: if (status === Image.Ready) {
            dialog.naturalWidth = implicitWidth;
            dialog.naturalHeight = implicitHeight;
        }
    }

    ColumnLayout {
        visible: dialog.arranging
        Layout.fillWidth: true
        spacing: 8
        SliderRow {
            Layout.fillWidth: true
            label: "Bezels (mm)"
            to: 40
            value: dialog.bezel
            onMoved: v => dialog.bezel = Math.round(v)
        }
        RowLayout {
            Layout.fillWidth: true
            spacing: 12
            Muted {
                text: (dialog.enough ? "" : "Choose two monitors or more. ") + "Click a monitor to put it in or out of the span; drag it to where it stands on your desk. It snaps to the others, the bezels apart. Sizes are the monitors' real ones."
            }
            Btn {
                variant: "ghost"
                text: "From Hyprland"
                implicitHeight: 32
                tip: "Place them as in Hyprland's layout again, the bezels apart"
                onClicked: dialog.load(true)
            }
        }
    }
    SpanLookControls {
        visible: !dialog.arranging
        Layout.fillWidth: true
        look: dialog.look
        onSlid: (field, value) => {
            const f = {};
            f[field] = value;
            dialog.setLook(f);
        }
        onMirrored: axis => {
            const flip = Object.assign({}, dialog.look.flip);
            flip[axis] = !flip[axis];
            dialog.setLook({
                flip: flip
            });
        }
    }
    Muted {
        visible: !dialog.arranging
        text: "Drag the picture to move it, scroll to zoom. Dimmed: what the bezels hide."
    }

    footerLeft: [
        Spinner {
            visible: dialog.busy
            size: 14
        },
        Muted {
            Layout.fillWidth: false
            wrapMode: Text.NoWrap
            font.pixelSize: 12
            text: dialog.busy ? "Showing it on the monitors…" : dialog.shown ? "Ends when another wallpaper goes on one of them." : ""
        }
    ]
    footer: [
        Btn {
            variant: "ghost"
            text: "Reset framing"
            visible: !dialog.arranging
            onClicked: dialog.setLook(SpanLayout.initial())
        },
        Btn {
            visible: !dialog.shown
            variant: "ghost"
            text: "Cancel"
            onClicked: dialog.close()
        },
        Btn {
            variant: "primary"
            text: dialog.shown ? "Done" : "Span"
            opacity: dialog.shown || dialog.enough ? 1 : 0.5
            onClicked: {
                if (dialog.shown) {
                    dialog.close();
                    return;
                }
                if (!dialog.enough)
                    return;
                Span.span(dialog.path, dialog.chosen, dialog.layout(), dialog.look);
                dialog.close();
            }
        }
    ]

    component Muted: Text {
        Layout.fillWidth: true
        wrapMode: Text.Wrap
        color: Theme.mutedForeground
        font.family: Theme.font
        font.pixelSize: 12
    }
}
