// Unsplash's filters: order, orientation, color and how strict its content
// filter is. Color and the content filter only apply to a search (the
// Wallpapers topic and a photographer's photos don't take them).
import QtQuick
import QtQuick.Layouts
import qs.components
import qs.services

ColumnLayout {
    id: form

    readonly property var f: Online.filters.unsplash || {}
    readonly property var o: Online.options ? Online.options.unsplash : null
    function pairs(list) {
        return list ? list.map(x => [x.value, x.label]) : [];
    }

    spacing: 20

    RowLayout {
        spacing: 16
        Field {
            label: "Sort by"
            Select {
                minWidth: 200
                options: form.pairs(form.o && form.o.orders)
                value: form.f.order || "relevant"
                onPicked: value => Online.setFilter("order", value)
            }
        }
        Field {
            label: "Orientation"
            Select {
                minWidth: 144
                options: form.pairs(form.o && form.o.orientations)
                value: form.f.orientation || ""
                onPicked: value => Online.setFilter("orientation", value)
            }
        }
    }
    RowLayout {
        spacing: 16
        Field {
            label: "Color"
            Select {
                minWidth: 200
                options: form.pairs(form.o && form.o.colors)
                value: form.f.color || ""
                onPicked: value => Online.setFilter("color", value)
            }
        }
        Field {
            label: "Content filter"
            Select {
                minWidth: 144
                options: form.pairs(form.o && form.o.content_filters)
                value: form.f.content_filter || "low"
                onPicked: value => Online.setFilter("content_filter", value)
            }
        }
    }
    Text {
        Layout.fillWidth: true
        wrapMode: Text.Wrap
        text: (Settings.data.unsplash_access_key ? "" : "Unsplash needs a free Access Key: Settings → Unsplash.\n") + "An empty search shows Unsplash's Wallpapers topic; user:<name> a photographer's photos. Color and the content filter apply to searches only. Photos are free to use; credit the photographer where you can."
        color: Theme.mutedForeground
        font.family: Theme.font
        font.pixelSize: 13
    }
}
