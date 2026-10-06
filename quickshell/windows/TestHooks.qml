// For headless checks only (WALLPICKER_TEST=1): clicks by position over
// IPC, since a headless compositor has no pointer to drive.
//   qs -p quickshell ipc call wallpicker-test click 1848 166
//   qs -p quickshell ipc call wallpicker-test probe 600 400
import QtQuick
import Quickshell.Io

QtObject {
    id: hooks

    /// The window's content.
    required property Item root

    /// The deepest visible item at (x, y), and the MouseArea over it.
    function hit(x, y) {
        let item = root, area = null;
        for (;;) {
            if (item instanceof MouseArea && item.enabled)
                area = item;
            const p = root.mapToItem(item, x, y);
            const next = item.childAt(p.x, p.y);
            if (!next)
                return {
                    item: item,
                    area: area
                };
            item = next;
        }
    }

    // Only what's here crosses IPC (plain types).
    property IpcHandler ipc: IpcHandler {
        target: "wallpicker-test"

        /// Click at (x, y) in the window; says what took it.
        function click(x: real, y: real): string {
            const h = hooks.hit(x, y);
            if (!h.area)
                return "nothing to click at " + x + "," + y + " (" + h.item + ")";
            h.area.clicked(null);
            return "clicked " + h.area.parent;
        }

        /// What's at (x, y).
        function probe(x: real, y: real): string {
            const h = hooks.hit(x, y);
            return String(h.item) + (h.area ? " in " + h.area.parent : "");
        }
    }
}
