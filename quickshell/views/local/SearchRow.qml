// The search card's first row: the search box, Filters, Favorites, the
// sort, Color, the layout, Random, the count, and ⋯. Narrow (the control
// panel open, a Compact picker), Color, the layout and Random are icons.
import QtQuick
import QtQuick.Layouts
import qs.components
import qs.services

RowLayout {
    id: row

    required property LocalModel local

    function focusSearch() {
        search.forceFocus();
    }

    /// Too narrow for every label.
    readonly property bool compact: width < 1150

    spacing: 8

    // Takes what the row has left, within 160-320 px (as in the app); the
    // spacer before ⋯ gets the rest.
    TextField {
        id: search
        Layout.fillWidth: true
        Layout.minimumWidth: 160
        Layout.maximumWidth: 320
        icon: "search"
        placeholder: "Filter by name, tag or artist"
        // Back from another tab: the filter is still on.
        text: row.local.query
        onTextChanged: row.local.query = text
    }
    // The tags and artists: a popover, or the rows under this one
    // (Settings → Appearance). Says how many are picked.
    Btn {
        id: filtersButton
        readonly property int picked: row.local.tagFilter.length + row.local.artistFilter.length
        readonly property bool rows: Prefs.filtersLayout === "rows"
        visible: Library.tagCounts.length + Library.artistCounts.length > 0
        icon: "funnel"
        text: picked > 0 ? "Filters · " + picked : "Filters"
        active: picked > 0
        tip: rows ? (Prefs.filtersOpen ? "Hide the tags and artists" : "Show the tags and artists") : "Filter by tag or artist"
        onClicked: rows ? Prefs.filtersOpen = !Prefs.filtersOpen : filtersPopover.openUnder(filtersButton)
        FiltersPopover {
            id: filtersPopover
            local: row.local
        }
    }
    Btn {
        icon: "star"
        text: "Favorites"
        active: row.local.favoritesOnly
        onClicked: row.local.favoritesOnly = !row.local.favoritesOnly
    }
    Select {
        minWidth: 144
        options: row.local.sorts
        value: row.local.sort
        onPicked: value => row.local.sort = value
    }
    ColorPicker {
        local: row.local
        compact: row.compact
    }
    // Everything at once, or folder by folder (remembered).
    Btn {
        minWidth: row.compact ? 0 : 104
        icon: row.local.folders ? "folder" : "grid"
        text: row.compact ? "" : row.local.folders ? "Folders" : "Flat"
        onClicked: row.local.layout = row.local.folders ? "flat" : "folders"
        tip: "Show everything at once, or browse folder by folder"
    }
    Btn {
        icon: "shuffle"
        text: row.compact ? "" : "Random"
        tip: "Apply a random one of those shown (Ctrl+R)"
        onClicked: row.local.applyRandom()
    }
    Text {
        Layout.minimumWidth: 80
        text: Library.loading ? "Scanning…" : row.local.shown.length + (row.local.shown.length === 1 ? " image" : " images")
        color: Theme.mutedForeground
        font.family: Theme.font
        font.pixelSize: 14
    }
    Item {
        Layout.fillWidth: true
    }
    MoreMenu {}
}
