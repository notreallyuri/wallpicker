// "APPLY TO", a pill per monitor, and All (a desk of every monitor's
// screen). One of them is chosen at a time; right-click a monitor's to
// adjust what it shows. Spanned, one pill for them all (right-click:
// adjust the span).
import QtQuick
import QtQuick.Layouts
import qs.services

RowLayout {
    spacing: 8

    Text {
        text: "APPLY TO"
        color: Theme.mutedForeground
        font.family: Theme.font
        font.pixelSize: 11
        font.weight: Font.Bold
        font.letterSpacing: 1.5
    }
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
