// The Local grid: the page's tiles (the app's minmax(220px, 1fr) columns and
// 16 px gaps) and the pager under them. `headerSpace` is kept clear at the
// top for the floating card; when it grows (tags expanded) the content moves
// down with it, as the app's padding does.
import QtQuick
import QtQuick.Controls
import qs.components
import qs.services

GridView {
    id: grid

    required property LocalModel local
    property real headerSpace: 0

    readonly property int columns: Math.max(1, Math.floor(width / (220 + 16)))

    topMargin: headerSpace
    bottomMargin: 20
    cellWidth: Math.floor(width / columns)
    // The tallest card (one with a row of tags) and the gap.
    cellHeight: (cellWidth - 16 - 12) * 10 / 16 + 12 + 54 + 16
    model: grid.local.pageItems
    clip: true
    boundsBehavior: Flickable.StopAtBounds
    ScrollBar.vertical: ScrollBar {}

    // The space at the top changed: keep the content where it was relative
    // to it (so it visibly moves), like a scroll area's padding.
    property real lastSpace: 0
    onHeaderSpaceChanged: {
        const delta = headerSpace - lastSpace;
        lastSpace = headerSpace;
        contentY -= delta;
    }

    function toTop() {
        positionViewAtBeginning();
    }

    delegate: Item {
        id: cell
        required property var modelData
        width: grid.cellWidth
        height: grid.cellHeight
        Tile {
            y: 4
            width: parent.width - 16
            img: cell.modelData
        }
    }

    footer: Item {
        width: grid.width - 16
        height: grid.local.lastPage > 1 ? 80 : 0
        Pager {
            visible: grid.local.lastPage > 1
            anchors.centerIn: parent
            current: grid.local.currentPage
            last: grid.local.lastPage
            onGoTo: page => {
                grid.local.page = page;
                grid.toTop();
            }
        }
    }
}
