// A wallpaper's tags (tags.rs's TagsBody): the ones it has (each removable),
// a field to add one, and the tags used elsewhere that match what's typed.
import QtQuick
import QtQuick.Layouts
import qs.components
import qs.services

Dialog {
    id: dialog

    readonly property string path: Dialogs.tagsFor
    readonly property var current: path !== "" ? Library.tags(path) : []
    // Tags used elsewhere that it doesn't have, matching what's typed.
    readonly property var suggestions: {
        const typed = field.text.trim().toLowerCase();
        const mine = current.map(t => t.toLowerCase());
        const seen = {};
        const all = [];
        for (const p in Library.entries)
            for (const t of (Library.entries[p].tags || [])) {
                const k = t.toLowerCase();
                if (!mine.includes(k) && k.includes(typed) && !seen[k]) {
                    seen[k] = true;
                    all.push(t);
                }
            }
        return all.sort((a, b) => a.toLowerCase().localeCompare(b.toLowerCase())).slice(0, 12);
    }

    /// As core's normalize_tag: spaces collapsed, 1 to 64 characters.
    function normalized(raw) {
        const tag = raw.split(/\s+/).filter(w => w !== "").join(" ");
        return tag.length > 0 && tag.length <= 64 && !/[\u0000-\u001f\u007f]/.test(tag) ? tag : "";
    }
    function add(raw) {
        const tag = normalized(raw);
        if (tag === "")
            return;
        field.text = "";
        Library.addTag(path, tag);
    }

    title: "Tags"
    description: path.split("/").pop()
    onClosed: Dialogs.tagsFor = ""
    onOpened: field.forceFocus()

    Connections {
        target: Dialogs
        function onTagsForChanged() {
            if (Dialogs.tagsFor !== "")
                dialog.open();
        }
    }

    Flow {
        Layout.fillWidth: true
        spacing: 6
        Text {
            visible: dialog.current.length === 0
            text: "No tags yet."
            color: Theme.mutedForeground
            font.family: Theme.font
            font.pixelSize: 14
        }
        Repeater {
            model: dialog.current
            Rectangle {
                id: chip
                required property string modelData
                width: row.implicitWidth + 16
                height: 26
                radius: 13
                color: Theme.secondary
                Row {
                    id: row
                    anchors.verticalCenter: parent.verticalCenter
                    x: 10
                    spacing: 4
                    Text {
                        anchors.verticalCenter: parent.verticalCenter
                        text: chip.modelData
                        color: Theme.foreground
                        font.family: Theme.font
                        font.pixelSize: 12
                    }
                    Rectangle {
                        width: 18
                        height: 18
                        radius: 9
                        color: removeMouse.containsMouse ? Theme.a(Theme.background, 0.6) : "transparent"
                        Icon {
                            anchors.centerIn: parent
                            name: "x"
                            size: 12
                        }
                        MouseArea {
                            id: removeMouse
                            anchors.fill: parent
                            hoverEnabled: true
                            cursorShape: Qt.PointingHandCursor
                            onClicked: Library.removeTag(dialog.path, chip.modelData)
                        }
                    }
                }
            }
        }
    }
    RowLayout {
        Layout.fillWidth: true
        spacing: 8
        TextField {
            id: field
            Layout.fillWidth: true
            placeholder: "Add a tag"
            onAccepted: dialog.add(text)
        }
        Btn {
            variant: "primary"
            text: "Add"
            opacity: dialog.normalized(field.text) !== "" ? 1 : 0.5
            onClicked: dialog.add(field.text)
        }
    }
    Flow {
        visible: dialog.suggestions.length > 0
        Layout.fillWidth: true
        spacing: 6
        Repeater {
            model: dialog.suggestions
            Btn {
                required property string modelData
                implicitHeight: 26
                text: "+ " + modelData
                onClicked: dialog.add(modelData)
            }
        }
    }

    footer: [
        Btn {
            variant: "ghost"
            text: "Done"
            onClicked: dialog.close()
        }
    ]
}
