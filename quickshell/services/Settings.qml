// wallpicker's settings (config.json), as far as the interface needs them
// so far: the wallpaper backend and the fits (the Adjust dialog). Read once
// at start; editing them comes with the Settings tab.
pragma Singleton
import QtQuick
import Quickshell

Singleton {
    id: settings

    property var data: ({})
    /// [{value, label}] the backend does ("" is its own default).
    property var fitModes: []

    /// "hyprpaper", "awww" or "noctalia".
    readonly property string backend: data.wallpaper_backend || "hyprpaper"

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
}
