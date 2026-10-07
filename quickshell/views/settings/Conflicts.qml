// A warning when something besides the chosen backend draws wallpapers too
// (hyprpaper still running, Noctalia's own wallpaper on...): they fight over
// the screen. Checked again when the backend changes; each program can be
// stopped from here (Noctalia's wallpaper is switched off in Noctalia, so
// that one only says how).
import QtQuick
import QtQuick.Layouts
import qs.components
import qs.services

Rectangle {
    id: conflicts

    property string backend
    property var found: []
    property string stopping: ""

    function check() {
        const asked = backend;
        if (!asked)
            return;
        Backend.request("settings.conflicts", {
            backend: asked
        }, list => {
            if (conflicts.backend === asked)
                conflicts.found = list || [];
        }, () => {});
    }
    onBackendChanged: check()
    Component.onCompleted: check()

    function stop(name) {
        stopping = name;
        Backend.request("settings.stop_program", {
            name: name
        }, () => {
            Toasts.info("Stopped " + name, "If it's in Hyprland's autostart, take it out there or it starts again at login.");
            conflicts.stopping = "";
            conflicts.check();
        }, e => {
            Toasts.error("Could not stop " + name, e);
            conflicts.stopping = "";
            conflicts.check();
        });
    }

    visible: found.length > 0
    Layout.fillWidth: true
    implicitHeight: rows.implicitHeight + 24
    radius: 14
    color: Qt.rgba(0.96, 0.62, 0.04, 0.12)
    border.width: 1
    border.color: Qt.rgba(0.96, 0.62, 0.04, 0.4)

    ColumnLayout {
        id: rows
        x: 14
        y: 12
        width: parent.width - 28
        spacing: 8
        Text {
            Layout.fillWidth: true
            wrapMode: Text.Wrap
            text: "Also drawing wallpapers: they fight over the screen with " + conflicts.backend + "."
            color: Theme.foreground
            font.family: Theme.font
            font.pixelSize: 13
            font.weight: Font.Bold
        }
        Repeater {
            model: conflicts.found
            RowLayout {
                id: row
                required property string modelData
                readonly property bool noctalia: modelData.startsWith("Noctalia")
                Layout.fillWidth: true
                spacing: 8
                Text {
                    Layout.fillWidth: true
                    text: row.modelData
                    color: Theme.foreground
                    font.family: Theme.font
                    font.pixelSize: 13
                }
                Text {
                    visible: row.noctalia
                    text: "Turn it off in Noctalia's settings (Wallpaper)."
                    color: Theme.mutedForeground
                    font.family: Theme.font
                    font.pixelSize: 12
                }
                Btn {
                    visible: !row.noctalia
                    small: true
                    text: conflicts.stopping === row.modelData ? "Stopping…" : "Stop it"
                    onClicked: if (conflicts.stopping === "")
                        conflicts.stop(row.modelData)
                }
            }
        }
    }
}
