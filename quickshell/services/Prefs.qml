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
        // Read (or found missing) yet: until then a change would be written
        // over a load in progress.
        property bool ready: false
        // Changes are written as they're made (atomically).
        onAdapterUpdated: if (ready)
            writeAdapter()
        onLoaded: ready = true
        // None yet (a first run): write the defaults, once the failed load
        // is done (writing from inside its handler drops one of the two).
        printErrors: false
        onLoadFailed: error => {
            ready = true;
            if (error === FileViewError.FileNotFound)
                Qt.callLater(writeAdapter);
            else
                console.warn("wallpicker: can't read", path, FileViewError.toString(error));
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
