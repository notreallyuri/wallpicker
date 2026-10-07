// Duplicates (duplicates.rs): exact copies and the same picture at another
// size, a group at a time; the copy to keep is picked already (the
// largest). Looked for when opened.
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import qs.components
import qs.services

Dialog {
    id: dialog

    /// null while looking; then the groups left.
    property var groups: null
    property string failure: ""

    title: "Duplicates"
    description: "Exact copies and the same picture at another size. The copy to keep is picked already (the largest); click another to keep it instead."
    preferredWidth: 820
    onClosed: Dialogs.duplicates = false
    onOpened: {
        groups = null;
        failure = "";
        Library.duplicates(g => dialog.groups = g, e => dialog.failure = e);
    }

    Connections {
        target: Dialogs
        function onDuplicatesChanged() {
            if (Dialogs.duplicates)
                dialog.open();
        }
    }

    RowLayout {
        visible: dialog.groups === null && dialog.failure === ""
        spacing: 8
        Spinner {
            size: 16
        }
        Text {
            text: "Looking…"
            color: Theme.mutedForeground
            font.family: Theme.font
            font.pixelSize: 14
        }
    }
    Text {
        visible: dialog.failure !== "" || (dialog.groups !== null && dialog.groups.length === 0)
        Layout.fillWidth: true
        wrapMode: Text.Wrap
        text: dialog.failure !== "" ? "Could not look for duplicates: " + dialog.failure : "No duplicates."
        color: dialog.failure !== "" ? "#fd4663" : Theme.mutedForeground
        font.family: Theme.font
        font.pixelSize: 14
    }
    Flickable {
        id: scroller
        visible: dialog.groups !== null && dialog.groups.length > 0
        Layout.fillWidth: true
        Layout.preferredHeight: Math.min(list.implicitHeight, 560)
        contentHeight: list.implicitHeight
        clip: true
        ScrollBar.vertical: ScrollBar {}
        SmoothScroll {
            flickable: scroller
        }
        ColumnLayout {
            id: list
            width: parent.width - 8
            spacing: 16
            Repeater {
                model: dialog.groups || []
                DuplicateGroup {
                    required property var modelData
                    Layout.fillWidth: true
                    info: modelData
                    onDone: first => dialog.groups = dialog.groups.filter(g => g.items[0].path !== first)
                }
            }
        }
    }

    footer: [
        Btn {
            variant: "ghost"
            text: "Close"
            onClicked: dialog.close()
        }
    ]
}
