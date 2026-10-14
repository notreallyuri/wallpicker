// A local wallpaper in its Details: the thumbnail for a still; an animated
// GIF or WebP plays, decoded at the preview's size, only while it's shown
// (`live`), and is let go when it isn't. Its bar pauses the preview (not
// the wallpaper) and shows where it is in the loop.
import QtQuick
import QtQuick.Layouts
import Quickshell.Widgets
import qs.components
import qs.services

ClippingRectangle {
    id: preview

    /// The library's image ({path, thumb, frames...}).
    property var image: null
    /// The dialog is open.
    property bool live: false
    readonly property bool animated: !!image && image.frames > 1
    property bool held: false

    implicitHeight: Math.round(width * 10 / 16)
    radius: 14
    color: Theme.muted
    onImageChanged: held = false

    Image {
        anchors.fill: parent
        visible: !preview.animated || movie.status !== AnimatedImage.Ready
        source: preview.image && preview.image.thumb ? "file://" + preview.image.thumb : ""
        fillMode: Image.PreserveAspectCrop
        asynchronous: true
    }
    AnimatedImage {
        id: movie
        anchors.fill: parent
        source: preview.animated && preview.live ? "file://" + preview.image.path : ""
        sourceSize.width: Math.ceil(preview.width * Screen.devicePixelRatio / 128) * 128
        fillMode: Image.PreserveAspectCrop
        asynchronous: true
        cache: false
        playing: preview.live && !preview.held
    }

    Rectangle {
        visible: preview.animated
        x: 10
        y: 10
        width: liveRow.implicitWidth + 16
        height: 22
        radius: 11
        color: Theme.primary
        Row {
            id: liveRow
            anchors.centerIn: parent
            spacing: 4
            Icon {
                anchors.verticalCenter: parent.verticalCenter
                name: "film"
                size: 12
                color: Theme.primaryForeground
            }
            Text {
                anchors.verticalCenter: parent.verticalCenter
                text: preview.held ? "Paused" : "Playing"
                color: Theme.primaryForeground
                font.family: Theme.font
                font.pixelSize: 12
                font.weight: Font.Bold
            }
        }
    }

    Rectangle {
        visible: preview.animated && movie.status === AnimatedImage.Ready
        x: 10
        y: parent.height - height - 10
        width: Math.min(parent.width - 20, bar.implicitWidth + 20)
        height: 40
        radius: 20
        color: Theme.a(Theme.card, 0.85)
        RowLayout {
            id: bar
            anchors.fill: parent
            anchors.leftMargin: 4
            anchors.rightMargin: 14
            spacing: 10
            Btn {
                variant: "ghost"
                icon: preview.held ? "play" : "pause"
                tip: preview.held ? "Play the preview" : "Pause the preview (not the wallpaper)"
                onClicked: preview.held = !preview.held
            }
            Rectangle {
                implicitWidth: 180
                implicitHeight: 4
                radius: 2
                color: Theme.a(Theme.foreground, 0.12)
                Rectangle {
                    width: parent.width * (movie.frameCount > 1 ? movie.currentFrame / (movie.frameCount - 1) : 0)
                    height: parent.height
                    radius: 2
                    color: Theme.primary
                }
            }
            Text {
                text: "Frame " + (movie.currentFrame + 1) + " / " + movie.frameCount
                color: Theme.foreground
                font.family: Theme.font
                font.pixelSize: 12
                font.weight: Font.DemiBold
            }
        }
    }
}
