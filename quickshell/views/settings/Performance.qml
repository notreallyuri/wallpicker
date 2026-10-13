// Settings → Performance: what the picker uses right now (memory of the
// interface and of its backend, `wallpicker serve`; CPU) and what the
// wallpaper renderer does (the daemon's GPU time and memory, CPU, animation
// decoders), refreshed every 2 s; the effects that cost the most (at once); full-size previews (saved
// with the form).
import QtQuick
import QtQuick.Layouts
import qs.components
import qs.services

ColumnLayout {
    id: section

    property var form
    readonly property var d: form ? form.draft : ({})
    /// {interface_mb, backend_mb, total_mb, cpu}; null until read.
    property var usage: null

    /// The daemon's (services/renderer `Usage`); null until read.
    property var renderer: null

    function poll() {
        Backend.request("app.usage", {}, u => section.usage = u, () => {});
        Backend.request("renderer.usage", {}, u => section.renderer = u, () => {});
    }
    /// "0.3%", "12%"; "…" before there's a rate (the first poll).
    function percent(v) {
        return v === null || v === undefined ? "…" : (v < 1 ? v.toFixed(1) : Math.round(v)) + "%";
    }
    function mb(v) {
        return Math.round(v) + " MB";
    }
    /// [label, value] rows for the renderer card.
    readonly property var rendererRows: {
        const r = renderer;
        if (!r)
            return [];
        if (!r.running)
            return [["wallpicker daemon", "not running"]];
        const regions = Object.keys(r.gpu_memory_mb || {}).map(k => k.toUpperCase() + " " + mb(r.gpu_memory_mb[k]));
        const rows = [["GPU time", r.gpu_reported ? percent(r.gpu) : "not reported by this GPU driver"]];
        if (regions.length > 0)
            rows.push(["GPU memory", regions.join(" · ")]);
        if (r.gpu_busy !== null && r.gpu_busy !== undefined)
            rows.push(["Whole GPU busy (every program)", r.gpu_busy + "%"]);
        rows.push(["CPU (of one core)", percent(r.cpu)]);
        rows.push(["Memory", mb(r.memory_mb) + " (+ " + mb(r.libraries_mb) + " shared libraries)"]);
        rows.push(["Animation decoders", r.decoders === 0 ? "none running" : r.decoders + " · " + percent(r.decoders_cpu) + " CPU · " + mb(r.decoders_memory_mb)]);
        return rows;
    }
    Component.onCompleted: poll()
    Timer {
        interval: 2000
        running: true
        repeat: true
        onTriggered: section.poll()
    }

    spacing: 20

    Rectangle {
        Layout.fillWidth: true
        implicitHeight: usageRows.implicitHeight + 32
        radius: 18
        color: Theme.a(Theme.card, 0.6)
        ColumnLayout {
            id: usageRows
            x: 16
            y: 16
            width: parent.width - 32
            spacing: 8
            RowLayout {
                Text {
                    Layout.fillWidth: true
                    text: "Memory now"
                    color: Theme.foreground
                    font.family: Theme.font
                    font.pixelSize: 14
                    font.weight: Font.Bold
                }
                Text {
                    text: section.usage ? section.usage.total_mb + " MB" : "…"
                    color: Theme.foreground
                    font.family: Theme.font
                    font.pixelSize: 22
                    font.weight: Font.ExtraBold
                }
            }
            Repeater {
                model: [["Interface (Quickshell)", "interface_mb"], ["Backend (wallpicker serve)", "backend_mb"]]
                RowLayout {
                    required property var modelData
                    Text {
                        Layout.fillWidth: true
                        text: modelData[0]
                        color: Theme.mutedForeground
                        font.family: Theme.font
                        font.pixelSize: 13
                    }
                    Text {
                        text: section.usage ? section.usage[modelData[1]] + " MB" : "…"
                        color: Theme.foreground
                        font.family: Theme.font
                        font.pixelSize: 13
                    }
                }
            }
            RowLayout {
                Text {
                    Layout.fillWidth: true
                    text: "CPU (of one core)"
                    color: Theme.mutedForeground
                    font.family: Theme.font
                    font.pixelSize: 13
                }
                Text {
                    text: section.usage && section.usage.cpu !== null && section.usage.cpu !== undefined ? Math.round(section.usage.cpu) + "%" : "…"
                    color: Theme.foreground
                    font.family: Theme.font
                    font.pixelSize: 13
                }
            }
        }
    }
    Rectangle {
        Layout.fillWidth: true
        visible: section.renderer !== null
        implicitHeight: rendererColumn.implicitHeight + 32
        radius: 18
        color: Theme.a(Theme.card, 0.6)
        ColumnLayout {
            id: rendererColumn
            x: 16
            y: 16
            width: parent.width - 32
            spacing: 8
            RowLayout {
                Text {
                    Layout.fillWidth: true
                    text: "Wallpaper renderer"
                    color: Theme.foreground
                    font.family: Theme.font
                    font.pixelSize: 14
                    font.weight: Font.Bold
                }
                Text {
                    // The headline: GPU time, what animations cost most.
                    visible: !!section.renderer && section.renderer.running && section.renderer.gpu_reported
                    text: section.renderer ? section.percent(section.renderer.gpu) + " GPU" : ""
                    color: Theme.foreground
                    font.family: Theme.font
                    font.pixelSize: 22
                    font.weight: Font.ExtraBold
                }
            }
            Repeater {
                model: section.rendererRows
                RowLayout {
                    required property var modelData
                    Text {
                        Layout.fillWidth: true
                        text: modelData[0]
                        color: Theme.mutedForeground
                        font.family: Theme.font
                        font.pixelSize: 13
                    }
                    Text {
                        text: modelData[1]
                        color: Theme.foreground
                        font.family: Theme.font
                        font.pixelSize: 13
                    }
                }
            }
            Text {
                Layout.fillWidth: true
                visible: !!section.renderer && section.renderer.running
                text: "wallpicker daemon: its share of the GPU's time and the memory it holds there, as the kernel counts them. Animations are decoded in separate processes."
                wrapMode: Text.Wrap
                color: Theme.mutedForeground
                font.family: Theme.font
                font.pixelSize: 12
            }
        }
    }
    Field {
        Layout.fillWidth: true
        label: "Keep the picker ready after closing"
        description: Prefs.keepOpen > 0 ? "Closed, it stays loaded (about this much memory) for that long: opening it again then is instant, where you left it. Unused that long, it frees all of it. Applies from the next close." : "Off: closing frees all of it at once; each open starts fresh (about half a second)."
        Select {
            minWidth: 200
            options: [["0", "Off"], ["30", "30 seconds"], ["60", "1 minute"], ["300", "5 minutes"], ["600", "10 minutes"], ["1800", "30 minutes"]]
            value: String(Prefs.keepOpen)
            onPicked: v => Prefs.keepOpen = parseInt(v)
        }
    }

    ToggleField {
        Layout.fillWidth: true
        label: "Frosted glass"
        description: "The blur behind the search cards. It's redrawn whenever what's under it moves, which is most of the CPU while scrolling. Off: solid cards. Applies right away."
        checked: Prefs.glass
        onToggled: Prefs.glass = !Prefs.glass
    }
    ToggleField {
        Layout.fillWidth: true
        label: "Wallpaper behind the picker"
        description: "Your current wallpaper, blurred, under everything (also in Appearance). Applies right away."
        checked: Prefs.backdrop
        onToggled: Prefs.backdrop = !Prefs.backdrop
    }
    ToggleField {
        Layout.fillWidth: true
        label: "Animations"
        description: "Hops, slides and fades. Off: things change at once. Applies right away."
        checked: Prefs.motion
        onToggled: Prefs.motion = !Prefs.motion
    }
    ToggleField {
        Layout.fillWidth: true
        label: "Full-size previews in Online details"
        description: "Off: the preview is fitted to 1600 px, plenty for the details view. On: the original, which for an 8K wallpaper is over 100 MB while it's shown. Save to apply."
        checked: !!section.d.full_previews
        onToggled: section.form.set("full_previews", !section.d.full_previews)
    }
}
