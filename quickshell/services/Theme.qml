// The app's look: colors, the font, radii and motion. The colors are a
// color theme (Themes.js: Blossom, Matcha...; light or dark, following the
// system or not), or, with "Colors from the wallpaper" on, the wallpaper's
// palette mapped onto the same tokens as ui/src/palette.rs (surfaces tinted
// toward the primary color in oklch, like CSS color-mix). Oklab math is in
// Oklab.js.
pragma Singleton
import QtQuick
import Quickshell
import "Oklab.js" as Oklab
import "Themes.js" as Themes

Singleton {
    id: theme

    /// The wallpaper's palette: {light: roles, dark: roles}, or null.
    property var palette: null

    readonly property bool dark: Prefs.darkMode === "dark" || (Prefs.darkMode !== "light" && Qt.styleHints.colorScheme !== Qt.ColorScheme.Light)
    readonly property var base: (Themes.themes[Prefs.colorTheme] || Themes.themes.blossom)[dark ? "dark" : "light"]
    readonly property bool fromWallpaper: Settings.data.theme_from_wallpaper !== false
    readonly property var roles: fromWallpaper && palette ? (palette[dark ? "dark" : "light"] || {}) : ({})

    // ui/src/palette.rs TOKENS: token, role, % of primary mixed in.
    readonly property color background: token("surface", 12, "background")
    readonly property color foreground: token("on_surface", 0, "foreground")
    readonly property color card: token("surface_container_low", 14, "card")
    readonly property color popover: token("surface_container", 14, "popover")
    readonly property color primary: token("primary", 0, "primary")
    readonly property color primaryForeground: token("on_primary", 0, "primaryForeground")
    readonly property color secondary: token("secondary_container", 0, "secondary")
    readonly property color muted: token("surface_container_high", 16, "muted")
    readonly property color mutedForeground: token("on_surface_variant", 0, "mutedForeground")
    readonly property color accent: token("surface_container_highest", 24, "accent")
    readonly property color border: token("outline_variant", 20, "border")
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
            return base[fallback];
        const p = roles["primary"];
        return tint > 0 && p ? Oklab.mix(Qt.color(hex), Qt.color(p), 1 - tint / 100) : hex;
    }

    /// `color` at `alpha` (Tailwind's bg-x/NN).
    function a(color, alpha) {
        return Qt.rgba(color.r, color.g, color.b, alpha);
    }

    function reload() {
        Backend.request("app.palette", {}, p => theme.palette = p || null);
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
