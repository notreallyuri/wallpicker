// wallpicker's interface (Quickshell). The entry only: the picker (a
// floating panel, or a window with WALLPICKER_WINDOW=1), and the IPC
// `wallpicker` uses (--toggle, --open, opening it again). Closing hides it
// for a while (Settings → Performance), then exits, the backend with it:
// nothing stays in memory between opens (services/Lifetime).
// The Rust side runs as `wallpicker serve` (services/Backend.qml).
import QtQuick
import Quickshell
import Quickshell.Io
import qs.services
import qs.windows

ShellRoot {
    id: root

    // Not reloaded when its files change: installing over a picker that's
    // still loaded (closed, waiting) reloaded it from half-copied files.
    Component.onCompleted: Quickshell.watchFiles = false

    readonly property bool asWindow: Quickshell.env("WALLPICKER_WINDOW") === "1"

    IpcHandler {
        target: "wallpicker"
        /// Close it, or show it again while it's still loaded.
        function toggle(): void {
            Lifetime.toggle();
        }
        /// Show it again (`wallpicker` while it's still loaded). Not `show`:
        /// `qs ipc call … show` lists the targets instead.
        function reopen(): void {
            Lifetime.show();
        }
        /// Show it at a tab or a dialog (`wallpicker --open PLACE [IMAGE]`).
        function open(place: string, path: string): void {
            Lifetime.show();
            Views.go(place, path);
        }
        /// Up, with the library read (scripts/open-timing.sh waits for it).
        function ready(): bool {
            return !Library.loading;
        }
    }

    Loader {
        id: host
        sourceComponent: root.asWindow ? windowHost : panelHost
    }
    Component {
        id: panelHost
        PickerPanel {
            onDismissed: Lifetime.close()
        }
    }
    Component {
        id: windowHost
        PickerWindow {
            onDismissed: Lifetime.close()
        }
    }

    // Opened somewhere (`wallpicker --open`): there once the picker is up,
    // so its dialogs are there to answer.
    Timer {
        running: host.item !== null && !!Quickshell.env("WALLPICKER_OPEN")
        interval: 150
        onTriggered: {
            const [place, path] = String(Quickshell.env("WALLPICKER_OPEN")).split("\n");
            Views.go(place, path || "");
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
