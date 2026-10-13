// Adjust what one monitor shows (monitor_bar.rs's AdjustDialog): frame it
// (drag, scroll, sliders), mirror it, give it its own fit. Applied there at
// once (wallpicker makes the changed copy), and temporary: another
// wallpaper on that monitor goes on as it is. Opened with a right-click on
// a monitor's pill (Dialogs.adjusting).
import QtQuick
import QtQuick.Layouts
import qs.components
import qs.services
import "../components/Frame.js" as Frame

Dialog {
    id: dialog

    readonly property string monitor: Dialogs.adjusting
    readonly property var info: Monitors.list.find(m => m.name === monitor) || {
        width: 1920,
        height: 1080
    }
    readonly property string shown: Monitors.current[monitor] || ""
    readonly property var current: Monitors.adjustOf(monitor)
    /// The frame being moved: the preview follows it at once, the monitor
    /// when it stops for a moment (cutting a big picture takes a while).
    /// Loaded when it opens; after that it's the dialog's (a late answer
    /// mustn't move the frame under the pointer).
    property var draft: null
    // Changes on their way to the monitor (each makes a copy).
    property int pending: 0
    readonly property bool busy: pending > 0

    title: "Adjust " + monitor
    description: shown.split("/").pop()
    preferredWidth: 576
    onClosed: Dialogs.adjusting = ""

    Connections {
        target: Dialogs
        function onAdjustingChanged() {
            if (Dialogs.adjusting === "")
                return;
            dialog.draft = Monitors.adjustOf(Dialogs.adjusting).frame;
            dialog.open();
        }
    }

    function set(next) {
        pending++;
        Monitors.setAdjust(monitor, next, () => dialog.pending--);
    }
    function changed(change) {
        return Object.assign({}, current, change);
    }
    // The frame, once it's been still for a moment.
    Timer {
        id: settle
        interval: 350
        onTriggered: dialog.set(dialog.changed({
            frame: dialog.draft
        }))
    }
    /// A slider moved: the frame (made at the default if there was none).
    function slide(field, value) {
        const f = Object.assign({}, draft || Frame.initial(info.width, info.height));
        f[field] = Math.round(value);
        draft = f;
        settle.restart();
    }

    Framer {
        Layout.alignment: Qt.AlignHCenter
        Layout.topMargin: 4
        Layout.bottomMargin: 4
        monitor: dialog.monitor
        path: dialog.shown
        monitorWidth: dialog.info.width
        monitorHeight: dialog.info.height
        frame: dialog.draft
        onMoved: f => dialog.draft = f
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
            value: dialog.draft ? dialog.draft.zoom : 100
            onMoved: v => dialog.slide("zoom", v)
        }
        SliderRow {
            Layout.fillWidth: true
            label: "Left – right"
            to: 1000
            value: dialog.draft ? dialog.draft.x : 500
            onMoved: v => dialog.slide("x", v)
        }
        SliderRow {
            Layout.fillWidth: true
            label: "Up – down"
            to: 1000
            value: dialog.draft ? dialog.draft.y : 500
            onMoved: v => dialog.slide("y", v)
        }
        Muted {
            text: "Drag the picture to move it, scroll to zoom."
        }
    }

    Section {
        title: "Mirror"
        Btn {
            icon: "flip-horizontal"
            text: "Left to right"
            active: dialog.current.flip.h
            onClicked: dialog.set(dialog.changed({
                flip: {
                    h: !dialog.current.flip.h,
                    v: dialog.current.flip.v
                }
            }))
        }
        Btn {
            icon: "flip-vertical"
            text: "Top to bottom"
            active: dialog.current.flip.v
            onClicked: dialog.set(dialog.changed({
                flip: {
                    h: dialog.current.flip.h,
                    v: !dialog.current.flip.v
                }
            }))
        }
    }

    // Framed: the fit doesn't apply.
    Rectangle {
        visible: dialog.draft !== null
        Layout.fillWidth: true
        implicitHeight: framedRow.implicitHeight + 16
        radius: 12
        color: Theme.a(Theme.primary, 0.1)
        RowLayout {
            id: framedRow
            x: 12
            width: parent.width - 24
            anchors.verticalCenter: parent.verticalCenter
            spacing: 12
            Text {
                Layout.fillWidth: true
                wrapMode: Text.Wrap
                text: "Framed: cut to " + dialog.monitor + "'s exact size, so Fit doesn't apply."
                color: Theme.foreground
                font.family: Theme.font
                font.pixelSize: 14
            }
            Btn {
                text: "Reset frame"
                implicitHeight: 32
                onClicked: {
                    dialog.draft = null;
                    settle.restart();
                }
            }
        }
    }

    Section {
        visible: dialog.draft === null
        title: "Fit"
        Repeater {
            model: Settings.fitModes
            Btn {
                required property var modelData
                implicitHeight: 32
                // "" is whatever Settings says for this monitor.
                text: modelData.value === "" ? "As in Settings" : modelData.label
                active: (dialog.current.fit || "") === modelData.value
                onClicked: dialog.set(dialog.changed({
                    fit: modelData.value || null
                }))
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
            text: dialog.busy ? "Showing it on " + dialog.monitor + "…" : "Goes back when another wallpaper is put on " + dialog.monitor + "."
        }
    ]
    footer: [
        Btn {
            variant: "ghost"
            text: "Reset"
            readonly property bool none: !dialog.current.flip.h && !dialog.current.flip.v && !dialog.current.fit && !dialog.current.frame
            opacity: none ? 0.5 : 1
            onClicked: {
                if (none)
                    return;
                dialog.draft = null;
                settle.stop();
                dialog.set({
                    flip: {
                        h: false,
                        v: false
                    },
                    fit: null,
                    frame: null
                });
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
    // A bold title over a row of buttons.
    component Section: ColumnLayout {
        property string title
        default property alias buttons: buttonRow.data
        Layout.fillWidth: true
        spacing: 8
        Text {
            text: parent.title
            color: Theme.foreground
            font.family: Theme.font
            font.pixelSize: 14
            font.weight: Font.Bold
        }
        Flow {
            id: buttonRow
            Layout.fillWidth: true
            spacing: 8
        }
    }
}
