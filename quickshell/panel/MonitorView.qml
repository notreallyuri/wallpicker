// The control panel on one monitor (what the Adjust dialog was): its
// wallpaper in a big preview where it's framed (drag to move, scroll to
// zoom), the framing sliders, mirroring and its own fit, then what the
// picture is. Applied there at once (wallpicker makes the changed copy),
// and temporary: another wallpaper on that monitor goes on as it is.
import QtQuick
import QtQuick.Layouts
import qs.components
import qs.dialogs
import qs.services
import "../components/Frame.js" as Frame

ColumnLayout {
    id: view

    required property string monitor
    readonly property var info: Monitors.list.find(m => m.name === monitor) || {
        width: 1920,
        height: 1080
    }
    /// The picture there ("" none), for the footer's actions.
    readonly property string path: Monitors.current[monitor] || ""
    /// Where the footer's Previous goes back.
    readonly property var names: [monitor]
    readonly property var current: Monitors.adjustOf(monitor)
    readonly property var image: Library.images.find(i => i.path === path) || null
    /// The frame being moved: the preview follows it at once, the monitor
    /// when it stops for a moment (cutting a big picture takes a while).
    property var draft: null
    // Changes on their way to the monitor (each makes a copy).
    property int pending: 0
    readonly property bool busy: pending > 0
    readonly property bool adjusted: current.flip.h || current.flip.v || !!current.fit || !!current.frame
    readonly property string busyText: "Showing it on " + monitor + "…"

    // The frame as it is there, for another monitor or wallpaper, and
    // whenever it changes there while nothing of the panel's is on its way
    // (a late answer mustn't move the frame under the pointer).
    readonly property var saved: current.frame
    function load() {
        settle.stop();
        draft = saved;
    }
    onMonitorChanged: load()
    onPathChanged: load()
    onSavedChanged: if (!settle.running && pending === 0)
        draft = saved
    Component.onCompleted: load()

    function set(next) {
        pending++;
        Monitors.setAdjust(monitor, next, () => view.pending--);
    }
    function changed(change) {
        return Object.assign({}, current, change);
    }
    // The frame, once it's been still for a moment.
    Timer {
        id: settle
        interval: 350
        onTriggered: view.set(view.changed({
            frame: view.draft
        }))
    }
    /// A slider moved: the frame (made at the default if there was none).
    function slide(field, value) {
        const f = Object.assign({}, draft || Frame.initial(info.width, info.height));
        f[field] = Math.round(value);
        draft = f;
        settle.restart();
    }
    /// Back as it came: no frame, mirroring or fit of its own.
    function reset() {
        if (!adjusted)
            return;
        draft = null;
        settle.stop();
        set({
            flip: {
                h: false,
                v: false
            },
            fit: null,
            frame: null
        });
    }
    function flipped(axis) {
        const flip = Object.assign({}, current.flip);
        flip[axis] = !flip[axis];
        set(changed({
            flip: flip
        }));
    }

    spacing: 0

    Text {
        visible: view.path === ""
        Layout.fillWidth: true
        Layout.topMargin: 24
        horizontalAlignment: Text.AlignHCenter
        wrapMode: Text.Wrap
        text: "Nothing on " + view.monitor + " yet: pick a wallpaper."
        color: Theme.mutedForeground
        font.family: Theme.font
        font.pixelSize: 13
    }

    ColumnLayout {
        visible: view.path !== ""
        Layout.fillWidth: true
        Layout.bottomMargin: 14
        spacing: 6
        Framer {
            Layout.alignment: Qt.AlignHCenter
            fitWidth: view.width
            monitor: view.monitor
            path: view.path
            monitorWidth: view.info.width
            monitorHeight: view.info.height
            frame: view.draft
            onMoved: f => view.draft = f
            onSettled: settle.restart()
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
            text: "Drag the picture to move it, scroll to zoom"
            color: Theme.mutedForeground
            font.family: Theme.font
            font.pixelSize: 12
        }
    }

    PanelSection {
        visible: view.path !== ""
        Layout.fillWidth: true
        title: "Frame"
        hint: view.draft ? "Zoom " + view.draft.zoom + "%" : "Not framed"
        SliderRow {
            Layout.fillWidth: true
            label: "Zoom"
            from: 100
            to: 400
            value: view.draft ? view.draft.zoom : 100
            onMoved: v => view.slide("zoom", v)
        }
        SliderRow {
            Layout.fillWidth: true
            label: "Left – right"
            to: 1000
            value: view.draft ? view.draft.x : 500
            onMoved: v => view.slide("x", v)
        }
        SliderRow {
            Layout.fillWidth: true
            label: "Up – down"
            to: 1000
            value: view.draft ? view.draft.y : 500
            onMoved: v => view.slide("y", v)
        }
        // Framed: cut to the monitor's size, so the fit doesn't apply.
        RowLayout {
            visible: view.draft !== null
            Layout.fillWidth: true
            spacing: 8
            Text {
                Layout.fillWidth: true
                wrapMode: Text.Wrap
                text: "Cut to " + view.monitor + "'s exact size, so Fit doesn't apply."
                color: Theme.mutedForeground
                font.family: Theme.font
                font.pixelSize: 12
            }
            Btn {
                small: true
                text: "Reset frame"
                onClicked: {
                    view.draft = null;
                    settle.restart();
                }
            }
        }
    }

    PanelSection {
        visible: view.path !== ""
        Layout.fillWidth: true
        title: "Mirror"
        hint: view.current.flip.h && view.current.flip.v ? "Both ways" : view.current.flip.h ? "Left to right" : view.current.flip.v ? "Top to bottom" : ""
        Flow {
            Layout.fillWidth: true
            spacing: 8
            Btn {
                small: true
                icon: "flip-horizontal"
                text: "Left to right"
                active: view.current.flip.h
                onClicked: view.flipped("h")
            }
            Btn {
                small: true
                icon: "flip-vertical"
                text: "Top to bottom"
                active: view.current.flip.v
                onClicked: view.flipped("v")
            }
        }
    }

    PanelSection {
        visible: view.path !== "" && view.draft === null
        Layout.fillWidth: true
        title: "Fit"
        hint: {
            const mode = Settings.fitModes.find(m => m.value === (view.current.fit || ""));
            return !view.current.fit ? "As in Settings" : mode ? mode.label : view.current.fit;
        }
        Flow {
            Layout.fillWidth: true
            spacing: 6
            Repeater {
                model: Settings.fitModes
                Btn {
                    required property var modelData
                    small: true
                    // "" is whatever Settings says for this monitor.
                    text: modelData.value === "" ? "As in Settings" : modelData.label
                    active: (view.current.fit || "") === modelData.value
                    onClicked: view.set(view.changed({
                        fit: modelData.value || null
                    }))
                }
            }
        }
    }

    PanelSection {
        visible: view.path !== ""
        Layout.fillWidth: true
        title: "About"
        InfoRow {
            Layout.fillWidth: true
            label: "Size"
            value: view.image && view.image.width > 0 ? view.image.width + " × " + view.image.height : ""
        }
        InfoRow {
            Layout.fillWidth: true
            label: "File"
            value: view.image ? (view.image.size / 1048576).toFixed(1) + " MB" : ""
        }
        InfoRow {
            Layout.fillWidth: true
            label: "Folder"
            value: view.path.replace(/\/[^/]*$/, "").replace(/^\/home\/[^/]+/, "~")
        }
        InfoRow {
            Layout.fillWidth: true
            label: "Artists"
            value: Library.artists(view.path).join(", ")
        }
        InfoRow {
            Layout.fillWidth: true
            label: "Tags"
            value: Library.tags(view.path).join(", ")
        }
    }

    Text {
        visible: view.path !== ""
        Layout.fillWidth: true
        Layout.topMargin: 4
        wrapMode: Text.Wrap
        text: "Frame, Mirror and Fit go back when another wallpaper is put on " + view.monitor + "."
        color: Theme.mutedForeground
        font.family: Theme.font
        font.pixelSize: 12
    }
}
