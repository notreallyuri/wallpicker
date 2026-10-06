// The star in a tile's corner: yellow on a favorite, else shown while the
// tile is hovered. Toggles the favorite.
import QtQuick
import qs.components
import qs.services

Rectangle {
    id: star

    property bool favorite: false
    /// Whether the tile under it is hovered.
    property bool tileHovered: false
    readonly property bool hovered: mouse.containsMouse
    signal toggled

    width: 28
    height: 28
    radius: 14
    color: Theme.a(Theme.background, hovered ? 1 : 0.8)
    opacity: favorite || tileHovered ? 1 : 0
    scale: hovered ? 1.1 : 1
    Behavior on opacity {
        NumberAnimation {
            duration: Theme.duration
        }
    }
    Behavior on scale {
        NumberAnimation {
            duration: Theme.duration
            easing.type: Easing.BezierSpline
            easing.bezierCurve: Theme.bounce
        }
    }

    Icon {
        anchors.centerIn: parent
        name: "star"
        filled: star.favorite
        color: star.favorite ? Theme.star : Theme.a(Theme.foreground, 0.8)
    }
    MouseArea {
        id: mouse
        anchors.fill: parent
        hoverEnabled: true
        cursorShape: Qt.PointingHandCursor
        onClicked: star.toggled()
    }
}
