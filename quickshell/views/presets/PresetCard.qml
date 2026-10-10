import QtQuick
import QtQuick.Layouts
import qs.components
import qs.services

Rectangle {
    id: card
    required property var preset
    readonly property var names: Object.keys(preset.monitors)
    readonly property int previewColumns: Math.max(1, Math.min(3, names.length))
    readonly property string mode: preset.span ? (preset.span.monitors.length < names.length ? "Spanned + separate" : "Spanned") : "Separate"
    implicitHeight: body.implicitHeight + 36
    radius: 18
    color: Theme.a(Theme.card, 0.8)
    border.color: Theme.border

    ColumnLayout {
        id: body
        anchors.fill: parent
        anchors.margins: 18
        spacing: 14
        RowLayout {
            Layout.fillWidth: true
            spacing: 8
            Text {
                Layout.fillWidth: true
                Layout.minimumWidth: 0
                text: card.preset.name
                elide: Text.ElideRight
                color: Theme.foreground
                font.family: Theme.font
                font.pixelSize: 17
                font.bold: true
            }
            PresetButton {
                id: more
                icon: "ellipsis"
                variant: "ghost"
                small: true
                tip: "Manage " + card.preset.name
                Accessible.name: tip
                enabled: !Presets.busy
                onClicked: menu.openAt(more, more.width - menu.width, more.height + 6)
            }
        }
        Flow {
            id: previews
            Layout.fillWidth: true
            Layout.preferredHeight: childrenRect.height
            spacing: 10
            Repeater {
                model: card.names
                PresetMonitor {
                    required property string modelData
                    width: Math.max(1, Math.floor((previews.width - (card.previewColumns - 1) * previews.spacing) / card.previewColumns))
                    monitor: modelData
                    wallpaper: card.preset.monitors[modelData]
                    missing: (card.preset.missing_files || []).indexOf(wallpaper.wallpaper) >= 0
                }
            }
        }
        RowLayout {
            Layout.fillWidth: true
            spacing: 8
            Badge { text: card.mode; soft: true }
            Text {
                Layout.fillWidth: true
                text: card.names.length + (card.names.length === 1 ? " monitor" : " monitors")
                color: Theme.mutedForeground
                font.family: Theme.font
                font.pixelSize: 12
            }
            PresetButton {
                text: "Apply"
                variant: "primary"
                small: true
                enabled: !Presets.busy
                onClicked: Presets.run("apply", {id: card.preset.id})
            }
        }
    }
    PresetMenu { id: menu; preset: card.preset }
}
