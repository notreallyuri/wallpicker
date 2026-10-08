// A tag or an artist in a local wallpaper's details: its name (an artist's
// with a brush) and an x that takes it off.
import QtQuick
import qs.components
import qs.services

Rectangle {
    id: chip

    property string text
    property string icon
    /// The x's tooltip.
    property string removeTip
    signal removed

    implicitWidth: row.implicitWidth + 16
    implicitHeight: 26
    radius: 13
    color: Theme.secondary

    Row {
        id: row
        anchors.verticalCenter: parent.verticalCenter
        x: 10
        spacing: 4
        Icon {
            visible: chip.icon !== ""
            anchors.verticalCenter: parent.verticalCenter
            name: chip.icon
            size: 12
            color: Theme.mutedForeground
        }
        Text {
            anchors.verticalCenter: parent.verticalCenter
            text: chip.text
            color: Theme.foreground
            font.family: Theme.font
            font.pixelSize: 12
        }
        Rectangle {
            width: 18
            height: 18
            radius: 9
            color: removeMouse.containsMouse ? Theme.a(Theme.background, 0.6) : "transparent"
            Icon {
                anchors.centerIn: parent
                name: "x"
                size: 12
            }
            MouseArea {
                id: removeMouse
                anchors.fill: parent
                hoverEnabled: true
                cursorShape: Qt.PointingHandCursor
                onClicked: chip.removed()
            }
            Tip {
                text: chip.removeTip
                over: removeMouse.containsMouse
                pressed: removeMouse.pressed
            }
        }
    }
}
