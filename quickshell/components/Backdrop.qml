// The wallpaper, big and blurred, behind everything (palette.rs's Backdrop:
// blur-2xl, saturate-150, 70%, scaled 110%).
import QtQuick
import QtQuick.Effects

Item {
    id: backdrop

    /// The picture (a thumbnail is plenty, blurred).
    property string source

    Image {
        id: picture
        anchors.fill: parent
        source: backdrop.source
        fillMode: Image.PreserveAspectCrop
        sourceSize.width: 360
        asynchronous: true
        visible: false
    }
    MultiEffect {
        anchors.fill: parent
        source: picture
        scale: 1.1
        opacity: picture.status === Image.Ready ? 0.7 : 0
        blurEnabled: true
        blur: 1
        blurMax: 64
        saturation: 0.5
        Behavior on opacity {
            NumberAnimation {
                duration: 400
            }
        }
    }
}
