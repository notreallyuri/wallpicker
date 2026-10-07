// The clipboard, through the backend (`wl-copy`): Quickshell's own would
// go with the interface, which exits when it closes (on Wayland the program
// that copied serves the clipboard); wl-copy stays to serve it.
pragma Singleton
import QtQuick
import Quickshell

Singleton {
    /// Copy `text`; `message` is the toast when it's done.
    function copy(text, message) {
        Backend.request("app.copy_text", {
            text: text
        }, () => Toasts.info(message), e => Toasts.error("Could not copy it", e));
    }
}
