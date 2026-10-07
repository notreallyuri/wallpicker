// Settings → Unsplash: the Access Key every request needs.
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
        label: "Access Key"
        description: "Free: at unsplash.com/developers, register an application (New Application) and copy its Access Key. A new one allows 50 requests an hour, plenty for browsing. Stored in your config file, readable only by you."
        TextField {
            Layout.fillWidth: true
            Layout.maximumWidth: 420
            placeholder: "Needed for Unsplash"
            echoMode: TextInput.Password
            text: section.d.unsplash_access_key || ""
            onTextChanged: if ((text.trim() || null) !== (section.d.unsplash_access_key || null))
                section.form.set("unsplash_access_key", text.trim() || null)
        }
    }
    Note {
        Layout.fillWidth: true
        text: "Each photo's photographer is credited on its tile and in its details, with a link to their Unsplash page, as Unsplash asks. Downloads are reported to Unsplash (it counts them for the photographer)."
    }
}
