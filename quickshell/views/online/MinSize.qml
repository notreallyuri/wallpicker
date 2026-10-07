// Minimum size (sizes.rs MinSize): a preset, or any size typed in (shown as
// the choice then).
import QtQuick
import QtQuick.Layouts
import qs.components
import qs.services

RowLayout {
    id: minSize

    /// "WIDTHxHEIGHT", or "" for any.
    property string value
    signal picked(string value)

    readonly property var presets: Online.options ? Online.options.min_sizes.map(o => [o.value, o.label]) : []

    spacing: 8

    Select {
        minWidth: 176
        options: minSize.presets.some(o => o[0] === minSize.value) ? minSize.presets : minSize.presets.concat([[minSize.value, "≥ " + minSize.value.replace("x", "×")]])
        value: minSize.value
        onPicked: value => minSize.picked(value)
    }
    SizeEntry {
        placeholder: "or e.g. 1700x900"
        button: "Set"
        onAdded: size => minSize.picked(size)
    }
}
