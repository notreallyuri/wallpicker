// Wallhaven's filters (ui/src/views/online/wallhaven.rs): sorting (and the
// toplist's period), minimum size, exact resolutions, aspect ratios,
// categories, purity. Each change is kept (services/Online saves it).
import QtQuick
import QtQuick.Layouts
import qs.components
import qs.services

ColumnLayout {
    id: form

    readonly property var f: Online.filters.wallhaven || {}
    readonly property var o: Online.options ? Online.options.wallhaven : null
    readonly property var pairs: list => list ? list.map(x => [x.value, x.label]) : []

    function set(name, value) {
        Online.setFilter(name, value);
    }
    /// One of an object filter's switches (categories, purity).
    function flip(name, key) {
        const next = Object.assign({}, f[name]);
        next[key] = !next[key];
        set(name, next);
    }

    spacing: 20

    RowLayout {
        spacing: 16
        Field {
            label: "Sort by"
            Select {
                minWidth: 176
                options: form.pairs(form.o ? form.o.sorts : null)
                value: form.f.sorting || ""
                onPicked: value => form.set("sorting", value)
            }
        }
        Field {
            visible: form.f.sorting === "toplist"
            label: "Top over"
            Select {
                minWidth: 176
                options: form.pairs(form.o ? form.o.ranges : null)
                value: form.f.top_range || "1M"
                onPicked: value => form.set("top_range", value)
            }
        }
    }
    Field {
        Layout.fillWidth: true
        label: "Minimum size"
        description: (form.f.resolutions || []).length > 0 && form.f.atleast ? "Not used while exact resolutions are picked (Wallhaven takes one or the other)." : ""
        MinSize {
            value: form.f.atleast || ""
            onPicked: value => form.set("atleast", value || null)
        }
    }
    Field {
        Layout.fillWidth: true
        label: "Exact resolutions"
        description: "Any number; none means any. Your monitors' sizes come first."
        ExactSizes {
            Layout.fillWidth: true
            values: form.f.resolutions || []
            onChanged: values => form.set("resolutions", values)
        }
    }
    Field {
        Layout.fillWidth: true
        label: "Aspect ratios"
        ChipChoice {
            Layout.fillWidth: true
            options: form.o ? form.o.ratios : []
            values: form.f.ratios || []
            onChanged: values => form.set("ratios", values)
        }
    }
    Field {
        label: "Categories"
        RowLayout {
            spacing: 20
            Repeater {
                model: [["general", "General"], ["anime", "Anime"], ["people", "People"]]
                Check {
                    required property var modelData
                    text: modelData[1]
                    checked: !!(form.f.categories && form.f.categories[modelData[0]])
                    onToggled: form.flip("categories", modelData[0])
                }
            }
        }
    }
    Field {
        label: "Purity"
        description: Online.hasApiKey ? "" : "NSFW needs an API key (Settings → Wallhaven)."
        RowLayout {
            spacing: 20
            Repeater {
                model: [["sfw", "SFW"], ["sketchy", "Sketchy"], ["nsfw", "NSFW"]]
                Check {
                    required property var modelData
                    enabled: modelData[0] !== "nsfw" || Online.hasApiKey
                    text: modelData[1]
                    checked: !!(form.f.purity && form.f.purity[modelData[0]])
                    onToggled: form.flip("purity", modelData[0])
                }
            }
        }
    }
}
