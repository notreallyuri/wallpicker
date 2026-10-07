// A form field (the app's Field): a bold label, the controls, and an
// optional muted note under them.
import QtQuick
import QtQuick.Layouts
import qs.services

ColumnLayout {
    id: field

    property string label
    property string description
    default property alias content: body.data

    spacing: 8

    Text {
        text: field.label
        color: Theme.foreground
        font.family: Theme.font
        font.pixelSize: 14
        font.weight: Font.Bold
    }
    ColumnLayout {
        id: body
        Layout.fillWidth: true
        spacing: 8
    }
    Text {
        visible: field.description !== ""
        Layout.fillWidth: true
        text: field.description
        wrapMode: Text.Wrap
        color: Theme.mutedForeground
        font.family: Theme.font
        font.pixelSize: 13
    }
}
