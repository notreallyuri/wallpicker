// The details' picture, sharper as it comes in: the tile's thumbnail at
// once (already loaded), the provider's preview faded in over it (yande.re's
// takes seconds), then the sharp one when there is one (the backend
// downloads it and fits it to the screen: an 8K original decoded here would
// cost over 100 MB). What's covered is let go.
import QtQuick
import Quickshell.Widgets
import qs.services

ClippingRectangle {
    id: preview

    property var w: null
    /// The sharp one's file ("" until it's in, or when there's none).
    property string sharp: ""

    radius: 14
    color: Theme.muted

    onWChanged: {
        sharp = "";
        if (!w)
            return;
        const asked = w;
        Backend.request("online.preview", {
            wallpaper: asked
        }, path => {
            if (preview.w === asked && path)
                preview.sharp = path;
        }, () => {});
    }

    // Each layer, once it's in, covers (and drops) the ones under it.
    component Layer: Image {
        property bool covered: false
        property string url
        anchors.fill: parent
        source: covered ? "" : url
        visible: !covered
        fillMode: Image.PreserveAspectCrop
        asynchronous: true
        opacity: status === Image.Ready ? 1 : 0
        Behavior on opacity {
            NumberAnimation {
                duration: 300
            }
        }
    }

    Layer {
        url: preview.w ? preview.w.thumb : ""
        covered: quick.opacity === 1 || sharpLayer.opacity === 1
    }
    Layer {
        id: quick
        url: preview.w && preview.w.preview !== preview.w.thumb ? preview.w.preview : ""
        covered: sharpLayer.opacity === 1
    }
    Layer {
        id: sharpLayer
        url: preview.sharp ? "file://" + preview.sharp : ""
        cache: false
    }
}
