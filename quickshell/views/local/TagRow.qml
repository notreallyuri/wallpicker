// The search card's tag row (tags.rs): every tag in use with its count,
// the chosen ones first, toggling the filter; All | Any once two are chosen;
// Clear; one line, or all of them with Expand (only when they don't fit).
// Hidden when there are no tags.
import QtQuick
import QtQuick.Layouts
import qs.components
import qs.services

RowLayout {
    id: row

    required property LocalModel local
    property bool expanded: Prefs.tagsExpanded
    onExpandedChanged: Prefs.tagsExpanded = expanded

    readonly property var ordered: {
        const chosen = local.tagFilter;
        return Library.tagCounts.slice().sort((a, b) => chosen.includes(b.name) - chosen.includes(a.name));
    }
    // Whether the chips run past one line.
    readonly property bool overflowing: tags.implicitHeight > 30

    visible: Library.tagCounts.length > 0
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
        spacing: 6
        clip: true
        Behavior on Layout.preferredHeight {
            NumberAnimation {
                duration: Theme.duration
                easing.type: Easing.OutCubic
            }
        }
        Repeater {
            model: row.ordered
            Chip {
                required property var modelData
                text: modelData.name
                count: String(modelData.count)
                on: row.local.tagFilter.includes(modelData.name)
                onClicked: row.local.toggleTag(modelData.name)
            }
        }
    }
    Switch2 {
        id: matchSwitch
        visible: row.local.tagFilter.length > 1
        Layout.alignment: Qt.AlignTop
        onRight: !row.local.tagsMatchAll
        onToggled: row.local.tagsMatchAll = !row.local.tagsMatchAll
        Tip {
            over: matchSwitch.hovered
            pressed: matchSwitch.pressed
            text: row.local.tagsMatchAll ? "Wallpapers with every selected tag" : "Wallpapers with any selected tag"
        }
    }
    Btn {
        visible: row.local.tagFilter.length > 0
        Layout.alignment: Qt.AlignTop
        implicitHeight: 28
        variant: "ghost"
        icon: "x"
        text: "Clear"
        onClicked: row.local.tagFilter = []
    }
    Btn {
        visible: row.overflowing || row.expanded
        Layout.alignment: Qt.AlignTop
        implicitHeight: 28
        variant: "ghost"
        text: row.expanded ? "Less" : "Expand"
        onClicked: row.expanded = !row.expanded
    }
}
