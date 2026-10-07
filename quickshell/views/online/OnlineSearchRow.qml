// The Online search row: the provider (a select: the list grows), the
// search box (Enter searches), Filters (with how many are set) and Search.
import QtQuick
import QtQuick.Layouts
import qs.components
import qs.services

RowLayout {
    id: row

    function focusSearch() {
        search.forceFocus();
    }

    readonly property var placeholders: ({
            wallhaven: "Search Wallhaven: tags, +include -exclude, @uploader, id:<tag id>",
            konachan: "Search Konachan: tags, comma-separated (long hair, night) or long_hair night; -tag to exclude",
            yandere: "Search yande.re: tags, comma-separated (long hair, night) or long_hair night; -tag to exclude",
            unsplash: "Search Unsplash: photos (empty: its Wallpapers topic); user:<name> for a photographer's"
        })

    spacing: 8

    Select {
        minWidth: 144
        options: Online.options ? Online.options.providers.map(p => [p.value, p.label]) : []
        value: Online.provider
        onPicked: value => Online.setProvider(value)
    }
    TextField {
        id: search
        Layout.fillWidth: true
        Layout.minimumWidth: 256
        icon: "search"
        placeholder: row.placeholders[Online.provider] || ""
        text: Online.text
        onTextChanged: Online.text = text
        onAccepted: Online.search()
    }
    Btn {
        icon: "funnel"
        text: "Filters"
        tip: "Sorting and filters"
        onClicked: Dialogs.onlineFilters = true
        // How many differ from the defaults.
        Rectangle {
            visible: Online.activeFilters > 0
            implicitWidth: Math.max(20, count.implicitWidth + 12)
            implicitHeight: 20
            radius: height / 2
            color: Theme.primary
            Text {
                id: count
                anchors.centerIn: parent
                text: Online.activeFilters
                color: Theme.primaryForeground
                font.family: Theme.font
                font.pixelSize: 12
                font.weight: Font.Bold
            }
        }
    }
    Btn {
        variant: "primary"
        text: "Search"
        opacity: Online.loading ? 0.5 : 1
        onClicked: if (!Online.loading)
            Online.search()
    }
}
