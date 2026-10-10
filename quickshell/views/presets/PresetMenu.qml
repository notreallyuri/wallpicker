import QtQuick
import qs.components
import qs.services

Menu {
    id: menu
    required property var preset
    enabled: !Presets.busy
    component Action: MenuItem {
        activeFocusOnTab: true
        Accessible.role: Accessible.MenuItem
        Accessible.name: text
        Keys.onReturnPressed: { triggered(); closeMenu(); }
        Keys.onSpacePressed: { triggered(); closeMenu(); }
    }
    Action {
        text: "Update from current setup…"
        description: "Replace the saved wallpapers and adjustments"
        icon: "refresh"
        onTriggered: Presets.editing = {action: "update", preset: menu.preset}
    }
    Action {
        text: "Rename…"
        onTriggered: Presets.editing = {action: "rename", preset: menu.preset}
    }
    MenuSeparator {}
    Action {
        text: "Delete preset…"
        icon: "trash"
        destructive: true
        onTriggered: Presets.editing = {action: "delete", preset: menu.preset}
    }
}
