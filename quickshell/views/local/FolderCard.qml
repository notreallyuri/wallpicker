// A subfolder in the Folders layout (folders.rs's FolderCard): its first
// wallpaper as a dimmed cover with a folder icon, its name and how many are
// in it or below. Lifts on hover like a tile; opens on click.
import QtQuick
import QtQuick.Effects
import QtQuick.Layouts
import Quickshell.Widgets
import qs.components
import qs.services

Item {
    id: card

    required property var folder
    signal opened

    readonly property bool hovered: mouse.containsMouse
    property string thumb: folder.cover.thumb || ""

    implicitHeight: box.height + 8

    Component.onCompleted: {
        if (!thumb)
            Library.thumbnail(folder.cover.path, file => card.thumb = file);
    }

    RectangularShadow {
        anchors.fill: box
        radius: box.radius
        offset.y: card.hovered ? 10 : 4
        blur: card.hovered ? 15 : 6
        spread: card.hovered ? -3 : -1
        color: card.hovered ? Theme.a(Theme.primary, 0.2) : Qt.rgba(0, 0, 0, 0.1)
        Behavior on offset.y {
            NumberAnimation {
                duration: Theme.duration
                easing.type: Easing.BezierSpline
                easing.bezierCurve: Theme.bounce
            }
        }
    }
    Rectangle {
        id: box
        width: parent.width
        height: content.implicitHeight + 12
        y: card.hovered ? -4 : 0
        radius: 24
        color: Theme.a(Theme.card, 0.8)
        border.width: card.hovered ? 2 : 1
        border.color: card.hovered ? Theme.a(Theme.primary, 0.7) : Theme.a(Theme.foreground, 0.05)
        Behavior on y {
            NumberAnimation {
                duration: Theme.duration
                easing.type: Easing.BezierSpline
                easing.bezierCurve: Theme.bounce
            }
        }

        ColumnLayout {
            id: content
            x: 6
            y: 6
            width: parent.width - 12
            spacing: 0
            ClippingRectangle {
                Layout.fillWidth: true
                Layout.preferredHeight: width * 10 / 16
                radius: 17.6
                color: Theme.muted
                Image {
                    anchors.fill: parent
                    source: card.thumb ? "file://" + card.thumb : ""
                    fillMode: Image.PreserveAspectCrop
                    asynchronous: true
                    cache: false
                    // In 128 px steps, the old one shown until a new size is in.
                    sourceSize.width: Math.ceil(width * Screen.devicePixelRatio / 128) * 128
                    retainWhileLoading: true
                    opacity: card.hovered ? 0.7 : 0.5
                    Behavior on opacity {
                        NumberAnimation {
                            duration: Theme.ms(200)
                        }
                    }
                }
                Icon {
                    anchors.centerIn: parent
                    name: "folder"
                    size: 40
                    color: Theme.foreground
                }
            }
            RowLayout {
                Layout.fillWidth: true
                Layout.margins: 12
                Layout.topMargin: 8
                Layout.bottomMargin: 8
                Text {
                    Layout.fillWidth: true
                    text: card.folder.name
                    elide: Text.ElideRight
                    color: Theme.foreground
                    font.family: Theme.font
                    font.pixelSize: 12
                    font.weight: Font.Medium
                }
                Text {
                    text: card.folder.count
                    color: Theme.mutedForeground
                    font.family: Theme.font
                    font.pixelSize: 12
                }
            }
        }
        MouseArea {
            id: mouse
            anchors.fill: parent
            hoverEnabled: true
            cursorShape: Qt.PointingHandCursor
            onClicked: card.opened()
        }
    }
}
