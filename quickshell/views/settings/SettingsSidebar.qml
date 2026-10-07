// The Settings dialog's sections, by group (App, Providers): the chosen one
// highlighted; a click shows it.
import QtQuick
import QtQuick.Layouts
import qs.services

Rectangle {
    id: sidebar

    property var sections: []
    property string current
    signal picked(string slug)

    color: Theme.a(Theme.card, 0.6)

    ColumnLayout {
        x: 10
        y: 0
        width: parent.width - 20
        spacing: 2

        // The dialog's name, as tall as the pane's header (the rules meet).
        Text {
            Layout.preferredHeight: 52
            Layout.leftMargin: 8
            verticalAlignment: Text.AlignVCenter
            text: "Settings"
            color: Theme.foreground
            font.family: Theme.font
            font.pixelSize: 14
            font.weight: Font.ExtraBold
        }
        Repeater {
            model: ["App", "Providers"]
            ColumnLayout {
                id: group
                required property string modelData
                Layout.fillWidth: true
                Layout.topMargin: 8
                spacing: 2
                Text {
                    Layout.leftMargin: 8
                    Layout.bottomMargin: 4
                    text: group.modelData
                    color: Theme.mutedForeground
                    font.family: Theme.font
                    font.pixelSize: 12
                    font.weight: Font.Bold
                }
                Repeater {
                    model: sidebar.sections.filter(s => s.group === group.modelData)
                    Rectangle {
                        id: entry
                        required property var modelData
                        readonly property bool on: modelData.slug === sidebar.current
                        Layout.fillWidth: true
                        implicitHeight: 34
                        radius: 10
                        color: on ? Theme.a(Theme.primary, 0.18) : mouse.containsMouse ? Theme.a(Theme.primary, 0.08) : "transparent"
                        Text {
                            anchors.verticalCenter: parent.verticalCenter
                            x: 10
                            text: entry.modelData.label
                            color: entry.on ? Theme.primary : Theme.foreground
                            font.family: Theme.font
                            font.pixelSize: 14
                            font.weight: entry.on ? Font.Bold : Font.Medium
                        }
                        MouseArea {
                            id: mouse
                            anchors.fill: parent
                            hoverEnabled: true
                            cursorShape: Qt.PointingHandCursor
                            onClicked: sidebar.picked(entry.modelData.slug)
                        }
                    }
                }
            }
        }
    }

    // The rule between it and the pane.
    Rectangle {
        anchors.right: parent.right
        width: 1
        height: parent.height
        color: Theme.a(Theme.border, 0.6)
    }
}
