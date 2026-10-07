// A checkbox with its label (the app's Checkbox in a Label): a rounded
// box, filled with the primary color and a check when on. Disabled ones
// are faded and ignore clicks.
import QtQuick
import QtQuick.Layouts
import qs.services

Item {
    id: check

    property string text
    property bool checked: false
    signal toggled

    implicitWidth: row.implicitWidth
    implicitHeight: row.implicitHeight
    opacity: enabled ? 1 : 0.5

    RowLayout {
        id: row
        spacing: 8
        Rectangle {
            implicitWidth: 18
            implicitHeight: 18
            radius: 5
            color: check.checked ? Theme.primary : Theme.a(Theme.input, 0.3)
            border.width: check.checked ? 0 : 1
            border.color: mouse.containsMouse ? Theme.a(Theme.primary, 0.7) : Theme.input
            Behavior on color {
                ColorAnimation {
                    duration: 150
                }
            }
            Icon {
                visible: check.checked
                anchors.centerIn: parent
                name: "check"
                size: 14
                color: Theme.primaryForeground
            }
        }
        Text {
            text: check.text
            color: Theme.foreground
            font.family: Theme.font
            font.pixelSize: 14
        }
    }
    // Over the box and the label both.
    MouseArea {
        id: mouse
        anchors.fill: parent
        hoverEnabled: true
        cursorShape: Qt.PointingHandCursor
        onClicked: check.toggled()
    }
}
