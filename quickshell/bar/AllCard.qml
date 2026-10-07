// All's card in the drawer: every monitor, with what it shows, side by side
// on one desk.
import QtQuick
import QtQuick.Effects
import QtQuick.Layouts
import qs.components
import qs.services

DrawerCard {
    id: card

    picked: Monitors.selection === Monitors.all
    onClicked: Monitors.select(Monitors.all)

    // h-32: as tall as a monitor's card picture, the screens on the desk.
    Item {
        Layout.fillWidth: true
        implicitHeight: 128
        Row {
            id: screens
            anchors.horizontalCenter: parent.horizontalCenter
            anchors.bottom: desk.top
            spacing: 8
            Repeater {
                model: Monitors.list
                MonitorFigure {
                    required property var modelData
                    anchors.bottom: parent.bottom
                    size: "desk"
                    monitor: modelData.name
                    path: Monitors.current[modelData.name] || ""
                    monitorWidth: modelData.width
                    monitorHeight: modelData.height
                }
            }
        }
        RectangularShadow {
            anchors.fill: desk
            radius: desk.radius
            offset.y: 3
            blur: 6
            color: Qt.rgba(0, 0, 0, 0.35)
        }
        Rectangle {
            id: desk
            anchors.bottom: parent.bottom
            width: parent.width
            height: 6
            radius: 3
            gradient: Gradient {
                GradientStop {
                    position: 0
                    color: "#52525b"
                }
                GradientStop {
                    position: 1
                    color: "#27272a"
                }
            }
        }
    }
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
