// An on/off switch (the app's Switch): a pill track, primary when on, and a
// knob that slides over with the bouncy ease.
import QtQuick
import qs.services

Rectangle {
    id: toggle

    property bool checked: false
    signal toggled

    readonly property alias hovered: mouse.containsMouse

    implicitWidth: 36
    implicitHeight: 20
    radius: height / 2
    color: checked ? Theme.primary : Theme.a(Theme.input, mouse.containsMouse ? 0.9 : 0.6)
    opacity: enabled ? 1 : 0.5
    Behavior on color {
        ColorAnimation {
            duration: Theme.ms(150)
        }
    }

    Rectangle {
        x: toggle.checked ? toggle.width - width - 2 : 2
        y: 2
        width: toggle.height - 4
        height: width
        radius: width / 2
        color: toggle.checked ? Theme.primaryForeground : Theme.foreground
        Behavior on x {
            NumberAnimation {
                duration: Theme.duration
                easing.type: Easing.BezierSpline
                easing.bezierCurve: Theme.bounce
            }
        }
    }
    MouseArea {
        id: mouse
        anchors.fill: parent
        hoverEnabled: true
        cursorShape: Qt.PointingHandCursor
        onClicked: toggle.toggled()
    }
}
