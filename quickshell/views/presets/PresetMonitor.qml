import QtQuick
import qs.components
import qs.services

Item {
    id: tile
    required property string monitor
    required property var wallpaper
    required property bool missing
    readonly property bool connected: Monitors.list.some(m => m.name === monitor)
    implicitHeight: body.implicitHeight
    Column {
        id: body
        width: parent.width
        spacing: 6

        LibraryThumb {
            width: tile.width
            height: Math.round(width * 9 / 16)
            path: tile.missing || tile.width < 32 ? "" : tile.wallpaper.wallpaper
            Icon {
                visible: tile.missing
                anchors.centerIn: parent
                name: "info"
                size: 24
                color: Theme.mutedForeground
            }
        }
        Text {
            width: parent.width
            text: tile.monitor
            color: Theme.foreground
            font.family: Theme.font
            font.pixelSize: 13
            font.bold: true
            elide: Text.ElideRight
        }
        Text {
            width: parent.width
            text: tile.missing ? "Missing image" : tile.connected ? tile.wallpaper.wallpaper.split("/").pop() : "Disconnected"
            elide: Text.ElideMiddle
            color: tile.missing || !tile.connected ? "#fd4663" : Theme.mutedForeground
            font.family: Theme.font
            font.pixelSize: 12
        }
    }
    HoverHandler { id: hover }
    Tip {
        text: tile.monitor + "\n" + tile.wallpaper.wallpaper + (tile.missing ? "\nMissing image" : "") + (!tile.connected ? "\nDisconnected" : "")
        over: hover.hovered
    }
}
