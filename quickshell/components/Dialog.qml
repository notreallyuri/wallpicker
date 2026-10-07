// A modal dialog, centered in the window over a dim (the Tags, Trash and
// Duplicates dialogs): a title, a description, the content, the footer's
// buttons at the right. Esc or a click on the dim closes it.
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
    property real preferredWidth: 520

    parent: Overlay.overlay
    anchors.centerIn: parent
    width: Math.min(preferredWidth, parent ? parent.width - 48 : preferredWidth)
    modal: true
    padding: 24
    closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutside
    // Takes the keyboard while open (Esc reaches it); Esc closes the top
    // one (services/Dialogs.closeTop).
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
        // Esc closes it (as in Popover).
        Shortcut {
            sequence: "Escape"
            onActivated: dialog.close()
        }
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
            id: footerRow
            Layout.alignment: Qt.AlignRight
            spacing: 8
        }
    }
}
