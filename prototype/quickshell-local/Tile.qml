// A Local tile, as ui/src/views/local/tile.rs draws it: a rounded card
// (bg-card/80, p-1.5, ring-1, shadow-md) holding the picture (16:10, its own
// rounded corners) with the monitors showing it, the name and size, and the
// tags; the star in the corner. Hover: lifts 4 px with the app's bouncy ease,
// a primary ring and glow, the picture zooms 3%.
import QtQuick
import QtQuick.Effects
import QtQuick.Layouts
import Quickshell.Widgets

Item {
    id: tile

    required property var img
    /// Monitor names showing it.
    property var shownOn: []
    property bool busy: false
    signal apply
    signal toggleFavorite
    signal thumbReady

    readonly property bool hovered: mouse.containsMouse || star.hovered
    // ease-bounce in globals.css: cubic-bezier(0.34, 1.56, 0.64, 1).
    readonly property var bounce: [0.34, 1.56, 0.64, 1, 1, 1]

    implicitHeight: card.height + 8

    RectangularShadow {
        visible: !Theme.noEffects
        anchors.fill: card
        radius: card.radius
        offset.y: tile.hovered ? 10 : 4
        blur: tile.hovered ? 15 : 6
        spread: tile.hovered ? -3 : -1
        color: tile.hovered ? Theme.a(Theme.primary, 0.2) : Qt.rgba(0, 0, 0, 0.1)
        Behavior on offset.y {
            NumberAnimation {
                duration: 300
                easing.type: Easing.BezierSpline
                easing.bezierCurve: tile.bounce
            }
        }
        Behavior on color {
            ColorAnimation {
                duration: 300
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
                duration: 300
                easing.type: Easing.BezierSpline
                easing.bezierCurve: tile.bounce
            }
        }
        Behavior on border.color {
            ColorAnimation {
                duration: 300
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
                    source: tile.img.thumb && !Theme.noImages ? "file://" + tile.img.thumb : ""
                    fillMode: Image.PreserveAspectCrop
                    asynchronous: true
                    // Not kept in Qt's image cache once the tile is gone.
                    cache: false
                    // Decoded at the size it's shown, not the file's.
                    sourceSize.width: picture.width * 2
                    scale: tile.hovered ? 1.03 : 1
                    opacity: status === Image.Ready ? 1 : 0
                    onStatusChanged: if (status === Image.Ready)
                        tile.thumbReady()
                    Behavior on scale {
                        NumberAnimation {
                            duration: 300
                        }
                    }
                    Behavior on opacity {
                        NumberAnimation {
                            duration: 200
                        }
                    }
                }
                Row {
                    x: 8
                    y: 8
                    spacing: 4
                    Repeater {
                        model: tile.shownOn
                        Rectangle {
                            required property string modelData
                            width: badge.implicitWidth + 16
                            height: 20
                            radius: 10
                            color: Theme.primary
                            Text {
                                id: badge
                                anchors.centerIn: parent
                                text: parent.modelData
                                color: Theme.primaryForeground
                                font.family: Theme.font
                                font.pixelSize: 12
                                font.weight: Font.Bold
                            }
                        }
                    }
                }
                Rectangle {
                    visible: tile.busy
                    anchors.fill: parent
                    color: Theme.a(Theme.background, 0.6)
                    Rectangle {
                        anchors.centerIn: parent
                        width: 22
                        height: 22
                        radius: 11
                        color: "transparent"
                        border.width: 2.5
                        border.color: Theme.primary
                        RotationAnimation on rotation {
                            from: 0
                            to: 360
                            duration: 900
                            loops: Animation.Infinite
                            running: tile.busy
                        }
                        Rectangle {
                            width: 8
                            height: 8
                            color: Theme.a(Theme.background, 1)
                        }
                    }
                }
            }

            ColumnLayout {
                Layout.fillWidth: true
                Layout.leftMargin: 12
                Layout.rightMargin: 12
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
                    Text {
                        visible: tile.img.width > 0
                        text: tile.img.width + "×" + tile.img.height
                        color: Theme.a(Theme.mutedForeground, 0.7)
                        font.family: Theme.font
                        font.pixelSize: 10
                    }
                }
                Row {
                    visible: (tile.img.tags || []).length > 0
                    Layout.fillWidth: true
                    spacing: 4
                    clip: true
                    Repeater {
                        model: tile.img.tags || []
                        Rectangle {
                            required property string modelData
                            width: chip.implicitWidth + 16
                            height: chip.implicitHeight + 4
                            radius: height / 2
                            color: Theme.a(Theme.primary, 0.15)
                            Text {
                                id: chip
                                anchors.centerIn: parent
                                text: parent.modelData
                                color: Theme.primary
                                font.family: Theme.font
                                font.pixelSize: 10
                                font.weight: Font.Medium
                            }
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
            onClicked: tile.apply()
        }

        // The star: yellow on a favorite, else only on hover.
        Rectangle {
            id: star
            property bool hovered: starMouse.containsMouse
            x: parent.width - width - 14
            y: 14
            width: 28
            height: 28
            radius: 14
            color: Theme.a(Theme.background, hovered ? 1 : 0.8)
            opacity: tile.img.favorite || tile.hovered ? 1 : 0
            scale: hovered ? 1.1 : 1
            Behavior on opacity {
                NumberAnimation {
                    duration: 300
                }
            }
            Behavior on scale {
                NumberAnimation {
                    duration: 300
                    easing.type: Easing.BezierSpline
                    easing.bezierCurve: tile.bounce
                }
            }
            Icon {
                anchors.centerIn: parent
                name: "star"
                filled: tile.img.favorite
                color: tile.img.favorite ? "#facc15" : Theme.a(Theme.foreground, 0.8)
            }
            MouseArea {
                id: starMouse
                anchors.fill: parent
                hoverEnabled: true
                cursorShape: Qt.PointingHandCursor
                onClicked: tile.toggleFavorite()
            }
        }
    }
}
