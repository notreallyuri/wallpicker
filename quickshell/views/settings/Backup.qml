// Settings → Backup: export and import (through the system's file dialog),
// the daily snapshots and their folder, restoring one (asking first). These
// act at once, not with Save: they're about the library, not the settings.
import QtQuick
import QtQuick.Layouts
import qs.components
import qs.services

ColumnLayout {
    id: section

    property var form
    property bool replace: false
    property bool busy: false
    property string report: ""
    /// [{id, label, items}]; null while loading.
    property var snapshots: null
    property string folder: ""

    function reload() {
        Backend.request("backup.snapshots", {}, s => section.snapshots = s || [], () => section.snapshots = []);
        Backend.request("backup.snapshot_dir", {}, d => section.folder = d || "");
    }
    Component.onCompleted: reload()

    function exportTo() {
        Backend.request("backup.default_path", {}, suggested => {
            const cut = suggested.lastIndexOf("/");
            Files.save({
                title: "Export the wallpicker library",
                folder: suggested.slice(0, cut),
                name: suggested.slice(cut + 1),
                filter: {
                    name: "wallpicker backup",
                    glob: "*.json"
                }
            }, path => {
                if (!path)
                    return;
                section.busy = true;
                Backend.request("backup.export", {
                    path: path
                }, e => {
                    section.busy = false;
                    Toasts.info("Exported " + e.items + " wallpapers", e.path);
                }, e => {
                    section.busy = false;
                    Toasts.error("Could not export", e);
                });
            });
        });
    }

    function importFrom() {
        Files.open({
            title: "Import a wallpicker backup",
            filter: {
                name: "wallpicker backup",
                glob: "*.json"
            }
        }, path => {
            if (!path)
                return;
            section.busy = true;
            Backend.request("backup.import", {
                path: path,
                replace: section.replace
            }, r => {
                section.busy = false;
                let text = "Found " + r.same_place + " where they were, " + r.moved + " moved.";
                if (r.missing.length > 0)
                    text += " Not in your library now (" + r.missing.length + "): " + r.missing.join(", ") + ".";
                section.report = text;
                section.reload();
            }, e => {
                section.busy = false;
                Toasts.error("Could not import", e);
            });
        });
    }

    function moveFolder(reset) {
        const set = dir => {
            section.busy = true;
            Backend.request("backup.set_snapshot_dir", {
                dir: dir
            }, now => {
                section.busy = false;
                Toasts.info("Snapshots now go to " + now);
                section.reload();
            }, e => {
                section.busy = false;
                Toasts.error("Could not use that folder", e);
            });
        };
        if (reset)
            set(null);
        else
            Files.folder({
                title: "A folder for the daily snapshots",
                folder: section.folder || null
            }, dir => {
                if (dir)
                    set(dir);
            });
    }

    function restore(id) {
        busy = true;
        Backend.request("backup.restore", {
            id: id
        }, c => {
            section.busy = false;
            Toasts.info("Library restored (" + Object.keys(c.items || {}).length + " wallpapers)");
            section.reload();
        }, e => {
            section.busy = false;
            Toasts.error("Could not restore", e);
        });
    }

    spacing: 20

    Field {
        Layout.fillWidth: true
        label: "To a file"
        description: "Favorites, tags and fits, in one file. An import is merged into your library: a wallpaper that moved (another folder, another home) is found by its place in your wallpaper folders, or by name and size."
        RowLayout {
            spacing: 8
            Btn {
                variant: "primary"
                icon: "download"
                text: "Export…"
                opacity: section.busy ? 0.5 : 1
                onClicked: if (!section.busy)
                    section.exportTo()
            }
            Btn {
                text: "Import…"
                opacity: section.busy ? 0.5 : 1
                onClicked: if (!section.busy)
                    section.importFrom()
            }
        }
    }
    ToggleField {
        label: "Replace the library instead of merging"
        checked: section.replace
        onToggled: section.replace = !section.replace
    }
    Note {
        visible: section.report !== ""
        text: section.report
        color: Theme.foreground
    }

    Field {
        Layout.fillWidth: true
        label: "Daily snapshots"
        description: "A copy of the library before the first change of each day; the last 14 are kept. Restoring keeps what's there now as a snapshot too. A synced folder (Syncthing, Nextcloud…) makes them backups off this machine."
        RowLayout {
            Layout.fillWidth: true
            spacing: 8
            Text {
                Layout.fillWidth: true
                text: section.folder
                elide: Text.ElideMiddle
                color: Theme.foreground
                font.family: "monospace"
                font.pixelSize: 12
            }
            Btn {
                small: true
                text: "Change…"
                onClicked: if (!section.busy)
                    section.moveFolder(false)
            }
            Btn {
                small: true
                variant: "ghost"
                text: "Default"
                onClicked: if (!section.busy)
                    section.moveFolder(true)
            }
        }
    }
    Spinner {
        visible: section.snapshots === null
    }
    Note {
        visible: section.snapshots !== null && section.snapshots.length === 0
        text: "None yet: the first comes with the first change to the library today."
    }
    Repeater {
        model: section.snapshots || []
        SnapshotRow {
            required property var modelData
            Layout.fillWidth: true
            snapshot: modelData
            busy: section.busy
            onRestore: section.restore(modelData.id)
        }
    }
}
