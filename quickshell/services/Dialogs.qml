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
    /// The Span dialog: the picture ("" closed), and whether it's the span
    /// already shown (changes go on the monitors as they're made) or a new
    /// one (spanned with the dialog's Span button).
    property string spanPath: ""
    property bool spanShown: false
    /// The Online provider's filters.
    property bool onlineFilters: false
    /// The Settings section shown ("" closed).
    property string settings: ""
    /// The online wallpaper whose details are shown (null none).
    property var details: null
    /// Popovers, menus and dialogs open now, the last on top: Esc closes
    /// the top one (not the picker), and the shortcuts wait while any is.
    property var open: []
    readonly property int popups: open.length
    readonly property bool calm: popups === 0

    /// Still there: a popup destroyed without closing (while open or
    /// closing) is left in the list as a dead reference, and calling it
    /// throws: the picker couldn't close (Esc closed "it", then nothing).
    function alive(p) {
        return !!p && typeof p.close === "function";
    }
    function opened(popup) {
        open = open.filter(p => p !== popup && alive(p)).concat([popup]);
    }
    function closed(popup) {
        open = open.filter(p => p !== popup && alive(p));
    }
    /// Close the top popup; false if there was none.
    function closeTop() {
        open = open.filter(alive);
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
    /// Choose the monitors for `path` and arrange them, then span it.
    function spanAcross(path) {
        spanShown = false;
        spanPath = path;
    }
    /// Adjust the span shown: its look, its monitors and their places.
    function adjustSpan() {
        if (!Monitors.spanState)
            return;
        spanShown = true;
        spanPath = Monitors.spanState.path;
    }
    function openSettings(section) {
        settings = section || "general";
    }
    function showDetails(w) {
        details = w;
    }
}
