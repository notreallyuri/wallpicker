// The Local tab: the grid, with the search card floating over its top.
import QtQuick
import qs.services

Item {
    id: view

    LocalModel {
        id: localModel
    }

    LocalGrid {
        id: grid
        anchors.fill: parent
        anchors.leftMargin: 20
        anchors.rightMargin: 4
        local: localModel
        headerSpace: card.height + 12 + 28
    }

    SearchCard {
        id: card
        x: 12
        y: 12
        width: parent.width - 24
        local: localModel
        behind: grid
    }

    Text {
        visible: !Library.loading && localModel.shown.length === 0
        anchors.centerIn: parent
        text: "No wallpapers match."
        color: Theme.mutedForeground
        font.family: Theme.font
        font.pixelSize: 14
    }
}
