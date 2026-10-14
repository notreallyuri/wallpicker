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
    readonly property var connected: names.filter(n => Monitors.list.some(m => m.name === n))
    /// On screen now: every connected monitor of it shows its wallpaper.
    readonly property bool current: connected.length > 0 && connected.every(n => Monitors.current[n] === preset.monitors[n].wallpaper)
    implicitHeight: body.implicitHeight + 32
    radius: Theme.radiusXl
    color: Theme.a(Theme.card, 0.8)
    // The tiles' hairline; outlined when it's the one on screen.
    border.width: current ? 2 : 1
    border.color: current ? Theme.a(Theme.primary, 0.45) : Theme.a(Theme.foreground, 0.05)

    ColumnLayout {
        id: body
        anchors.fill: parent
        anchors.margins: 16
        spacing: 12
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
                font.pixelSize: 16
                font.weight: Font.ExtraBold
            }
            Badge {
                visible: card.current
                icon: "check"
                text: "On now"
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
                elide: Text.ElideRight
                readonly property int away: card.names.length - card.connected.length
                text: card.names.length + (card.names.length === 1 ? " monitor" : " monitors") + (away > 0 ? " · " + away + " not connected" : "")
                color: Theme.mutedForeground
                font.family: Theme.font
                font.pixelSize: 12
            }
            PresetButton {
                text: card.current ? "Apply again" : "Apply"
                variant: card.current ? "outline" : "primary"
                small: true
                enabled: !Presets.busy
                onClicked: Presets.run("apply", {id: card.preset.id})
            }
        }
    }
    PresetMenu { id: menu; preset: card.preset }
}
