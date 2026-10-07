// Settings → yande.re: nothing to set up; what to know about it.
import QtQuick
import QtQuick.Layouts
import qs.services

ColumnLayout {
    property var form

    spacing: 12

    Text {
        text: "Nothing to set up: no account or key is needed."
        color: Theme.foreground
        font.family: Theme.font
        font.pixelSize: 14
    }
    Note {
        text: "yande.re has all ratings (Safe only until you tick others in the Filters dialog) and many very large scans. Originals over 64 MiB are downloaded as yande.re's full-resolution JPEG instead. Like Konachan, its API doesn't mark artist tags."
    }
}
