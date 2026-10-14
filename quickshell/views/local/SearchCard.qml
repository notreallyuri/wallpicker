// The card floating over the top of the grid (rounded-2xl, frosted glass
// over the grid, a soft shadow): the search row, and with the "rows"
// filters layout (Settings → Appearance) the tags and the artists under
// it while Filters has them open.
import QtQuick
import QtQuick.Effects
import QtQuick.Layouts
import qs.components
import qs.services

Item {
    id: card

    required property LocalModel local
    /// What's under the card, for the glass: the grid, the wallpaper
    /// behind it, and what moves the card relative to them.
    property Item behind
    property Item backdrop
    property var follow

    implicitHeight: rows.implicitHeight + 18

    function focusSearch() {
        searchRow.focusSearch();
    }

    RectangularShadow {
        anchors.fill: parent
        radius: 22
        offset.y: 8
        blur: 20
        color: Qt.rgba(0, 0, 0, 0.15)
    }
    Glass {
        anchors.fill: parent
        radius: 22
        source: card.behind
        backdrop: card.backdrop
        follow: card.follow
    }
    Rectangle {
        anchors.fill: parent
        radius: 22
        color: "transparent"
        border.width: 1
        border.color: Theme.a(Theme.border, 0.6)
    }

    ColumnLayout {
        id: rows
        x: 16
        y: 10
        width: parent.width - 32
        spacing: 8
        SearchRow {
            id: searchRow
            Layout.fillWidth: true
            local: card.local
        }
        // The rows layout, opened by Filters.
        ColumnLayout {
            Layout.fillWidth: true
            visible: Prefs.filtersLayout === "rows" && Prefs.filtersOpen
            spacing: 8
            // The two rows' chips start at one place.
            ChipRow {
                id: tagRow
                Layout.fillWidth: true
                labelWidth: artistRow.visible ? Math.max(labelImplicitWidth, artistRow.labelImplicitWidth) : -1
                label: "Tags"
                order: Prefs.tagOrder
                counts: Library.tagCounts
                chosen: card.local.tagFilter
                matchAll: card.local.tagsMatchAll
                expanded: Prefs.tagsExpanded
                onExpandedChanged: Prefs.tagsExpanded = expanded
                onToggled: name => card.local.toggleTag(name)
                onCleared: card.local.tagFilter = []
                onMatchToggled: card.local.tagsMatchAll = !card.local.tagsMatchAll
            }
            // Any of the artists picked (a wallpaper rarely has two).
            ChipRow {
                id: artistRow
                Layout.fillWidth: true
                labelWidth: tagRow.visible ? Math.max(labelImplicitWidth, tagRow.labelImplicitWidth) : -1
                label: "Artists"
                order: Prefs.tagOrder
                counts: Library.artistCounts
                chosen: card.local.artistFilter
                expanded: Prefs.artistsExpanded
                onExpandedChanged: Prefs.artistsExpanded = expanded
                onToggled: name => card.local.toggleArtist(name)
                onCleared: card.local.artistFilter = []
            }
        }
    }
}
