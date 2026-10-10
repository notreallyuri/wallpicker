// One daily snapshot: its day and how many wallpapers; Restore asks first
// (Cancel, or Restore in red).
import QtQuick
import QtQuick.Layouts
import qs.components
import qs.services

RowLayout {
    id: row

    required property var snapshot
    property bool busy: false
    property bool asking: false
    signal restore

    spacing: 8

    Text {
        Layout.preferredWidth: 120
        text: row.snapshot.label
        color: Theme.foreground
        font.family: Theme.font
        font.pixelSize: 13
        font.weight: Font.Bold
    }
    Text {
        Layout.fillWidth: true
        text: row.snapshot.items + (row.snapshot.items === 1 ? " wallpaper" : " wallpapers") + (row.snapshot.presets === null ? " · library only" : " · " + row.snapshot.presets + " presets")
        color: Theme.mutedForeground
        font.family: Theme.font
        font.pixelSize: 13
    }
    Btn {
        visible: !row.asking
        small: true
        variant: "ghost"
        text: "Restore"
        opacity: row.busy ? 0.5 : 1
        onClicked: if (!row.busy)
            row.asking = true
    }
    Btn {
        visible: row.asking
        small: true
        variant: "ghost"
        text: "Cancel"
        onClicked: row.asking = false
    }
    Btn {
        visible: row.asking
        small: true
        variant: "destructive"
        text: "Restore"
        onClicked: {
            row.asking = false;
            row.restore();
        }
    }
}
