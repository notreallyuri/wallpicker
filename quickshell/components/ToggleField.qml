// A switch with its label beside it (the app's horizontal Field), and an
// optional muted note under them. The label toggles it too.
import QtQuick
import QtQuick.Layouts
import qs.services

ColumnLayout {
    id: field

    property string label
    property string description
    property bool checked: false
    signal toggled

    spacing: 6

    RowLayout {
        spacing: 12
        Toggle {
            checked: field.checked
            enabled: field.enabled
            onToggled: field.toggled()
        }
        Text {
            Layout.fillWidth: true
            text: field.label
            wrapMode: Text.Wrap
            color: Theme.foreground
            font.family: Theme.font
            font.pixelSize: 14
            font.weight: Font.Bold
            MouseArea {
                anchors.fill: parent
                enabled: field.enabled
                cursorShape: Qt.PointingHandCursor
                onClicked: field.toggled()
            }
        }
    }
    Text {
        visible: field.description !== ""
        Layout.fillWidth: true
        Layout.leftMargin: 48
        text: field.description
        wrapMode: Text.Wrap
        color: Theme.mutedForeground
        font.family: Theme.font
        font.pixelSize: 13
    }
}
