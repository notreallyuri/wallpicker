// A row of the search card (tags.rs): every tag (or artist) in use with its
// count, the chosen ones first, toggling the filter; All | Any once two are
// chosen (where `matchAll` applies); Clear; one line, or all of them with
// Expand (only when they don't fit). Hidden when there are none.
import QtQuick
import QtQuick.Layouts
import qs.components
import qs.services

RowLayout {
    id: row

    property string label
    /// [{name, count}]
    property var counts: []
    /// The names chosen.
    property var chosen: []
    /// The All | Any switch: shown when it's not null.
    property var matchAll: null
    property bool expanded: false
    /// The label's width, to line rows up (-1: its own).
    property real labelWidth: -1
    readonly property real labelImplicitWidth: labelText.implicitWidth
    signal toggled(string name)
    signal cleared
    signal matchToggled

    readonly property var ordered: counts.slice().sort((a, b) => chosen.includes(b.name) - chosen.includes(a.name))
    // Whether the chips run past one line.
    readonly property bool overflowing: chips.implicitHeight > 30

    visible: counts.length > 0
    spacing: 8

    Text {
        id: labelText
        Layout.alignment: Qt.AlignTop
        Layout.topMargin: 5
        Layout.preferredWidth: row.labelWidth
        text: row.label
        color: Theme.mutedForeground
        font.family: Theme.font
        font.pixelSize: 14
    }
    Flow {
        id: chips
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
                on: row.chosen.includes(modelData.name)
                onClicked: row.toggled(modelData.name)
            }
        }
    }
    Switch2 {
        id: matchSwitch
        visible: row.matchAll !== null && row.chosen.length > 1
        Layout.alignment: Qt.AlignTop
        onRight: row.matchAll === false
        onToggled: row.matchToggled()
        Tip {
            over: matchSwitch.hovered
            pressed: matchSwitch.pressed
            text: row.matchAll ? "Wallpapers with every selected tag" : "Wallpapers with any selected tag"
        }
    }
    Btn {
        visible: row.chosen.length > 0
        Layout.alignment: Qt.AlignTop
        implicitHeight: 28
        variant: "ghost"
        icon: "x"
        text: "Clear"
        onClicked: row.cleared()
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
