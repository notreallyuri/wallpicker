// Something to know in a Settings section (not a choice's meaning, which
// is a Note): a tinted box with an info mark.
import QtQuick
import QtQuick.Layouts
import qs.components
import qs.services

Rectangle {
    id: notice

    property string text

    Layout.fillWidth: true
    implicitHeight: row.implicitHeight + 20
    radius: 12
    color: Theme.a(Theme.primary, 0.06)

    RowLayout {
        id: row
        x: 14
        y: 10
        width: parent.width - 28
        spacing: 10
        Icon {
            Layout.alignment: Qt.AlignTop
            Layout.topMargin: 1
            name: "info"
            size: 16
            color: Theme.primary
        }
        Text {
            Layout.fillWidth: true
            wrapMode: Text.Wrap
            text: notice.text
            color: Theme.mutedForeground
            font.family: Theme.font
            font.pixelSize: 12
        }
    }
}
