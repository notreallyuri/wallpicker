// An online result (ui/src/views/online/tile.rs) on a card like Local's
// (its text stays readable over a bright wallpaper): the picture in its own
// rounded box, its rating as the border (green sfw, yellow sketchy, red
// nsfw: fixed colors, the same in any theme), the size and likes under it.
// Hover: the card lifts, the ring thickens, and Details, Apply and Download
// (an icon) show; a click on the picture opens the details. Marked when it's in the
// download folder; a download in progress shows how far it is.
import QtQuick
import QtQuick.Effects
import QtQuick.Layouts
import Quickshell.Widgets
import qs.components
import qs.services

Item {
    id: tile

    required property var w
    /// A right-click at (x, y) of the tile.
    signal menuRequested(real x, real y)

    function rightClicked(x, y) {
        menuRequested(x, y);
    }

    readonly property bool hovered: hover.hovered
    readonly property bool saved: Downloads.isSaved(w)
    readonly property bool working: Downloads.isWorking(w)
    readonly property real progress: Downloads.progress(w)
    readonly property color ring: w.purity === "nsfw" ? "#ef4444" : w.purity === "sketchy" ? "#facc15" : "#22c55e"

    HoverHandler {
        id: hover
    }

    implicitHeight: card.height + 8

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
    }

    Rectangle {
        id: card
        width: parent.width
        height: picture.height + 12 + 30
        y: tile.hovered ? -4 : 0
        radius: 24
        color: Theme.a(Theme.card, 0.8)
        border.width: 1
        border.color: Theme.a(Theme.foreground, 0.05)
        Behavior on y {
            NumberAnimation {
                duration: Theme.duration
                easing.type: Easing.BezierSpline
                easing.bezierCurve: Theme.bounce
            }
        }

        ClippingRectangle {
            id: picture
            x: 6
            y: 6
            width: parent.width - 12
            height: width * 10 / 16
            radius: 17.6
            color: Theme.muted

            Image {
                anchors.fill: parent
                source: tile.w.thumb
                fillMode: Image.PreserveAspectCrop
                asynchronous: true
                sourceSize.width: picture.width * Screen.devicePixelRatio
                opacity: status === Image.Ready ? 1 : 0
                Behavior on opacity {
                    NumberAnimation {
                        duration: Theme.ms(200)
                    }
                }
            }

            // Right-click for the menu; a click opens the details.
            MouseArea {
                anchors.fill: parent
                acceptedButtons: Qt.LeftButton | Qt.RightButton
                cursorShape: Qt.PointingHandCursor
                onClicked: mouseEvent => {
                    if (mouseEvent && mouseEvent.button === Qt.RightButton) {
                        const p = mapToItem(tile, mouseEvent.x, mouseEvent.y);
                        tile.rightClicked(p.x, p.y);
                    } else {
                        Dialogs.showDetails(tile.w);
                    }
                }
            }

            // On hover: a shade from the bottom, and what to do with it.
            Item {
                anchors.fill: parent
                opacity: tile.hovered && !tile.working ? 1 : 0
                visible: opacity > 0
                Behavior on opacity {
                    NumberAnimation {
                        duration: Theme.ms(150)
                    }
                }
                Rectangle {
                    anchors.fill: parent
                    gradient: Gradient {
                        GradientStop {
                            position: 0.4
                            color: "transparent"
                        }
                        GradientStop {
                            position: 1
                            color: Qt.rgba(0, 0, 0, 0.6)
                        }
                    }
                }
                RowLayout {
                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.bottom: parent.bottom
                    anchors.margins: 8
                    spacing: 8
                    Btn {
                        variant: "ghost"
                        small: true
                        text: "Details"
                        onClicked: Dialogs.showDetails(tile.w)
                    }
                    Item {
                        Layout.fillWidth: true
                    }
                    Btn {
                        variant: "primary"
                        small: true
                        text: "Apply"
                        onClicked: Downloads.fetch(tile.w, Monitors.targets)
                    }
                    // An icon: three labels don't fit a 220 px tile.
                    Btn {
                        visible: !tile.saved
                        small: true
                        icon: "download"
                        tip: "Download"
                        onClicked: Downloads.fetch(tile.w, null)
                    }
                }
            }

            RowLayout {
                x: 8
                y: 8
                spacing: 4
                OnlineBadge {
                    visible: tile.saved
                    icon: "check"
                    text: "Downloaded"
                    tip: "Already in your download folder"
                }
                OnlineBadge {
                    visible: tile.w.purity !== "sfw"
                    destructive: true
                    text: tile.w.purity
                }
            }

            // Downloading: how far it is.
            Rectangle {
                visible: tile.working
                anchors.fill: parent
                color: Theme.a(Theme.background, 0.6)
                ColumnLayout {
                    anchors.centerIn: parent
                    spacing: 6
                    Spinner {
                        Layout.alignment: Qt.AlignHCenter
                    }
                    Text {
                        visible: tile.progress >= 0
                        Layout.alignment: Qt.AlignHCenter
                        text: Math.round(tile.progress * 100) + "%"
                        color: Theme.foreground
                        font.family: Theme.font
                        font.pixelSize: 12
                        font.weight: Font.Bold
                    }
                }
                Rectangle {
                    visible: tile.progress >= 0
                    anchors.bottom: parent.bottom
                    height: 4
                    width: parent.width * Math.max(0, tile.progress)
                    color: Theme.primary
                    Behavior on width {
                        NumberAnimation {
                            duration: Theme.ms(100)
                        }
                    }
                }
            }
        }

        // The rating, as the picture's border.
        Rectangle {
            anchors.fill: picture
            radius: picture.radius
            color: "transparent"
            border.width: tile.hovered ? 3 : 2
            border.color: tile.ring
        }

        RowLayout {
            anchors.top: picture.bottom
            anchors.topMargin: 7
            x: 12
            width: parent.width - 24
            Text {
                Layout.fillWidth: true
                // Unsplash: its photographer, credited as its guidelines ask.
                readonly property var by: tile.w.provider === "unsplash" && tile.w.details ? tile.w.details.uploader : null
                text: tile.w.width + "×" + tile.w.height + (by ? " · " + by.name : "")
                elide: Text.ElideRight
                color: Theme.foreground
                font.family: Theme.font
                font.pixelSize: 12
            }
            Text {
                // "♥ 12" for Wallhaven favorites and Unsplash likes, "Score 19"
                // on the Moebooru sites.
                text: tile.w.provider === "konachan" || tile.w.provider === "yandere" ? "Score " + tile.w.likes : "♥ " + tile.w.likes
                color: Theme.mutedForeground
                font.family: Theme.font
                font.pixelSize: 12
            }
        }
    }
}
