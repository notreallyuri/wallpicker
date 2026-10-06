// The connection to Rust: one `wallpicker serve` for as long as the UI runs,
// JSON lines both ways (src-tauri/src/serve). `request` sends a method and
// calls back with its result; unasked events come through `event`.
// WALLPICKER_BIN picks the binary (development); else `wallpicker` on PATH.
pragma Singleton
import QtQuick
import Quickshell
import Quickshell.Io

Singleton {
    id: backend

    property bool ready: false
    /// Something's wrong with the connection (shown by the UI).
    property string failure: ""

    signal event(string name, var data)

    property int nextId: 1
    // id -> {done, failed}; requests made before `ready` wait in `queued`.
    property var pending: ({})
    property var queued: []

    /// `method` with `params`; `done(result)` or `failed(message)` later.
    function request(method, params, done, failed) {
        const id = nextId++;
        pending[id] = {
            done: done,
            failed: failed
        };
        const line = JSON.stringify({
            id: id,
            method: method,
            params: params || {}
        }) + "\n";
        if (ready)
            serve.write(line);
        else
            queued.push(line);
    }

    function handle(line) {
        let msg;
        try {
            msg = JSON.parse(line);
        } catch (e) {
            console.warn("serve said something that isn't JSON:", line);
            return;
        }
        if (msg.event !== undefined) {
            if (msg.event === "ready") {
                ready = true;
                for (const l of queued)
                    serve.write(l);
                queued = [];
            }
            event(msg.event, msg.data);
            return;
        }
        const waiting = pending[msg.id];
        delete pending[msg.id];
        if (!waiting)
            return;
        if (msg.error !== undefined) {
            if (waiting.failed)
                waiting.failed(msg.error);
            else
                console.warn("wallpicker serve:", msg.error);
        } else if (waiting.done) {
            waiting.done(msg.result);
        }
    }

    Process {
        id: serve
        running: true
        stdinEnabled: true
        command: [Quickshell.env("WALLPICKER_BIN") || "wallpicker", "serve"]
        stdout: SplitParser {
            onRead: data => backend.handle(data)
        }
        stderr: SplitParser {
            onRead: data => console.warn("wallpicker serve:", data)
        }
        onExited: (code, status) => {
            backend.ready = false;
            backend.failure = "The backend stopped (" + code + ")";
        }
    }
}
