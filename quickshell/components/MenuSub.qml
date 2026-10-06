// A menu entry opening another menu at its side (on hover or click).
import QtQuick
import qs.services

MenuItem {
    id: sub

    default property alias items: nested.items

    closes: false
    onTriggered: nested.openAt(sub, sub.width + 4, -6)
    onHoveredChanged: if (hovered)
        nested.openAt(sub, sub.width + 4, -6)

    Icon {
        name: "chevron-right"
        color: Theme.mutedForeground
    }

    Menu {
        id: nested
    }
}
