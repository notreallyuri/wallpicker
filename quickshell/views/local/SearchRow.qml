// The search card's first row: the search box, Favorites, the sort, Color,
// the layout, Random, the count, and ⋯.
import QtQuick
import QtQuick.Layouts
import qs.components
import qs.services

RowLayout {
    id: row

    required property LocalModel local

    spacing: 8

    // Takes what the row has left, within 160-320 px (as in the app).
    TextField {
        Layout.preferredWidth: Math.min(320, Math.max(160, row.width - 760))
        icon: "search"
        placeholder: "Filter by name or tag"
        onTextChanged: row.local.query = text
    }
    Btn {
        icon: "star"
        text: "Favorites"
        active: row.local.favoritesOnly
        onClicked: row.local.favoritesOnly = !row.local.favoritesOnly
    }
    Select {
        minWidth: 144
        options: row.local.sorts
        value: row.local.sort
        onPicked: value => row.local.sort = value
    }
    Btn {
        minWidth: 112
        text: "Color"
        Row {
            spacing: -5
            Repeater {
                model: ["#ef4444", "#eab308", "#3b82f6"]
                Rectangle {
                    required property string modelData
                    width: 12
                    height: 12
                    radius: 6
                    color: modelData
                    border.width: 2
                    border.color: Theme.card
                }
            }
        }
    }
    Btn {
        icon: "grid"
        text: "Flat"
    }
    Btn {
        icon: "shuffle"
        text: "Random"
        onClicked: row.local.applyRandom()
    }
    Text {
        Layout.minimumWidth: 80
        text: row.local.shown.length + " images"
        color: Theme.mutedForeground
        font.family: Theme.font
        font.pixelSize: 14
    }
    Item {
        Layout.fillWidth: true
    }
    Btn {
        variant: "ghost"
        icon: "ellipsis"
    }
}
