// The Online tab (ui/src/views/online/mod.rs): search a provider (Wallhaven,
// Konachan, yande.re), download and apply. The results' grid, with the
// search card floating over its top; the state is in services/Online.
import QtQuick
import qs.components
import qs.services

Item {
    id: view

    /// The panel's wallpaper, for the search card's glass.
    property Item backdrop

    // Searches the first time it's shown (not at start), and after the
    // provider changed elsewhere.
    Component.onCompleted: Online.activate()

    FadeBottom {
        anchors.fill: parent
        anchors.leftMargin: 20
        anchors.rightMargin: 4
        flickable: grid
        OnlineGrid {
            id: grid
            anchors.fill: parent
            headerSpace: card.height + 12 + 28
            results: Online.results
            current: Online.page ? Online.page.page : 0
            last: Online.page ? Online.page.last_page : 0
            loading: Online.loading
            loadingLabel: "Loading " + Online.label + "…"
            status: {
                const p = Online.page;
                if (!p)
                    return "";
                // Konachan's count can be missing: then only the page.
                return p.total > 0 ? p.total + " results · page " + p.page + " of " + p.last_page : "page " + p.page;
            }
            onGoTo: page => Online.goTo(page)
            onTileMenu: (w, at, x, y) => tileMenu.openFor(w, at, x, y)
        }
    }

    Connections {
        target: Online
        function onLanded() {
            grid.toTop();
        }
    }

    OnlineTileMenu {
        id: tileMenu
    }

    OnlineCard {
        id: card
        x: 12
        y: 12
        width: parent.width - 24
        behind: grid
        backdrop: view.backdrop
        follow: view.y
    }

    Text {
        visible: Online.page !== null && !Online.loading && Online.results.length === 0
        anchors.centerIn: parent
        horizontalAlignment: Text.AlignHCenter
        text: "Nothing found\nNo wallpapers match these filters."
        color: Theme.mutedForeground
        font.family: Theme.font
        font.pixelSize: 14
    }

    Shortcut {
        enabled: Dialogs.calm
        sequence: "Ctrl+F"
        onActivated: card.focusSearch()
    }
    Shortcut {
        enabled: Dialogs.calm && !Online.loading && grid.current > 1
        sequence: "Alt+Left"
        onActivated: Online.goTo(grid.current - 1)
    }
    Shortcut {
        enabled: Dialogs.calm && !Online.loading && grid.current < grid.last
        sequence: "Alt+Right"
        onActivated: Online.goTo(grid.current + 1)
    }
}
