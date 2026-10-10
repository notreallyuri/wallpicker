import QtQuick
import qs.components

Btn {
    activeFocusOnTab: true
    active: activeFocus
    Accessible.role: Accessible.Button
    Accessible.name: text
    Keys.onReturnPressed: clicked()
    Keys.onEnterPressed: clicked()
    Keys.onSpacePressed: clicked()
}
