// The app's look: colors (the wallpaper's palette mapped onto the same
// tokens as ui/src/palette.rs, surfaces tinted toward the primary color in
// oklab like CSS color-mix; Blossom dark until it's read), the font, radii
// and motion. Oklab math is in Oklab.js.
pragma Singleton
import QtQuick
import Quickshell
import "Oklab.js" as Oklab

Singleton {
    id: theme

    property var roles: ({})

    // ui/src/palette.rs TOKENS: token, role, % of primary mixed in.
    readonly property color background: token("surface", 12, "#18141e")
    readonly property color foreground: token("on_surface", 0, "#f2ecf4")
    readonly property color card: token("surface_container_low", 14, "#221c29")
    readonly property color popover: token("surface_container", 14, "#221c29")
    readonly property color primary: token("primary", 0, "#f1a1ce")
    readonly property color primaryForeground: token("on_primary", 0, "#351929")
    readonly property color secondary: token("secondary_container", 0, "#3a2d42")
    readonly property color muted: token("surface_container_high", 16, "#2c2535")
    readonly property color mutedForeground: token("on_surface_variant", 0, "#b4a9bd")
    readonly property color accent: token("surface_container_highest", 24, "#3a2f45")
    readonly property color border: token("outline_variant", 20, "#3b3343")
    readonly property color input: border
    readonly property color star: "#facc15"

    readonly property string font: fonts.objectAt(0) && fonts.objectAt(0).status === FontLoader.Ready ? fonts.objectAt(0).name : "sans-serif"

    // --radius: 0.9rem, and Tailwind's steps from it.
    readonly property real radius: 14.4
    readonly property real radiusXl: 20

    // ease-bounce (globals.css): cubic-bezier(0.34, 1.56, 0.64, 1).
    readonly property var bounce: [0.34, 1.56, 0.64, 1, 1, 1]
    readonly property int duration: 300

    function token(role, tint, fallback) {
        const hex = roles[role];
        if (!hex)
            return fallback;
        const p = roles["primary"];
        return tint > 0 && p ? Oklab.mix(Qt.color(hex), Qt.color(p), 1 - tint / 100) : hex;
    }

    /// `color` at `alpha` (Tailwind's bg-x/NN).
    function a(color, alpha) {
        return Qt.rgba(color.r, color.g, color.b, alpha);
    }

    function reload() {
        Backend.request("app.palette", {}, p => theme.roles = p && p.dark ? p.dark : {});
    }

    Component.onCompleted: reload()

    Connections {
        target: Backend
        // A new wallpaper, new colors.
        function onEvent(name) {
            if (name === "state-changed" || name === "settings-changed")
                theme.reload();
        }
    }

    // The app's Nunito is one variable font, and Qt only uses its default
    // (light) weight: fixed weights cut from it (fonttools varLib.instancer).
    Instantiator {
        id: fonts
        model: [400, 500, 600, 700, 800, 900]
        FontLoader {
            required property int modelData
            source: Qt.resolvedUrl("../fonts/Nunito-" + modelData + ".ttf")
        }
    }
}
