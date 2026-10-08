// For headless checks only (WALLPICKER_TEST=1): clicks by position over
// IPC, since a headless compositor has no pointer to drive.
//   qs -p quickshell ipc call wallpicker-test click 1848 166
//   qs -p quickshell ipc call wallpicker-test probe 600 400
import QtQuick
import QtQuick.Window
import QtTest
import Quickshell.Io
import qs.services

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
            // Topmost first: higher z, then later in the file.
            const kids = [];
            for (let i = 0; i < item.children.length; i++)
                kids.push({
                    item: item.children[i],
                    order: i
                });
            kids.sort((a, b) => (b.item.z - a.item.z) || (b.order - a.order));
            for (const k of kids) {
                const c = k.item;
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
        // The whole window: popovers, menus and dialogs live in its
        // overlay, outside the content.
        return search(root.Window.contentItem || root, []);
    }

    /// The deepest scrolling view under (x, y) (as a wheel finds it:
    /// whatever is clickable there or not).
    function flickableAt(x, y) {
        let found = null;
        function walk(item) {
            for (let i = 0; i < item.children.length; i++) {
                const c = item.children[i];
                if (!c.visible)
                    continue;
                const p = root.mapToItem(c, x, y);
                if (p.x < 0 || p.y < 0 || p.x > c.width || p.y > c.height)
                    continue;
                if (c instanceof Flickable)
                    found = c;
                walk(c);
            }
        }
        walk(root.Window.contentItem || root);
        return found;
    }

    // Real input events (QtTest posts them to the window, so they're
    // delivered as a mouse's would be: handlers, overlays, popups).
    property TestEvent events: TestEvent {}

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

        /// A real click at (x, y) (left, or right with button 2).
        function press(x: real, y: real, button: int): string {
            const b = button === 2 ? Qt.RightButton : Qt.LeftButton;
            hooks.events.mouseMove(hooks.root, x, y, -1, Qt.NoButton, Qt.NoModifier);
            return hooks.events.mouseClick(hooks.root, x, y, b, Qt.NoModifier, -1) ? "pressed" : "no window";
        }

        /// A real drag from (x1, y1) to (x2, y2), in a few steps.
        function drag(x1: real, y1: real, x2: real, y2: real): string {
            const e = hooks.events;
            e.mouseMove(hooks.root, x1, y1, -1, Qt.NoButton, Qt.NoModifier);
            e.mousePress(hooks.root, x1, y1, Qt.LeftButton, Qt.NoModifier, -1);
            for (let i = 1; i <= 8; i++)
                e.mouseMove(hooks.root, x1 + (x2 - x1) * i / 8, y1 + (y2 - y1) * i / 8, 16, Qt.LeftButton, Qt.NoModifier);
            e.mouseRelease(hooks.root, x2, y2, Qt.LeftButton, Qt.NoModifier, 16);
            return "dragged";
        }

        /// A real pointer move to (x, y).
        function move(x: real, y: real): string {
            return hooks.events.mouseMove(hooks.root, x, y, -1, Qt.NoButton, Qt.NoModifier) ? "moved" : "no window";
        }

        /// A real wheel at (x, y): `notches` of 120 (positive: down).
        function roll(x: real, y: real, notches: real): string {
            hooks.events.mouseMove(hooks.root, x, y, -1, Qt.NoButton, Qt.NoModifier);
            const ok = hooks.events.mouseWheel(hooks.root, x, y, Qt.NoButton, Qt.NoModifier, 0, -120 * notches, -1);
            return ok ? "rolled" : "no window";
        }

        /// Right-click at (x, y): the nearest item there with a
        /// rightClicked(x, y) gets it (a tile opens its menu).
        function rightclick(x: real, y: real): string {
            let item = hooks.hit(x, y).area;
            while (item && typeof item.rightClicked !== "function")
                item = item.parent;
            if (!item)
                return "nothing takes a right-click at " + x + "," + y;
            const p = hooks.root.mapToItem(item, x, y);
            item.rightClicked(p.x, p.y);
            return "right-clicked " + String(item).split("(")[0];
        }

        /// Wheel notches over (x, y), through the view's SmoothScroll (as a
        /// mouse's would); says where it's heading.
        function wheel(x: real, y: real, notches: real): string {
            const item = hooks.flickableAt(x, y);
            if (!item)
                return "no scrolling view at " + x + "," + y;
            let smooth = null;
            for (let i = 0; i < item.children.length; i++)
                if (typeof item.children[i].notches === "function")
                    smooth = item.children[i];
            if (!smooth)
                return "no SmoothScroll in " + item;
            smooth.notches(notches);
            return "from " + Math.round(item.contentY) + " to " + Math.round(smooth.goal);
        }

        /// A fast spin at (x, y): `notches` real notches, `gap` ms apart.
        function spin(x: real, y: real, notches: int, gap: int): string {
            hooks.events.mouseMove(hooks.root, x, y, -1, Qt.NoButton, Qt.NoModifier);
            for (let i = 0; i < notches; i++)
                hooks.events.mouseWheel(hooks.root, x, y, Qt.NoButton, Qt.NoModifier, 0, -120, gap);
            return "spun";
        }

        /// The scroll position of the view at (x, y), and its range.
        function range(x: real, y: real): string {
            const item = hooks.flickableAt(x, y);
            return item ? Math.round(item.contentY) + " of " + Math.round(item.originY - item.topMargin) + ".." + Math.round(item.originY + item.contentHeight + item.bottomMargin - item.height) : "no scrolling view";
        }

        /// The scroll position of the view at (x, y).
        function scrollY(x: real, y: real): string {
            const item = hooks.flickableAt(x, y);
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

        /// The interface's state that checks look at.
        function state(): string {
            return JSON.stringify({
                popups: Dialogs.popups,
                shown: Lifetime.shown,
                trashFor: Dialogs.trashFor,
                localDetailsFor: Dialogs.localDetailsFor,
                adjusting: Dialogs.adjusting,
                adjust: Monitors.adjust,
                current: Monitors.current,
                selection: Monitors.selection,
                spanned: Monitors.spanned,
                applying: Monitors.applying,
                drawer: Monitors.drawerOpen,
                focus: String(hooks.root.Window.activeFocusItem).split("(")[0],
                active: hooks.root.Window.active
            });
        }

        /// Pretend `monitor` shows `path` (in memory only): the panel's
        /// backdrop follows, e.g. a bright wallpaper to check contrast on.
        function showing(monitor: string, path: string): string {
            const next = Object.assign({}, Monitors.current);
            next[monitor] = path;
            Monitors.current = next;
            return "ok";
        }

        /// The saved settings (as the interface has them), without the key.
        function settings(): string {
            const s = Object.assign({}, Settings.data);
            s.wallhaven_api_key = s.wallhaven_api_key ? "(set)" : null;
            return JSON.stringify(s);
        }

        /// Open a dialog: settings (at section `arg`), span (the span's
        /// Adjust), across (the Span dialog for picture `arg`), adjust
        /// (monitor `arg`), filters (Online's), details (picture `arg`'s).
        function dialog(name: string, arg: string): string {
            switch (name) {
            case "settings":
                Dialogs.openSettings(arg);
                break;
            case "span":
                Dialogs.adjustSpan();
                break;
            case "across":
                Dialogs.spanAcross(arg);
                break;
            case "adjust":
                Dialogs.adjust(arg);
                break;
            case "filters":
                Dialogs.onlineFilters = true;
                break;
            case "details":
                Dialogs.localDetails(arg);
                break;
            default:
                return "no " + name;
            }
            return "ok";
        }

        /// The last toasts.
        function toasts(): string {
            return JSON.stringify(Toasts.list.map(t => t.title + (t.detail ? ": " + t.detail : "")));
        }

        /// The Online search and the downloads.
        function online(): string {
            return JSON.stringify({
                provider: Online.provider,
                text: Online.text,
                restored: Online.restored,
                options: !!Online.options,
                loading: Online.loading,
                results: Online.results.length,
                page: Online.page,
                active: Online.activeFilters,
                suggestions: Online.suggestions.map(s => s.name),
                working: Downloads.working,
                saved: Object.keys(Downloads.saved).length
            });
        }

        /// Drive the Online search without its view: activate, provider P,
        /// text T, search, page N, filter NAME=VALUE (JSON, else a string),
        /// reset, suggest I (pick
        /// the I-th suggestion), download I (the I-th result), details I
        /// (its details dialog).
        function onlineDo(what: string, arg: string): string {
            const n = parseInt(arg);
            switch (what) {
            case "activate":
                Online.activate();
                break;
            case "provider":
                Online.setProvider(arg);
                break;
            case "text":
                Online.text = arg;
                break;
            case "search":
                Online.search();
                break;
            case "page":
                Online.goTo(n);
                break;
            case "filter":
                const name = arg.split("=")[0];
                const raw = arg.slice(name.length + 1);
                let value = raw;
                try {
                    value = JSON.parse(raw);
                } catch (e) {}
                Online.setFilter(name, value);
                break;
            case "reset":
                Online.resetFilters();
                break;
            case "suggest":
                Online.pickSuggestion(Online.suggestions[n]);
                break;
            case "download":
                Downloads.fetch(Online.results[n], null);
                break;
            case "details":
                Dialogs.showDetails(Online.results[n]);
                break;
            default:
                return "no " + what;
            }
            return "ok";
        }

        /// Every item the search looks into at (x, y), for debugging hits.
        function trace(x: real, y: real): string {
            const seen = [];
            function walk(item, depth) {
                if (depth > 9 || seen.length > 60)
                    return;
                for (let i = 0; i < item.children.length; i++) {
                    const c = item.children[i];
                    const p = hooks.root.mapToItem(c, x, y);
                    const inside = p.x >= 0 && p.y >= 0 && p.x <= c.width && p.y <= c.height;
                    if (!c.visible || (!inside && c.width > 0 && c.height > 0))
                        continue;
                    seen.push(" ".repeat(depth) + String(c).split("(")[0] + (c instanceof MouseArea ? " [area]" : "") + " z" + c.z);
                    walk(c, depth + 1);
                }
            }
            walk(hooks.root.Window.contentItem, 0);
            return seen.join(" | ");
        }

        /// What's at (x, y).
        function probe(x: real, y: real): string {
            return hooks.hit(x, y).chain.join(" > ");
        }
    }
}
