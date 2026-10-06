//! What colors a wallpaper has, for filtering by color: the share of the
//! picture in each of eight hues, how light it is overall, and how much of
//! it is colorful at all. Made from a small copy of the image (its cached
//! thumbnail, scaled down), and cached per file.

use serde::{Deserialize, Serialize};

/// The hues, in order, with labels. A pixel counts for one when it's
/// colorful enough (not grey, not near black or white).
pub const HUES: [(&str, &str); 8] = [
    ("red", "Red"),
    ("orange", "Orange"),
    ("yellow", "Yellow"),
    ("green", "Green"),
    ("teal", "Teal"),
    ("blue", "Blue"),
    ("purple", "Purple"),
    ("pink", "Pink"),
];

/// The other filters: by lightness, warmth, and pictures with little color
/// (mostly white, grey or black, maybe a small colored subject).
pub const TONES: [(&str, &str); 4] = [("dark", "Dark"), ("light", "Light"), ("warm", "Warm"), ("muted", "Muted")];

/// Reference colors (sRGB hex) for each hue, then shades that aren't a
/// color here: skin, brown and beige. Each pixel takes the name of the
/// nearest reference (in OKLab, where distance follows what people see), so
/// gold is yellow, light and hot pinks are pink, and skin, wood and warm
/// light don't make a picture "red" or "orange". Tuned on a real library.
const REFERENCES: [(usize, &[&str]); 11] = [
    (0, &["#d01c1c", "#ff2a2a", "#8b0000", "#e0303c", "#b22222", "#ff4040"]),
    (1, &["#ff7f00", "#f28c28", "#ff9a3c", "#e06000", "#ffa64d"]),
    (2, &["#ffd700", "#ffe135", "#f5c518", "#ffeb80", "#d4a017", "#ffcc33", "#e8b923"]),
    (3, &["#2ecc40", "#7cfc00", "#228b22", "#a8e6a1", "#9acd32", "#3cb371", "#556b2f"]),
    (4, &["#20b2aa", "#40e0d0", "#008080", "#7fd8d0", "#2f8f8f"]),
    (5, &["#1e90ff", "#4169e1", "#0b3d91", "#87ceeb", "#6495ed", "#1a237e", "#5dade2"]),
    (6, &["#8a2be2", "#9370db", "#4b0082", "#b19cd9", "#6a0dad", "#5b2a86", "#7b3fa0"]),
    (7, &["#ff69b4", "#ff1493", "#ffb6c1", "#f4a6c8", "#e75480", "#ff8fab", "#d63384", "#f9c6d8", "#c2185b", "#ff77aa"]),
    // Skin.
    (SKIN, &["#f1c27d", "#e0ac69", "#c68642", "#ffdbac", "#f5cba7", "#d9a38b", "#eac0a8", "#f0b8a0", "#e8a090", "#c88a78", "#8d5524"]),
    // Brown.
    (SKIN, &["#8b5a2b", "#a0522d", "#6f4e37", "#4a2c1d", "#a67b5b"]),
    // Beige.
    (NEUTRAL, &["#e8d8c3", "#f5f0e1", "#d2c4b0"]),
];
/// Not a hue: skin and brown (warm), beige (neither).
const SKIN: usize = 8;
const NEUTRAL: usize = 9;

/// A hue counts when it's this share of the picture, or (for a small
/// subject) this smaller share that's most of what's colorful.
pub const HUE_SHARE: f32 = 0.10;
const SUBJECT_SHARE: f32 = 0.04;
const SUBJECT_OF_COLOR: f32 = 0.35;
/// Below this OKLab chroma a pixel is grey; below this lightness, black.
const CHROMA: f32 = 0.05;
const BLACK: f32 = 0.2;
const DARK: f32 = 0.33;
const LIGHT: f32 = 0.66;
const MUTED: f32 = 0.10;
const WARM: f32 = 0.40;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ColorProfile {
    /// Share of the picture (0..1) in each of [`HUES`].
    pub hues: [f32; 8],
    /// Mean lightness (0 black .. 1 white).
    pub lightness: f32,
    /// Share that's one of the hues.
    pub colorful: f32,
    /// Share with any color at all (skin and brown too).
    pub chromatic: f32,
    /// Share that's warm: skin, brown, red, orange or yellow.
    pub warm: f32,
}

fn srgb_to_linear(c: u8) -> f32 {
    let c = f32::from(c) / 255.0;
    if c <= 0.04045 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) }
}

/// OKLab (L, a, b) of an sRGB color.
fn oklab(r: u8, g: u8, b: u8) -> [f32; 3] {
    let (r, g, b) = (srgb_to_linear(r), srgb_to_linear(g), srgb_to_linear(b));
    let l = (0.412_221_46 * r + 0.536_332_55 * g + 0.051_445_995 * b).cbrt();
    let m = (0.211_903_5 * r + 0.680_699_5 * g + 0.107_396_96 * b).cbrt();
    let s = (0.088_302_46 * r + 0.281_718_85 * g + 0.629_978_7 * b).cbrt();
    [
        0.210_454_26 * l + 0.793_617_8 * m - 0.004_072_047 * s,
        1.977_998_5 * l - 2.428_592_2 * m + 0.450_593_7 * s,
        0.025_904_037 * l + 0.782_771_77 * m - 0.808_675_77 * s,
    ]
}

fn hex(h: &str) -> (u8, u8, u8) {
    let v = |i: usize| u8::from_str_radix(&h[i..i + 2], 16).unwrap_or(0);
    (v(1), v(3), v(5))
}

/// The references in OKLab, with what they stand for.
static LABS: std::sync::LazyLock<Vec<(usize, [f32; 3])>> = std::sync::LazyLock::new(|| {
    REFERENCES
        .iter()
        .flat_map(|(class, hexes)| {
            hexes.iter().map(move |h| {
                let (r, g, b) = hex(h);
                (*class, oklab(r, g, b))
            })
        })
        .collect()
});

/// The nearest reference's class. Lightness counts half: a darker or
/// lighter shade of a color is still that color.
fn nearest(lab: [f32; 3]) -> usize {
    let d = |x: &[f32; 3]| 0.5 * (x[0] - lab[0]).powi(2) + (x[1] - lab[1]).powi(2) + (x[2] - lab[2]).powi(2);
    LABS.iter().min_by(|a, b| d(&a.1).total_cmp(&d(&b.1))).map_or(NEUTRAL, |(c, _)| *c)
}

/// The profile of RGB pixels (`[r, g, b, r, g, b, ...]`).
pub fn profile(rgb: &[u8]) -> ColorProfile {
    let n = rgb.len() / 3;
    if n == 0 {
        return ColorProfile::default();
    }
    let (mut hues, mut colorful, mut chromatic, mut warm) = ([0usize; 8], 0usize, 0usize, 0usize);
    let mut light_sum = 0.0f32;
    for px in rgb.chunks_exact(3) {
        let (r, g, b) = (px[0], px[1], px[2]);
        let (max, min) = (r.max(g).max(b), r.min(g).min(b));
        light_sum += (f32::from(max) + f32::from(min)) / 510.0;
        let lab = oklab(r, g, b);
        if lab[1].hypot(lab[2]) < CHROMA || lab[0] < BLACK {
            continue;
        }
        chromatic += 1;
        match nearest(lab) {
            i @ 0..=7 => {
                hues[i] += 1;
                colorful += 1;
                if i <= 2 {
                    warm += 1;
                }
            }
            SKIN => warm += 1,
            _ => {}
        }
    }
    let share = |c: usize| round3(c as f32 / n as f32);
    ColorProfile {
        hues: hues.map(share),
        lightness: round3(light_sum / n as f32),
        colorful: share(colorful),
        chromatic: share(chromatic),
        warm: share(warm),
    }
}

/// Three decimals: the cache stays small and readable.
fn round3(x: f32) -> f32 {
    (x * 1000.0).round() / 1000.0
}

/// How strongly `p` is `key` (a hue or a tone), for sorting: higher first.
pub fn score(p: &ColorProfile, key: &str) -> f32 {
    if let Some(i) = HUES.iter().position(|(k, _)| *k == key) {
        return p.hues[i];
    }
    match key {
        "dark" => 1.0 - p.lightness,
        "light" => p.lightness,
        "warm" => p.warm,
        "muted" => 1.0 - p.chromatic,
        _ => 0.0,
    }
}

/// Whether `p` is `key` enough to be shown for it.
pub fn matches(p: &ColorProfile, key: &str) -> bool {
    if HUES.iter().any(|(k, _)| *k == key) {
        let share = score(p, key);
        return share >= HUE_SHARE || (share >= SUBJECT_SHARE && share >= SUBJECT_OF_COLOR * p.colorful);
    }
    match key {
        "dark" => p.lightness <= DARK,
        "light" => p.lightness >= LIGHT,
        "warm" => p.warm >= WARM,
        "muted" => p.chromatic <= MUTED,
        _ => false,
    }
}

#[cfg(feature = "native")]
mod native {
    use std::collections::BTreeMap;
    use std::path::{Path, PathBuf};

    use serde::{Deserialize, Serialize};

    use super::{ColorProfile, profile};
    use crate::library::LocalImage;

    #[derive(Default, Serialize, Deserialize)]
    struct Cache {
        items: BTreeMap<PathBuf, Cached>,
    }

    #[derive(Clone, Serialize, Deserialize)]
    struct Cached {
        size: u64,
        modified: u64,
        colors: ColorProfile,
    }

    fn compute(cache_dir: &Path, path: &Path, thumb_width: u32) -> Option<ColorProfile> {
        // From the (cached) thumbnail, scaled to ~3k pixels: plenty for shares.
        let thumb = crate::thumbs::ensure_thumb(cache_dir, path, thumb_width).ok()?;
        let img = image::open(thumb).ok()?;
        let small = img.resize(64, 64, image::imageops::FilterType::Triangle).to_rgb8();
        Some(profile(small.as_raw()))
    }

    /// Every image's colors, computed once and cached in `cache_dir` while
    /// the file's size and time stay the same.
    pub fn profiles(images: &[LocalImage], cache_dir: &Path, thumb_width: u32) -> BTreeMap<PathBuf, ColorProfile> {
        let cache_file = cache_dir.join("colors-v4.json");
        let mut cache: Cache = std::fs::read_to_string(&cache_file)
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default();
        let mut changed = false;
        let mut out = BTreeMap::new();
        for i in images {
            if let Some(c) = cache.items.get(&i.path)
                && c.size == i.size
                && c.modified == i.modified
            {
                out.insert(i.path.clone(), c.colors.clone());
                continue;
            }
            if let Some(colors) = compute(cache_dir, &i.path, thumb_width) {
                cache.items.insert(i.path.clone(), Cached { size: i.size, modified: i.modified, colors: colors.clone() });
                out.insert(i.path.clone(), colors);
                changed = true;
            }
        }
        let before = cache.items.len();
        cache.items.retain(|p, _| out.contains_key(p));
        if (changed || cache.items.len() != before)
            && let Ok(text) = serde_json::to_string(&cache)
        {
            let _ = std::fs::create_dir_all(cache_dir);
            let _ = std::fs::write(&cache_file, text);
        }
        out
    }
}

#[cfg(feature = "native")]
pub use native::profiles;

#[cfg(test)]
mod tests {
    use super::*;

    fn solid(r: u8, g: u8, b: u8, n: usize) -> Vec<u8> {
        [r, g, b].repeat(n)
    }

    #[test]
    fn hues_and_tones() {
        let blue = profile(&solid(30, 80, 220, 100));
        assert!(matches(&blue, "blue") && !matches(&blue, "red"));

        // Mostly dark grey with a pink fifth: dark, and pink, not muted.
        let mut px = solid(25, 25, 30, 80);
        px.extend(solid(240, 90, 170, 20));
        let p = profile(&px);
        assert!(matches(&p, "pink"), "{p:?}");
        assert!(matches(&p, "dark"));
        assert!(!matches(&p, "muted"));

        let paper = profile(&solid(245, 245, 240, 50));
        assert!(matches(&paper, "light") && matches(&paper, "muted"));
        assert!(!HUES.iter().any(|(k, _)| matches(&paper, k)));

        // Skin and wood aren't colors, but they're warm.
        let skin = profile(&solid(225, 160, 130, 10));
        assert!(!HUES.iter().any(|(k, _)| matches(&skin, k)), "{skin:?}");
        assert!(matches(&skin, "warm") && !matches(&skin, "muted"));
        // Gold is yellow; light pink is pink, not red; deep red is red.
        assert!(matches(&profile(&solid(230, 180, 30, 10)), "yellow"));
        assert!(matches(&profile(&solid(255, 182, 193, 10)), "pink"));
        assert!(matches(&profile(&solid(170, 20, 25, 10)), "red"));
        assert!(score(&blue, "blue") > score(&p, "blue"));

        // A small subject that's most of the color counts.
        let mut px = solid(240, 240, 240, 94);
        px.extend(solid(255, 215, 0, 6));
        assert!(matches(&profile(&px), "yellow"));
    }
}
