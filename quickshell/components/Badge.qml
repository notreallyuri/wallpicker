// A small pill: filled (a monitor on a tile) or soft (a tag on a tile),
// with an icon before the text if given (an artist's brush).
import QtQuick
import qs.services

Rectangle {
    id: badge

    property string text
    property bool soft: false
    property string icon

    implicitWidth: content.implicitWidth + 16
    implicitHeight: soft ? label.implicitHeight + 4 : 20
    radius: height / 2
    color: soft ? Theme.a(Theme.primary, 0.15) : Theme.primary

    Row {
        id: content
        anchors.centerIn: parent
        spacing: 3
        Icon {
            visible: badge.icon !== ""
            anchors.verticalCenter: parent.verticalCenter
            name: badge.icon
            size: label.font.pixelSize
            color: label.color
        }
        Text {
            id: label
            text: badge.text
            color: badge.soft ? Theme.primary : Theme.primaryForeground
            font.family: Theme.font
            font.pixelSize: badge.soft ? 10 : 12
            font.weight: badge.soft ? Font.Medium : Font.Bold
        }
    }
}
