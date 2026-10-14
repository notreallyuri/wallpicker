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
            headerSpace: card.height + 12 + 28
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

    // The collections couldn't be listed (no API key...), there are none,
    // or the chosen one couldn't be opened.
    EmptyState {
        readonly property bool listFailed: Collections.list !== null && Collections.listError !== ""
        readonly property bool none: Collections.list !== null && !listFailed && Collections.list.length === 0
        visible: listFailed || none || Collections.error !== ""
        anchors.centerIn: parent
        width: Math.min(440, view.width - 48)
        error: !none
        icon: none ? "star" : "x"
        title: listFailed ? "Couldn't list your collections" : none ? "No collections yet" : "Couldn't open this collection"
        text: listFailed ? Collections.listError : none ? "The collections you make on Wallhaven show here." : Collections.error
        action: none ? "" : "Wallhaven settings…"
        onTriggered: Dialogs.openSettings("wallhaven")
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
