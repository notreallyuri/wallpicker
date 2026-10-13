import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import qs.components
import qs.services

Dialog {
    id: dialog
    property var edit: null
    property var mapping: ({})
    property var preview: null
    property string problem: ""
    property int revision: 0
    readonly property var names: edit ? Object.keys(edit.preset.monitors) : []
    readonly property bool changed: names.some(n => mapping[n] && mapping[n] !== n)
    title: "Reassign preset monitors"
    description: "Choose where each saved wallpaper belongs. This changes the saved preset; use Apply afterward to show it. Span positions follow their assignments."
    onClosed: { revision++; Presets.editing = null; }

    Connections {
        target: Presets
        function onEditingChanged() {
            if (!Presets.editing || Presets.editing.action !== "reassign") return;
            dialog.edit = Presets.editing;
            dialog.mapping = {};
            dialog.preview = null;
            dialog.problem = "";
            Presets.error = "";
            dialog.check();
            dialog.open();
        }
    }
    function options(source) {
        const choices = [[source, source + " (keep" + (Monitors.list.some(m => m.name === source) ? ")" : "; disconnected)")]];
        for (const monitor of Monitors.list)
            if (monitor.name !== source) choices.push([monitor.name, monitor.name]);
        return choices;
    }
    function pick(source, target) {
        const next = Object.assign({}, mapping);
        next[source] = target;
        mapping = next;
        check();
    }
    function check() {
        preview = null;
        problem = "";
        const request = ++revision;
        Backend.request("presets.preview_reassignment", {id: edit.preset.id, mapping: mapping}, result => {
            if (request === revision) preview = result;
        }, message => {
            if (request === revision) problem = message;
        });
    }
    Text {
        Layout.fillWidth: true
        text: dialog.edit ? dialog.edit.preset.name : ""
        color: Theme.foreground
        font.family: Theme.font
        font.bold: true
    }
    Flickable {
        id: rows
        Layout.fillWidth: true
        Layout.preferredHeight: Math.min(assignments.implicitHeight, 340)
        contentHeight: assignments.implicitHeight
        clip: true
        boundsBehavior: Flickable.StopAtBounds
        ScrollBar.vertical: ScrollBar {}
        SmoothScroll { flickable: rows }
        ColumnLayout {
            id: assignments
            width: rows.width - 16
            spacing: 12
            Repeater {
                model: dialog.names
                RowLayout {
                    required property string modelData
                    Layout.fillWidth: true
                    spacing: 12
                    PresetMonitor {
                        Layout.preferredWidth: 110
                        monitor: modelData
                        wallpaper: dialog.edit.preset.monitors[modelData]
                        missing: (dialog.edit.preset.missing_files || []).indexOf(wallpaper.wallpaper) >= 0
                    }
                    Text {
                        text: "→"
                        color: Theme.mutedForeground
                        font.pixelSize: 18
                    }
                    Select {
                        Layout.fillWidth: true
                        options: dialog.options(modelData)
                        value: dialog.mapping[modelData] || modelData
                        onPicked: value => dialog.pick(modelData, value)
                        enabled: !Presets.busy
                        Accessible.name: "Destination for " + modelData
                    }
                }
            }
        }
    }
    Text {
        Layout.fillWidth: true
        text: dialog.preview ? "Preview: " + dialog.names.map(n => n + " → " + (dialog.mapping[n] || n)).join("; ") + (dialog.preview.span ? "\nSpan: " + dialog.preview.span.monitors.join(" + ") : "") : dialog.problem || "Checking mapping…"
        wrapMode: Text.Wrap
        color: dialog.problem ? "#fd4663" : Theme.mutedForeground
        font.family: Theme.font
    }
    Text {
        visible: Presets.error !== ""
        Layout.fillWidth: true
        text: Presets.error
        wrapMode: Text.Wrap
        color: "#fd4663"
        font.family: Theme.font
    }
    footer: [
        PresetButton {
            text: "Cancel"
            variant: "ghost"
            enabled: !Presets.busy
            onClicked: dialog.close()
        },
        PresetButton {
            text: Presets.busy ? "Saving…" : "Save reassignment"
            variant: "primary"
            enabled: !Presets.busy && dialog.preview !== null && dialog.changed
            onClicked: {
                const pending = dialog.edit;
                Presets.run("reassign", {id: pending.preset.id, mapping: dialog.mapping, expected: pending.preset}, () => {
                    if (Presets.editing === pending) dialog.close();
                });
            }
        }
    ]
}
