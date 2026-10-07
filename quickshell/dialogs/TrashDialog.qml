// Move a wallpaper to the trash? (trash.rs): restorable from the file
// manager; its favorite and tags go with it; if a monitor shows it, it stays
// until something else is applied but won't come back after a restart.
import QtQuick
import QtQuick.Layouts
import qs.components
import qs.services

Dialog {
    id: dialog

    readonly property string path: Dialogs.trashFor
    readonly property bool marked: path !== "" && (Library.favorite(path) || Library.tags(path).length > 0)
    readonly property var shownOn: path !== "" ? Monitors.showing(path) : []
    property bool working: false

    title: "Move to trash?"
    description: path.split("/").pop()
    onClosed: Dialogs.trashFor = ""

    Connections {
        target: Dialogs
        function onTrashForChanged() {
            if (Dialogs.trashFor !== "")
                dialog.open();
        }
    }

    Note {
        text: "You can restore it from your file manager's trash."
    }
    Note {
        visible: dialog.marked
        text: "Its favorite and tags are removed with it."
    }
    Note {
        visible: dialog.shownOn.length > 0
        text: "It's on " + dialog.shownOn.join(", ") + " right now: it stays there until you apply something else, but won't come back after a restart."
    }

    footer: [
        Btn {
            variant: "ghost"
            text: "Cancel"
            onClicked: dialog.close()
        },
        Btn {
            variant: "destructive"
            text: "Move to trash"
            opacity: dialog.working ? 0.5 : 1
            onClicked: {
                if (dialog.working)
                    return;
                dialog.working = true;
                Library.trash(dialog.path, () => {
                    dialog.working = false;
                    dialog.close();
                });
            }
        }
    ]

    component Note: Text {
        Layout.fillWidth: true
        wrapMode: Text.Wrap
        color: Theme.mutedForeground
        font.family: Theme.font
        font.pixelSize: 14
    }
}
