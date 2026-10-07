// One workspace in Settings → Workspaces: its name, its wallpaper (a click
// chooses another; "+ Choose" when it has none), where it is, "Use DP-1's"
// when its monitor shows something else, Clear.
import QtQuick
import QtQuick.Layouts
import qs.components
import qs.services

Rectangle {
    id: row

    required property var ws
    signal choose
    signal assign(var path)

    /// What its monitor shows now, when that isn't its wallpaper.
    readonly property string shownElse: {
        const shown = ws.monitor ? Monitors.current[ws.monitor] || "" : "";
        return shown && shown !== ws.wallpaper ? shown : "";
    }

    implicitHeight: 64
    radius: 16
    color: Theme.a(Theme.card, 0.6)

    RowLayout {
        anchors.fill: parent
        anchors.margins: 8
        spacing: 12
        Rectangle {
            implicitWidth: 40
            implicitHeight: 40
            radius: 12
            color: Theme.a(Theme.primary, 0.15)
            Text {
                anchors.centerIn: parent
                text: row.ws.name.slice(0, 3)
                color: Theme.primary
                font.family: Theme.font
                font.pixelSize: 14
                font.weight: Font.ExtraBold
            }
        }
        // The picture is a way in to choosing too.
        Item {
            implicitWidth: 80
            implicitHeight: 48
            LibraryThumb {
                visible: !!row.ws.wallpaper
                anchors.fill: parent
                path: row.ws.wallpaper || ""
            }
            Rectangle {
                visible: !row.ws.wallpaper
                anchors.fill: parent
                radius: 10
                color: "transparent"
                border.width: 2
                border.color: pick.containsMouse ? Theme.primary : Theme.border
                Text {
                    anchors.centerIn: parent
                    text: "+ Choose"
                    color: pick.containsMouse ? Theme.primary : Theme.mutedForeground
                    font.family: Theme.font
                    font.pixelSize: 12
                    font.weight: Font.Bold
                }
            }
            MouseArea {
                id: pick
                anchors.fill: parent
                hoverEnabled: true
                cursorShape: Qt.PointingHandCursor
                onClicked: row.choose()
            }
        }
        ColumnLayout {
            Layout.fillWidth: true
            spacing: 2
            Text {
                text: row.ws.monitor ? "on " + row.ws.monitor : row.ws.open ? "open" : "not open now"
                color: Theme.mutedForeground
                font.family: Theme.font
                font.pixelSize: 12
            }
            Text {
                Layout.fillWidth: true
                text: row.ws.wallpaper ? row.ws.wallpaper.split("/").pop() : "No wallpaper"
                elide: Text.ElideMiddle
                color: row.ws.wallpaper ? Theme.foreground : Theme.mutedForeground
                font.family: Theme.font
                font.pixelSize: 13
            }
        }
        Btn {
            visible: row.shownElse !== ""
            small: true
            text: "Use " + row.ws.monitor + "'s"
            tip: "Make " + row.shownElse.split("/").pop() + " this workspace's wallpaper"
            onClicked: row.assign(row.shownElse)
        }
        Btn {
            visible: !!row.ws.wallpaper
            small: true
            variant: "ghost"
            text: "Clear"
            onClicked: row.assign(null)
        }
    }
}
