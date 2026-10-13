// Settings → Animated wallpapers: whether GIF and animated WebP wallpapers
// play and how fast (saved with the form), pausing them (at once), and what
// each monitor's is doing now (read every 2 s while this is shown).
import QtQuick
import QtQuick.Layouts
import qs.components
import qs.services

ColumnLayout {
    id: section

    property var form
    readonly property var d: form ? form.draft : ({})
    readonly property var a: d.animation || ({})
    readonly property int fps: a.max_fps === undefined ? 60 : a.max_fps

    Component.onCompleted: Playback.watchers++
    Component.onDestruction: Playback.watchers--

    spacing: 20

    ToggleField {
        Layout.fillWidth: true
        label: "Play animated wallpapers (experimental)"
        description: "GIF and animated WebP wallpapers play on the GPU, one decoder for every monitor showing the same one. Off, or without hardware GPU playback, they show their first frame. Save to apply."
        checked: !!section.a.enabled
        onToggled: section.form.set("animation.enabled", !section.a.enabled)
    }
    ToggleField {
        Layout.fillWidth: true
        visible: Playback.enabled || !!section.a.enabled
        label: "Pause animations"
        description: "Every animated wallpaper holds its frame until resumed, after a restart too. Also in the monitor drawer. Applies right away."
        checked: Playback.paused
        onToggled: Playback.setPaused(!Playback.paused)
    }
    Field {
        Layout.fillWidth: true
        label: "Frame rate"
        description: "At most this many new frames a second on each monitor. Animations slower than that keep their own pace; a lower cap saves GPU and CPU on fast ones. Save to apply."
        Select {
            minWidth: 240
            options: {
                const choices = [["15", "15 a second"], ["24", "24 a second"], ["30", "30 a second"], ["60", "60 a second (default)"], ["120", "120 a second"], ["0", "As fast as the display refreshes"]];
                // One set from the command line keeps its own entry.
                if (!choices.some(c => c[0] === String(section.fps)))
                    choices.push([String(section.fps), section.fps + " a second"]);
                return choices;
            }
            value: String(section.fps)
            onPicked: v => section.form.set("animation.max_fps", parseInt(v))
        }
    }

    Rectangle {
        Layout.fillWidth: true
        implicitHeight: rows.implicitHeight + 32
        radius: 18
        color: Theme.a(Theme.card, 0.6)
        ColumnLayout {
            id: rows
            x: 16
            y: 16
            width: parent.width - 32
            spacing: 8
            Text {
                text: "On your monitors now"
                color: Theme.foreground
                font.family: Theme.font
                font.pixelSize: 14
                font.weight: Font.Bold
            }
            Text {
                visible: Playback.data !== null && !Playback.data.running
                text: "The wallpaper daemon isn't running."
                color: Theme.mutedForeground
                font.family: Theme.font
                font.pixelSize: 13
            }
            Repeater {
                model: Playback.data && Playback.data.running ? Monitors.list : []
                RowLayout {
                    id: row
                    required property var modelData
                    readonly property var animation: Playback.of(modelData.name)
                    Text {
                        Layout.fillWidth: true
                        text: row.modelData.name
                        color: Theme.mutedForeground
                        font.family: Theme.font
                        font.pixelSize: 13
                    }
                    Text {
                        Layout.maximumWidth: rows.width * 0.65
                        text: row.animation ? Playback.describe(row.animation) : "A still picture"
                        elide: Text.ElideRight
                        color: row.animation ? Theme.primary : Theme.foreground
                        font.family: Theme.font
                        font.pixelSize: 13
                        font.weight: row.animation ? Font.DemiBold : Font.Normal
                    }
                }
            }
        }
    }
    Note {
        text: "Each animation is decoded in a separate, memory-capped process (up to four at once); frames are drawn on the GPU. The renderer's GPU, CPU and memory use are in Performance."
    }
}
