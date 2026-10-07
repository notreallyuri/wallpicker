// A tile's right-click menu (tile_menu.rs + Local's items): apply to one
// monitor, to all, or to the ticked ones (Apply to…: starts from the bar's
// choice, kept between right-clicks); the favorite, tags, show in folder,
// copy the path; move to the trash.
import QtQuick
import Quickshell
import qs.components
import qs.services

Menu {
    id: menu

    /// The wallpaper it was opened on.
    property var img: null
    /// Monitors ticked under Apply to…
    property var picked: []

    readonly property string path: img ? img.path : ""
    readonly property bool favorite: path !== "" && Library.favorite(path)
    // The ticked ones still connected, in the bar's order.
    readonly property var pickedNow: Monitors.list.map(m => m.name).filter(n => picked.includes(n))

    /// Open for `image` at (x, y) of `at`.
    function openFor(image, at, x, y) {
        img = image;
        if (pickedNow.length === 0)
            picked = Monitors.targets.slice();
        openAt(at, x, y);
    }

    function toggle(name) {
        picked = picked.includes(name) ? picked.filter(n => n !== name) : picked.concat([name]);
    }

    Repeater {
        model: Monitors.list
        MenuItem {
            required property var modelData
            icon: "monitor"
            text: "Apply to " + modelData.name
            onTriggered: Monitors.applyTo(menu.path, [modelData.name])
        }
    }
    MenuItem {
        visible: Monitors.list.length > 1
        icon: "monitor"
        text: "Apply to all"
        onTriggered: Monitors.applyTo(menu.path, Monitors.list.map(m => m.name))
    }
    MenuSub {
        visible: Monitors.list.length > 1
        text: "Apply to…"
        checkSpace: true
        Repeater {
            model: Monitors.list
            MenuCheck {
                required property var modelData
                text: modelData.name
                checked: menu.picked.includes(modelData.name)
                onTriggered: menu.toggle(modelData.name)
            }
        }
        MenuSeparator {}
        MenuItem {
            enabled: menu.pickedNow.length > 0
            text: "Apply to these"
            onTriggered: {
                Monitors.applyTo(menu.path, menu.pickedNow);
                menu.close();
            }
        }
    }
    MenuSeparator {}
    MenuItem {
        icon: "star"
        text: menu.favorite ? "Remove from favorites" : "Add to favorites"
        onTriggered: Library.setFavorite(menu.path, !menu.favorite)
    }
    MenuItem {
        icon: "tag"
        text: "Edit tags…"
        onTriggered: Dialogs.editTags(menu.path)
    }
    MenuItem {
        icon: "folder"
        text: "Show in folder"
        onTriggered: Library.showInFolder(menu.path)
    }
    MenuItem {
        icon: "copy"
        text: "Copy path"
        onTriggered: {
            Quickshell.clipboardText = menu.path;
            Toasts.info("Path copied");
        }
    }
    MenuSeparator {}
    MenuItem {
        icon: "trash"
        destructive: true
        text: "Move to trash…"
        hint: "Del"
        onTriggered: Dialogs.confirmTrash(menu.path)
    }
}
