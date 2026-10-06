// CSS color-mix(in oklab, a w, b) for Theme: sRGB -> linear -> oklab, mixed,
// and back.
.pragma library

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

/// `a` with weight `w` (0-1), the rest `b`.
function mix(a, b, w) {
    const A = toOklab(a), B = toOklab(b);
    return fromOklab([0, 1, 2].map(i => A[i] * w + B[i] * (1 - w)));
}
