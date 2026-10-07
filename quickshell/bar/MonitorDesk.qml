// Every monitor on one desk, side by side, each with what it shows (its
// part, when spanned): All's card and the Spanned card. As tall as a
// monitor's card picture (h-32).
import QtQuick
import QtQuick.Effects
import QtQuick.Layouts
import qs.components
import qs.services

Item {
    Layout.fillWidth: true
    implicitHeight: 128

    Row {
        id: screens
        anchors.horizontalCenter: parent.horizontalCenter
        anchors.bottom: desk.top
        spacing: 8
        // Three monitors or more: smaller, to stay on the card.
        scale: Math.min(1, (parent.width - 8) / Math.max(1, implicitWidth))
        transformOrigin: Item.Bottom
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
