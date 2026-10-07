// A card in the monitor drawer (a monitor's, or All's): w-72, rounded-3xl,
// the primary border and a raised edge when a click applies there; lifts a
// little on hover with the bouncy ease. A click picks it.
import QtQuick
import QtQuick.Layouts
import qs.services

Item {
    id: card

    property bool picked: false
    default property alias content: column.data
    readonly property bool hovered: mouse.containsMouse
    signal clicked

    implicitWidth: 288
    implicitHeight: column.implicitHeight + 24

    // The raised edge (shadow-[0_3px_0_0_primary/45]) when picked.
    Rectangle {
        visible: card.picked
        anchors.fill: face
        anchors.topMargin: 3
        anchors.bottomMargin: -3
        radius: face.radius
        color: Theme.a(Theme.primary, 0.45)
    }
    Rectangle {
        id: face
        width: parent.width
        height: parent.height
        y: card.hovered ? -2 : 0
        radius: 24
        color: card.picked ? Theme.a(Theme.primary, 0.12) : card.hovered ? Theme.card : Theme.a(Theme.card, 0.6)
        border.width: 2
        border.color: card.picked ? Theme.primary : "transparent"
        Behavior on y {
            NumberAnimation {
                duration: Theme.duration
                easing.type: Easing.BezierSpline
                easing.bezierCurve: Theme.bounce
            }
        }
        Behavior on color {
            ColorAnimation {
                duration: Theme.ms(200)
            }
        }
        MouseArea {
            id: mouse
            anchors.fill: parent
            hoverEnabled: true
            cursorShape: Qt.PointingHandCursor
            onClicked: card.clicked()
        }
        ColumnLayout {
            id: column
            x: 12
            y: 12
            width: parent.width - 24
            spacing: 8
        }
    }
}
