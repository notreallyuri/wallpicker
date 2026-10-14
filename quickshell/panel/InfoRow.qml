// A fact in the control panel's About: the label at the left, the value
// at the right (cut short with … when it doesn't fit). Hidden when empty.
import QtQuick
import QtQuick.Layouts
import qs.services

RowLayout {
    property string label
    property string value

    visible: value !== ""
    spacing: 12

    Text {
        text: parent.label
        color: Theme.mutedForeground
        font.family: Theme.font
        font.pixelSize: 13
    }
    Text {
        Layout.fillWidth: true
        horizontalAlignment: Text.AlignRight
        elide: Text.ElideMiddle
        text: parent.value
        color: Theme.foreground
        font.family: Theme.font
        font.pixelSize: 13
        font.weight: Font.Bold
    }
}
