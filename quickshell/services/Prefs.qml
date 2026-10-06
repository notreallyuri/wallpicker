// The interface's remembered choices (the Local sort and layout, the tag
// row's expansion...), in a small JSON file next to wallpicker's state:
// $XDG_STATE_HOME/wallpicker/ui.json. Set a property and it's saved.
pragma Singleton
import QtQuick
import Quickshell
import Quickshell.Io

Singleton {
    id: prefs

    property alias localSort: data.localSort
    property alias localLayout: data.localLayout
    property alias tagsExpanded: data.tagsExpanded
    property alias tagsMatchAll: data.tagsMatchAll

    FileView {
        id: file
        path: (Quickshell.env("XDG_STATE_HOME") || Quickshell.env("HOME") + "/.local/state") + "/wallpicker/ui.json"
        // Changes are written as they're made (atomically).
        onAdapterUpdated: writeAdapter()
        // None yet: write the defaults.
        onLoadFailed: error => {
            if (error === FileViewError.FileNotFound)
                writeAdapter();
        }

        JsonAdapter {
            id: data
            property string localSort: "newest"
            property string localLayout: "flat"
            property bool tagsExpanded: false
            property bool tagsMatchAll: true
        }
    }
}
