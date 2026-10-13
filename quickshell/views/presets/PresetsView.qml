import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import qs.components
import qs.services

Item {
    id: view
    readonly property var matches: {
        const term = search.text.trim().toLowerCase();
        return Presets.items.filter(p => !term || p.name.toLowerCase().indexOf(term) >= 0);
    }
    Component.onCompleted: Presets.reload()
    onMatchesChanged: Qt.callLater(() => scrolling.by(0))
    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 20
        spacing: 16
        RowLayout {
            Layout.fillWidth: true
            ColumnLayout {
                Layout.fillWidth: true
                spacing: 4
                Text {
                    Layout.fillWidth: true
                    Layout.minimumWidth: 0
                    elide: Text.ElideRight
                    text: "Wallpaper presets"
                    color: Theme.foreground
                    font.family: Theme.font
                    font.pixelSize: 20
                    font.bold: true
                }
                Text {
                    Layout.fillWidth: true
                    Layout.minimumWidth: 0
                    wrapMode: Text.Wrap
                    text: "Your wallpapers and adjustments, saved together."
                    color: Theme.mutedForeground
                    font.family: Theme.font
                    font.pixelSize: 13
                }
            }
            PresetButton {
                text: "Save current setup…"
                variant: "primary"
                enabled: !Presets.busy
                onClicked: { Presets.error = ""; Presets.editing = {action: "save"}; }
            }
        }
        RowLayout {
            visible: Presets.items.length > 0
            Layout.fillWidth: true
            TextField {
                id: search
                Layout.fillWidth: true
                Layout.maximumWidth: 360
                placeholder: "Find a preset…"
                icon: "search"
                onTextChanged: scrolling.by(-grid.contentY)
            }
            Item { Layout.fillWidth: true }
            Text {
                text: (view.matches.length === Presets.items.length ? "" : view.matches.length + " of ") + Presets.items.length + (Presets.items.length === 1 ? " preset" : " presets")
                color: Theme.mutedForeground
                font.family: Theme.font
                font.pixelSize: 12
            }
        }
        Text {
            Layout.fillWidth: true
            visible: Presets.error !== "" || Presets.report !== ""
            text: Presets.error || Presets.report
            wrapMode: Text.Wrap
            color: Presets.error ? "#fd4663" : Theme.mutedForeground
            font.family: Theme.font
            font.pixelSize: 13
        }
        Item {
            visible: view.matches.length === 0
            Layout.fillWidth: true
            Layout.fillHeight: true
            Column {
                anchors.centerIn: parent
                spacing: 12
                Icon { anchors.horizontalCenter: parent.horizontalCenter; name: "monitor"; size: 32; color: Theme.mutedForeground }
                Text {
                    anchors.horizontalCenter: parent.horizontalCenter
                    text: Presets.items.length ? "No matching presets" : "Save your first setup"
                    color: Theme.foreground
                    font.family: Theme.font
                    font.pixelSize: 18
                    font.bold: true
                }
                Text {
                    anchors.horizontalCenter: parent.horizontalCenter
                    text: Presets.items.length ? "Try another name." : "Arrange your wallpapers, then save the setup above."
                    color: Theme.mutedForeground
                    font.family: Theme.font
                    font.pixelSize: 13
                }
                PresetButton {
                    visible: search.text !== ""
                    anchors.horizontalCenter: parent.horizontalCenter
                    text: "Clear search"
                    variant: "ghost"
                    onClicked: search.text = ""
                }
            }
        }
        FadeBottom {
            visible: view.matches.length > 0
            Layout.fillWidth: true
            Layout.fillHeight: true
            flickable: grid
            Flickable {
                id: grid
                anchors.fill: parent
                contentHeight: cards.implicitHeight + 12
                boundsBehavior: Flickable.StopAtBounds
                flickableDirection: Flickable.VerticalFlick
                clip: true
                ScrollBar.vertical: ScrollBar {}
                SmoothScroll { id: scrolling; flickable: grid }
                GridLayout {
                    id: cards
                    width: grid.width - 16
                    columns: Math.max(1, Math.min(3, Math.floor((width + 16) / 416)))
                    columnSpacing: 16
                    rowSpacing: 16
                    Repeater {
                        model: view.matches
                        PresetCard {
                            required property var modelData
                            Layout.fillWidth: true
                            Layout.fillHeight: true
                            Layout.minimumWidth: 0
                            preset: modelData
                        }
                    }
                }
            }
        }
    }
    Shortcut {
        enabled: Dialogs.calm && Presets.items.length > 0
        sequence: "Ctrl+F"
        onActivated: search.forceFocus()
    }
    PresetDialog {}
    ReassignDialog {}
}
