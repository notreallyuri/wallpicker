// A monitor (monitor_bar.rs's Screen): a dark bezel with a thin chin around
// the screen, which shows its wallpaper in the monitor's own proportions
// under a faint glare, on a tapered neck and a base. Sizes: "card" (the
// drawer), "desk" (one of several on the All card: no base, the desk draws
// one) and "big" (the Adjust dialog). An empty screen when nothing's set.
// Children replace the wallpaper on the screen (the Adjust dialog's framer).
import QtQuick
import QtQuick.Effects
import QtQuick.Shapes
import Quickshell.Widgets
import qs.services

Item {
    id: figure

    property string size: "card"
    /// The wallpaper shown ("" none).
    property string path
    /// Whose mirroring it shows.
    property string monitor
    /// The monitor's resolution, for its proportions.
    property real monitorWidth: 16
    property real monitorHeight: 9
    /// The screen's own contents, instead of `path`'s picture.
    default property alias screenContent: custom.data
    readonly property alias screen: screen

    // (screen height, bezel padding, chin, bezel radius, screen radius,
    // neck height, neck width, base height, base width), as the app's
    // Tailwind sizes.
    readonly property var dims: ({
            card: [96, 4, 8, 10, 6, 14, 32, 6, 96],
            desk: [56, 3, 6, 8, 5, 10, 24, 0, 0],
            big: [192, 6, 12, 14, 8, 20, 48, 8, 144]
        })[size]
    readonly property real screenHeight: dims[0]
    readonly property real screenWidth: Math.round(screenHeight * (monitorWidth > 0 && monitorHeight > 0 ? monitorWidth / monitorHeight : 16 / 9))
    readonly property var flip: Monitors.adjustOf(monitor).flip
    /// Its part of a picture spanned across the monitors, and the
    /// picture's size: the screen shows that part (else the whole picture,
    /// cropped like Cover).
    readonly property var cut: Monitors.adjustOf(monitor).span
    readonly property var natural: {
        const img = Library.images.find(i => i.path === path);
        return img && img.width > 0 ? [img.width, img.height] : null;
    }

    // The library's thumbnail, else one made on demand.
    property string thumb: ""
    function loadThumb() {
        // Looked up from `path` here: a binding on it can still hold the
        // last wallpaper inside onPathChanged (the screen stayed one behind).
        const image = Library.images.find(i => i.path === path);
        thumb = image && image.thumb ? image.thumb : "";
        if (path !== "" && thumb === "") {
            const wanted = path;
            Library.thumbnail(path, file => {
                if (figure.path === wanted)
                    figure.thumb = file;
            });
        }
    }
    onPathChanged: loadThumb()
    Component.onCompleted: loadThumb()
    // A rescan can bring the thumbnail (or a new file).
    Connections {
        target: Library
        function onImagesChanged() {
            figure.loadThumb();
        }
    }

    implicitWidth: Math.max(bezel.width, dims[8])
    implicitHeight: bezel.height + dims[5] + dims[7]

    RectangularShadow {
        anchors.fill: bezel
        radius: bezel.radius
        offset.y: figure.size === "big" ? 8 : 4
        blur: figure.size === "big" ? 20 : 12
        spread: -2
        color: Qt.rgba(0, 0, 0, 0.35)
    }
    Rectangle {
        id: bezel
        anchors.horizontalCenter: parent.horizontalCenter
        width: figure.screenWidth + 2 * figure.dims[1]
        height: figure.screenHeight + figure.dims[1] + figure.dims[2]
        radius: figure.dims[3]
        color: "#18181b"
        border.width: 1
        border.color: Qt.rgba(1, 1, 1, 0.1)

        ClippingRectangle {
            id: screen
            x: figure.dims[1]
            y: figure.dims[1]
            width: figure.screenWidth
            height: figure.screenHeight
            radius: figure.dims[4]
            color: "#27272a"

            // Spanned: the whole picture, scaled so the cut fills the screen
            // and moved so it's there; mirrored like the span.
            Item {
                visible: custom.children.length === 0 && figure.cut !== null && figure.natural !== null
                anchors.fill: parent
                transform: Scale {
                    origin.x: figure.screenWidth / 2
                    origin.y: figure.screenHeight / 2
                    xScale: figure.flip.h ? -1 : 1
                    yScale: figure.flip.v ? -1 : 1
                }
                Image {
                    readonly property real k: figure.cut ? figure.screenWidth / Math.max(1, figure.cut.width) : 1
                    x: figure.cut ? -figure.cut.x * k : 0
                    y: figure.cut ? -figure.cut.y * k : 0
                    width: figure.natural ? figure.natural[0] * k : 0
                    height: figure.natural ? figure.natural[1] * k : 0
                    source: parent.visible && figure.thumb ? "file://" + figure.thumb : ""
                    fillMode: Image.Stretch
                    asynchronous: true
                }
            }
            Image {
                visible: custom.children.length === 0 && !(figure.cut !== null && figure.natural !== null)
                anchors.fill: parent
                source: figure.thumb ? "file://" + figure.thumb : ""
                fillMode: Image.PreserveAspectCrop
                sourceSize.width: figure.screenWidth * Screen.devicePixelRatio
                asynchronous: true
                // Mirrored like it's shown (Adjust).
                transform: Scale {
                    origin.x: figure.screenWidth / 2
                    origin.y: figure.screenHeight / 2
                    xScale: figure.flip.h ? -1 : 1
                    yScale: figure.flip.v ? -1 : 1
                    Behavior on xScale {
                        NumberAnimation {
                            duration: Theme.ms(300)
                            easing.type: Easing.OutCubic
                        }
                    }
                    Behavior on yScale {
                        NumberAnimation {
                            duration: Theme.ms(300)
                            easing.type: Easing.OutCubic
                        }
                    }
                }
            }
            Item {
                id: custom
                anchors.fill: parent
            }
            // Glass: a glare from the top left corner.
            Shape {
                anchors.fill: parent
                ShapePath {
                    strokeWidth: -1
                    fillGradient: LinearGradient {
                        x1: 0
                        y1: 0
                        x2: screen.width
                        y2: screen.height
                        GradientStop {
                            position: 0
                            color: Qt.rgba(1, 1, 1, 0.15)
                        }
                        GradientStop {
                            position: 0.5
                            color: "transparent"
                        }
                    }
                    PathRectangle {
                        width: screen.width
                        height: screen.height
                    }
                }
            }
        }
    }
    // A neck that widens toward the base.
    Shape {
        id: neck
        anchors.horizontalCenter: parent.horizontalCenter
        y: bezel.height
        width: figure.dims[6]
        height: figure.dims[5]
        preferredRendererType: Shape.CurveRenderer
        ShapePath {
            strokeWidth: -1
            fillGradient: LinearGradient {
                y2: neck.height
                GradientStop {
                    position: 0
                    color: "#3f3f46"
                }
                GradientStop {
                    position: 1
                    color: "#27272a"
                }
            }
            PathPolyline {
                path: [Qt.point(neck.width * 0.28, 0), Qt.point(neck.width * 0.72, 0), Qt.point(neck.width, neck.height), Qt.point(0, neck.height), Qt.point(neck.width * 0.28, 0)]
            }
        }
    }
    RectangularShadow {
        visible: base.visible
        anchors.fill: base
        radius: base.radius
        offset.y: 3
        blur: 6
        color: Qt.rgba(0, 0, 0, 0.35)
    }
    Rectangle {
        id: base
        visible: figure.dims[7] > 0
        anchors.horizontalCenter: parent.horizontalCenter
        y: neck.y + neck.height
        width: figure.dims[8]
        height: figure.dims[7]
        radius: height / 2
        gradient: Gradient {
            GradientStop {
                position: 0
                color: "#52525b"
            }
            GradientStop {
                position: 1
                color: "#27272a"
            }
        }
    }
}
