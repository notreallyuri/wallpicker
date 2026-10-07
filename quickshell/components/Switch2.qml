// A two-word switch with a sliding knob (Local's All | Any for tags): the
// knob sits under the chosen word and slides with the bouncy ease.
import QtQuick
import qs.services

Rectangle {
    id: sw

    property string first: "All"
    property string second: "Any"
    /// Whether the second word is chosen.
    property bool onRight: false
    signal toggled
    /// For a tip on it.
    readonly property alias hovered: mouse.containsMouse
    readonly property alias pressed: mouse.pressed

    readonly property real half: Math.max(leftLabel.implicitWidth, rightLabel.implicitWidth) + 24

    implicitWidth: half * 2 + 4
    implicitHeight: 28
    radius: height / 2
    color: Theme.muted
    border.width: 1
    border.color: Theme.a(Theme.foreground, 0.1)

    Rectangle {
        id: knob
        x: sw.onRight ? sw.half + 2 : 2
        y: 2
        width: sw.half
        height: parent.height - 4
        radius: height / 2
        color: Theme.primary
        Behavior on x {
            NumberAnimation {
                duration: Theme.duration
                easing.type: Easing.BezierSpline
                easing.bezierCurve: Theme.bounce
            }
        }
    }
    Text {
        id: leftLabel
        x: 2 + (sw.half - width) / 2
        anchors.verticalCenter: parent.verticalCenter
        text: sw.first
        color: sw.onRight ? Theme.a(Theme.mutedForeground, 0.6) : Theme.primaryForeground
        font.family: Theme.font
        font.pixelSize: 12
        font.weight: Font.Bold
        Behavior on color {
            ColorAnimation {
                duration: Theme.ms(200)
            }
        }
    }
    Text {
        id: rightLabel
        x: sw.half + 2 + (sw.half - width) / 2
        anchors.verticalCenter: parent.verticalCenter
        text: sw.second
        color: sw.onRight ? Theme.primaryForeground : Theme.a(Theme.mutedForeground, 0.6)
        font.family: Theme.font
        font.pixelSize: 12
        font.weight: Font.Bold
        Behavior on color {
            ColorAnimation {
                duration: Theme.ms(200)
            }
        }
    }
    MouseArea {
        id: mouse
        anchors.fill: parent
        hoverEnabled: true
        cursorShape: Qt.PointingHandCursor
        onClicked: sw.toggled()
    }
}
