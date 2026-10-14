// Local's Filters (the Filters button's popover; Settings → Appearance can
// put them in rows under the search bar instead): Match all tags and
// Clear, a field narrowing both lists, then the tags and the artists as
// chips. Picking one filters at once; the popover stays open.
import QtQuick
import QtQuick.Layouts
import qs.components
import qs.services

Popover {
    id: pop

    required property LocalModel local

    /// Open under `item` (the Filters button), its field empty.
    function openUnder(item) {
        parent = item;
        x = 0;
        y = item.height + 8;
        find.text = "";
        open();
    }

    padding: 16
    margins: 8
    width: 560

    contentItem: ColumnLayout {
        spacing: 14

        RowLayout {
            Layout.fillWidth: true
            spacing: 10
            Text {
                text: "Filters"
                color: Theme.foreground
                font.family: Theme.font
                font.pixelSize: 15
                font.weight: Font.ExtraBold
            }
            Item {
                Layout.fillWidth: true
            }
            Toggle {
                checked: pop.local.tagsMatchAll
                onToggled: pop.local.tagsMatchAll = !pop.local.tagsMatchAll
            }
            Text {
                text: "Match all tags"
                color: Theme.mutedForeground
                font.family: Theme.font
                font.pixelSize: 13
            }
            Btn {
                enabled: pop.local.tagFilter.length + pop.local.artistFilter.length > 0
                opacity: enabled ? 1 : 0.4
                variant: "ghost"
                small: true
                text: "Clear"
                onClicked: {
                    pop.local.tagFilter = [];
                    pop.local.artistFilter = [];
                }
            }
        }

        TextField {
            id: find
            Layout.fillWidth: true
            icon: "search"
            placeholder: "Find a tag or artist"
        }

        FilterGroup {
            Layout.fillWidth: true
            title: "Tags"
            counts: Library.tagCounts
            chosen: pop.local.tagFilter
            find: find.text
            expanded: Prefs.tagsExpanded
            onExpandedChanged: Prefs.tagsExpanded = expanded
            onToggled: name => pop.local.toggleTag(name)
        }
        FilterGroup {
            Layout.fillWidth: true
            title: "Artists"
            counts: Library.artistCounts
            chosen: pop.local.artistFilter
            find: find.text
            expanded: Prefs.artistsExpanded
            onExpandedChanged: Prefs.artistsExpanded = expanded
            onToggled: name => pop.local.toggleArtist(name)
        }

        Text {
            Layout.fillWidth: true
            wrapMode: Text.Wrap
            text: pop.local.tagsMatchAll ? "Wallpapers with every tag picked, by any of the artists picked." : "Wallpapers with any tag picked, by any of the artists picked."
            color: Theme.mutedForeground
            font.family: Theme.font
            font.pixelSize: 12
        }
    }
}
