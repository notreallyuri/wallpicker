// The span's actions: span a picture across every monitor, change its look
// (zoom, center, mirroring; one change on its way at a time, the latest
// waiting one next), separate the monitors again. Its state (whether
// they're spanned, the picture and look) is Monitors'.
pragma Singleton
import QtQuick
import Quickshell

Singleton {
    id: span

    /// `path` across every monitor, each showing its part.
    function span(path) {
        Monitors.mine(Monitors.list.map(m => m.name));
        Monitors.setApplying(path, true);
        Backend.request("wallpapers.span", {
            path: path
        }, report => {
            Monitors.setApplying(path, false);
            if (report && report.failed && report.failed.length > 0)
                Toasts.error("Not spanned on every monitor", report.failed.map(f => f[0] + ": " + f[1]).join("\n"));
        }, e => {
            Monitors.setApplying(path, false);
            Toasts.error("Could not span it", e);
        });
    }

    /// A new look for the span (zoom, center, mirroring), shown here at
    /// once; `done()` when the monitors show it. Like Monitors.setAdjust, one goes
    /// at a time and only the latest waiting one is sent.
    function setLook(look, done) {
        if (!Monitors.spanState)
            return;
        Monitors.spanState = Object.assign({}, Monitors.spanState, {
            look: look
        });
        const dones = done ? [done] : [];
        if (sending) {
            waiting = {
                look: look,
                dones: (waiting ? waiting.dones : []).concat(dones)
            };
            return;
        }
        sendLook(look, dones);
    }
    property bool sending: false
    property var waiting: null
    function sendLook(look, dones) {
        sending = true;
        Monitors.mine(Monitors.list.map(m => m.name));
        const finish = () => {
            for (const d of dones)
                d();
            const next = waiting;
            waiting = null;
            if (next)
                span.sendLook(next.look, next.dones);
            else
                sending = false;
        };
        Backend.request("wallpapers.span", {
            path: Monitors.spanState.path,
            look: look
        }, () => finish(), e => {
            Toasts.error("Could not adjust the span", e);
            finish();
        });
    }

    /// The monitors on their own again, each keeping its part.
    function separate() {
        Monitors.spanState = null;
        Backend.request("wallpapers.separate", {}, s => Monitors.takeState(s), e => Toasts.error("Could not separate them", e));
    }
}
