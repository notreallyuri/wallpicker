// Settings → Wallhaven: the API key (NSFW results, your collections) and
// your username (opening a collection needs it).
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
        label: "API key"
        description: "Needed for NSFW results and your collections. Find it at wallhaven.cc → Settings → Account. Stored in your config file, readable only by you."
        TextField {
            Layout.fillWidth: true
            Layout.maximumWidth: 420
            placeholder: "Optional"
            echoMode: TextInput.Password
            text: section.d.wallhaven_api_key || ""
            onTextChanged: if ((text.trim() || null) !== (section.d.wallhaven_api_key || null))
                section.form.set("wallhaven_api_key", text.trim() || null)
        }
    }
    Field {
        Layout.fillWidth: true
        label: "Username"
        description: "Your Wallhaven username, for the Collections tab: opening a collection needs it (the API doesn't say whose key it is)."
        TextField {
            Layout.fillWidth: true
            Layout.maximumWidth: 420
            placeholder: "Optional"
            text: section.d.wallhaven_username || ""
            onTextChanged: if ((text.trim() || null) !== (section.d.wallhaven_username || null))
                section.form.set("wallhaven_username", text.trim() || null)
        }
    }
}
