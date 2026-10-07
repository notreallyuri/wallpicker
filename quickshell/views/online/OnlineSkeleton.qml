// A placeholder for an online result while the first ones load: the
// picture's shape and the caption's, softly pulsing, each a little after
// the one before.
import QtQuick
import QtQuick.Layouts
import qs.services

ColumnLayout {
    id: skeleton

    property int delay: 0

    spacing: 6
    opacity: 1

    Rectangle {
        Layout.fillWidth: true
        Layout.preferredHeight: width * 10 / 16
        radius: 16
        color: Theme.muted
    }
    RowLayout {
        Layout.fillWidth: true
        Layout.leftMargin: 4
        Layout.rightMargin: 4
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
