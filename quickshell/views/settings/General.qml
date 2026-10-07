// Settings → General: the wallpaper folders, subfolders, the backend (what
// else draws wallpapers, awww's transition, Noctalia's note), the fit for
// every monitor and each one's own, the thumbnails' width.
import QtQuick
import QtQuick.Layouts
import qs.components
import qs.services

ColumnLayout {
    id: section

    property var form
    readonly property var d: form ? form.draft : ({})
    readonly property var o: form && form.options ? form.options : null
    readonly property string backend: d.wallpaper_backend || "hyprpaper"
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

    Field {
        label: "Wallpaper backend"
        description: "What draws the wallpaper; wallpicker applies through it."
        Select {
            minWidth: 220
            options: section.pairs(section.o ? section.o.backends : null)
            value: section.backend
            onPicked: v => section.form.set("wallpaper_backend", v)
        }
    }
    Conflicts {
        backend: section.backend
    }

    // awww: its transition.
    ColumnLayout {
        visible: section.backend === "awww"
        Layout.fillWidth: true
        spacing: 16
        Note {
            text: "awww (called swww before) draws the wallpaper with a transition and keeps it after a restart. wallpicker starts its daemon when it isn't running; to have it from login, add awww-daemon to Hyprland's autostart."
        }
        RowLayout {
            spacing: 16
            Field {
                label: "Transition"
                Select {
                    minWidth: 180
                    options: section.pairs(section.o ? section.o.transitions : null)
                    value: section.d.transition ? section.d.transition.kind : ""
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
            Field {
                label: "Frames per second"
                NumberField {
                    to: 255
                    value: section.d.transition ? section.d.transition.fps : 0
                    onEdited: v => section.form.set("transition.fps", v)
                }
            }
        }
        Note {
            text: "Your monitors' refresh rate is the smoothest frame rate."
        }
    }
    Note {
        visible: section.backend === "noctalia"
        text: "Noctalia keeps the wallpapers itself, so they come back after a restart. Its fill mode and transitions are set in Noctalia's settings (Wallpaper). Turn Noctalia's wallpaper on there, and stop hyprpaper (take it out of your Hyprland autostart): both draw the background."
    }

    // Noctalia keeps its own fill mode.
    Field {
        visible: section.backend !== "noctalia"
        Layout.fillWidth: true
        label: "Fit mode"
        description: "For every monitor, unless it has its own below; a monitor's Adjust (the drawer) wins over both, until its wallpaper changes." + (section.backend === "hyprpaper" ? " Needs a newer hyprpaper: leave it on default if applying fails." : "")
        Select {
            minWidth: 260
            options: section.pairs(section.o ? section.o.fits[section.backend] : null)
            value: section.d.fit_mode || ""
            onPicked: v => section.form.set("fit_mode", v || null)
        }
    }
    Field {
        visible: section.backend !== "noctalia" && Monitors.list.length > 0
        label: "Fit per monitor"
        MonitorFits {
            value: section.d.monitor_fits || ({})
            choices: section.o ? section.o.fits[section.backend] : []
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
}
