// The toasts, bottom right of `host` (the panel), newest at the bottom; each
// slides in and can be clicked away. They live in the window's overlay, above
// dialogs too (an error from Settings was dimmed under its shade); looked up
// each time one comes, like Tip's bubble: the panel's window can be replaced.
import QtQuick
import QtQuick.Controls
import QtQuick.Effects
import QtQuick.Layouts
import QtQuick.Window
import qs.services

ColumnLayout {
    id: stack

    /// What they sit in the corner of.
    property Item host

    function place() {
        if (!host)
            return;
        const layer = stack.Overlay.overlay || stack.Window.contentItem;
        if (!layer)
            return;
        parent = layer;
        const corner = host.mapToItem(layer, host.width, host.height);
        x = corner.x - width - 20;
        y = corner.y - height - 20;
    }

    z: 1000
    spacing: 8
    onHeightChanged: place()
    onWidthChanged: place()

    Connections {
        target: Toasts
        function onListChanged() {
            stack.place();
        }
    }

    Repeater {
        model: Toasts.list
        Item {
            id: toast
            required property var modelData
            Layout.alignment: Qt.AlignRight
            implicitWidth: Math.min(380, body.implicitWidth + 32)
            implicitHeight: body.implicitHeight + 24
            opacity: 0
            x: 24
            Component.onCompleted: {
                opacity = 1;
                x = 0;
            }
            Behavior on opacity {
                NumberAnimation {
                    duration: 200
                }
            }
            Behavior on x {
                NumberAnimation {
                    duration: Theme.duration
                    easing.type: Easing.BezierSpline
                    easing.bezierCurve: Theme.bounce
                }
            }

            RectangularShadow {
                anchors.fill: card
                radius: card.radius
                offset.y: 6
                blur: 18
                color: Qt.rgba(0, 0, 0, 0.3)
            }
            Rectangle {
                id: card
                anchors.fill: parent
                radius: 16
                color: Theme.popover
                border.width: 1
                border.color: toast.modelData.error ? "#fd4663" : Theme.a(Theme.border, 0.6)
            }
            ColumnLayout {
                id: body
                x: 16
                y: 12
                width: parent.width - 32
                spacing: 2
                Text {
                    Layout.fillWidth: true
                    text: toast.modelData.title
                    wrapMode: Text.Wrap
                    color: toast.modelData.error ? "#fd4663" : Theme.foreground
                    font.family: Theme.font
                    font.pixelSize: 14
                    font.weight: Font.Bold
                }
                Text {
                    visible: toast.modelData.detail !== ""
                    Layout.fillWidth: true
                    text: toast.modelData.detail
                    wrapMode: Text.Wrap
                    color: Theme.mutedForeground
                    font.family: Theme.font
                    font.pixelSize: 12
                }
            }
            MouseArea {
                anchors.fill: parent
                cursorShape: Qt.PointingHandCursor
                onClicked: Toasts.dismiss(toast.modelData.id)
            }
        }
    }
}
