// A Local tile (ui/src/views/local/tile.rs): a rounded card holding the
// picture (16:10, its own rounded corners) with the monitors showing it, the
// name and size, the tags; the star in the corner. Hover: lifts with the
// bouncy ease, a primary ring and glow, the picture zooms 3%. A click
// applies it.
import QtQuick
import QtQuick.Effects
import QtQuick.Layouts
import Quickshell.Widgets
import qs.components
import qs.services

Item {
    id: tile

    required property var img
    /// A right-click at (x, y) of the tile.
    signal menuRequested(real x, real y)

    /// For a right-click (and the test hooks).
    function rightClicked(x, y) {
        menuRequested(x, y);
    }

    readonly property bool hovered: mouse.containsMouse || star.hovered
    signal hoverChanged(bool on)
    onHoveredChanged: hoverChanged(hovered)
    readonly property bool favorite: Library.favorite(img.path)
    readonly property var tags: Library.tags(img.path)
    readonly property bool busy: !!Monitors.applying[img.path]
    /// The pointer is on its size (for the tip there).
    readonly property bool overSize: mouse.containsMouse && sizeText.contains(mouse.mapToItem(sizeText, mouse.mouseX, mouse.mouseY))
    /// The chosen monitors it's smaller than ("DP-1's 1920×1080").
    readonly property var tooSmallFor: img.width > 0 && img.height > 0 ? Monitors.list.filter(m => Monitors.targets.includes(m.name) && (img.width < m.width || img.height < m.height)).map(m => m.name + "'s " + m.width + "×" + m.height) : []
    /// The thumbnail: from the scan, or made on demand.
    property string thumb: img.thumb || ""

    implicitHeight: card.height + 8

    Component.onCompleted: {
        if (!thumb)
            Library.thumbnail(img.path, file => tile.thumb = file);
    }

    RectangularShadow {
        anchors.fill: card
        radius: card.radius
        offset.y: tile.hovered ? 10 : 4
        blur: tile.hovered ? 15 : 6
        spread: tile.hovered ? -3 : -1
        color: tile.hovered ? Theme.a(Theme.primary, 0.2) : Qt.rgba(0, 0, 0, 0.1)
        Behavior on offset.y {
            NumberAnimation {
                duration: Theme.duration
                easing.type: Easing.BezierSpline
                easing.bezierCurve: Theme.bounce
            }
        }
        Behavior on color {
            ColorAnimation {
                duration: Theme.duration
            }
        }
    }

    Rectangle {
        id: card
        width: parent.width
        height: content.implicitHeight + 12
        y: tile.hovered ? -4 : 0
        radius: 24
        color: Theme.a(Theme.card, 0.8)
        border.width: tile.hovered ? 2 : 1
        border.color: tile.hovered ? Theme.a(Theme.primary, 0.7) : Theme.a(Theme.foreground, 0.05)
        Behavior on y {
            NumberAnimation {
                duration: Theme.duration
                easing.type: Easing.BezierSpline
                easing.bezierCurve: Theme.bounce
            }
        }
        Behavior on border.color {
            ColorAnimation {
                duration: Theme.duration
            }
        }

        ColumnLayout {
            id: content
            x: 6
            y: 6
            width: parent.width - 12
            spacing: 0

            ClippingRectangle {
                id: picture
                Layout.fillWidth: true
                Layout.preferredHeight: width * 10 / 16
                radius: 17.6
                color: Theme.muted

                Image {
                    anchors.fill: parent
                    source: tile.thumb ? "file://" + tile.thumb : ""
                    fillMode: Image.PreserveAspectCrop
                    asynchronous: true
                    cache: false
                    // Decoded at the size it's shown (in the screen's pixels: 1x at
                    // scale 1; 2x took ~20 MB more), not the file's.
                    sourceSize.width: picture.width * Screen.devicePixelRatio
                    scale: tile.hovered ? 1.03 : 1
                    opacity: status === Image.Ready ? 1 : 0
                    Behavior on scale {
                        NumberAnimation {
                            duration: Theme.duration
                        }
                    }
                    Behavior on opacity {
                        NumberAnimation {
                            duration: Theme.ms(200)
                        }
                    }
                }
                Row {
                    x: 8
                    y: 8
                    spacing: 4
                    Repeater {
                        // Spanned: one badge for them all.
                        model: Monitors.spanned && Monitors.spanState.path === tile.img.path ? ["Spanned"] : Monitors.showing(tile.img.path)
                        Badge {
                            required property string modelData
                            text: modelData
                        }
                    }
                }
                Rectangle {
                    visible: tile.busy
                    anchors.fill: parent
                    color: Theme.a(Theme.background, 0.6)
                    Spinner {
                        anchors.centerIn: parent
                    }
                }
            }

            ColumnLayout {
                Layout.fillWidth: true
                Layout.margins: 12
                Layout.topMargin: 8
                Layout.bottomMargin: 8
                spacing: 4
                RowLayout {
                    Layout.fillWidth: true
                    spacing: 8
                    Text {
                        Layout.fillWidth: true
                        text: tile.img.name
                        elide: Text.ElideRight
                        color: Theme.mutedForeground
                        font.family: Theme.font
                        font.pixelSize: 12
                    }
                    // Its size; amber when it's smaller than a monitor it
                    // would go on (scaled up), which the tip says.
                    Text {
                        id: sizeText
                        visible: tile.img.width > 0
                        text: tile.img.width + "×" + tile.img.height
                        color: tile.tooSmallFor.length > 0 ? "#f59e0b" : Theme.a(Theme.mutedForeground, 0.7)
                        font.family: Theme.font
                        font.pixelSize: 10
                        // Only there: a tip in every tile would be dozens.
                        Loader {
                            anchors.fill: parent
                            active: tile.tooSmallFor.length > 0
                            sourceComponent: Tip {
                                // The tile's MouseArea covers it: it says.
                                over: tile.overSize
                                pressed: mouse.pressed
                                text: "Smaller than " + tile.tooSmallFor.join(", ") + " (it would be scaled up)"
                            }
                        }
                    }
                }
                Row {
                    visible: tile.tags.length > 0
                    Layout.fillWidth: true
                    spacing: 4
                    clip: true
                    Repeater {
                        model: tile.tags
                        Badge {
                            required property string modelData
                            soft: true
                            text: modelData
                        }
                    }
                }
            }
        }

        MouseArea {
            id: mouse
            anchors.fill: parent
            hoverEnabled: true
            cursorShape: Qt.PointingHandCursor
            acceptedButtons: Qt.LeftButton | Qt.RightButton
            onClicked: mouseEvent => {
                if (mouseEvent && mouseEvent.button === Qt.RightButton) {
                    const p = mapToItem(tile, mouseEvent.x, mouseEvent.y);
                    tile.rightClicked(p.x, p.y);
                } else {
                    Monitors.apply(tile.img.path);
                }
            }
        }

        TileStar {
            id: star
            x: parent.width - width - 14
            y: 14
            favorite: tile.favorite
            tileHovered: tile.hovered
            onToggled: Library.setFavorite(tile.img.path, !tile.favorite)
        }
    }
}
