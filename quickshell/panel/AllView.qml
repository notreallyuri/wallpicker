// The control panel on All: every monitor's wallpaper side by side, and
// Mirror and Fit for all of them at once (on when every monitor has it).
// Framing is each monitor's own: pick one in the bar.
import QtQuick
import QtQuick.Layouts
import qs.components
import qs.services

ColumnLayout {
    id: view

    readonly property var names: Monitors.list.filter(m => !Monitors.inSpan(m.name)).map(m => m.name)
    readonly property var paths: names.map(n => Monitors.current[n] || "")
    /// One picture everywhere, else "" (the footer's Favorite and Details).
    readonly property string path: paths.length > 0 && paths.every(p => p !== "" && p === paths[0]) ? paths[0] : ""
    property int pending: 0
    readonly property bool busy: pending > 0
    readonly property string busyText: "Showing it on the monitors…"
    readonly property var adjusts: names.map(n => Monitors.adjustOf(n))
    readonly property bool adjusted: adjusts.some(a => a.flip.h || a.flip.v || !!a.fit || !!a.frame)

    /// `change(name, its adjustments)` on every monitor.
    function setAll(change) {
        for (const name of names) {
            const a = Monitors.adjustOf(name);
            pending++;
            Monitors.setAdjust(name, Object.assign({}, a, change(a)), () => view.pending--);
        }
    }
    function flipAll(axis) {
        const on = !adjusts.every(a => a.flip[axis]);
        setAll(a => {
            const flip = Object.assign({}, a.flip);
            flip[axis] = on;
            return {
                flip: flip
            };
        });
    }
    function reset() {
        if (!adjusted)
            return;
        setAll(a => ({
                    flip: {
                        h: false,
                        v: false
                    },
                    fit: null,
                    frame: null,
                    paused: a.paused
                }));
    }

    spacing: 0

    ColumnLayout {
        Layout.fillWidth: true
        Layout.bottomMargin: 14
        spacing: 6
        Row {
            Layout.alignment: Qt.AlignHCenter
            spacing: 12
            Repeater {
                model: view.names
                MonitorFigure {
                    required property string modelData
                    readonly property var info: Monitors.list.find(m => m.name === modelData) || {}
                    size: "card"
                    fitWidth: (view.width - 12 * (view.names.length - 1)) / Math.max(1, view.names.length)
                    monitor: modelData
                    path: Monitors.current[modelData] || ""
                    monitorWidth: info.width || 16
                    monitorHeight: info.height || 9
                }
            }
        }
        Text {
            Layout.fillWidth: true
            horizontalAlignment: Text.AlignHCenter
            elide: Text.ElideMiddle
            text: view.path !== "" ? view.path.split("/").pop() : "Different wallpapers"
            color: Theme.foreground
            font.family: Theme.font
            font.pixelSize: 13
            font.weight: Font.Bold
        }
        Text {
            Layout.fillWidth: true
            horizontalAlignment: Text.AlignHCenter
            text: "Mirror and Fit apply to every monitor"
            color: Theme.mutedForeground
            font.family: Theme.font
            font.pixelSize: 12
        }
    }

    PanelSection {
        Layout.fillWidth: true
        title: "Frame"
        Text {
            Layout.fillWidth: true
            wrapMode: Text.Wrap
            text: "Moving and zooming are each monitor's own: pick " + view.names.join(" or ") + " in the bar."
            color: Theme.mutedForeground
            font.family: Theme.font
            font.pixelSize: 13
        }
    }

    PanelSection {
        Layout.fillWidth: true
        title: "Mirror"
        Flow {
            Layout.fillWidth: true
            spacing: 8
            Btn {
                small: true
                icon: "flip-horizontal"
                text: "Left to right"
                active: view.adjusts.length > 0 && view.adjusts.every(a => a.flip.h)
                onClicked: view.flipAll("h")
            }
            Btn {
                small: true
                icon: "flip-vertical"
                text: "Top to bottom"
                active: view.adjusts.length > 0 && view.adjusts.every(a => a.flip.v)
                onClicked: view.flipAll("v")
            }
        }
    }

    PanelSection {
        Layout.fillWidth: true
        title: "Fit"
        Flow {
            Layout.fillWidth: true
            spacing: 6
            Repeater {
                model: Settings.fitModes
                Btn {
                    required property var modelData
                    small: true
                    text: modelData.value === "" ? "As in Settings" : modelData.label
                    active: view.adjusts.length > 0 && view.adjusts.every(a => (a.fit || "") === modelData.value)
                    onClicked: view.setAll(() => ({
                                fit: modelData.value || null
                            }))
                }
            }
        }
    }

    PanelSection {
        Layout.fillWidth: true
        title: "About"
        Repeater {
            model: view.names
            InfoRow {
                required property string modelData
                Layout.fillWidth: true
                label: modelData
                value: (Monitors.current[modelData] || "nothing set").split("/").pop()
            }
        }
    }

    Text {
        Layout.fillWidth: true
        Layout.topMargin: 4
        wrapMode: Text.Wrap
        text: "Mirror and Fit go back on each monitor when its wallpaper changes."
        color: Theme.mutedForeground
        font.family: Theme.font
        font.pixelSize: 12
    }
}
