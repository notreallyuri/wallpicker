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
    // As the app's (min-w-32): otherwise as wide as its entries.
    property real minWidth: 128

    padding: 6
    margins: 8
    width: Math.max(minWidth, column.implicitWidth) + leftPadding + rightPadding

    /// Open with its corner at (x, y) of `item`.
    function openAt(item, x, y) {
        parent = item;
        menu.x = x;
        menu.y = y;
        open();
    }

    /// The pointer is over its entries (a submenu stays open while it is).
    readonly property bool hovered: hover.hovered

    contentItem: ColumnLayout {
        id: column
        /// What MenuItems close when chosen.
        property var owner: menu
        spacing: 1
        HoverHandler {
            id: hover
        }
    }
}
