// A list of folders (the wallpaper folders): each with its remove button;
// Add… picks one in the system's dialog, or one can be typed (absolute, or
// starting with ~).
import QtQuick
import QtQuick.Layouts
import qs.components
import qs.services

ColumnLayout {
    id: list

    property var folders: []
    signal changed(var folders)

    function add(path) {
        const p = path.trim().replace(/\/+$/, "");
        if (p === "" || folders.includes(p))
            return;
        changed(folders.concat([p]));
    }

    spacing: 6

    Repeater {
        model: list.folders
        Rectangle {
            id: row
            required property string modelData
            Layout.fillWidth: true
            implicitHeight: 36
            radius: 12
            color: Theme.a(Theme.input, 0.25)
            Text {
                x: 12
                width: parent.width - 56
                anchors.verticalCenter: parent.verticalCenter
                text: row.modelData
                elide: Text.ElideMiddle
                color: Theme.foreground
                font.family: "monospace"
                font.pixelSize: 12
            }
            Btn {
                anchors.right: parent.right
                anchors.rightMargin: 2
                anchors.verticalCenter: parent.verticalCenter
                variant: "ghost"
                small: true
                icon: "x"
                tip: "Stop looking here"
                onClicked: list.changed(list.folders.filter(f => f !== row.modelData))
            }
        }
    }
    RowLayout {
        Layout.fillWidth: true
        spacing: 8
        Btn {
            icon: "folder"
            text: "Add…"
            onClicked: Files.folder({
                title: "A folder of wallpapers"
            }, path => {
                if (path)
                    list.add(path);
            })
        }
        TextField {
            id: typed
            Layout.fillWidth: true
            placeholder: "or type one: ~/Pictures/wallpapers"
            onAccepted: {
                list.add(text);
                text = "";
            }
        }
    }
}
