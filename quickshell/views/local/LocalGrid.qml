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

    SmoothScroll {
        flickable: grid
    }

    // The space at the top changed: keep the content where it was relative
    // to it (so it visibly moves), like a scroll area's padding.
    // The same when the header (the breadcrumb) comes or goes: the view
    // keeps its position, which would leave it behind the card.
    readonly property real headerHeight: headerItem ? headerItem.height : 0
    property real lastTop: 0
    function keepPlace() {
        const top = headerSpace + headerHeight;
        contentY -= top - lastTop;
        lastTop = top;
    }
    onHeaderSpaceChanged: keepPlace()
    onHeaderHeightChanged: keepPlace()

    function toTop() {
        positionViewAtBeginning();
    }

    // In the Folders layout: where we are, above the grid.
    header: Item {
        width: grid.width - 16
        height: grid.local.folders ? crumbs.implicitHeight + 16 : 0
        visible: grid.local.folders
        Breadcrumb {
            id: crumbs
            local: grid.local
        }
    }

    // A folder card or a wallpaper tile.
    delegate: Item {
        id: cell
        required property var modelData
        width: grid.cellWidth
        height: grid.cellHeight
        Loader {
            y: 4
            width: parent.width - 16
            height: item ? item.implicitHeight : 0
            sourceComponent: cell.modelData.kind === "folder" ? folderCard : tile
        }
        Component {
            id: tile
            Tile {
                img: cell.modelData.img
            }
        }
        Component {
            id: folderCard
            FolderCard {
                folder: cell.modelData.folder
                onOpened: grid.local.openFolder(folder.place)
            }
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
