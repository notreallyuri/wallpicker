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
    /// The monitor being adjusted ("" none).
    property string adjusting: ""
    /// Popovers, menus and dialogs open now, the last on top: Esc closes
    /// the top one (not the picker), and the shortcuts wait while any is.
    property var open: []
    readonly property int popups: open.length
    readonly property bool calm: popups === 0

    function opened(popup) {
        open = open.filter(p => p !== popup).concat([popup]);
    }
    function closed(popup) {
        open = open.filter(p => p !== popup);
    }
    /// Close the top popup; false if there was none.
    function closeTop() {
        if (open.length === 0)
            return false;
        open[open.length - 1].close();
        return true;
    }

    function editTags(path) {
        tagsFor = path;
    }
    function confirmTrash(path) {
        trashFor = path;
    }
    function findDuplicates() {
        duplicates = true;
    }
    function adjust(monitor) {
        adjusting = monitor;
    }
}
