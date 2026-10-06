// The card floating over the top of the grid (rounded-2xl, frosted glass
// over the grid, a soft shadow): the search row and the tag row.
import QtQuick
import QtQuick.Effects
import QtQuick.Layouts
import qs.components
import qs.services

Item {
    id: card

    required property LocalModel local
    /// What's under the card, for the glass.
    property Item behind

    implicitHeight: rows.implicitHeight + 18

    RectangularShadow {
        anchors.fill: parent
        radius: 22
        offset.y: 8
        blur: 20
        color: Qt.rgba(0, 0, 0, 0.15)
    }
    Glass {
        anchors.fill: parent
        radius: 22
        source: card.behind
        origin: card.behind ? card.mapToItem(card.behind, 0, 0) : Qt.point(0, 0)
    }
    Rectangle {
        anchors.fill: parent
        radius: 22
        color: "transparent"
        border.width: 1
        border.color: Theme.a(Theme.border, 0.6)
    }

    ColumnLayout {
        id: rows
        x: 16
        y: 10
        width: parent.width - 32
        spacing: 8
        SearchRow {
            Layout.fillWidth: true
            local: card.local
        }
        TagRow {
            Layout.fillWidth: true
            local: card.local
        }
    }
}
