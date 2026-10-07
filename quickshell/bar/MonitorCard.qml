// A monitor's card in the drawer: the monitor bigger, showing its
// wallpaper; its name, size and the file; Adjust and Previous (on this
// monitor only). A click picks it as where a click applies.
import QtQuick
import QtQuick.Layouts
import qs.components
import qs.services

DrawerCard {
    id: card

    required property var monitor
    readonly property string name: monitor.name
    readonly property string shown: Monitors.current[name] || ""

    picked: Monitors.selection === name || Monitors.selection === Monitors.all
    onClicked: Monitors.select(name)

    MonitorFigure {
        Layout.alignment: Qt.AlignHCenter
        size: "card"
        monitor: card.name
        path: card.shown
        monitorWidth: card.monitor.width
        monitorHeight: card.monitor.height
    }
    RowLayout {
        Layout.fillWidth: true
        spacing: 8
        ColumnLayout {
            Layout.fillWidth: true
            spacing: 0
            RowLayout {
                spacing: 6
                Text {
                    Layout.alignment: Qt.AlignBaseline
                    text: card.name
                    color: Theme.foreground
                    font.family: Theme.font
                    font.pixelSize: 14
                    font.weight: Font.ExtraBold
                }
                Text {
                    Layout.alignment: Qt.AlignBaseline
                    text: card.monitor.width + "×" + card.monitor.height
                    color: Theme.mutedForeground
                    font.family: Theme.font
                    font.pixelSize: 12
                    font.weight: Font.DemiBold
                }
            }
            Text {
                Layout.fillWidth: true
                text: card.shown ? card.shown.split("/").pop() : "nothing set"
                elide: Text.ElideRight
                color: Theme.mutedForeground
                font.family: Theme.font
                font.pixelSize: 12
            }
        }
        Btn {
            variant: "ghost"
            icon: "sliders-horizontal"
            implicitHeight: 32
            tip: "Adjust what " + card.name + " shows: mirror, fit, frame"
            onClicked: Dialogs.adjust(card.name)
        }
        Btn {
            variant: "ghost"
            icon: "rotate-ccw"
            tip: "Back to the wallpaper before, on " + card.name
            implicitHeight: 32
            onClicked: Monitors.previousOn([card.name], applied => applied.length > 0 ? "Back on " + card.name : "Nothing earlier on " + card.name)
        }
    }
}
