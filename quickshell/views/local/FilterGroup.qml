// A group of the Filters popover (the tags, or the artists): its title and
// All N, then its chips with their counts in Filters.js's order, picking
// one filters at once. Three lines until All N (then a scroll past ten);
// while something is typed in the popover's field, every match. Hidden
// when there are none.
import QtQuick
import QtQuick.Layouts
import qs.components
import qs.services
import "Filters.js" as Filters

ColumnLayout {
    id: group

    property string title
    /// [{name, count}]
    property var counts: []
    /// The names chosen.
    property var chosen: []
    /// What's typed in the popover's field ("" for everything).
    property string find
    /// Every chip, not only the first three lines (remembered).
    property bool expanded: false
    signal toggled(string name)

    readonly property var ordered: {
        const all = Filters.ordered(counts, chosen, Prefs.tagOrder);
        const want = find.trim().toLowerCase();
        return want ? all.filter(c => c.name.toLowerCase().includes(want)) : all;
    }
    readonly property bool finding: find.trim() !== ""
    // Three lines of 28 px chips, 6 apart; ten when expanded (then a scroll).
    readonly property real collapsed: 3 * 28 + 2 * 6
    readonly property real tall: 10 * 28 + 9 * 6
    readonly property bool overflowing: chips.implicitHeight > collapsed

    visible: counts.length > 0
    spacing: 8

    RowLayout {
        Layout.fillWidth: true
        spacing: 8
        Text {
            text: group.title
            color: Theme.mutedForeground
            font.family: Theme.font
            font.pixelSize: 12
            font.weight: Font.Bold
        }
        Item {
            Layout.fillWidth: true
        }
        Btn {
            visible: !group.finding && (group.overflowing || group.expanded)
            variant: "ghost"
            small: true
            text: group.expanded ? "Less" : "All " + group.counts.length
            onClicked: group.expanded = !group.expanded
        }
    }

    Flickable {
        Layout.fillWidth: true
        Layout.preferredHeight: Math.min(chips.implicitHeight, group.expanded || group.finding ? group.tall : group.collapsed)
        contentHeight: chips.implicitHeight
        interactive: contentHeight > height
        boundsBehavior: Flickable.StopAtBounds
        clip: true
        Behavior on Layout.preferredHeight {
            NumberAnimation {
                duration: Theme.duration
                easing.type: Easing.OutCubic
            }
        }
        Flow {
            id: chips
            width: parent.width
            spacing: 6
            Repeater {
                model: group.ordered
                Chip {
                    required property var modelData
                    text: modelData.name
                    count: String(modelData.count)
                    on: group.chosen.includes(modelData.name)
                    onClicked: group.toggled(modelData.name)
                }
            }
        }
    }

    Text {
        visible: group.finding && group.ordered.length === 0
        text: "No " + group.title.toLowerCase() + " match."
        color: Theme.mutedForeground
        font.family: Theme.font
        font.pixelSize: 13
    }
}
