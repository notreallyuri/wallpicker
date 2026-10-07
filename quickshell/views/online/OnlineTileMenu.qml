// An online tile's right-click menu (tile_menu.rs + RemoteMenuItems):
// apply to one monitor or all (downloading it first), Details…, and
// Download unless it's in the download folder already.
import QtQuick
import qs.components
import qs.services

Menu {
    id: menu

    /// The wallpaper it was opened on.
    property var w: null

    function openFor(wallpaper, at, x, y) {
        w = wallpaper;
        openAt(at, x, y);
    }

    Repeater {
        model: Monitors.list
        MenuItem {
            required property var modelData
            icon: "monitor"
            text: "Apply to " + modelData.name
            onTriggered: Downloads.fetch(menu.w, [modelData.name])
        }
    }
    MenuItem {
        visible: Monitors.list.length > 1
        icon: "monitor"
        text: "Apply to all"
        onTriggered: Downloads.fetch(menu.w, Monitors.list.map(m => m.name))
    }
    MenuItem {
        visible: Monitors.list.length > 1
        icon: "monitor"
        text: "Span across monitors"
        onTriggered: Downloads.fetch(menu.w, "span")
    }
    MenuSeparator {}
    MenuItem {
        icon: "info"
        text: "Details…"
        onTriggered: Dialogs.showDetails(menu.w)
    }
    MenuSeparator {
        visible: download.visible
    }
    MenuItem {
        id: download
        visible: menu.w !== null && !Downloads.isSaved(menu.w)
        icon: "download"
        text: "Download"
        onTriggered: Downloads.fetch(menu.w, null)
    }
}
