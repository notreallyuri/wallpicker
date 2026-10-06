// wallpicker's interface (Quickshell). The entry only: the window, and the
// IPC a keybind uses (`wallpicker ui --toggle` calls it). Closing exits, the
// backend with it: nothing stays in memory between opens.
// The Rust side runs as `wallpicker serve` (services/Backend.qml).
import QtQuick
import Quickshell
import Quickshell.Io
import qs.windows

ShellRoot {
    IpcHandler {
        target: "wallpicker"
        /// Close (the launcher only calls it when it's open).
        function toggle(): void {
            Qt.quit();
        }
    }

    MainWindow {
        // Closed by the window manager: done.
        onVisibleChanged: if (!visible)
            Qt.quit()
    }
}
