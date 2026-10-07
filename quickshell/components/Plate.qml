// A frosted plate behind text that would otherwise sit straight on the
// wallpaper (the pager, a status line, "Loading…", "Nothing found"): on a
// bright, busy wallpaper bare text was hard to read and looked ragged.
// The search card's tint and hairline; as big as what it holds.
import QtQuick
import QtQuick.Effects
import qs.services

Item {
    id: plate

    default property alias content: holder.data
    property real padX: 14
    property real padY: 8

    implicitWidth: holder.childrenRect.width + padX * 2
    implicitHeight: holder.childrenRect.height + padY * 2

    RectangularShadow {
        anchors.fill: face
        radius: face.radius
        offset.y: 6
        blur: 16
        color: Qt.rgba(0, 0, 0, 0.15)
    }
    Rectangle {
        id: face
        anchors.fill: parent
        radius: Math.min(22, height / 2)
        color: Theme.a(Theme.background, 0.8)
        border.width: 1
        border.color: Theme.a(Theme.border, 0.6)
    }
    Item {
        id: holder
        x: plate.padX
        y: plate.padY
        width: childrenRect.width
        height: childrenRect.height
    }
}
