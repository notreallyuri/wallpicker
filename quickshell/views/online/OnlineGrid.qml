// A grid of online results (the Online and Collections tabs): the tiles in
// the app's minmax(220px, 1fr) columns with 16 px gaps; placeholders while
// the first results load, the shown ones dimmed while others replace them;
// the pager and a status line under them. `headerSpace` is kept clear at
// the top for the floating card.
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import qs.components
import qs.services

GridView {
    id: grid

    property var results: []
    property int current: 0
    property int last: 0
    property bool loading: false
    property string loadingLabel: "Loading…"
    /// Under the pager ("1200 results · page 2 of 50").
    property string status: ""
    property real headerSpace: 0
    signal goTo(int page)
    /// A tile asked for its menu: the wallpaper, and where.
    signal tileMenu(var w, Item at, real x, real y)

    readonly property int columns: Math.max(1, Math.floor(width / (220 + 16)))
    readonly property bool placeholders: loading && results.length === 0

    topMargin: headerSpace
    bottomMargin: 20
    cellWidth: Math.floor(width / columns)
    // The card: the picture (16:10) and the caption; and the gap.
    cellHeight: (cellWidth - 16 - 12) * 10 / 16 + 12 + 30 + 16
    model: placeholders ? 12 : results
    clip: true
    boundsBehavior: Flickable.StopAtBounds
    ScrollBar.vertical: ScrollBar {}

    SmoothScroll {
        flickable: grid
    }

    // At the top when the card's height changes (suggestions come and go).
    property bool atTop: true
    onContentYChanged: atTop = contentY <= originY - topMargin + 1
    onHeaderSpaceChanged: if (atTop)
        Qt.callLater(() => contentY = originY - topMargin)

    // positionViewAtBeginning alone puts the first row at the very edge,
    // under the floating card (it ignores topMargin): on a page that barely
    // scrolls, that's near the bottom.
    function toTop() {
        positionViewAtBeginning();
        contentY = originY - topMargin;
    }

    // What's loading, over the placeholders.
    header: Item {
        width: grid.width - 16
        height: grid.placeholders ? 52 : 0
        visible: grid.placeholders
        Plate {
            RowLayout {
                spacing: 8
                Spinner {
                    size: 16
                }
                Text {
                    text: grid.loadingLabel
                    color: Theme.foreground
                    font.family: Theme.font
                    font.pixelSize: 14
                    font.weight: Font.Bold
                }
            }
        }
    }

    delegate: Item {
        id: cell
        required property var modelData
        required property int index
        width: grid.cellWidth
        height: grid.cellHeight
        // Another page loading: what's here dims until it lands.
        opacity: grid.loading && !grid.placeholders ? 0.4 : 1
        enabled: !(grid.loading && !grid.placeholders)
        Behavior on opacity {
            NumberAnimation {
                duration: Theme.duration
            }
        }
        Loader {
            y: 4
            width: parent.width - 16
            height: parent.height - 16
            sourceComponent: grid.placeholders ? placeholder : tile
        }
        Component {
            id: tile
            OnlineTile {
                id: tileItem
                w: cell.modelData
                onMenuRequested: (x, y) => grid.tileMenu(w, tileItem, x, y)
            }
        }
        Component {
            id: placeholder
            OnlineSkeleton {
                delay: cell.index * 60
            }
        }
    }

    footer: Item {
        width: grid.width - 16
        height: grid.results.length > 0 ? (grid.last > 1 ? 124 : 56) : 0
        visible: grid.results.length > 0
        // The pager and the status line on one plate.
        Plate {
            anchors.centerIn: parent
            ColumnLayout {
                spacing: 8
                Pager {
                    visible: grid.last > 1
                    Layout.alignment: Qt.AlignHCenter
                    current: grid.current
                    last: grid.last
                    onGoTo: page => {
                        if (!grid.loading)
                            grid.goTo(page);
                    }
                }
                RowLayout {
                    Layout.alignment: Qt.AlignHCenter
                    spacing: 8
                    Spinner {
                        visible: grid.loading
                        size: 14
                    }
                    Text {
                        text: grid.status
                        color: Theme.mutedForeground
                        font.family: Theme.font
                        font.pixelSize: 12
                    }
                }
            }
        }
    }
}
