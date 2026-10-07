// "APPLY TO", a pill per monitor, and All (a desk of every monitor's
// screen). One of them is chosen at a time; right-click a monitor's to
// adjust what it shows.
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
    Repeater {
        model: Monitors.list
        MonitorPill {
            required property var modelData
            label: modelData.name
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
