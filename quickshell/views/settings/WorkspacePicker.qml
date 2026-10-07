// Choosing a workspace's wallpaper, in place of the list: the library,
// favorites first then the newest, searchable by name and tag; a click
// picks it. A few dozen at first, more on request.
import QtQuick
import QtQuick.Layouts
import qs.components
import qs.services

ColumnLayout {
    id: picker

    property string workspace
    signal picked(string path)
    signal back

    property string query: ""
    property bool favorites: false
    property int limit: 36

    readonly property var shown: {
        const q = query.trim().toLowerCase();
        return Library.images.filter(i => !favorites || Library.favorite(i.path)).filter(i => q === "" || i.name.toLowerCase().includes(q) || Library.tags(i.path).some(t => t.toLowerCase().includes(q))).sort((a, b) => (Library.favorite(b.path) - Library.favorite(a.path)) || ((b.modified || 0) - (a.modified || 0)));
    }
    onQueryChanged: limit = 36
    onFavoritesChanged: limit = 36

    spacing: 12

    RowLayout {
        spacing: 8
        Btn {
            variant: "ghost"
            small: true
            icon: "chevron-left"
            text: "Back"
            onClicked: picker.back()
        }
        Text {
            text: "Choose for workspace " + picker.workspace
            color: Theme.foreground
            font.family: Theme.font
            font.pixelSize: 14
            font.weight: Font.Bold
        }
    }
    RowLayout {
        Layout.fillWidth: true
        spacing: 8
        TextField {
            Layout.fillWidth: true
            icon: "search"
            placeholder: "Filter by name or tag"
            onTextChanged: picker.query = text
        }
        Btn {
            icon: "star"
            text: "Favorites"
            active: picker.favorites
            onClicked: picker.favorites = !picker.favorites
        }
    }
    GridLayout {
        Layout.fillWidth: true
        columns: Math.max(1, Math.floor(width / 140))
        columnSpacing: 8
        rowSpacing: 8
        Repeater {
            model: picker.shown.slice(0, picker.limit)
            Item {
                id: cell
                required property var modelData
                Layout.fillWidth: true
                Layout.preferredHeight: width * 10 / 16
                LibraryThumb {
                    anchors.fill: parent
                    path: cell.modelData.path
                    y: mouse.containsMouse ? -2 : 0
                }
                Rectangle {
                    anchors.fill: parent
                    radius: 10
                    color: "transparent"
                    border.width: mouse.containsMouse ? 2 : 0
                    border.color: Theme.primary
                }
                MouseArea {
                    id: mouse
                    anchors.fill: parent
                    hoverEnabled: true
                    cursorShape: Qt.PointingHandCursor
                    onClicked: picker.picked(cell.modelData.path)
                }
            }
        }
    }
    Btn {
        visible: picker.shown.length > picker.limit
        Layout.alignment: Qt.AlignHCenter
        text: "Show more (" + (picker.shown.length - picker.limit) + " left)"
        onClicked: picker.limit += 36
    }
    Note {
        visible: picker.shown.length === 0
        text: Library.images.length === 0 ? "No wallpapers in your folders." : "None match."
    }
}
