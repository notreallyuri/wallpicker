// wallpicker's interface (Quickshell). The entry only: the picker (a
// floating panel, or a window with WALLPICKER_WINDOW=1), and the IPC a
// keybind uses (`wallpicker ui --toggle` calls it). Closing exits, the
// backend with it: nothing stays in memory between opens.
// The Rust side runs as `wallpicker serve` (services/Backend.qml).
import QtQuick
import Quickshell
import Quickshell.Io
import qs.windows

ShellRoot {
    id: root

    readonly property bool asWindow: Quickshell.env("WALLPICKER_WINDOW") === "1"

    IpcHandler {
        target: "wallpicker"
        /// Close (the launcher only calls it when it's open).
        function toggle(): void {
            Qt.quit();
        }
    }

    Loader {
        id: host
        sourceComponent: root.asWindow ? windowHost : panelHost
    }
    Component {
        id: panelHost
        PickerPanel {
            onDismissed: Qt.quit()
        }
    }
    Component {
        id: windowHost
        PickerWindow {
            onDismissed: Qt.quit()
        }
    }

    // Headless checks only (WALLPICKER_TEST=1).
    Loader {
        active: Quickshell.env("WALLPICKER_TEST") === "1" && host.item !== null
        sourceComponent: TestHooks {
            root: host.item.picker.content
        }
    }
}
