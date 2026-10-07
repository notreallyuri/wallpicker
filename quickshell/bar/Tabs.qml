// The view tabs: a muted track, and a primary pill behind the chosen one
// that slides to the next with the bouncy ease (like the All | Any switch).
import QtQuick
import qs.services

Rectangle {
    id: tabs

    property var names: ["Local", "Online", "Collections"]
    property int current: 0
    signal picked(int index)

    implicitWidth: row.implicitWidth + 8
    implicitHeight: 40
    radius: height / 2
    color: Theme.muted

    // The pill, under the chosen tab.
    Rectangle {
        // (count: itemAt says nothing when the tabs are made.)
        readonly property Item under: tabRepeater.count > 0 ? tabRepeater.itemAt(tabs.current) : null
        x: row.x + (under ? under.x : 0)
        y: row.y
        width: under ? under.width : 0
        height: 32
        radius: height / 2
        color: Theme.primary
        Behavior on x {
            NumberAnimation {
                duration: Theme.duration
                easing.type: Easing.BezierSpline
                easing.bezierCurve: Theme.bounce
            }
        }
        Behavior on width {
            NumberAnimation {
                duration: Theme.duration
                easing.type: Easing.BezierSpline
                easing.bezierCurve: Theme.bounce
            }
        }
    }

    Row {
        id: row
        anchors.centerIn: parent
        spacing: 2
        Repeater {
            id: tabRepeater
            model: tabs.names
            Rectangle {
                id: tab
                required property string modelData
                required property int index
                readonly property bool on: index === tabs.current
                width: label.implicitWidth + 32
                height: 32
                radius: height / 2
                color: !on && mouse.containsMouse ? Theme.a(Theme.primary, 0.12) : "transparent"
                Behavior on color {
                    ColorAnimation {
                        duration: 200
                    }
                }
                Text {
                    id: label
                    anchors.centerIn: parent
                    text: tab.modelData
                    color: tab.on ? Theme.primaryForeground : Theme.mutedForeground
                    Behavior on color {
                        ColorAnimation {
                            duration: 200
                        }
                    }
                    font.family: Theme.font
                    font.pixelSize: 14
                    font.weight: Font.Bold
                }
                MouseArea {
                    id: mouse
                    anchors.fill: parent
                    hoverEnabled: true
                    cursorShape: Qt.PointingHandCursor
                    onClicked: tabs.picked(tab.index)
                }
            }
        }
    }
}
