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

    /// What a pointer at (x, y) would reach: the deepest MouseArea under
    /// it, searching every item there from the top down (not only the
    /// topmost, as childAt does: an empty item over a cell would stop it),
    /// with the way down for probe.
    function hit(x, y) {
        function search(item, chain) {
            chain = chain.concat([String(item).split("(")[0]]);
            const kids = item.children;
            for (let i = kids.length - 1; i >= 0; i--) {
                const c = kids[i];
                if (!c.visible || !c.enabled)
                    continue;
                const p = root.mapToItem(c, x, y);
                if (p.x < 0 || p.y < 0 || p.x > c.width || p.y > c.height) {
                    // A child of zero size can still hold children outside it.
                    if (c.width > 0 && c.height > 0 && c.clip)
                        continue;
                    if (c.width > 0 && c.height > 0)
                        continue;
                }
                const found = search(c, chain);
                if (found.area)
                    return found;
            }
            return {
                item: item,
                area: item instanceof MouseArea && item.enabled ? item : null,
                chain: chain
            };
        }
        return search(root, []);
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

        /// Wheel notches over (x, y), through the view's SmoothScroll (as a
        /// mouse's would); says where it's heading.
        function wheel(x: real, y: real, notches: real): string {
            let item = hooks.hit(x, y).item;
            while (item && !(item instanceof Flickable))
                item = item.parent;
            if (!item)
                return "no scrolling view at " + x + "," + y;
            const smooth = item.contentItem.data.find(o => typeof o.notches === "function");
            if (!smooth)
                return "no SmoothScroll in " + item;
            smooth.notches(notches);
            return "from " + Math.round(item.contentY) + " to " + Math.round(smooth.goal);
        }

        /// The scroll position of the view at (x, y).
        function scrollY(x: real, y: real): string {
            let item = hooks.hit(x, y).item;
            while (item && !(item instanceof Flickable))
                item = item.parent;
            return item ? String(Math.round(item.contentY)) : "no scrolling view";
        }

        /// The scrolling view at (x, y): its content's children, briefly.
        function contents(x: real, y: real): string {
            let item = hooks.hit(x, y).item;
            while (item && !(item instanceof Flickable))
                item = item.parent;
            if (!item)
                return "no scrolling view";
            const kids = item.contentItem.children;
            const out = [];
            for (let i = 0; i < Math.min(kids.length, 6); i++) {
                const c = kids[i];
                out.push(String(c).split("(")[0] + "[" + Math.round(c.x) + "," + Math.round(c.y) + " " + Math.round(c.width) + "x" + Math.round(c.height) + "] kids " + c.children.length);
            }
            return kids.length + ": " + out.join(", ");
        }

        /// What's at (x, y).
        function probe(x: real, y: real): string {
            return hooks.hit(x, y).chain.join(" > ");
        }
    }
}
