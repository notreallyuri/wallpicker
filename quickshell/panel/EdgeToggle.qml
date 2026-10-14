// The control panel's handle: a tab on the picker's right edge while the
// panel is closed, on the panel's left edge while it's open (tinted, the
// chevron pointing out). A click opens or closes it.
import QtQuick
import qs.components
import qs.services

Rectangle {
    id: toggle

    property bool open: false
    signal clicked

    implicitWidth: 28
    implicitHeight: 64
    topLeftRadius: 14
    bottomLeftRadius: 14
    color: open ? Theme.a(Theme.primary, mouse.containsMouse ? 0.25 : 0.12) : mouse.containsMouse ? Theme.muted : Theme.card
    border.width: 1
    border.color: open ? Theme.a(Theme.primary, 0.45) : Theme.a(Theme.border, 0.6)
    Behavior on color {
        ColorAnimation {
            duration: Theme.ms(150)
        }
    }

    Column {
        anchors.centerIn: parent
        anchors.horizontalCenterOffset: 1
        spacing: 8
        Icon {
            name: "sliders-horizontal"
            size: 16
            color: toggle.open ? Theme.primary : Theme.mutedForeground
        }
        Icon {
            anchors.horizontalCenter: parent.horizontalCenter
            name: "chevron-left"
            size: 12
            color: toggle.open ? Theme.primary : Theme.mutedForeground
            rotation: toggle.open ? 180 : 0
        }
    }

    MouseArea {
        id: mouse
        anchors.fill: parent
        hoverEnabled: true
        cursorShape: Qt.PointingHandCursor
        onClicked: toggle.clicked()
    }
    Tip {
        over: mouse.containsMouse
        pressed: mouse.pressed
        text: toggle.open ? "Close the panel" : "Adjust the wallpaper: frame, mirror, fit"
    }
}
