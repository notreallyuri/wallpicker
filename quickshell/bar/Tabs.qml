// The view tabs: a muted track, the chosen one a primary pill.
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

    Row {
        id: row
        anchors.centerIn: parent
        spacing: 2
        Repeater {
            model: tabs.names
            Rectangle {
                id: tab
                required property string modelData
                required property int index
                readonly property bool on: index === tabs.current
                width: label.implicitWidth + 32
                height: 32
                radius: height / 2
                color: on ? Theme.primary : (mouse.containsMouse ? Theme.a(Theme.primary, 0.12) : "transparent")
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
