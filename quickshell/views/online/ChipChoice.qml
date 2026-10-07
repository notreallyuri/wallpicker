// Several of a list at once, as chips (sizes.rs' toggle groups): exact
// resolutions, aspect ratios. `options`: [{value, label}]; `values` the
// picked ones; none picked means any.
import QtQuick
import qs.components

Flow {
    id: choice

    property var options: []
    property var values: []
    signal changed(var values)

    spacing: 6

    Repeater {
        model: choice.options
        Chip {
            required property var modelData
            text: modelData.label
            on: choice.values.includes(modelData.value)
            onClicked: choice.changed(on ? choice.values.filter(v => v !== modelData.value) : choice.values.concat([modelData.value]))
        }
    }
}
