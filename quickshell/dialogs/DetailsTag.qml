// A tag in the details (as components/Chip): its name searches for more
// (its category on hover); the + at its end keeps it, to be added to the
// library with Download or Apply. Kept, it's filled, with a check.
import QtQuick
import QtQuick.Layouts
import qs.components
import qs.services

Rectangle {
    id: tag

    property string text
    /// A tooltip (the tag's category).
    property string tip
    property bool kept: false
    signal search
    signal toggled

    implicitWidth: row.implicitWidth + 12 + 4
    implicitHeight: 28
    radius: height / 2
    color: kept ? Theme.primary : Theme.a(Theme.primary, mouse.containsMouse ? 0.15 : 0.05)
    border.width: kept ? 0 : 1
    border.color: Theme.a(Theme.primary, 0.25)
    Behavior on color {
        ColorAnimation {
            duration: Theme.ms(150)
        }
    }

    // The whole chip searches; the keep button sits over it.
    MouseArea {
        id: mouse
        anchors.fill: parent
        hoverEnabled: true
        cursorShape: Qt.PointingHandCursor
        onClicked: tag.search()
    }
    Tip {
        text: tag.tip
        over: mouse.containsMouse && !keepMouse.containsMouse
        pressed: mouse.pressed
    }

    RowLayout {
        id: row
        anchors.verticalCenter: parent.verticalCenter
        x: 12
        spacing: 4
        Text {
            text: tag.text
            color: tag.kept ? Theme.primaryForeground : Theme.foreground
            font.family: Theme.font
            font.pixelSize: 13
            font.weight: Font.Bold
        }
        Rectangle {
            implicitWidth: 20
            implicitHeight: 20
            radius: 10
            color: !keepMouse.containsMouse ? "transparent" : tag.kept ? Theme.a(Theme.primaryForeground, 0.2) : Theme.a(Theme.primary, 0.2)
            Icon {
                anchors.centerIn: parent
                name: tag.kept ? "check" : "plus"
                size: 12
                color: tag.kept ? Theme.primaryForeground : Theme.mutedForeground
            }
            MouseArea {
                id: keepMouse
                anchors.fill: parent
                hoverEnabled: true
                cursorShape: Qt.PointingHandCursor
                onClicked: tag.toggled()
            }
            Tip {
                text: tag.kept ? "Don't add to your library" : "Add to your library with Download or Apply"
                over: keepMouse.containsMouse
                pressed: keepMouse.pressed
            }
        }
    }
}
