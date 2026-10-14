// The card over the Collections grid: your collections as chips (name,
// count, private), the chosen one filled; or that they're loading, that
// there are none, or why they can't be listed (no API key). Refresh.
import QtQuick
import QtQuick.Effects
import QtQuick.Layouts
import qs.components
import qs.services

Item {
    id: card

    /// What's under the card, for the glass (see Glass).
    property Item behind
    property Item backdrop
    property var follow

    implicitHeight: row.implicitHeight + 20

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
        backdrop: card.backdrop
        follow: card.follow
    }
    Rectangle {
        anchors.fill: parent
        radius: 22
        color: "transparent"
        border.width: 1
        border.color: Theme.a(Theme.border, 0.6)
    }

    RowLayout {
        id: row
        x: 16
        y: 10
        width: parent.width - 32
        spacing: 12

        Text {
            Layout.alignment: Qt.AlignTop
            Layout.topMargin: 8
            text: "Wallhaven collections"
            color: Theme.mutedForeground
            font.family: Theme.font
            font.pixelSize: 14
        }
        Spinner {
            visible: Collections.list === null
            size: 18
        }
        // None, or they couldn't be listed: the view says so (EmptyState).
        Item {
            visible: Collections.list !== null && (Collections.listError !== "" || Collections.list.length === 0)
            Layout.fillWidth: true
        }
        Flow {
            visible: Collections.list !== null && Collections.list.length > 0
            Layout.fillWidth: true
            spacing: 6
            Repeater {
                model: Collections.list || []
                Chip {
                    required property var modelData
                    text: modelData.label
                    count: modelData.count + (modelData.public ? "" : "  private")
                    on: modelData.id === Collections.chosen
                    // Picking the chosen one again keeps it.
                    onClicked: if (!on)
                        Collections.choose(modelData.id)
                }
            }
        }
        Item {
            visible: Collections.list === null
            Layout.fillWidth: true
        }
        Btn {
            Layout.alignment: Qt.AlignTop
            icon: "refresh"
            text: "Refresh"
            onClicked: Collections.reload()
        }
    }
}
