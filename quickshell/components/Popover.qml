// A floating panel (the app's popovers and menus): rounded, the popover
// color, a soft shadow; fades and scales in. Closes on a click outside or
// Esc.
import QtQuick
import QtQuick.Controls
import QtQuick.Effects
import qs.services

Popup {
    id: popover

    padding: 6
    closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutside
    // Takes the keyboard while open and closes itself on Esc (closePolicy):
    // the window's shortcuts are held back while a popup is open. (One Esc
    // shortcut in all: several for the same key make Qt fire none.)
    focus: true
    onOpened: Dialogs.opened(this)
    onClosed: Dialogs.closed(this)

    background: Item {
        RectangularShadow {
            anchors.fill: panel
            radius: panel.radius
            offset.y: 8
            blur: 24
            color: Qt.rgba(0, 0, 0, 0.25)
        }
        Rectangle {
            id: panel
            anchors.fill: parent
            radius: Theme.radiusXl
            color: Theme.popover
            border.width: 1
            border.color: Theme.a(Theme.border, 0.6)
        }
    }

    enter: Transition {
        NumberAnimation {
            property: "opacity"
            from: 0
            to: 1
            duration: Theme.ms(150)
        }
        NumberAnimation {
            property: "scale"
            from: 0.96
            to: 1
            duration: Theme.ms(200)
            easing.type: Easing.OutCubic
        }
    }
    exit: Transition {
        NumberAnimation {
            property: "opacity"
            to: 0
            duration: Theme.ms(100)
        }
    }
}
