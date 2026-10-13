import QtQuick
import QtQuick.Layouts
import qs.components
import qs.services

Dialog {
    id: dialog
    property var edit: null
    readonly property string action: edit ? edit.action : "save"
    readonly property bool named: action === "save" || action === "rename"
    title: action === "save" ? "Save current setup" : action === "rename" ? "Rename preset" : action === "update" ? "Replace saved setup?" : "Delete preset?"
    description: action === "update" ? "Replace this preset with the wallpapers and adjustments currently shown on your monitors." : action === "delete" ? "Remove the saved preset. Your current wallpapers stay as they are." : "Save wallpaper assignments and adjustments for your monitors."
    onClosed: Presets.editing = null
    Connections {
        target: Presets
        function onEditingChanged() {
            if (!Presets.editing || Presets.editing.action === "reassign") return;
            dialog.edit = Presets.editing;
            Presets.error = "";
            name.text = dialog.edit.preset ? dialog.edit.preset.name : "";
            dialog.open();
            if (dialog.named) Qt.callLater(() => name.forceFocus());
        }
    }
    TextField {
        id: name
        visible: dialog.named
        Layout.fillWidth: true
        placeholder: "Preset name"
        onAccepted: dialog.submit()
    }
    Text {
        visible: !dialog.named
        Layout.fillWidth: true
        text: dialog.edit && dialog.edit.preset ? dialog.edit.preset.name : ""
        wrapMode: Text.Wrap
        color: Theme.foreground
        font.family: Theme.font
    }
    Text {
        visible: Presets.error !== ""
        Layout.fillWidth: true
        text: Presets.error
        wrapMode: Text.Wrap
        color: "#fd4663"
        font.family: Theme.font
    }
    function submit() {
        if (Presets.busy || (named && !name.text.trim())) return;
        const pending = edit;
        Presets.run(action, {id: edit.preset ? edit.preset.id : "", name: name.text}, () => {
            if (Presets.editing === pending) dialog.close();
        });
    }
    footer: [
        PresetButton { text: "Cancel"; variant: "ghost"; enabled: !Presets.busy; onClicked: dialog.close() },
        PresetButton {
            text: Presets.busy ? "Working…" : dialog.action === "update" ? "Replace setup" : dialog.action === "delete" ? "Delete preset" : "Save"
            variant: dialog.action === "delete" ? "destructive" : "primary"
            enabled: !Presets.busy && (!dialog.named || name.text.trim().length > 0)
            onClicked: dialog.submit()
        }
    ]
}
