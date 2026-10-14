// The Settings form's state: a draft of the settings (config.json), made
// when the dialog opens, which the sections edit (`set("transition.kind",
// "fade")`) and Save sends as a whole. Closing without saving drops it.
// Some things act at once instead (dark mode, the backup, a workspace's
// wallpaper): those sections call the services themselves.
import QtQuick
import qs.services

QtObject {
    id: form

    property var draft: ({})
    property bool saving: false
    /// The draft as loaded (or last saved): it's unsaved when it differs.
    property string loaded: "{}"
    readonly property bool dirty: JSON.stringify(draft) !== loaded
    /// The settings' choices (settings.options): backends, fits...
    property var options: null

    /// A fresh draft from the saved settings.
    function load() {
        draft = JSON.parse(JSON.stringify(Settings.data || {}));
        loaded = JSON.stringify(draft);
        if (!options)
            Backend.request("settings.options", {}, o => form.options = o);
    }

    /// The value at `path` ("slideshow.night.from").
    function get(path) {
        return path.split(".").reduce((v, k) => v === undefined || v === null ? undefined : v[k], draft);
    }

    /// Set the value at `path`: the draft is replaced, so bindings on it update.
    function set(path, value) {
        const next = JSON.parse(JSON.stringify(draft));
        const keys = path.split(".");
        let at = next;
        for (const k of keys.slice(0, -1)) {
            if (at[k] === undefined || at[k] === null)
                at[k] = {};
            at = at[k];
        }
        at[keys[keys.length - 1]] = value;
        draft = next;
    }

    /// Save the draft; `done()` once it's saved (the backend says the
    /// settings changed, and everything that depends on them reloads).
    function save(done) {
        saving = true;
        Backend.request("settings.save", {
            settings: draft
        }, saved => {
            form.saving = false;
            Settings.data = saved;
            form.loaded = JSON.stringify(form.draft);
            Toasts.info("Settings saved");
            if (done)
                done();
        }, e => {
            form.saving = false;
            Toasts.error("Could not save the settings", e);
        });
    }
}
