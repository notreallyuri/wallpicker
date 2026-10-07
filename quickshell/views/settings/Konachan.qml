// Settings → Konachan: which site (konachan.net is safe-for-work only).
import QtQuick
import QtQuick.Layouts
import qs.components
import qs.services

ColumnLayout {
    id: section

    property var form
    readonly property var d: form ? form.draft : ({})

    spacing: 20

    Field {
        Layout.fillWidth: true
        label: "Site"
        description: "Same posts on both. konachan.net only has safe-for-work ones; konachan.com adds the Questionable and Explicit ratings, which the Filters dialog then offers. No account or key is needed."
        Select {
            minWidth: 300
            options: section.form && section.form.options ? section.form.options.konachan_sites.map(x => [x.value, x.label]) : []
            value: section.d.konachan_site || "net"
            onPicked: v => section.form.set("konachan_site", v)
        }
    }
    Note {
        text: "Konachan doesn't mark artist tags in its API, so downloads from it aren't tagged with their artist."
    }
}
