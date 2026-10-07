// A toggling chip (the Local tag row, as the app's outline toggle): a soft
// primary tint, filled with the primary color when on; the label and a
// muted count.
import QtQuick
import QtQuick.Layouts
import qs.services

Rectangle {
    id: chip

    property string text
    property string count
    property bool on: false
    /// A tooltip (a tag's category).
    property string tip
    signal clicked

    implicitWidth: row.implicitWidth + 24
    implicitHeight: 28
    radius: height / 2
    color: on ? Theme.primary : Theme.a(Theme.primary, mouse.containsMouse ? 0.15 : 0.05)
    border.width: on ? 0 : 1
    border.color: Theme.a(Theme.primary, 0.25)
    Behavior on color {
        ColorAnimation {
            duration: 150
        }
    }

    RowLayout {
        id: row
        anchors.centerIn: parent
        spacing: 6
        Text {
            text: chip.text
            color: chip.on ? Theme.primaryForeground : Theme.foreground
            font.family: Theme.font
            font.pixelSize: 13
            font.weight: Font.Bold
        }
        Text {
            visible: chip.count !== ""
            text: chip.count
            color: chip.on ? Theme.a(Theme.primaryForeground, 0.7) : Theme.mutedForeground
            font.family: Theme.font
            font.pixelSize: 13
            font.weight: Font.Bold
        }
    }
    MouseArea {
        id: mouse
        anchors.fill: parent
        hoverEnabled: true
        cursorShape: Qt.PointingHandCursor
        onClicked: chip.clicked()
    }
    Tip {
        text: chip.tip
        over: mouse.containsMouse
        pressed: mouse.pressed
    }
}
