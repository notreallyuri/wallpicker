// The search card's tag row: every tag with its count, toggling the filter;
// one line, or all of them when expanded.
import QtQuick
import QtQuick.Layouts
import qs.components
import qs.services

RowLayout {
    id: row

    required property LocalModel local
    property bool expanded: false

    spacing: 8

    Text {
        Layout.alignment: Qt.AlignTop
        Layout.topMargin: 5
        text: "Tags"
        color: Theme.mutedForeground
        font.family: Theme.font
        font.pixelSize: 14
    }
    Flow {
        id: tags
        Layout.fillWidth: true
        Layout.preferredHeight: row.expanded ? implicitHeight : 28
        spacing: 8
        clip: true
        Behavior on Layout.preferredHeight {
            NumberAnimation {
                duration: Theme.duration
                easing.type: Easing.OutCubic
            }
        }
        Repeater {
            model: Library.tagCounts
            Chip {
                required property var modelData
                text: modelData.name + "  " + modelData.count
                on: row.local.tagFilter.includes(modelData.name)
                onClicked: row.local.toggleTag(modelData.name)
            }
        }
    }
    Btn {
        Layout.alignment: Qt.AlignTop
        implicitHeight: 28
        variant: "ghost"
        text: row.expanded ? "Less" : "Expand"
        onClicked: row.expanded = !row.expanded
    }
}
