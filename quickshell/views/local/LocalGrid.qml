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
    /// A tile asked for its menu: the wallpaper, and where (an item and a
    /// point in it).
    signal tileMenu(var img, Item at, real x, real y)

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

    // The space at the top changed (tags expanded) or the header (the
    // breadcrumb) came or went. At the top, the view stays at the top once
    // it's laid out again (shifting by the difference added up with the
    // view's own repositioning: the content landed too low or too high).
    // Scrolled down, the content moves with the card, like a scroll area's
    // padding.
    readonly property real headerHeight: headerItem ? headerItem.height : 0
    readonly property real topY: originY - topMargin
    property bool atTop: true
    property real lastSpace: 0
    onContentYChanged: atTop = contentY <= topY + 1
    function settle() {
        if (atTop)
            Qt.callLater(() => contentY = topY);
    }
    onHeaderSpaceChanged: {
        if (!atTop)
            contentY -= headerSpace - lastSpace;
        lastSpace = headerSpace;
        settle();
    }
    onHeaderHeightChanged: settle()
    onTopYChanged: settle()

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
            sourceComponent: cell.modelData.kind === "folder" ? folderCard : cell.modelData.kind === "skeleton" ? skeleton : tile
        }
        Component {
            id: tile
            Tile {
                id: tileItem
                img: cell.modelData.img
                onMenuRequested: (x, y) => grid.tileMenu(img, tileItem, x, y)
                onHoverChanged: on => {
                    if (on)
                        grid.local.hovered = img.path;
                    else if (grid.local.hovered === img.path)
                        grid.local.hovered = "";
                }
            }
        }
        Component {
            id: skeleton
            SkeletonTile {}
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
        height: grid.local.lastPage > 1 ? 92 : 0
        Plate {
            visible: grid.local.lastPage > 1
            anchors.centerIn: parent
            Pager {
                current: grid.local.currentPage
                last: grid.local.lastPage
                onGoTo: page => {
                    grid.local.page = page;
                    grid.toTop();
                }
            }
        }
    }
}
