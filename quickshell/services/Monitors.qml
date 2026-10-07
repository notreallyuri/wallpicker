// The monitors, what each shows (and how it's adjusted), and where a click
// applies: one monitor or all of them (the app's rule: single choice;
// picking the chosen one again keeps it). Whether the drawer of monitors
// under the top bar is open.
pragma Singleton
import QtQuick
import Quickshell

Singleton {
    id: monitors

    readonly property string all: "__all__"

    property var list: []
    /// monitor -> the wallpaper it shows
    property var current: ({})
    /// monitor -> its adjustments ({flip: {h, v}, fit, frame}), until
    /// another wallpaper goes on it
    property var adjust: ({})
    /// The drawer of monitors under the top bar (remembered).
    readonly property bool drawerOpen: Prefs.monitorsOpen
    /// A monitor's name, or `all`.
    property string selection: ""
    /// Paths being applied (a spinner on their tile).
    property var applying: ({})

    readonly property var targets: selection === all ? list.map(m => m.name) : (selection ? [selection] : [])

    /// Monitor names showing `path`.
    function showing(path) {
        return list.filter(m => current[m.name] === path).map(m => m.name);
    }

    function toggleDrawer() {
        Prefs.monitorsOpen = !Prefs.monitorsOpen;
    }

    function select(name) {
        selection = name;
    }

    /// On the chosen monitor(s).
    function apply(path) {
        applyTo(path, targets);
    }

    /// On `names` (the tile menu's Apply to...).
    function applyTo(path, names) {
        if (names.length === 0)
            return;
        setApplying(path, true);
        Backend.request("wallpapers.apply", {
            monitors: names,
            path: path
        }, () => setApplying(path, false), e => {
            setApplying(path, false);
            Toasts.error("Could not apply it", e);
        });
    }

    /// A monitor's adjustments (none: all off).
    function adjustOf(name) {
        const a = adjust[name] || {};
        return {
            flip: {
                h: !!(a.flip && a.flip.h),
                v: !!(a.flip && a.flip.v)
            },
            fit: a.fit || null,
            frame: a.frame || null
        };
    }

    /// Back to the wallpaper before, on the chosen monitors.
    function previous() {
        previousOn(targets, applied => applied.length > 0 ? "Back on " + applied.join(", ") : "Nothing earlier to go back to");
    }

    /// Back on `names`; `message(applied)` says how it went.
    function previousOn(names, message) {
        Backend.request("wallpapers.previous", {
            monitors: names
        }, report => Toasts.info(message(report && report.applied ? report.applied : [])), e => Toasts.error("Could not go back", e));
    }

    /// Adjust what `name` shows; shown here at once, on the monitor when
    /// the changed copy is made. `done()` either way. One change per
    /// monitor is on its way at a time (each takes a while: a copy is
    /// made); changes asked meanwhile wait, and only the latest is sent.
    function setAdjust(name, next, done) {
        const none = !next.flip.h && !next.flip.v && !next.fit && !next.frame;
        const map = Object.assign({}, adjust);
        if (none)
            delete map[name];
        else
            map[name] = next;
        adjust = map;
        const dones = done ? [done] : [];
        if (adjustSending[name]) {
            const waiting = adjustWaiting[name];
            adjustWaiting[name] = {
                next: next,
                dones: (waiting ? waiting.dones : []).concat(dones)
            };
            return;
        }
        sendAdjust(name, next, dones);
    }

    // monitor -> a change is on its way / the latest one waiting to go.
    property var adjustSending: ({})
    property var adjustWaiting: ({})

    function sendAdjust(name, next, dones) {
        adjustSending[name] = true;
        const finish = s => {
            const waiting = adjustWaiting[name];
            delete adjustWaiting[name];
            for (const d of dones)
                d();
            if (waiting) {
                // Its answer is already out of date: the next one's will do.
                monitors.sendAdjust(name, waiting.next, waiting.dones);
                return;
            }
            adjustSending[name] = false;
            if (s)
                monitors.takeState(s);
            else
                monitors.reloadState();
        };
        Backend.request("wallpapers.adjust", {
            monitor: name,
            adjust: next
        }, s => finish(s), e => {
            Toasts.error("Could not adjust " + name, e);
            finish(null);
        });
    }

    function setApplying(path, on) {
        const next = Object.assign({}, applying);
        if (on)
            next[path] = true;
        else
            delete next[path];
        applying = next;
    }

    function reload() {
        Backend.request("wallpapers.monitors", {}, ms => {
            monitors.list = ms || [];
            if (!monitors.selection) {
                const f = monitors.list.find(m => m.focused) || monitors.list[0];
                monitors.selection = f ? f.name : "";
            }
        });
        reloadState();
    }

    function reloadState() {
        Backend.request("app.state", {}, s => monitors.takeState(s));
    }

    function takeState(s) {
        current = s && s.current ? s.current : {};
        adjust = s && s.adjust ? s.adjust : {};
    }

    Component.onCompleted: reload()

    Connections {
        target: Backend
        function onEvent(name) {
            if (name === "state-changed")
                monitors.reloadState();
        }
    }
}
