// Which tab the picker shows: Local, Online, Collections or Presets. Anything can
// switch it (a tag in Collections' details searches on Online), and
// `wallpicker --open PLACE [IMAGE]` sends it somewhere (go).
pragma Singleton
import QtQuick
import Quickshell

Singleton {
    readonly property int local: 0
    readonly property int online: 1
    readonly property int collections: 2
    readonly property int presets: 3

    property int current: local

    /// A tab (local, online, collections, presets), or a dialog over Local
    /// (settings, duplicates, tags for `path`).
    function go(place, path) {
        switch (place) {
        case "online":
            current = online;
            break;
        case "presets":
            current = presets;
            break;
        case "collections":
            current = collections;
            break;
        case "settings":
            Dialogs.openSettings();
            break;
        case "duplicates":
            current = local;
            Dialogs.findDuplicates();
            break;
        // "tags": what the details were before (the plugin's Edit tags).
        case "details":
        case "tags":
            current = local;
            if (path)
                Dialogs.localDetails(path);
            break;
        default:
            current = local;
        }
    }
}
