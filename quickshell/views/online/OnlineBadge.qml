// A small label on an online tile's picture: Downloaded (with its tip), or
// the rating when it isn't sfw (red).
import QtQuick
import QtQuick.Layouts
import qs.components
import qs.services

Rectangle {
    id: badge

    property string text
    property string icon
    property string tip
    property bool destructive: false

    implicitWidth: row.implicitWidth + 16
    implicitHeight: 22
    radius: height / 2
    color: destructive ? "#e7000b" : Theme.secondary

    RowLayout {
        id: row
        anchors.centerIn: parent
        spacing: 4
        Icon {
            visible: badge.icon !== ""
            name: badge.icon
            size: 12
            color: badge.destructive ? "white" : Theme.foreground
        }
        Text {
            text: badge.text
            color: badge.destructive ? "white" : Theme.foreground
            font.family: Theme.font
            font.pixelSize: 12
            font.weight: Font.Bold
        }
    }
    // Hover only (for the tip): clicks go to the picture.
    MouseArea {
        id: mouse
        anchors.fill: parent
        enabled: badge.tip !== ""
        hoverEnabled: true
        acceptedButtons: Qt.NoButton
    }
    Tip {
        text: badge.tip
        over: mouse.containsMouse
    }
}
