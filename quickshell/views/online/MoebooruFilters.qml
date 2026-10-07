// Konachan's and yande.re's filters (ui/src/views/online/moebooru.rs):
// sorting, minimum size and ratings (which konachan.net doesn't have: it's
// safe-for-work only).
import QtQuick
import QtQuick.Layouts
import qs.components
import qs.services

ColumnLayout {
    id: form

    readonly property var f: Online.filters[Online.provider] || {}

    function flip(key) {
        const next = Object.assign({}, f.ratings);
        next[key] = !next[key];
        Online.setFilter("ratings", next);
    }

    spacing: 20

    RowLayout {
        spacing: 16
        Field {
            label: "Sort by"
            Select {
                minWidth: 144
                options: Online.options ? Online.options.moebooru.sorts.map(x => [x.value, x.label]) : []
                value: form.f.sorting || ""
                onPicked: value => Online.setFilter("sorting", value)
            }
        }
        Field {
            label: "Minimum size"
            MinSize {
                value: form.f.atleast || ""
                onPicked: value => Online.setFilter("atleast", value || null)
            }
        }
    }
    Field {
        visible: Online.ratingsApply
        label: "Ratings"
        RowLayout {
            spacing: 20
            Repeater {
                model: [["safe", "Safe"], ["questionable", "Questionable"], ["explicit", "Explicit"]]
                Check {
                    required property var modelData
                    text: modelData[1]
                    checked: !!(form.f.ratings && form.f.ratings[modelData[0]])
                    onToggled: form.flip(modelData[0])
                }
            }
        }
    }
    Text {
        Layout.fillWidth: true
        wrapMode: Text.Wrap
        text: (Online.ratingsApply ? "" : "konachan.net only has safe-for-work posts. Switch to konachan.com in Settings → Konachan for all ratings.\n") + (Online.provider === "yandere" ? "yande.re: high-resolution scans, all ratings. " : "") + "In the search box: tags, -tag to exclude, user:<name> for an uploader."
        color: Theme.mutedForeground
        font.family: Theme.font
        font.pixelSize: 13
    }
}
