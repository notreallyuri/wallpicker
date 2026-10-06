// A small pill: filled (a monitor on a tile) or soft (a tag on a tile).
import QtQuick
import qs.services

Rectangle {
    id: badge

    property string text
    property bool soft: false

    implicitWidth: label.implicitWidth + 16
    implicitHeight: soft ? label.implicitHeight + 4 : 20
    radius: height / 2
    color: soft ? Theme.a(Theme.primary, 0.15) : Theme.primary

    Text {
        id: label
        anchors.centerIn: parent
        text: badge.text
        color: badge.soft ? Theme.primary : Theme.primaryForeground
        font.family: Theme.font
        font.pixelSize: badge.soft ? 10 : 12
        font.weight: badge.soft ? Font.Medium : Font.Bold
    }
}
