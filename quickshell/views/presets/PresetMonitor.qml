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
            height: Math.round(width * 10 / 16)
            // Not connected: dimmed, it still applies where it can.
            opacity: tile.connected || tile.missing ? 1 : 0.4
            path: tile.missing || tile.width < 32 ? "" : tile.wallpaper.wallpaper
            Icon {
                visible: tile.missing
                anchors.centerIn: parent
                name: "info"
                size: 24
                color: Theme.mutedForeground
            }
        }
        // Its name, and its file (or why not) at the right.
        Row {
            width: parent.width
            spacing: 6
            Text {
                id: name
                text: tile.monitor
                color: tile.connected ? Theme.foreground : Theme.mutedForeground
                font.family: Theme.font
                font.pixelSize: 13
                font.bold: true
            }
            Text {
                width: parent.width - name.width - parent.spacing
                anchors.baseline: name.baseline
                horizontalAlignment: Text.AlignRight
                text: tile.missing ? "Missing image" : tile.connected ? tile.wallpaper.wallpaper.split("/").pop() : "Not connected"
                elide: Text.ElideMiddle
                // A missing file is a problem; a monitor away isn't.
                color: tile.missing ? Theme.destructive : Theme.mutedForeground
                font.family: Theme.font
                font.pixelSize: 12
            }
        }
    }
    HoverHandler { id: hover }
    Tip {
        text: tile.monitor + "\n" + tile.wallpaper.wallpaper + (tile.missing ? "\nMissing image" : "") + (!tile.connected ? "\nNot connected: the rest of the preset still applies" : "")
        over: hover.hovered
    }
}
