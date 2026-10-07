// Adjust the span, as framing one monitor but over all of them: zoom and
// move the picture (drag, scroll, sliders), mirror it as a whole. Shown on
// the monitors once it's been still for a moment. Opened from the Spanned
// card or a right-click on the Spanned pill (Dialogs.spanAdjusting).
import QtQuick
import QtQuick.Layouts
import qs.components
import qs.services
import "Span.js" as SpanLayout

Dialog {
    id: dialog

    readonly property string shown: Monitors.spanState ? Monitors.spanState.path : ""
    /// The look being shaped: the preview follows it at once, the monitors
    /// when it stops for a moment. Loaded when it opens; after that it's
    /// the dialog's (a late answer mustn't move it under the pointer).
    property var draft: SpanLayout.initial()
    // Changes on their way to the monitors (each cuts every part again).
    property int pending: 0
    readonly property bool busy: pending > 0
    readonly property bool none: draft.zoom === 100 && draft.x === 500 && draft.y === 500 && !draft.flip.h && !draft.flip.v

    title: "Adjust the span"
    description: shown.split("/").pop() + " · " + Monitors.list.map(m => m.name).join(" + ")
    preferredWidth: 576
    onClosed: Dialogs.spanAdjusting = false

    readonly property bool wanted: Dialogs.spanAdjusting
    onWantedChanged: if (wanted) {
        const look = Monitors.spanState && Monitors.spanState.look ? Monitors.spanState.look : SpanLayout.initial();
        draft = JSON.parse(JSON.stringify(look));
        open();
    }
    // Separated or ended meanwhile (another wallpaper went on a monitor).
    readonly property bool gone: !Monitors.spanned
    onGoneChanged: if (gone && visible)
        close()

    function send() {
        pending++;
        Span.setLook(draft, () => dialog.pending--);
    }
    // The look, once it's been still for a moment.
    Timer {
        id: settle
        interval: 350
        onTriggered: dialog.send()
    }
    function change(fields) {
        draft = Object.assign({}, draft, fields);
    }
    function slide(field, value) {
        const f = {};
        f[field] = Math.round(value);
        change(f);
        settle.restart();
    }
    function mirror(axis) {
        const flip = Object.assign({}, draft.flip);
        flip[axis] = !flip[axis];
        change({
            flip: flip
        });
        settle.stop();
        send();
    }

    SpanFramer {
        Layout.alignment: Qt.AlignHCenter
        path: dialog.shown
        look: dialog.draft
        onMoved: l => dialog.draft = l
        onSettled: settle.restart()
    }

    ColumnLayout {
        Layout.fillWidth: true
        spacing: 8
        SliderRow {
            Layout.fillWidth: true
            label: "Zoom"
            from: 100
            to: 400
            value: dialog.draft.zoom
            onMoved: v => dialog.slide("zoom", v)
        }
        SliderRow {
            Layout.fillWidth: true
            label: "Left – right"
            to: 1000
            value: dialog.draft.x
            onMoved: v => dialog.slide("x", v)
        }
        SliderRow {
            Layout.fillWidth: true
            label: "Up – down"
            to: 1000
            value: dialog.draft.y
            onMoved: v => dialog.slide("y", v)
        }
        Muted {
            text: "Drag the picture to move it, scroll to zoom. Dimmed: what falls between the monitors (the bezel gap is in Settings → General)."
        }
    }

    ColumnLayout {
        Layout.fillWidth: true
        spacing: 8
        Text {
            text: "Mirror"
            color: Theme.foreground
            font.family: Theme.font
            font.pixelSize: 14
            font.weight: Font.Bold
        }
        Flow {
            Layout.fillWidth: true
            spacing: 8
            Btn {
                icon: "flip-horizontal"
                text: "Left to right"
                active: dialog.draft.flip.h
                onClicked: dialog.mirror("h")
            }
            Btn {
                icon: "flip-vertical"
                text: "Top to bottom"
                active: dialog.draft.flip.v
                onClicked: dialog.mirror("v")
            }
        }
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
            text: dialog.busy ? "Showing it on the monitors…" : "Ends when another wallpaper goes on a monitor."
        }
    ]
    footer: [
        Btn {
            variant: "ghost"
            text: "Reset"
            opacity: dialog.none ? 0.5 : 1
            onClicked: {
                if (dialog.none)
                    return;
                dialog.draft = SpanLayout.initial();
                settle.stop();
                dialog.send();
            }
        },
        Btn {
            variant: "primary"
            text: "Done"
            onClicked: dialog.close()
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
