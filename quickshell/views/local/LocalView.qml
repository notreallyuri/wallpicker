// The Local tab: the grid, with the search card floating over its top.
// What it shows (`local`) outlives it: the picker keeps it while another
// tab is open.
import QtQuick
import qs.components
import qs.services

Item {
    id: view

    required property LocalModel local
    /// The panel's wallpaper, for the search card's glass.
    property Item backdrop

    // The grid, its bottom edge fading out (things scroll in gradually).
    FadeBottom {
        anchors.fill: parent
        anchors.leftMargin: 20
        anchors.rightMargin: 4
        flickable: grid
        LocalGrid {
            id: grid
            anchors.fill: parent
            local: view.local
            headerSpace: card.height + 12 + 16
            onTileMenu: (img, at, x, y) => tileMenu.openFor(img, at, x, y)
        }
    }

    // One menu for every tile, as in the app.
    TileMenu {
        id: tileMenu
    }

    SearchCard {
        id: card
        x: 12
        y: 12
        width: parent.width - 24
        local: view.local
        behind: grid
        backdrop: view.backdrop
        // What moves the view, and the card with it.
        follow: view.y
    }

    // Nothing in the folders at all.
    EmptyState {
        visible: !Library.loading && Library.images.length === 0
        anchors.centerIn: parent
        width: Math.min(440, view.width - 48)
        icon: "folder"
        title: "No images found"
        text: "Looking in " + Library.roots.map(r => r.path).join(", ") + ". Add folders in Settings, or download something from the Online tab."
        action: "Open Settings"
        onTriggered: Dialogs.openSettings("general")
    }
    // Some, but none pass the search and filters.
    EmptyState {
        visible: !Library.loading && Library.images.length > 0 && view.local.pageItems.length === 0
        anchors.centerIn: parent
        width: Math.min(440, view.width - 48)
        icon: "funnel"
        title: "No wallpapers match"
        text: "Nothing passes the search and filters you've set."
        action: "Clear filters"
        onTriggered: view.local.clearFilters()
    }

    // Keyboard (as shortcuts.rs), while no menu or dialog is open.
    Shortcut {
        enabled: Dialogs.calm
        sequence: "Ctrl+F"
        onActivated: card.focusSearch()
    }
    Shortcut {
        enabled: Dialogs.calm
        sequence: "Ctrl+R"
        onActivated: view.local.applyRandom()
    }
    Shortcut {
        enabled: Dialogs.calm && view.local.folders && view.local.canGoUp
        sequence: "Alt+Up"
        onActivated: view.local.up()
    }
    Shortcut {
        enabled: Dialogs.calm && view.local.currentPage > 1
        sequence: "Alt+Left"
        onActivated: {
            view.local.page = view.local.currentPage - 1;
            grid.toTop();
        }
    }
    Shortcut {
        enabled: Dialogs.calm && view.local.currentPage < view.local.lastPage
        sequence: "Alt+Right"
        onActivated: {
            view.local.page = view.local.currentPage + 1;
            grid.toTop();
        }
    }
    // Delete: the tile under the pointer, to the trash (asking first).
    Shortcut {
        enabled: Dialogs.calm && view.local.hovered !== ""
        sequence: "Delete"
        onActivated: Dialogs.confirmTrash(view.local.hovered)
    }
}
