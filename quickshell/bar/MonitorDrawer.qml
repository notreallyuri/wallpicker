// Under the top bar, opened by its chevron: each monitor bigger, with what
// it shows (MonitorCard), and All. A card picks where a click applies, like
// the pills. It slides open and pushes the tab below down.
import QtQuick
import qs.services

Item {
    id: drawer

    readonly property bool open: Monitors.drawerOpen && Monitors.list.length > 0

    implicitHeight: open ? cards.implicitHeight + 32 + 1 : 0
    clip: true
    Behavior on implicitHeight {
        NumberAnimation {
            duration: Theme.ms(300)
            easing.type: Easing.OutCubic
        }
    }

    Rectangle {
        // As tall as when open: the drawer only uncovers it.
        width: parent.width
        height: cards.implicitHeight + 32 + 1
        color: Theme.a(Theme.background, 0.45)
        Rectangle {
            anchors.bottom: parent.bottom
            width: parent.width
            height: 1
            color: Theme.a(Theme.border, 0.6)
        }
        Flow {
            id: cards
            readonly property real rowWidth: Math.min(drawer.width - 40, (Monitors.list.length + (Monitors.list.length > 1 ? 1 : 0)) * (288 + spacing) - spacing)
            x: (drawer.width - width) / 2
            y: 16
            width: Math.max(288, rowWidth)
            spacing: 16
            Repeater {
                model: Monitors.list
                MonitorCard {
                    required property var modelData
                    monitor: modelData
                }
            }
            AllCard {
                visible: Monitors.list.length > 1
            }
        }
    }
}
