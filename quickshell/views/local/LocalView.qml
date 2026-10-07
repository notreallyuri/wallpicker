// The Local tab: the grid, with the search card floating over its top.
import QtQuick
import qs.components
import qs.services

Item {
    id: view

    /// The panel's wallpaper, for the search card's glass.
    property Item backdrop

    LocalModel {
        id: localModel
    }

    // The grid, its bottom edge fading out (things scroll in gradually).
    FadeBottom {
        anchors.fill: parent
        anchors.leftMargin: 20
        anchors.rightMargin: 4
        flickable: grid
        LocalGrid {
            id: grid
            anchors.fill: parent
            local: localModel
            headerSpace: card.height + 12 + 28
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
        local: localModel
        behind: grid
        backdrop: view.backdrop
        // The drawer opening moves the view, and the card with it.
        follow: view.y
    }

    // Nothing in the folders at all.
    Column {
        visible: !Library.loading && Library.images.length === 0
        anchors.centerIn: parent
        width: Math.min(520, parent.width - 48)
        spacing: 8
        Text {
            width: parent.width
            horizontalAlignment: Text.AlignHCenter
            text: "No images found"
            color: Theme.foreground
            font.family: Theme.font
            font.pixelSize: 18
            font.weight: Font.ExtraBold
        }
        Text {
            width: parent.width
            horizontalAlignment: Text.AlignHCenter
            wrapMode: Text.Wrap
            text: "Looking in: " + Library.roots.map(r => r.path).join(", ") + ". Add folders in Settings, or download something from the Wallhaven tab."
            color: Theme.mutedForeground
            font.family: Theme.font
            font.pixelSize: 14
        }
    }
    // Some, but none pass the filters.
    Text {
        visible: !Library.loading && Library.images.length > 0 && localModel.pageItems.length === 0
        anchors.centerIn: parent
        text: "No wallpapers match."
        color: Theme.mutedForeground
        font.family: Theme.font
        font.pixelSize: 14
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
        onActivated: localModel.applyRandom()
    }
    Shortcut {
        enabled: Dialogs.calm && localModel.folders && localModel.canGoUp
        sequence: "Alt+Up"
        onActivated: localModel.up()
    }
    Shortcut {
        enabled: Dialogs.calm && localModel.currentPage > 1
        sequence: "Alt+Left"
        onActivated: {
            localModel.page = localModel.currentPage - 1;
            grid.toTop();
        }
    }
    Shortcut {
        enabled: Dialogs.calm && localModel.currentPage < localModel.lastPage
        sequence: "Alt+Right"
        onActivated: {
            localModel.page = localModel.currentPage + 1;
            grid.toTop();
        }
    }
    // Delete: the tile under the pointer, to the trash (asking first).
    Shortcut {
        enabled: Dialogs.calm && localModel.hovered !== ""
        sequence: "Delete"
        onActivated: Dialogs.confirmTrash(localModel.hovered)
    }
}
