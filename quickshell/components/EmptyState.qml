// Where content would be and there's none (or something went wrong): a
// tinted mark, a title, a line of help and at most one action, on a card
// that reads over any wallpaper. One pattern for every view; the toasts
// use the same marks.
import QtQuick
import QtQuick.Layouts
import qs.services

Rectangle {
    id: empty

    property string icon: "folder"
    property string title
    property string text
    /// The one action's label ("" none).
    property string action
    /// Something went wrong: the mark is red.
    property bool error: false
    signal triggered

    implicitWidth: 420
    implicitHeight: column.implicitHeight + 56
    radius: 24
    color: Theme.a(Theme.card, 0.9)
    // The tiles' hairline.
    border.width: 1
    border.color: Theme.a(Theme.foreground, 0.05)

    ColumnLayout {
        id: column
        anchors.centerIn: parent
        width: parent.width - 56
        spacing: 10
        Rectangle {
            Layout.alignment: Qt.AlignHCenter
            implicitWidth: 48
            implicitHeight: 48
            radius: 24
            color: Theme.a(empty.error ? Theme.destructive : Theme.primary, 0.12)
            Icon {
                anchors.centerIn: parent
                name: empty.icon
                size: 22
                color: empty.error ? Theme.destructive : Theme.primary
            }
        }
        Text {
            Layout.fillWidth: true
            horizontalAlignment: Text.AlignHCenter
            wrapMode: Text.Wrap
            text: empty.title
            color: Theme.foreground
            font.family: Theme.font
            font.pixelSize: 16
            font.weight: Font.ExtraBold
        }
        Text {
            visible: empty.text !== ""
            Layout.fillWidth: true
            horizontalAlignment: Text.AlignHCenter
            wrapMode: Text.Wrap
            text: empty.text
            color: Theme.mutedForeground
            font.family: Theme.font
            font.pixelSize: 13
        }
        Btn {
            visible: empty.action !== ""
            Layout.alignment: Qt.AlignHCenter
            Layout.topMargin: 4
            small: true
            text: empty.action
            onClicked: empty.triggered()
        }
    }
}
