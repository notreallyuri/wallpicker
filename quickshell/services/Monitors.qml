// The monitors, what each shows, and where a click applies: one monitor or
// all of them (the app's rule: single choice; picking the chosen one again
// keeps it).
pragma Singleton
import QtQuick
import Quickshell

Singleton {
    id: monitors

    readonly property string all: "__all__"

    property var list: []
    /// monitor -> the wallpaper it shows
    property var current: ({})
    /// A monitor's name, or `all`.
    property string selection: ""
    /// Paths being applied (a spinner on their tile).
    property var applying: ({})

    readonly property var targets: selection === all ? list.map(m => m.name) : (selection ? [selection] : [])

    /// Monitor names showing `path`.
    function showing(path) {
        return list.filter(m => current[m.name] === path).map(m => m.name);
    }

    function select(name) {
        selection = name;
    }

    function apply(path) {
        if (targets.length === 0)
            return;
        setApplying(path, true);
        Backend.request("wallpapers.apply", {
            monitors: targets,
            path: path
        }, () => setApplying(path, false), e => {
            setApplying(path, false);
            Toasts.error("Could not apply it", e);
        });
    }

    function previous() {
        Backend.request("wallpapers.previous", {
            monitors: targets
        }, null, e => Toasts.error("Could not go back", e));
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
        Backend.request("app.state", {}, s => monitors.current = s && s.current ? s.current : {});
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
