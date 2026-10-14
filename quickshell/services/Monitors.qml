// The monitors, what each shows (and how it's adjusted), and where a click
// applies: one monitor, all of them, or the span (the app's rule: single
// choice; picking the chosen one again keeps it). Spanned, the span's
// monitors act as one: picked, a click spans the picture across them; the
// others stay on their own (the span's actions: services/Span). A
// wallpaper changed by something else while the picker is open (the
// slideshow, a workspace, the plugin) gets a toast.
pragma Singleton
import QtQuick
import Quickshell

Singleton {
    id: monitors

    readonly property string all: "__all__"
    readonly property string spanTarget: "__span__"

    property var list: []
    /// monitor -> the wallpaper it shows
    property var current: ({})
    /// monitor -> its adjustments ({flip: {h, v}, fit, frame}), until
    /// another wallpaper goes on it
    property var adjust: ({})
    /// A monitor's name, `all` or `spanTarget`.
    property string selection: ""
    /// Paths being applied (a spinner on their tile).
    property var applying: ({})
    /// The span ({path, look: {zoom, x, y, flip}, monitors, places: {name:
    /// {x, y, w, h} mm}}) while there's one.
    property var spanState: null
    /// The connected monitors it's across (none listed: all, from before
    /// spans chose).
    readonly property var spanNames: spanState ? list.map(m => m.name).filter(n => !spanState.monitors || spanState.monitors.length === 0 || spanState.monitors.indexOf(n) >= 0) : []
    /// They all show their part of one picture: they act as one.
    readonly property bool spanned: spanState !== null && spanNames.length > 1 && spanNames.every(n => current[n] === spanState.path)
    /// `name` is one of the span's.
    function inSpan(name) {
        return spanned && spanNames.indexOf(name) >= 0;
    }
    /// The span's monitors on the canvas: [{name, x, y, w, h}] (mm).
    readonly property var spanPlaces: spanState && spanState.places ? Object.keys(spanState.places).map(n => Object.assign({
                name: n
            }, spanState.places[n])) : []
    // The span ended while picked: back to a monitor.
    onSpannedChanged: if (!spanned && selection === spanTarget)
        selection = spanNames[0] || (list[0] || {}).name || ""

    readonly property var targets: selection === spanTarget ? spanNames : selection === all ? list.map(m => m.name) : (selection ? [selection] : [])

    /// monitor -> when the picker last changed it itself (ms): changes then
    /// aren't announced.
    property var own: ({})
    function mine(names) {
        const next = Object.assign({}, own);
        for (const n of names)
            next[n] = Date.now();
        own = next;
    }
    /// Every monitor is about to change by the picker's doing (Change now).
    function expectAll() {
        mine(list.map(m => m.name));
    }

    /// Monitor names showing `path`.
    function showing(path) {
        return list.filter(m => current[m.name] === path).map(m => m.name);
    }

    /// A monitor of the span picks the span.
    function select(name) {
        selection = inSpan(name) ? spanTarget : name;
    }

    /// On the chosen monitor(s); the span picked, across its monitors.
    function apply(path) {
        if (selection === spanTarget && spanned)
            Span.span(path, spanNames);
        else
            applyTo(path, targets);
    }

    /// On `names` (the tile menu's Apply to...).
    function applyTo(path, names) {
        if (names.length === 0)
            return;
        mine(names);
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
            frame: a.frame || null,
            span: a.span || null,
            // Its animated wallpaper held (the control panel's Pause).
            paused: !!a.paused
        };
    }

    /// Back to the wallpaper before, on the chosen monitors.
    function previous() {
        previousOn(targets, applied => applied.length > 0 ? "Back on " + applied.join(", ") : "Nothing earlier to go back to");
    }

    /// Back on `names`; `message(applied)` says how it went.
    function previousOn(names, message) {
        mine(names);
        Backend.request("wallpapers.previous", {
            monitors: names
        }, report => Toasts.info(message(report && report.applied ? report.applied : [])), e => Toasts.error("Could not go back", e));
    }

    /// Adjust what `name` shows; shown here at once, on the monitor when
    /// the changed copy is made. `done()` either way. One change per
    /// monitor is on its way at a time (each takes a while: a copy is
    /// made); changes asked meanwhile wait, and only the latest is sent.
    function setAdjust(name, next, done) {
        // Framed: out of the span (its own part of the picture, chosen).
        if (next.frame && next.span)
            next = Object.assign({}, next, {
                span: null
            });
        const none = !next.flip.h && !next.flip.v && !next.fit && !next.frame && !next.span && !next.paused;
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
        mine([name]);
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
        const next = s && s.current ? s.current : {};
        announce(current, next);
        current = next;
        adjust = s && s.adjust ? s.adjust : {};
        spanState = s && s.span ? s.span : null;
    }

    /// Toast the monitors whose wallpaper changed by something else.
    property bool started: false
    function announce(before, after) {
        if (!started) {
            started = true;
            return;
        }
        const now = Date.now();
        const changed = Object.keys(after).filter(m => after[m] !== before[m] && !(own[m] && now - own[m] < 8000));
        if (changed.length === 0)
            return;
        const name = p => p.split("/").pop();
        if (changed.length === 1)
            Toasts.info("New wallpaper on " + changed[0], name(after[changed[0]]));
        else
            Toasts.info("New wallpapers", changed.map(m => m + ": " + name(after[m])).join("\n"));
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
