// A pill per monitor, and All (a desk of every monitor's screen): where a
// click on a wallpaper applies. One of them is chosen at a time;
// right-click a monitor's to adjust what it shows. Spanned, one pill for
// them all (right-click: adjust the span).
import QtQuick
import QtQuick.Layouts
import qs.services

RowLayout {
    id: pills

    // While an animated wallpaper is on, the daemon's playback status is
    // read (every 2 s) for the pills' play and pause marks.
    readonly property bool animated: Monitors.list.some(m => {
        const img = Library.images.find(i => i.path === Monitors.current[m.name]);
        return !!img && img.frames > 1;
    })
    property bool watching: false
    onAnimatedChanged: {
        if (animated !== watching)
            Playback.watchers += animated ? 1 : -1;
        watching = animated;
    }
    Component.onCompleted: if (animated) {
        Playback.watchers++;
        watching = true;
    }
    Component.onDestruction: if (watching)
        Playback.watchers--

    spacing: 6

    MonitorPill {
        visible: Monitors.spanned
        label: "Spanned"
        tip: "One picture across " + Monitors.spanNames.join(" + ") + "\nPicked, a click on a wallpaper spans it; right-click to adjust the span"
        on: Monitors.selection === Monitors.spanTarget
        onPicked: Monitors.select(Monitors.spanTarget)
        onRightClicked: Dialogs.adjustSpan()
        Repeater {
            model: Monitors.spanned ? Monitors.list.filter(m => Monitors.inSpan(m.name)) : []
            MonitorScreen {
                required property var modelData
                screenWidth: 18
                screenHeight: 13
                radius: 2
                path: Monitors.current[modelData.name] || ""
            }
        }
    }
    Repeater {
        model: Monitors.list.filter(m => !Monitors.inSpan(m.name))
        MonitorPill {
            required property var modelData
            label: modelData.name
            // The name, its size, and the file it's showing.
            tip: {
                const shown = Monitors.current[modelData.name];
                return modelData.name + " · " + modelData.width + "×" + modelData.height + "\nShowing " + (shown ? shown.split("/").pop() : "nothing set");
            }
            on: Monitors.selection === modelData.name
            playback: {
                const p = Playback.of(modelData.name);
                return p ? (p.state === "hidden" ? "paused" : p.state) : "";
            }
            onPicked: Monitors.select(modelData.name)
            // Right-click: adjust what it shows.
            onRightClicked: Dialogs.adjust(modelData.name)
            MonitorScreen {
                path: Monitors.current[modelData.name] || ""
            }
        }
    }
    MonitorPill {
        visible: Monitors.list.length > 1
        label: "All"
        tip: "Every monitor"
        on: Monitors.selection === Monitors.all
        onPicked: Monitors.select(Monitors.all)
        Repeater {
            model: Monitors.list
            MonitorScreen {
                required property var modelData
                screenWidth: 18
                screenHeight: 13
                radius: 2
                path: Monitors.current[modelData.name] || ""
            }
        }
    }
}
