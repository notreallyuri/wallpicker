// The Collections tab (ui/src/views/online/collections.rs): your Wallhaven
// collections, shown like search results (apply, download, details). The
// grid, with the card of collections floating over its top; the state is
// in services/Collections.
import QtQuick
import qs.components
import qs.services
import qs.views.online

Item {
    id: view

    /// The panel's wallpaper, for the card's glass.
    property Item backdrop

    // The list the first time it's shown.
    Component.onCompleted: Collections.activate()

    FadeBottom {
        anchors.fill: parent
        anchors.leftMargin: 20
        anchors.rightMargin: 4
        flickable: grid
        OnlineGrid {
            id: grid
            anchors.fill: parent
            headerSpace: card.height + 12 + 28 + (error.visible ? error.height + 12 : 0)
            results: Collections.results
            current: Collections.page ? Collections.page.page : 0
            last: Collections.page ? Collections.page.last_page : 0
            loading: Collections.loading
            loadingLabel: "Loading the collection…"
            status: Collections.page ? Collections.page.total + " wallpapers · page " + Collections.page.page + " of " + Collections.page.last_page : ""
            onGoTo: page => Collections.goTo(page)
            onTileMenu: (w, at, x, y) => tileMenu.openFor(w, at, x, y)
        }
    }

    Connections {
        target: Collections
        function onLanded() {
            grid.toTop();
        }
    }

    OnlineTileMenu {
        id: tileMenu
    }

    CollectionsCard {
        id: card
        x: 12
        y: 12
        width: parent.width - 24
        behind: grid
        backdrop: view.backdrop
        follow: view.y
    }

    // The chosen one couldn't be opened (no username...).
    Plate {
        id: error
        visible: Collections.error !== ""
        x: 32
        y: card.y + card.height + 20
        Text {
            width: Math.min(implicitWidth, view.width - 92)
            wrapMode: Text.Wrap
            text: Collections.error
            color: "#fd4663"
            font.family: Theme.font
            font.pixelSize: 14
        }
    }

    Shortcut {
        enabled: Dialogs.calm && !Collections.loading && grid.current > 1
        sequence: "Alt+Left"
        onActivated: Collections.goTo(grid.current - 1)
    }
    Shortcut {
        enabled: Dialogs.calm && !Collections.loading && grid.current < grid.last
        sequence: "Alt+Right"
        onActivated: Collections.goTo(grid.current + 1)
    }
}
