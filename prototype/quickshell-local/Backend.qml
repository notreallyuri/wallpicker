// The Rust side, through the CLI: `wallpicker list --json --thumbs` for the
// library (images with favorite, tags, size and cached thumbnail; monitors
// with what they show; tag counts), and apply / favorite / previous for the
// actions, each followed by a fresh list so the grid shows what changed.
pragma Singleton
import QtQuick
import Quickshell
import Quickshell.Io

Singleton {
    id: backend

    property var images: []
    property var monitors: []
    property var tags: []
    property bool loading: true
    /// Paths being applied (a spinner on their tile).
    property var applying: ({})

    function reload() {
        list.running = true;
    }

    /// `path` on `monitors` (names).
    function apply(path, monitors) {
        const busy = Object.assign({}, applying);
        busy[path] = true;
        applying = busy;
        run(["wallpicker", "apply", path].concat(...monitors.map(m => ["--monitor", m])), () => {
            const left = Object.assign({}, applying);
            delete left[path];
            applying = left;
            Theme.reload();
        });
    }

    function setFavorite(path, on) {
        // Shown at once; the list confirms it.
        images = images.map(i => i.path === path ? Object.assign({}, i, {
                favorite: on
            }) : i);
        run(on ? ["wallpicker", "favorite", path] : ["wallpicker", "favorite", path, "--off"]);
    }

    function previous(monitors) {
        run(["wallpicker", "previous"].concat(...monitors.map(m => ["--monitor", m])), () => Theme.reload());
    }

    /// One command, then a fresh list (and `done`).
    function run(command, done) {
        const proc = runner.createObject(backend, {
            command: command
        });
        proc.exited.connect(() => {
            if (done)
                done();
            reload();
            proc.destroy();
        });
        proc.running = true;
    }

    Component {
        id: runner
        Process {}
    }

    Process {
        id: list
        running: true
        command: ["wallpicker", "list", "--json", "--thumbs"]
        stdout: StdioCollector {
            onStreamFinished: {
                try {
                    const d = JSON.parse(text);
                    backend.images = d.images || [];
                    backend.monitors = d.monitors || [];
                    backend.tags = d.all_tags || [];
                } catch (e) {
                    console.warn("wallpicker list:", e);
                }
                backend.loading = false;
            }
        }
    }
}
