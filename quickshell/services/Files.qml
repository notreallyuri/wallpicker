// The system's file dialogs (the XDG portal, through `serve`): save a file,
// open one, pick a folder; `done(path)`, or `done(null)` when cancelled.
// The floating panel is an overlay above every window, so it steps aside
// while a dialog is open (`asking`) and comes back after.
pragma Singleton
import QtQuick
import Quickshell

Singleton {
    id: files

    /// A dialog is open.
    property bool asking: false

    /// opts: {title, folder, name, filter: {name, glob}}.
    function save(opts, done) {
        ask("files.save", opts, done);
    }
    function open(opts, done) {
        ask("files.open", opts, done);
    }
    function folder(opts, done) {
        ask("files.folder", opts, done);
    }

    function ask(method, opts, done) {
        asking = true;
        Backend.request(method, opts, path => {
            files.asking = false;
            done(path || null);
        }, e => {
            files.asking = false;
            Toasts.error("The file dialog failed", e);
            done(null);
        });
    }
}
