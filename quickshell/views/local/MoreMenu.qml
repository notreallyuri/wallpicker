// Local's ⋯: Find duplicates… and Rescan the folders, each with a line
// saying what it does.
import QtQuick
import qs.components
import qs.services

Btn {
    id: more

    variant: "ghost"
    icon: "ellipsis"
    onClicked: menu.openAt(more, more.width - 300, more.height + 6)

    Menu {
        id: menu
        minWidth: 300
        MenuItem {
            icon: "copy"
            text: "Find duplicates…"
            description: "Copies, and the same picture at other sizes"
            onTriggered: Dialogs.findDuplicates()
        }
        MenuItem {
            icon: "refresh"
            text: "Rescan the folders"
            description: "Look for new, moved or deleted wallpapers"
            onTriggered: Library.reload()
        }
    }
}
