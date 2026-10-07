// A fit for each monitor of its own: "Same as above" (the general fit), or
// one of the backend's fits.
import QtQuick
import QtQuick.Layouts
import qs.components
import qs.services

ColumnLayout {
    id: fits

    /// monitor -> fit; a monitor without one follows the general fit.
    property var value: ({})
    /// The backend's fits: [{value, label}] ("" its default).
    property var choices: []
    signal changed(var value)

    spacing: 6

    Repeater {
        model: Monitors.list
        RowLayout {
            id: row
            required property var modelData
            spacing: 12
            Text {
                Layout.preferredWidth: 150
                text: row.modelData.name + "  <font color=\"" + Theme.mutedForeground + "\">" + row.modelData.width + "×" + row.modelData.height + "</font>"
                textFormat: Text.StyledText
                color: Theme.foreground
                font.family: Theme.font
                font.pixelSize: 13
            }
            Select {
                minWidth: 220
                options: [["", "Same as above"]].concat(fits.choices.filter(c => c.value !== "").map(c => [c.value, c.label]))
                value: fits.value[row.modelData.name] || ""
                onPicked: v => {
                    const next = Object.assign({}, fits.value);
                    if (v)
                        next[row.modelData.name] = v;
                    else
                        delete next[row.modelData.name];
                    fits.changed(next);
                }
            }
        }
    }
}
