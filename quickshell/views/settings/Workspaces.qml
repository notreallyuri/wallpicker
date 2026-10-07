// Settings → Workspaces: a wallpaper per Hyprland workspace. The switches
// are saved with the form; each workspace's wallpaper changes at once (as
// you apply wallpapers with "remember" on, and from here). Choosing one
// turns the section into a picker over the library, in place.
import QtQuick
import QtQuick.Layouts
import qs.components
import qs.services

ColumnLayout {
    id: section

    property var form
    readonly property var w: form ? (form.draft.workspaces || {}) : ({})
    /// [{name, monitor, open, wallpaper}]; null while loading.
    property var rows: null
    /// The workspace being chosen for ("" none).
    property string choosing: ""

    function load() {
        Backend.request("workspaces.rows", {}, r => section.rows = r || [], e => Toasts.error("Could not list the workspaces", e));
    }
    function assign(ws, path) {
        choosing = "";
        Backend.request("workspaces.set", {
            workspace: ws,
            path: path
        }, () => section.load(), e => Toasts.error("Could not change the workspace's wallpaper", e));
    }
    Component.onCompleted: load()
    // Workspaces open and close, monitors switch: kept up to date while shown.
    Timer {
        interval: 3000
        running: section.choosing === ""
        repeat: true
        onTriggered: section.load()
    }

    spacing: 20

    WorkspacePicker {
        visible: section.choosing !== ""
        Layout.fillWidth: true
        workspace: section.choosing
        onPicked: path => section.assign(section.choosing, path)
        onBack: section.choosing = ""
    }

    ColumnLayout {
        visible: section.choosing === ""
        Layout.fillWidth: true
        spacing: 20
        ToggleField {
            Layout.fillWidth: true
            label: "A wallpaper per workspace"
            description: "When a monitor switches to a workspace that has a wallpaper, it goes on that monitor. Workspaces without one leave the monitor as it is. Switching doesn't count as a pick: Previous still goes back to the one you chose before. Save to apply."
            checked: !!section.w.enabled
            onToggled: section.form.set("workspaces.enabled", !section.w.enabled)
        }
        ToggleField {
            Layout.fillWidth: true
            label: "Remember what I put on each workspace"
            description: "A wallpaper you apply (or go back to) becomes the wallpaper of the workspace its monitor is showing. Random and the slideshow don't change them."
            checked: section.w.remember !== false
            onToggled: section.form.set("workspaces.remember", section.w.remember === false)
        }
        Spinner {
            visible: section.rows === null
        }
        Note {
            visible: section.rows !== null && section.rows.length === 0
            text: "No workspaces found (is Hyprland running?)."
        }
        Repeater {
            model: section.rows || []
            WorkspaceRow {
                required property var modelData
                Layout.fillWidth: true
                ws: modelData
                onChoose: section.choosing = modelData.name
                onAssign: path => section.assign(modelData.name, path)
            }
        }
        Note {
            text: "Also from a terminal: `wallpicker workspaces`. wallpicker follows the workspaces while its window or `wallpicker workspaces run` is running."
        }
    }
}
