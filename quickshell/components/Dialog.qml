// A modal dialog, centered in the window over a dim (the Tags, Trash,
// Duplicates and Adjust dialogs): a title, a description, the content, the
// footer's buttons at the right (and anything at its left). Esc or a click
// on the dim closes it.
import QtQuick
import QtQuick.Layouts
import qs.services

DialogFrame {
    id: dialog

    property string title
    property string description
    default property alias content: body.data
    property alias footer: footerRow.data
    /// Footer content at the left (a status line).
    property alias footerLeft: footerLeftRow.data

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
