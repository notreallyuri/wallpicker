// A modal panel centered in the window over a dim: the frame every dialog
// shares (Dialog, Settings): the popover color, rounded, a soft shadow;
// fades and scales in. Esc or a click on the dim closes it. Its content is
// up to whoever uses it.
import QtQuick
import QtQuick.Controls
import QtQuick.Effects
import qs.services

Popup {
    id: frame

    property real preferredWidth: 520

    // Centered on what it's declared in (the picker), which it follows into
    // whatever window that is. Not Overlay.overlay: the floating panel's
    // window is replaced when it moves to the focused monitor, the old
    // overlay goes with it, and a dialog parented there was left with no
    // window: open() did nothing, silently.
    anchors.centerIn: parent
    width: Math.min(preferredWidth, parent ? parent.width - 48 : preferredWidth)
    modal: true
    padding: 24
    closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutside
    // Modal, so the window's shortcuts are held back while it's open: it
    // takes the keyboard and closes itself on Esc (closePolicy).
    focus: true
    onOpened: Dialogs.opened(this)
    onClosed: Dialogs.closed(this)

    Overlay.modal: Rectangle {
        color: Qt.rgba(0, 0, 0, 0.45)
        Behavior on opacity {
            NumberAnimation {
                duration: 150
            }
        }
    }

    background: Item {
        RectangularShadow {
            anchors.fill: panel
            radius: panel.radius
            offset.y: 16
            blur: 40
            color: Qt.rgba(0, 0, 0, 0.4)
        }
        Rectangle {
            id: panel
            anchors.fill: parent
            radius: 24
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
            duration: 150
        }
        NumberAnimation {
            property: "scale"
            from: 0.95
            to: 1
            duration: 220
            easing.type: Easing.OutCubic
        }
    }
    exit: Transition {
        NumberAnimation {
            property: "opacity"
            to: 0
            duration: 120
        }
    }
}
