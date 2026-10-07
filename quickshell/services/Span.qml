// The span's actions: span a picture across some monitors, change the
// span shown (its look, monitors and arrangement; one change on its way at
// a time, the latest waiting one next), separate the monitors again. Its state (whether
// they're spanned, the picture and look) is Monitors'.
pragma Singleton
import QtQuick
import Quickshell

Singleton {
    id: span

    /// `path` across `monitors` (names; none: the ones spanned last), each
    /// showing its part; `layout` ({places: {name: {x, y}}, bezel}) and
    /// `look` from the Span dialog. The span is then where a click applies.
    function span(path, monitors, layout, look) {
        Monitors.mine(monitors && monitors.length ? monitors : Monitors.list.map(m => m.name));
        Monitors.setApplying(path, true);
        Backend.request("wallpapers.span", {
            path: path,
            monitors: monitors && monitors.length ? monitors : null,
            layout: layout || null,
            look: look || null
        }, report => {
            Monitors.setApplying(path, false);
            Monitors.selection = Monitors.spanTarget;
            if (report && report.failed && report.failed.length > 0)
                Toasts.error("Not spanned on every monitor", report.failed.map(f => f[0] + ": " + f[1]).join("\n"));
        }, e => {
            Monitors.setApplying(path, false);
            Toasts.error("Could not span it", e);
        });
    }

    /// Change the span shown: `change` is {look, monitors, layout} (any
    /// left out stays); the look is shown here at once; `done()` when the
    /// monitors show it. Like Monitors.setAdjust, one goes at a time and
    /// only the latest waiting one is sent.
    function update(change, done) {
        if (!Monitors.spanState)
            return;
        if (change.look)
            Monitors.spanState = Object.assign({}, Monitors.spanState, {
                look: change.look
            });
        const dones = done ? [done] : [];
        if (sending) {
            waiting = {
                change: Object.assign({}, waiting ? waiting.change : {}, change),
                dones: (waiting ? waiting.dones : []).concat(dones)
            };
            return;
        }
        send(change, dones);
    }
    property bool sending: false
    property var waiting: null
    function send(change, dones) {
        sending = true;
        const names = change.monitors || Monitors.spanNames;
        Monitors.mine(names);
        const finish = () => {
            for (const d of dones)
                d();
            const next = waiting;
            waiting = null;
            if (next)
                span.send(next.change, next.dones);
            else
                sending = false;
        };
        Backend.request("wallpapers.span", {
            path: Monitors.spanState.path,
            monitors: names,
            layout: change.layout || null,
            look: change.look || Monitors.spanState.look
        }, () => finish(), e => {
            Toasts.error("Could not change the span", e);
            finish();
        });
    }

    /// The monitors on their own again, each keeping its part.
    function separate() {
        Monitors.selection = Monitors.spanNames[0] || Monitors.selection;
        Monitors.spanState = null;
        Backend.request("wallpapers.separate", {}, s => Monitors.takeState(s), e => Toasts.error("Could not separate them", e));
    }
}
