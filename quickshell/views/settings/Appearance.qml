// Settings → Appearance: the picker's size and where Local's filters are,
// dark mode and the color theme (all at once, kept in the interface's own
// choices), colors from the wallpaper and from which monitor (saved with
// the form), Noctalia's colors and the wallpaper behind the picker (at
// once).
import QtQuick
import QtQuick.Layouts
import qs.components
import qs.services
import "../../services/Themes.js" as Themes

ColumnLayout {
    id: section

    property var form
    readonly property var d: form ? form.draft : ({})
    readonly property bool following: d.theme_from_wallpaper !== false

    spacing: 20

    Field {
        label: "Picker size"
        description: "The floating panel's size on any screen, 720p to QHD and up, in the old window's shape: Large leaves 5% of the screen's height above and below, Comfortable 10%, Compact 15% (about the old window on a 1080p screen). Applies right away."
        Select {
            minWidth: 200
            options: [["large", "Large"], ["comfortable", "Comfortable"], ["compact", "Compact"]]
            value: ["large", "comfortable", "compact"].includes(Prefs.panelSize) ? Prefs.panelSize : "comfortable"
            onPicked: v => Prefs.panelSize = v
        }
    }
    Field {
        label: "Tag and artist filters"
        description: "Local's Filters button opens them in a popover, or shows them as rows under the search bar. Applies right away."
        Select {
            minWidth: 200
            options: [["popover", "In a popover"], ["rows", "As rows"]]
            value: Prefs.filtersLayout === "rows" ? "rows" : "popover"
            onPicked: v => Prefs.filtersLayout = v
        }
    }
    Field {
        label: "Dark mode"
        description: "Applies right away."
        Select {
            minWidth: 200
            options: [["system", "Like the system"], ["light", "Off"], ["dark", "On"]]
            value: Prefs.darkMode
            onPicked: v => Prefs.darkMode = v
        }
    }

    Field {
        Layout.fillWidth: true
        label: "Color theme"
        description: section.following ? "Colors from the wallpaper are on, so this one shows when they're off." : "Applies right away, light and dark."
        GridLayout {
            Layout.fillWidth: true
            columns: 3
            columnSpacing: 8
            rowSpacing: 8
            opacity: section.following ? 0.5 : 1
            Repeater {
                model: Themes.order
                ThemeSwatch {
                    required property string modelData
                    Layout.fillWidth: true
                    name: modelData
                    label: Themes.themes[modelData].label
                    colors: Themes.themes[modelData][Theme.dark ? "dark" : "light"]
                    picked: Prefs.colorTheme === modelData
                    onChosen: Prefs.colorTheme = modelData
                }
            }
        }
    }

    ToggleField {
        Layout.fillWidth: true
        label: "Colors from the wallpaper"
        description: "wallpicker takes its colors from a monitor's wallpaper, the way Noctalia makes them (needs Noctalia installed). Save to apply."
        checked: section.following
        onToggled: section.form.set("theme_from_wallpaper", !section.following)
    }
    RowLayout {
        visible: section.following && Monitors.list.length > 1
        Layout.leftMargin: 48
        spacing: 6
        Text {
            text: "From"
            color: Theme.foreground
            font.family: Theme.font
            font.pixelSize: 14
            font.weight: Font.Bold
        }
        Repeater {
            model: [""].concat(Monitors.list.map(m => m.name))
            Chip {
                required property string modelData
                text: modelData || "First monitor"
                on: (section.d.theme_monitor || "") === modelData
                onClicked: section.form.set("theme_monitor", modelData || null)
            }
        }
    }

    NoctaliaColors {
        Layout.fillWidth: true
    }

    ToggleField {
        Layout.fillWidth: true
        label: "Wallpaper behind the picker"
        description: "Your current wallpaper, blurred, as the picker's background. Applies right away."
        checked: Prefs.backdrop
        onToggled: Prefs.backdrop = !Prefs.backdrop
    }
}
