// Spanned, the drawer's only card: the monitors as one, each showing its
// part of the picture; Adjust (the span's zoom, position and mirroring),
// Separate (each monitor on its own again, keeping its part) and Previous
// (on every monitor). A click on a tile spans that picture.
import QtQuick
import QtQuick.Layouts
import qs.components
import qs.services

DrawerCard {
    id: card

    readonly property string shown: Monitors.spanState ? Monitors.spanState.path : ""

    implicitWidth: Math.max(288, desk.implicitWidth + 48)
    picked: true

    MonitorDesk {
        id: desk
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
                    text: "Spanned"
                    color: Theme.foreground
                    font.family: Theme.font
                    font.pixelSize: 14
                    font.weight: Font.ExtraBold
                }
                Text {
                    Layout.alignment: Qt.AlignBaseline
                    text: Monitors.list.map(m => m.name).join(" + ")
                    color: Theme.mutedForeground
                    font.family: Theme.font
                    font.pixelSize: 12
                    font.weight: Font.DemiBold
                }
            }
            Text {
                Layout.fillWidth: true
                text: card.shown.split("/").pop()
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
            tip: "Adjust the span: zoom, position, mirror"
            onClicked: Dialogs.adjustSpan()
        }
        Btn {
            variant: "ghost"
            icon: "unlink"
            implicitHeight: 32
            tip: "Separate: each monitor on its own again, keeping its part"
            onClicked: Span.separate()
        }
        Btn {
            variant: "ghost"
            icon: "rotate-ccw"
            implicitHeight: 32
            tip: "Back to the wallpapers before, on every monitor"
            onClicked: Monitors.previousOn(Monitors.list.map(m => m.name), applied => applied.length > 0 ? "Back on " + applied.join(", ") : "Nothing earlier to go back to")
        }
    }
}
