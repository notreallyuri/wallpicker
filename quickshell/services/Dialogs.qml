// Which dialog is open, for whom: anything can ask (a tile's menu, the ⋯
// menu, a shortcut); the picker hosts the dialogs once (windows/Picker.qml).
pragma Singleton
import QtQuick
import Quickshell

Singleton {
    /// The wallpaper whose tags are being edited ("" none).
    property string tagsFor: ""
    /// The wallpaper asked to be moved to the trash ("" none).
    property string trashFor: ""
    property bool duplicates: false

    function editTags(path) {
        tagsFor = path;
    }
    function confirmTrash(path) {
        trashFor = path;
    }
    function findDuplicates() {
        duplicates = true;
    }
}
