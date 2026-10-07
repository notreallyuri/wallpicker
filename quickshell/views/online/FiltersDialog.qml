// The current provider's sorting and filters, in a dialog so the results
// start right under the search box. Kept as they change; Reset brings back
// the defaults, Search runs the search with them.
import QtQuick
import QtQuick.Layouts
import qs.components
import qs.services

Dialog {
    id: dialog

    title: Online.label + " filters"
    description: "Kept as you change them, and used by the next search."
    preferredWidth: 600

    readonly property bool wanted: Dialogs.onlineFilters
    onWantedChanged: if (wanted)
        open()
    onClosed: Dialogs.onlineFilters = false

    Loader {
        Layout.fillWidth: true
        sourceComponent: Online.provider === "wallhaven" ? wallhaven : moebooru
    }
    Component {
        id: wallhaven
        WallhavenFilters {}
    }
    Component {
        id: moebooru
        MoebooruFilters {}
    }

    footerLeft: Btn {
        variant: "ghost"
        text: "Reset"
        onClicked: Online.resetFilters()
    }
    footer: Btn {
        variant: "primary"
        text: "Search"
        onClicked: {
            dialog.close();
            Online.search();
        }
    }
}
