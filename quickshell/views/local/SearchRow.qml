// The search card's first row: the search box, Favorites, the sort, Color,
// the layout, Random, the count, and ⋯.
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

    spacing: 8

    // Takes what the row has left, within 160-320 px (as in the app); the
    // spacer before ⋯ gets the rest.
    TextField {
        id: search
        Layout.fillWidth: true
        Layout.minimumWidth: 160
        Layout.maximumWidth: 320
        icon: "search"
        placeholder: "Filter by name or tag"
        // Back from another tab: the filter is still on.
        text: row.local.query
        onTextChanged: row.local.query = text
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
    }
    // Everything at once, or folder by folder (remembered).
    Btn {
        minWidth: 104
        icon: row.local.folders ? "folder" : "grid"
        text: row.local.folders ? "Folders" : "Flat"
        onClicked: row.local.layout = row.local.folders ? "flat" : "folders"
        tip: "Show everything at once, or browse folder by folder"
    }
    Btn {
        icon: "shuffle"
        text: "Random"
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
