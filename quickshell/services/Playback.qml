// The animated wallpapers on screen: what each monitor's is doing (playing,
// paused, a still) and how fast, read from the daemon every 2 s while
// something shows it (Settings → Animated wallpapers)
// and the picker is open. Pausing acts on all of them at once.
pragma Singleton
import QtQuick
import Quickshell

Singleton {
    id: playback

    /// {running, paused, monitors: {name: {state, fps, reason}}}; null until read.
    property var data: null
    /// Things showing it (a settings section).
    property int watchers: 0
    readonly property bool enabled: !!(Settings.data.animation && Settings.data.animation.enabled)
    readonly property bool paused: !!(Settings.data.animation && Settings.data.animation.paused)
    /// Some monitor plays an animation (or holds it paused): pausing does something.
    readonly property bool playing: data !== null && Object.keys(data.monitors || {}).some(n => ["playing", "paused", "hidden"].indexOf(data.monitors[n].state) >= 0)

    /// `name`'s animation ({state, fps, reason}), null when it shows a picture.
    function of(name) {
        return data && data.monitors ? data.monitors[name] || null : null;
    }
    /// A line for `m` (of()): "Playing · 24 fps", "Paused", "Still: playback is off"...
    function describe(m) {
        if (!m)
            return "";
        switch (m.state) {
        case "playing":
            return "Playing" + (m.fps !== null && m.fps !== undefined ? " · " + Math.round(m.fps) + " fps" : "");
        case "paused":
            return "Paused";
        case "hidden":
            return "Resting while it's not on screen";
        case "still":
            return playback.enabled ? "First frame" : "First frame: playback is off";
        case "unavailable":
            return "First frame: " + (m.reason || "no hardware GPU playback");
        case "frozen":
            return "Stopped: " + (m.reason || "a decoding error");
        default:
            return "Starting";
        }
    }
    function poll() {
        Backend.request("renderer.playback", {}, d => playback.data = d, () => {});
    }
    function setPaused(paused) {
        Backend.request("settings.pause_animation", {
            paused: paused
        }, saved => {
            Settings.data = saved;
            // The daemon follows within a second.
            settle.restart();
        }, e => Toasts.error(paused ? "Could not pause the animations" : "Could not resume the animations", e));
    }

    Timer {
        interval: 2000
        running: playback.watchers > 0 && Lifetime.shown
        repeat: true
        triggeredOnStart: true
        onTriggered: playback.poll()
    }
    Timer {
        id: settle
        interval: 1300
        onTriggered: playback.poll()
    }
}
