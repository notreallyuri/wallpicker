// Noctalia's own colors follow the wallpaper, or its scheme from before.
// Applies at once (it's Noctalia's setting, not wallpicker's); only shown
// when Noctalia is running.
import QtQuick
import QtQuick.Layouts
import qs.components
import qs.services

ColumnLayout {
    id: field

    /// {available, following, scheme}; null until read.
    property var status: null
    property bool busy: false

    visible: !!(status && status.available)
    spacing: 6

    Component.onCompleted: Backend.request("app.noctalia_colors", {}, s => field.status = s, () => {})

    ToggleField {
        label: "Noctalia's colors follow the wallpaper"
        description: "wallpicker makes Noctalia a palette from the wallpaper shown and keeps it current. Applies at once. Now: " + (field.status && field.status.scheme ? field.status.scheme : "…") + ". Turning it off goes back to the scheme Noctalia had before."
        checked: !!(field.status && field.status.following)
        enabled: !field.busy
        onToggled: {
            field.busy = true;
            Backend.request("app.set_noctalia_colors", {
                follow: !field.status.following
            }, s => {
                field.status = s;
                field.busy = false;
            }, e => {
                Toasts.error("Could not change Noctalia's colors", e);
                field.busy = false;
            });
        }
    }
}
