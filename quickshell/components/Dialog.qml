// A modal dialog, centered in the window over a dim (the Tags, Trash,
// Duplicates and Adjust dialogs): a title, a description, the content, the
// footer's buttons at the right (and anything at its left). Esc or a click
// on the dim closes it.
import QtQuick
import QtQuick.Controls
import QtQuick.Effects
import QtQuick.Layouts
import qs.services

Popup {
    id: dialog

    property string title
    property string description
    default property alias content: body.data
    property alias footer: footerRow.data
    /// Footer content at the left (a status line).
    property alias footerLeft: footerLeftRow.data
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

    contentItem: ColumnLayout {
        spacing: 16
        ColumnLayout {
            spacing: 4
            Text {
                Layout.fillWidth: true
                text: dialog.title
                wrapMode: Text.Wrap
                color: Theme.foreground
                font.family: Theme.font
                font.pixelSize: 18
                font.weight: Font.ExtraBold
            }
            Text {
                visible: dialog.description !== ""
                Layout.fillWidth: true
                text: dialog.description
                wrapMode: Text.Wrap
                color: Theme.mutedForeground
                font.family: Theme.font
                font.pixelSize: 13
            }
        }
        ColumnLayout {
            id: body
            Layout.fillWidth: true
            spacing: 12
        }
        RowLayout {
            Layout.fillWidth: true
            spacing: 8
            RowLayout {
                id: footerLeftRow
                spacing: 8
            }
            Item {
                Layout.fillWidth: true
            }
            RowLayout {
                id: footerRow
                spacing: 8
            }
        }
    }
}
