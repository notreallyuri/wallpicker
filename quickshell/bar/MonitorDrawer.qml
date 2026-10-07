// Under the top bar, opened by its chevron: each monitor bigger, with what
// it shows (MonitorCard), and All; spanned, one card for them all
// (SpanCard). A card picks where a click applies, like the pills. It slides
// open and pushes the tab below down.
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
            // Cards: the span's, each monitor outside it, All.
            readonly property var alone: Monitors.list.filter(m => !Monitors.inSpan(m.name))
            readonly property real rowWidth: Math.min(drawer.width - 40, (Monitors.spanned ? spanCard.implicitWidth + spacing : 0) + (alone.length + (Monitors.list.length > 1 ? 1 : 0)) * (288 + spacing) - spacing)
            x: (drawer.width - width) / 2
            y: 16
            width: Math.max(288, rowWidth)
            spacing: 16
            SpanCard {
                id: spanCard
                visible: Monitors.spanned
            }
            Repeater {
                model: cards.alone
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
