// wallpicker's settings (config.json), as saved: what the interface reads
// from them (the backend, the fits, the API key...) and what the Settings
// dialog's form starts from. Read at start and again when they're saved.
pragma Singleton
import QtQuick
import Quickshell

Singleton {
    id: settings

    property var data: ({})
    /// [{value, label}] the backend does ("" is its own default).
    property var fitModes: []


    /// The fit on `monitor` (as core's Settings::fit_for): its adjusted one,
    /// else the monitor's, else the general one; null leaves it to the
    /// backend.
    function fitFor(monitor, adjusted) {
        const fits = data.monitor_fits || {};
        return adjusted || fits[monitor] || data.fit_mode || null;
    }

    function reload() {
        Backend.request("app.settings", {}, s => settings.data = s || {});
        Backend.request("wallpapers.fit_modes", {}, modes => settings.fitModes = (modes || []).map(m => ({
                    value: m[0],
                    label: m[1]
                })));
    }

    Component.onCompleted: reload()

    Connections {
        target: Backend
        function onEvent(name) {
            if (name === "settings-changed")
                settings.reload();
        }
    }
}
