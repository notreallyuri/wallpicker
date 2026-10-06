// A little monitor showing its wallpaper (the pills and the All desk).
import QtQuick
import Quickshell.Widgets
import qs.services

Rectangle {
    id: screen

    /// The wallpaper's path, or "".
    property string path
    property real screenWidth: 30
    property real screenHeight: 20

    readonly property var image: Library.images.find(i => i.path === path)

    implicitWidth: screenWidth
    implicitHeight: screenHeight
    radius: 4
    color: "#18181b"

    ClippingRectangle {
        anchors.fill: parent
        anchors.margins: 2
        radius: 2
        color: "#27272a"
        Image {
            anchors.fill: parent
            source: screen.image && screen.image.thumb ? "file://" + screen.image.thumb : ""
            fillMode: Image.PreserveAspectCrop
            sourceSize.width: 64
            asynchronous: true
        }
    }
}
