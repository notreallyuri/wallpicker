// A menu entry opening another menu at its side (on hover or click). Its
// children are the nested menu's entries. It closes once the pointer has
// been off both this entry and the nested menu for a moment (on another
// entry, or out of the menus): the moment lets the pointer cross the gap.
import QtQuick
import qs.services

MenuItem {
    id: sub

    default property alias items: nested.items

    closes: false
    onTriggered: nested.openAt(sub, sub.width + 4, -6)
    onHoveredChanged: if (hovered)
        nested.openAt(sub, sub.width + 4, -6)

    trailing: [
        Icon {
            name: "chevron-right"
            color: Theme.mutedForeground
        }
    ]

    Menu {
        id: nested
    }

    Timer {
        running: nested.opened && !sub.hovered && !nested.hovered
        interval: 150
        onTriggered: nested.close()
    }
}
