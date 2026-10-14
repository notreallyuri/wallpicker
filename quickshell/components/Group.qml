// A titled card of settings: its rows (Field, ToggleField) on one card
// with a hairline between them, as the refreshed Settings lay them out.
import QtQuick
import QtQuick.Layouts
import qs.services

ColumnLayout {
    id: group

    property string title
    default property alias rows: list.data

    Layout.fillWidth: true
    spacing: 8

    Text {
        visible: group.title !== ""
        text: group.title
        color: Theme.mutedForeground
        font.family: Theme.font
        font.pixelSize: 12
        font.weight: Font.Bold
    }
    Rectangle {
        Layout.fillWidth: true
        implicitHeight: list.implicitHeight
        radius: 14
        color: Theme.a(Theme.card, 0.6)
        // The tiles' hairline.
        border.width: 1
        border.color: Theme.a(Theme.foreground, 0.05)
        ColumnLayout {
            id: list
            /// Its rows draw the hairlines between them (Field, ToggleField).
            readonly property bool grouped: true
            x: 16
            width: parent.width - 32
            spacing: 0
        }
    }
}
