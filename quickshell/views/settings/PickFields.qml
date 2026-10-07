// What a slideshow set picks from (the day's, or the night's): favorites
// only, a color, tags (all or any of them), a folder. `path` is where the
// set is in the settings ("slideshow", "slideshow.night").
import QtQuick
import QtQuick.Layouts
import qs.components
import qs.services

ColumnLayout {
    id: set

    property var form
    property string path
    readonly property var v: form ? (form.get(path) || {}) : ({})
    readonly property var colors: form && form.options ? [["", "Any color"]].concat(form.options.hues.concat(form.options.tones).map(c => [c.value, c.label])) : []

    function put(key, value) {
        form.set(path + "." + key, value);
    }

    spacing: 16

    ToggleField {
        label: "Only favorites"
        checked: !!set.v.favorites
        onToggled: set.put("favorites", !set.v.favorites)
    }
    Field {
        label: "Only with the color"
        Select {
            minWidth: 180
            options: set.colors
            value: set.v.color || ""
            onPicked: c => set.put("color", c || null)
        }
    }
    Field {
        visible: Library.tagCounts.length > 0
        Layout.fillWidth: true
        label: "Only with these tags"
        description: "None picked: any wallpaper."
        Flow {
            Layout.fillWidth: true
            spacing: 6
            Repeater {
                model: Library.tagCounts.map(t => t.name)
                Chip {
                    required property string modelData
                    readonly property var picked: set.v.tags || []
                    text: modelData
                    on: picked.includes(modelData)
                    onClicked: set.put("tags", on ? picked.filter(t => t !== modelData) : picked.concat([modelData]))
                }
            }
        }
        ToggleField {
            visible: (set.v.tags || []).length > 1
            label: "Any of them (off: all of them)"
            checked: !!set.v.any
            onToggled: set.put("any", !set.v.any)
        }
    }
    Field {
        Layout.fillWidth: true
        label: "Only in folder"
        RowLayout {
            Layout.fillWidth: true
            spacing: 8
            TextField {
                Layout.fillWidth: true
                placeholder: "Any folder"
                text: set.v.folder || ""
                onTextChanged: if ((text.trim() || null) !== (set.v.folder || null))
                    set.put("folder", text.trim() || null)
            }
            Btn {
                icon: "folder"
                text: "Choose…"
                onClicked: Files.folder({
                    title: "The slideshow's folder",
                    folder: set.v.folder || null
                }, p => {
                    if (p)
                        set.put("folder", p);
                })
            }
        }
    }
}
