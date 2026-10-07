// Local's color filter (colors.rs): a button (a little palette, or the
// chosen color; a spinner while the colors are read) opening the swatches:
// the eight hues, then Dark, Light, Warm and Muted. Picking one shows the
// wallpapers with it, the most of it first; picking it again, or "Any
// color", clears it.
import QtQuick
import QtQuick.Layouts
import qs.components
import qs.services

Btn {
    id: picker

    required property LocalModel local

    // Keys as the backend knows them (core's colors::HUES and TONES).
    readonly property var hues: [["red", "Red", "#ef4444"], ["orange", "Orange", "#f97316"], ["yellow", "Yellow", "#eab308"], ["green", "Green", "#22c55e"], ["teal", "Teal", "#14b8a6"], ["blue", "Blue", "#3b82f6"], ["purple", "Purple", "#a855f7"], ["pink", "Pink", "#ec4899"]]
    readonly property var tones: [["dark", "Dark", "#1c1917"], ["light", "Light", "#f5f5f4"], ["warm", "Warm", "#f59e0b"], ["muted", "Muted", "#d6d3d1"]]
    readonly property var chosen: hues.concat(tones).find(c => c[0] === local.color)

    function pick(key) {
        local.color = local.color === key ? "" : key;
        swatches.close();
    }

    // One width whatever is chosen.
    minWidth: 112
    text: chosen ? chosen[1] : "Color"
    tip: "Show the wallpapers with a color"
    onClicked: swatches.open()

    // Before the label: three dots, or the chosen color.
    leading: [
        Row {
            visible: !picker.chosen
            spacing: -5
            Repeater {
                model: ["#ef4444", "#eab308", "#3b82f6"]
                Rectangle {
                    required property string modelData
                    width: 12
                    height: 12
                    radius: 6
                    color: modelData
                    border.width: 2
                    border.color: Theme.card
                }
            }
        },
        Rectangle {
            visible: !!picker.chosen
            width: 14
            height: 14
            radius: 7
            color: picker.chosen ? picker.chosen[2] : "transparent"
            border.width: 2
            border.color: Theme.a(Theme.foreground, 0.15)
        }
    ]

    Spinner {
        visible: picker.local.colorLoading
        size: 14
    }

    Popover {
        id: swatches
        parent: picker
        y: picker.height + 6
        width: 288
        padding: 12

        contentItem: ColumnLayout {
            spacing: 8
            GridLayout {
                columns: 4
                columnSpacing: 8
                rowSpacing: 8
                Repeater {
                    model: picker.hues
                    Swatch {
                        required property var modelData
                        Layout.fillWidth: true
                        key: modelData[0]
                        label: modelData[1]
                        swatch: modelData[2]
                        on: picker.local.color === modelData[0]
                        onPicked: picker.pick(modelData[0])
                    }
                }
            }
            Rectangle {
                Layout.fillWidth: true
                implicitHeight: 2
                color: Theme.a(Theme.border, 0.6)
            }
            GridLayout {
                columns: 2
                columnSpacing: 8
                rowSpacing: 8
                Repeater {
                    model: picker.tones
                    Swatch {
                        required property var modelData
                        Layout.fillWidth: true
                        compact: true
                        key: modelData[0]
                        label: modelData[1]
                        swatch: modelData[2]
                        on: picker.local.color === modelData[0]
                        onPicked: picker.pick(modelData[0])
                    }
                }
            }
            Btn {
                visible: picker.local.color !== ""
                Layout.fillWidth: true
                variant: "ghost"
                text: "Any color"
                onClicked: picker.pick("")
            }
            Text {
                text: "The most of the color first."
                color: Theme.mutedForeground
                font.family: Theme.font
                font.pixelSize: 11
            }
        }
    }
}
