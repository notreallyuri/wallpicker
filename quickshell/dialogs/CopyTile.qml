// One copy in a duplicate group: its picture with Keep or Trash, its name,
// its folder (copies often share a name), size; a click keeps it instead.
import QtQuick
import QtQuick.Layouts
import Quickshell.Widgets
import qs.components
import qs.services

Rectangle {
    id: copy

    required property var item
    property bool kept: false
    signal picked

    property string thumb: ""
    readonly property var parts: item.path.split("/")

    implicitHeight: column.implicitHeight + 8
    radius: 12
    color: "transparent"
    border.width: kept ? 2 : 1
    border.color: kept ? Theme.primary : Theme.a(Theme.foreground, 0.1)
    opacity: kept || mouse.containsMouse ? 1 : 0.6
    Behavior on opacity {
        NumberAnimation {
            duration: Theme.ms(150)
        }
    }

    Component.onCompleted: Library.thumbnail(item.path, f => copy.thumb = f)

    ColumnLayout {
        id: column
        x: 4
        y: 4
        width: parent.width - 8
        spacing: 2
        ClippingRectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: width * 10 / 16
            radius: 8
            color: Theme.muted
            Image {
                anchors.fill: parent
                source: copy.thumb ? "file://" + copy.thumb : ""
                fillMode: Image.PreserveAspectCrop
                asynchronous: true
                cache: false
                sourceSize.width: width * 2
            }
            Rectangle {
                x: 4
                y: 4
                width: badge.implicitWidth + 12
                height: 18
                radius: 6
                color: copy.kept ? Theme.primary : Qt.rgba(0.99, 0.27, 0.39, 0.8)
                Text {
                    id: badge
                    anchors.centerIn: parent
                    text: copy.kept ? "Keep" : "Trash"
                    color: copy.kept ? Theme.primaryForeground : "white"
                    font.family: Theme.font
                    font.pixelSize: 10
                    font.weight: Font.Medium
                }
            }
        }
        Label2 {
            text: copy.parts[copy.parts.length - 1]
            muted: false
        }
        Label2 {
            text: "in " + (copy.parts[copy.parts.length - 2] || "")
        }
        Label2 {
            text: copy.item.width + "×" + copy.item.height + " · " + (copy.item.size / 1000000).toFixed(1) + " MB"
        }
    }
    MouseArea {
        id: mouse
        anchors.fill: parent
        hoverEnabled: true
        cursorShape: Qt.PointingHandCursor
        onClicked: copy.picked()
    }

    component Label2: Text {
        property bool muted: true
        Layout.fillWidth: true
        elide: Text.ElideRight
        color: muted ? Theme.mutedForeground : Theme.foreground
        font.family: Theme.font
        font.pixelSize: 12
    }
}
