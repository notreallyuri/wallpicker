// The app's colors: the wallpaper's palette (`wallpicker theme --json`, dark
// half) mapped onto the same tokens as ui/src/palette.rs, each surface
// tinted toward the primary color in oklab like CSS color-mix. Until it's
// read (or without a palette): Blossom dark, the app's built-in theme.
pragma Singleton
import QtQuick
import Quickshell
import Quickshell.Io

Singleton {
    id: theme

    property var roles: ({})
    // For measuring only: WP_NOIMG=1 shows no thumbnails, WP_NOFX=1 no
    // blur, glass or shadows.
    readonly property bool noImages: Quickshell.env("WP_NOIMG") === "1"
    readonly property bool noEffects: Quickshell.env("WP_NOFX") === "1"

    // ui/src/palette.rs TOKENS: token → role, and % of primary mixed in.
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

    readonly property string font: fonts.objectAt(0) && fonts.objectAt(0).status === FontLoader.Ready ? fonts.objectAt(0).name : "sans-serif"

    function token(role, tint, fallback) {
        const hex = roles[role];
        if (!hex)
            return fallback;
        const p = roles["primary"];
        return tint > 0 && p ? mix(hex, p, 1 - tint / 100) : hex;
    }

    // color-mix(in oklab, a w, b): sRGB → linear → oklab, mixed, and back.
    function mix(a, b, w) {
        const A = toOklab(Qt.color(a)), B = toOklab(Qt.color(b));
        return fromOklab([0, 1, 2].map(i => A[i] * w + B[i] * (1 - w)));
    }
    function lin(c) {
        return c <= 0.04045 ? c / 12.92 : Math.pow((c + 0.055) / 1.055, 2.4);
    }
    function gam(c) {
        c = Math.max(0, Math.min(1, c));
        return c <= 0.0031308 ? 12.92 * c : 1.055 * Math.pow(c, 1 / 2.4) - 0.055;
    }
    function toOklab(c) {
        const r = lin(c.r), g = lin(c.g), b = lin(c.b);
        const l = Math.cbrt(0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b);
        const m = Math.cbrt(0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b);
        const s = Math.cbrt(0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b);
        return [0.2104542553 * l + 0.793617785 * m - 0.0040720468 * s, 1.9779984951 * l - 2.428592205 * m + 0.4505937099 * s, 0.0259040371 * l + 0.7827717662 * m - 0.808675766 * s];
    }
    function fromOklab(o) {
        const l = Math.pow(o[0] + 0.3963377774 * o[1] + 0.2158037573 * o[2], 3);
        const m = Math.pow(o[0] - 0.1055613458 * o[1] - 0.0638541728 * o[2], 3);
        const s = Math.pow(o[0] - 0.0894841775 * o[1] - 1.291485548 * o[2], 3);
        return Qt.rgba(gam(4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s), gam(-1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s), gam(-0.0041960863 * l - 0.7034186147 * m + 1.707614701 * s), 1);
    }
    // `color` at `alpha` (Tailwind's bg-x/NN).
    function a(color, alpha) {
        return Qt.rgba(color.r, color.g, color.b, alpha);
    }

    function reload() {
        palette.running = true;
    }

    // The app's Nunito is one variable font, and Qt only uses its default
    // (light) weight: fixed weights cut from it (fonttools varLib.instancer),
    // inside the config folder (Quickshell loads nothing from outside).
    Instantiator {
        id: fonts
        model: [400, 500, 600, 700, 800, 900]
        FontLoader {
            required property int modelData
            source: Qt.resolvedUrl("fonts/Nunito-" + modelData + ".ttf")
        }
    }

    Process {
        id: palette
        running: true
        command: ["wallpicker", "theme", "--json"]
        stdout: StdioCollector {
            onStreamFinished: {
                try {
                    theme.roles = JSON.parse(text).dark || {};
                } catch (e) {
                    theme.roles = {};
                }
            }
        }
    }
}
