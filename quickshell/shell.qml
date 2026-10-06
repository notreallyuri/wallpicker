// wallpicker's interface (Quickshell). The entry only: the window, made when
// it's opened and gone when it's closed, and the IPC a keybind uses:
//   qs -p quickshell ipc call wallpicker toggle
// The Rust side runs as `wallpicker serve` (services/Backend.qml).
import QtQuick
import Quickshell
import Quickshell.Io
import qs.windows

ShellRoot {
    id: root

    property bool open: true

    IpcHandler {
        target: "wallpicker"
        function toggle(): void {
            root.open = !root.open;
        }
    }

    LazyLoader {
        active: root.open
        MainWindow {}
    }
}
