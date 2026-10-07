// A color theme to pick: its background with its accent as a dot (light or
// dark, as the interface is now), and its name; the chosen one outlined.
import QtQuick
import QtQuick.Layouts
import qs.services

Rectangle {
    id: swatch

    property string name
    property string label
    property var colors
    property bool picked: false
    signal chosen

    implicitHeight: 52
    radius: 16
    color: picked ? Theme.a(Theme.primary, 0.1) : "transparent"
    border.width: 2
    border.color: picked ? Theme.primary : mouse.containsMouse ? Theme.a(Theme.primary, 0.4) : Theme.border
    y: mouse.containsMouse ? -2 : 0
    Behavior on y {
        NumberAnimation {
            duration: Theme.ms(200)
            easing.type: Easing.BezierSpline
            easing.bezierCurve: Theme.bounce
        }
    }

    RowLayout {
        anchors.fill: parent
        anchors.margins: 8
        spacing: 10
        Rectangle {
            implicitWidth: 32
            implicitHeight: 32
            radius: 11
            color: swatch.colors ? swatch.colors.background : "transparent"
            border.width: 1
            border.color: Theme.a(Theme.foreground, 0.1)
            Rectangle {
                anchors.centerIn: parent
                width: 16
                height: 16
                radius: 8
                color: swatch.colors ? swatch.colors.primary : "transparent"
            }
        }
        Text {
            Layout.fillWidth: true
            text: swatch.label
            color: Theme.foreground
            font.family: Theme.font
            font.pixelSize: 14
            font.weight: Font.Bold
        }
    }
    MouseArea {
        id: mouse
        anchors.fill: parent
        hoverEnabled: true
        cursorShape: Qt.PointingHandCursor
        onClicked: swatch.chosen()
    }
}
