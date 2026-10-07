// Settings → Downloads: where wallpapers from the online providers go.
import QtQuick
import QtQuick.Layouts
import qs.components
import qs.services

ColumnLayout {
    id: section

    property var form
    readonly property var d: form ? form.draft : ({})

    spacing: 20

    Field {
        Layout.fillWidth: true
        label: "Download folder"
        description: "Where wallpapers from every online provider are saved, as wallhaven-<id> and konachan-<id> files. Put it inside one of your wallpaper folders to see downloads in the Local tab."
        RowLayout {
            Layout.fillWidth: true
            spacing: 8
            TextField {
                Layout.fillWidth: true
                text: section.d.download_dir || ""
                onTextChanged: if (text !== section.d.download_dir)
                    section.form.set("download_dir", text)
            }
            Btn {
                icon: "folder"
                text: "Choose…"
                onClicked: Files.folder({
                    title: "A folder for downloads",
                    folder: section.d.download_dir || null
                }, path => {
                    if (path)
                        section.form.set("download_dir", path);
                })
            }
        }
    }
}
