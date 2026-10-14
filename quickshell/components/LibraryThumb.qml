// A library image's thumbnail, rounded: the scan's cached one, else one made
// on demand. Empty (muted) for no path.
import QtQuick
import Quickshell.Widgets
import qs.services

ClippingRectangle {
    id: thumb

    property string path
    property string file: ""

    function load() {
        // From `path` itself: a binding on it can lag inside onPathChanged.
        const image = Library.images.find(i => i.path === path);
        file = image && image.thumb ? image.thumb : "";
        if (path !== "" && file === "") {
            const wanted = path;
            Library.thumbnail(path, f => {
                if (thumb.path === wanted)
                    thumb.file = f;
            });
        }
    }
    onPathChanged: load()
    Component.onCompleted: load()

    radius: 10
    color: Theme.muted

    Image {
        anchors.fill: parent
        source: thumb.file ? "file://" + thumb.file : ""
        fillMode: Image.PreserveAspectCrop
        asynchronous: true
        // In 128 px steps, the old one shown until a new size is in.
        sourceSize.width: Math.ceil(thumb.width * Screen.devicePixelRatio / 128) * 128
        retainWhileLoading: true
        property bool came: false
        onSourceChanged: came = false
        onStatusChanged: if (status === Image.Ready)
            came = true
        opacity: came ? 1 : 0
        Behavior on opacity {
            NumberAnimation {
                duration: Theme.ms(150)
            }
        }
    }
}
