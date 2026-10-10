pragma Singleton
import QtQuick
import Quickshell

Singleton {
    id: presets
    property var items: []
    property bool busy: false
    property string error: ""
    property string report: ""
    property var editing: null

    function reload() {
        Backend.request("presets.list", {}, result => {
            items = result;
            error = "";
        }, message => error = message);
    }
    function run(action, params, done) {
        if (busy) return;
        busy = true;
        error = "";
        report = "";
        Backend.request("presets." + action, params, result => {
            busy = false;
            reload();
            if (action === "apply") {
                report = "Applied: " + (result.applied.join(", ") || "none");
                for (const entry of result.skipped)
                    report += "\nSkipped " + entry[0] + ": " + entry[1];
                for (const entry of result.failed)
                    report += "\nFailed " + entry[0] + ": " + entry[1];
                if (result.failed.length || !result.applied.length)
                    Toasts.error("Preset application", report);
                else Toasts.info("Preset applied", result.applied.join(", "));
            }
            if (done) done();
        }, message => {
            busy = false;
            error = message;
            Toasts.error("Preset operation failed", message);
        });
    }
    Connections {
        target: Backend
        function onEvent(name, data) {
            if (name === "presets-changed") presets.reload();
        }
    }
}
