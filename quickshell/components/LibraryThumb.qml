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
        sourceSize.width: thumb.width * 2
        opacity: status === Image.Ready ? 1 : 0
        Behavior on opacity {
            NumberAnimation {
                duration: Theme.ms(150)
            }
        }
    }
}
