// All's card in the drawer: every monitor, with what it shows, side by side
// on one desk.
import QtQuick
import QtQuick.Layouts
import qs.services

DrawerCard {
    id: card

    picked: Monitors.selection === Monitors.all
    onClicked: Monitors.select(Monitors.all)

    MonitorDesk {}
    ColumnLayout {
        Layout.fillWidth: true
        spacing: 0
        Text {
            text: "All"
            color: Theme.foreground
            font.family: Theme.font
            font.pixelSize: 14
            font.weight: Font.ExtraBold
        }
        Text {
            text: "every monitor"
            color: Theme.mutedForeground
            font.family: Theme.font
            font.pixelSize: 12
        }
    }
}
