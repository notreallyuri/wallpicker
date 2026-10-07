// A placeholder for an online result while the first ones load: the card,
// the picture's shape and the caption's, softly pulsing, each a little
// after the one before.
import QtQuick
import QtQuick.Layouts
import qs.services

Rectangle {
    id: skeleton

    property int delay: 0

    height: content.implicitHeight + 12
    radius: 24
    color: Theme.a(Theme.card, 0.8)
    border.width: 1
    border.color: Theme.a(Theme.foreground, 0.05)

    ColumnLayout {
        id: content
        x: 6
        y: 6
        width: parent.width - 12
        spacing: 9
        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: width * 10 / 16
            radius: 17.6
            color: Theme.muted
        }
        RowLayout {
            Layout.fillWidth: true
            Layout.leftMargin: 6
            Layout.rightMargin: 6
            Layout.bottomMargin: 9
            Rectangle {
                implicitWidth: 96
                implicitHeight: 12
                radius: 6
                color: Theme.muted
            }
            Item {
                Layout.fillWidth: true
            }
            Rectangle {
                implicitWidth: 48
                implicitHeight: 12
                radius: 6
                color: Theme.muted
            }
        }
    }

    // Started `delay` late, once: a wave across the grid.
    Timer {
        interval: skeleton.delay
        running: true
        onTriggered: pulse.start()
    }
    SequentialAnimation {
        id: pulse
        loops: Animation.Infinite
        NumberAnimation {
            target: skeleton
            property: "opacity"
            to: 0.5
            duration: 800
            easing.type: Easing.InOutSine
        }
        NumberAnimation {
            target: skeleton
            property: "opacity"
            to: 1
            duration: 800
            easing.type: Easing.InOutSine
        }
    }
}
