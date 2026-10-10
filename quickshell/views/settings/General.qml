// Settings: library folders, native renderer transitions and fit, thumbnails and tag order.
import QtQuick
import QtQuick.Layouts
import qs.components
import qs.services

ColumnLayout {
    id: section

    property var form
    readonly property var d: form ? form.draft : ({})
    readonly property var o: form && form.options ? form.options : null
    readonly property var pairs: list => list ? list.map(x => [x.value, x.label]) : []

    spacing: 20

    Field {
        Layout.fillWidth: true
        label: "Wallpaper folders"
        description: "Where wallpicker looks for wallpapers. ~ is allowed."
        FolderList {
            Layout.fillWidth: true
            folders: section.d.folders || []
            onChanged: folders => section.form.set("folders", folders)
        }
    }
    ToggleField {
        label: "Include subfolders"
        checked: !!section.d.recursive
        onToggled: section.form.set("recursive", !section.d.recursive)
    }

    Conflicts {}

    ColumnLayout {
        Layout.fillWidth: true
        spacing: 16
        Note {
            text: "wallpicker draws your wallpapers through its daemon and restores them when it starts. Add wallpicker daemon to your session's autostart to show them from login."
        }
        RowLayout {
            spacing: 16
            Field {
                label: "Transition"
                Select {
                    minWidth: 180
                    options: section.pairs(section.o ? section.o.transitions : null)
                    value: section.d.transition ? section.d.transition.kind : "fade"
                    onPicked: v => section.form.set("transition.kind", v)
                }
            }
            Field {
                label: "Duration (ms)"
                NumberField {
                    to: 10000
                    value: section.d.transition ? section.d.transition.duration_ms : 0
                    onEdited: v => section.form.set("transition.duration_ms", v)
                }
            }
        }
    }

    Field {
        Layout.fillWidth: true
        label: "Fit mode"
        description: "For every monitor, unless it has its own below; a monitor's Adjust (the drawer) wins over both, until its wallpaper changes."
        Select {
            minWidth: 260
            options: section.pairs(section.o ? section.o.fits : null)
            value: section.d.fit_mode || ""
            onPicked: v => section.form.set("fit_mode", v || null)
        }
    }
    Field {
        visible: Monitors.list.length > 0
        label: "Fit per monitor"
        MonitorFits {
            value: section.d.monitor_fits || ({})
            choices: section.o ? section.o.fits : []
            onChanged: v => section.form.set("monitor_fits", v)
        }
    }
    Field {
        label: "Thumbnail width (px)"
        description: "The size thumbnails are made at (120 to 1024)."
        NumberField {
            to: 1024
            value: section.d.thumb_width || 0
            onEdited: v => section.form.set("thumb_width", v)
        }
    }
    // The interface's own choice (ui.json), not the settings: at once.
    Field {
        label: "Tag order"
        description: "How the tags and artists above the grid are ordered, the ones picked first. Applies right away."
        Select {
            minWidth: 200
            options: [["name", "By name (A to Z)"], ["count", "Most used first"]]
            value: Prefs.tagOrder === "count" ? "count" : "name"
            onPicked: v => Prefs.tagOrder = v
        }
    }
}
