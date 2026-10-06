// A menu entry that ticks on and off (Apply to…'s monitors); the menu stays
// open.
import QtQuick
import qs.services

MenuItem {
    id: check

    property bool checked: false

    closes: false
    checkSpace: true

    Icon {
        parent: check
        x: 10
        anchors.verticalCenter: parent.verticalCenter
        visible: check.checked
        name: "check"
        color: Theme.primary
    }
}
