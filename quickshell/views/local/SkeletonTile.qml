// A placeholder tile while the library is scanned: the card's shape, softly
// pulsing.
import QtQuick
import QtQuick.Layouts
import qs.services

Item {
    implicitHeight: card.height + 8

    Rectangle {
        id: card
        width: parent.width
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
            spacing: 8
            Rectangle {
                Layout.fillWidth: true
                Layout.preferredHeight: width * 10 / 16
                radius: 17.6
                color: Theme.muted
            }
            Rectangle {
                Layout.leftMargin: 12
                Layout.bottomMargin: 10
                implicitWidth: parent.width * 0.6
                implicitHeight: 10
                radius: 5
                color: Theme.muted
            }
        }

        SequentialAnimation on opacity {
            loops: Animation.Infinite
            NumberAnimation {
                to: 0.5
                duration: 800
                easing.type: Easing.InOutSine
            }
            NumberAnimation {
                to: 1
                duration: 800
                easing.type: Easing.InOutSine
            }
        }
    }
}
