// A menu (the tile's right-click menu, ⋯): a popover of MenuItem,
// MenuCheck, MenuSub and MenuSeparator, kept inside the window. `openAt`
// puts it at a point of an item (the cursor). Choosing an item closes it.
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import qs.services

Popover {
    id: menu

    default property alias items: column.data
    property real minWidth: 200

    padding: 6
    margins: 8

    /// Open with its corner at (x, y) of `item`.
    function openAt(item, x, y) {
        parent = item;
        menu.x = x;
        menu.y = y;
        open();
    }

    contentItem: ColumnLayout {
        id: column
        /// What MenuItems close when chosen.
        property var owner: menu
        spacing: 1
        width: Math.max(menu.minWidth, implicitWidth)
    }
}
