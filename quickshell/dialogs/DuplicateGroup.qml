// A group of copies (duplicates.rs's GroupCard): what kind, how many, the
// copies (the first, the largest, is kept unless another is clicked), Skip,
// and Keep it, trash the others.
import QtQuick
import QtQuick.Layouts
import qs.components
import qs.services

Rectangle {
    id: group

    required property var info
    /// Done with it (resolved or skipped): `first` names it.
    signal done(string first)

    property int keep: 0
    property bool working: false
    readonly property int others: info.items.length - 1

    implicitHeight: column.implicitHeight + 24
    radius: 16
    color: "transparent"
    border.width: 1
    border.color: Theme.a(Theme.border, 0.8)

    ColumnLayout {
        id: column
        x: 12
        y: 12
        width: parent.width - 24
        spacing: 12
        RowLayout {
            Layout.fillWidth: true
            Text {
                Layout.fillWidth: true
                text: group.info.likeness === "exact" ? "Exact copies" : "Same picture"
                color: Theme.foreground
                font.family: Theme.font
                font.pixelSize: 14
                font.weight: Font.Medium
            }
            Text {
                text: group.info.items.length + " files"
                color: Theme.mutedForeground
                font.family: Theme.font
                font.pixelSize: 14
            }
        }
        // A quarter of the width each, however many (the app's 4 columns),
        // from the group's own width (a grid sizing them from its own would
        // go in circles).
        Flow {
            Layout.fillWidth: true
            spacing: 12
            Repeater {
                model: group.info.items
                CopyTile {
                    required property var modelData
                    required property int index
                    width: (column.width - 3 * 12) / 4
                    item: modelData
                    kept: group.keep === index
                    onPicked: group.keep = index
                }
            }
        }
        RowLayout {
            Layout.alignment: Qt.AlignRight
            spacing: 8
            Btn {
                variant: "ghost"
                text: "Skip"
                onClicked: group.done(group.info.items[0].path)
            }
            Btn {
                variant: "destructive"
                text: "Keep it, trash the other" + (group.others === 1 ? "" : "s")
                opacity: group.working ? 0.5 : 1
                onClicked: {
                    if (group.working)
                        return;
                    group.working = true;
                    const items = group.info.items;
                    const keep = items[group.keep].path;
                    Library.resolveDuplicates(keep, items.filter((_, i) => i !== group.keep).map(i => i.path), () => group.done(items[0].path));
                }
            }
        }
    }
}
