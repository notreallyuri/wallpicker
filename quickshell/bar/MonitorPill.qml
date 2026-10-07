// A pill in the top bar: its screen(s) (the slot), a label, and a check when
// a click applies there. A monitor's, or All's.
import QtQuick
import QtQuick.Layouts
import qs.components
import qs.services

Rectangle {
    id: pill

    property string label
    property bool on: false
    default property alias screens: screens.data
    /// A tooltip.
    property string tip
    signal picked
    /// A right-click (a monitor's pill: Adjust).
    signal rightClicked

    implicitWidth: row.implicitWidth + 20
    implicitHeight: 36
    radius: height / 2
    color: mouse.containsMouse ? Theme.muted : Theme.a(Theme.card, 0.6)
    border.width: on ? 2 : 1
    border.color: on ? Theme.primary : Theme.a(Theme.foreground, 0.08)
    Behavior on border.color {
        ColorAnimation {
            duration: 150
        }
    }

    RowLayout {
        id: row
        anchors.centerIn: parent
        spacing: 8
        Row {
            id: screens
            spacing: 1
        }
        Text {
            text: pill.label
            color: Theme.foreground
            font.family: Theme.font
            font.pixelSize: 14
            font.weight: Font.Black
        }
        Rectangle {
            visible: pill.on
            implicitWidth: 18
            implicitHeight: 18
            radius: 9
            color: Theme.primary
            Icon {
                anchors.centerIn: parent
                name: "check"
                size: 12
                color: Theme.primaryForeground
            }
        }
    }
    MouseArea {
        id: mouse
        anchors.fill: parent
        hoverEnabled: true
        cursorShape: Qt.PointingHandCursor
        acceptedButtons: Qt.LeftButton | Qt.RightButton
        onClicked: mouseEvent => {
            if (mouseEvent && mouseEvent.button === Qt.RightButton)
                pill.rightClicked();
            else
                pill.picked();
        }
    }
    Tip {
        text: pill.tip
        over: mouse.containsMouse
        pressed: mouse.pressed
    }
}
