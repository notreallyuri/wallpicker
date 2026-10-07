// The interface's remembered choices (the Local sort and layout, the tag
// row's expansion, the monitor drawer...), in a small JSON file next to
// wallpicker's state: $XDG_STATE_HOME/wallpicker/ui.json. Set a property
// and it's saved.
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
    property alias monitorsOpen: data.monitorsOpen
    /// "system", "light" or "dark".
    property alias darkMode: data.darkMode
    /// A color theme (services/Themes.js).
    property alias colorTheme: data.colorTheme
    /// Settings → Performance: the frosted glass, the wallpaper behind the
    /// picker, animations.
    property alias glass: data.glass
    property alias backdrop: data.backdrop
    property alias motion: data.motion
    /// The floating panel's size, "WIDTHxHEIGHT" (Settings → Appearance).
    property alias panelSize: data.panelSize

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
            property bool monitorsOpen: false
            property string darkMode: "system"
            property string colorTheme: "blossom"
            property bool glass: true
            property bool backdrop: true
            property bool motion: true
            property string panelSize: "1366x768"
        }
    }
}
