// A rounded chip that toggles (the Local tag row): outlined, or filled with
// the primary color when on.
import QtQuick
import qs.services

Rectangle {
    id: chip

    property string text
    property bool on: false
    signal clicked

    implicitWidth: label.implicitWidth + 24
    implicitHeight: 28
    radius: height / 2
    color: on ? Theme.primary : (mouse.containsMouse ? Theme.muted : Theme.a(Theme.card, 0.8))
    border.width: on ? 0 : 1
    border.color: Theme.input
    Behavior on color {
        ColorAnimation {
            duration: 150
        }
    }

    Text {
        id: label
        anchors.centerIn: parent
        text: chip.text
        color: chip.on ? Theme.primaryForeground : Theme.foreground
        font.family: Theme.font
        font.pixelSize: 13
        font.weight: Font.Bold
    }
    MouseArea {
        id: mouse
        anchors.fill: parent
        hoverEnabled: true
        cursorShape: Qt.PointingHandCursor
        onClicked: chip.clicked()
    }
}
