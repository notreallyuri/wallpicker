// Konachan's and yande.re's real tags for what's typed (they match exact
// tag names only): a click puts one in the search and searches. Its kind
// (artist, character...) shows beside it, unless it's a general tag.
import QtQuick
import QtQuick.Layouts
import qs.components
import qs.services

Flow {
    id: row

    spacing: 6

    Text {
        height: 28
        verticalAlignment: Text.AlignVCenter
        text: "Tags:"
        color: Theme.mutedForeground
        font.family: Theme.font
        font.pixelSize: 12
    }
    Repeater {
        model: Online.suggestions
        Chip {
            required property var modelData
            text: modelData.name
            count: modelData.count + (modelData.kind !== "general" ? "  " + modelData.kind : "")
            onClicked: Online.pickSuggestion(modelData)
        }
    }
}
