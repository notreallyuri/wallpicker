// Wallhaven's exact resolutions (sizes.rs ExactSizes): your monitors' sizes
// first (named), the presets, any other picked size, as chips; several at
// once; and a field for another.
import QtQuick
import QtQuick.Layouts
import qs.services

ColumnLayout {
    id: sizes

    property var values: []
    signal changed(var values)

    readonly property var options: {
        const out = [];
        const pretty = s => s.replace("x", "×");
        for (const m of Monitors.list) {
            const size = m.width + "x" + m.height;
            const had = out.find(o => o.value === size);
            if (had)
                had.label += ", " + m.name;
            else
                out.push({
                    value: size,
                    label: pretty(size) + " · " + m.name
                });
        }
        const presets = Online.options ? Online.options.wallhaven.resolutions : [];
        for (const p of presets.concat(values.map(v => ({
                    value: v,
                    label: pretty(v)
                }))))
            if (!out.some(o => o.value === p.value))
                out.push(p);
        return out;
    }

    spacing: 8

    ChipChoice {
        Layout.fillWidth: true
        options: sizes.options
        values: sizes.values
        onChanged: values => sizes.changed(values)
    }
    SizeEntry {
        placeholder: "other, e.g. 1024x768"
        button: "Add"
        onAdded: size => {
            if (!sizes.values.includes(size))
                sizes.changed(sizes.values.concat([size]));
        }
    }
}
