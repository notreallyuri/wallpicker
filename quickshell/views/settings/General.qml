// Settings → General, in groups: the wallpaper folders, applying (the
// renderer's transition and the fits), the grid (thumbnails, tag order).
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

    Group {
        title: "Wallpaper folders"
        Field {
            Layout.fillWidth: true
            label: "Folders"
            description: "Where wallpicker looks for wallpapers. ~ is allowed."
            FolderList {
                Layout.fillWidth: true
                folders: section.d.folders || []
                onChanged: folders => section.form.set("folders", folders)
            }
        }
        ToggleField {
            Layout.fillWidth: true
            label: "Include subfolders"
            description: "Look inside every folder below these too."
            checked: !!section.d.recursive
            onToggled: section.form.set("recursive", !section.d.recursive)
        }
    }

    Conflicts {}

    Group {
        title: "Applying"
        Field {
            Layout.fillWidth: true
            side: true
            label: "Transition"
            description: "How a new wallpaper comes in."
            Select {
                minWidth: 180
                options: section.pairs(section.o ? section.o.transitions : null)
                value: section.d.transition ? section.d.transition.kind : "fade"
                onPicked: v => section.form.set("transition.kind", v)
            }
        }
        Field {
            Layout.fillWidth: true
            side: true
            label: "Duration"
            description: "Milliseconds; 0 cuts straight to it."
            NumberField {
                to: 10000
                value: section.d.transition ? section.d.transition.duration_ms : 0
                onEdited: v => section.form.set("transition.duration_ms", v)
            }
        }
        Field {
            Layout.fillWidth: true
            side: true
            label: "Fit mode"
            description: "For every monitor, unless it has its own below; the control panel's Fit wins over both, until the wallpaper changes."
            Select {
                minWidth: 200
                options: section.pairs(section.o ? section.o.fits : null)
                value: section.d.fit_mode || ""
                onPicked: v => section.form.set("fit_mode", v || null)
            }
        }
        Field {
            visible: Monitors.list.length > 0
            Layout.fillWidth: true
            label: "Fit per monitor"
            MonitorFits {
                value: section.d.monitor_fits || ({})
                choices: section.o ? section.o.fits : []
                onChanged: v => section.form.set("monitor_fits", v)
            }
        }
    }

    Notice {
        text: "wallpicker draws your wallpapers through its daemon and restores them when it starts: add wallpicker daemon to your session's autostart to show them from login."
    }

    Group {
        title: "Grid"
        Field {
            Layout.fillWidth: true
            side: true
            label: "Thumbnail width"
            description: "The size thumbnails are made at, in pixels (120 to 1024)."
            NumberField {
                to: 1024
                value: section.d.thumb_width || 0
                onEdited: v => section.form.set("thumb_width", v)
            }
        }
        // The interface's own choice (ui.json), not the settings: at once.
        Field {
            Layout.fillWidth: true
            side: true
            label: "Tag order"
            description: "How Filters orders the tags and artists, the ones picked first. Applies right away."
            Select {
                minWidth: 200
                options: [["name", "By name (A to Z)"], ["count", "Most used first"]]
                value: Prefs.tagOrder === "count" ? "count" : "name"
                onPicked: v => Prefs.tagOrder = v
            }
        }
    }
}
