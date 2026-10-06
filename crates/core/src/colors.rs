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

/// The other filters: by lightness, and pictures with little color (mostly
/// white, grey or black, maybe a small colored subject).
pub const TONES: [(&str, &str); 3] = [("dark", "Dark"), ("light", "Light"), ("muted", "Muted")];

/// Hue ranges in degrees (upper bounds), red wrapping around 0.
/// Pink runs up to 350°: pink hair (~345°) isn't red.
const HUE_ENDS: [f32; 8] = [15.0, 40.0, 70.0, 160.0, 195.0, 255.0, 290.0, 350.0];

/// When a pixel counts as colorful: some distance from grey (chroma, max -
/// min channel) and saturated for its brightness (chroma / max). HSL
/// saturation runs high for pale colors; with these, pale skin (about
/// 0.25 saturated) doesn't make a picture "red" or "orange", while pastel
/// and dark colors still count. Tuned on a real library.
const CHROMA: f32 = 0.08;
const SATURATION: f32 = 0.27;

/// A hue counts when at least this share of the picture has it.
pub const HUE_SHARE: f32 = 0.12;
const DARK: f32 = 0.33;
const LIGHT: f32 = 0.66;
const MUTED: f32 = 0.08;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ColorProfile {
    /// Share of the picture (0..1) in each of [`HUES`].
    pub hues: [f32; 8],
    /// Mean lightness (0 black .. 1 white).
    pub lightness: f32,
    /// Share of the picture that's colorful at all.
    pub colorful: f32,
}

/// The profile of RGB pixels (`[r, g, b, r, g, b, ...]`).
pub fn profile(rgb: &[u8]) -> ColorProfile {
    let n = rgb.len() / 3;
    if n == 0 {
        return ColorProfile::default();
    }
    let mut hues = [0usize; 8];
    let mut colorful = 0usize;
    let mut light_sum = 0.0f32;
    for px in rgb.chunks_exact(3) {
        let (h, chroma, saturation, l) = hue_chroma_lightness(px[0], px[1], px[2]);
        light_sum += l;
        if chroma >= CHROMA && saturation >= SATURATION && (0.12..=0.92).contains(&l) {
            colorful += 1;
            let i = HUE_ENDS.iter().position(|end| h < *end).unwrap_or(0);
            hues[i] += 1;
        }
    }
    let share = |c: usize| round3(c as f32 / n as f32);
    ColorProfile {
        hues: hues.map(share),
        lightness: round3(light_sum / n as f32),
        colorful: share(colorful),
    }
}

/// Three decimals: the cache stays small and readable.
fn round3(x: f32) -> f32 {
    (x * 1000.0).round() / 1000.0
}

/// Hue (degrees); chroma, saturation (chroma / max) and lightness (0..1).
fn hue_chroma_lightness(r: u8, g: u8, b: u8) -> (f32, f32, f32, f32) {
    let (r, g, b) = (f32::from(r) / 255.0, f32::from(g) / 255.0, f32::from(b) / 255.0);
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (max + min) / 2.0;
    let d = max - min;
    if d < f32::EPSILON {
        return (0.0, 0.0, 0.0, l);
    }
    let h = if max == r {
        60.0 * (((g - b) / d).rem_euclid(6.0))
    } else if max == g {
        60.0 * ((b - r) / d + 2.0)
    } else {
        60.0 * ((r - g) / d + 4.0)
    };
    (h, d, d / max, l)
}

/// How strongly `p` is `key` (a hue or a tone), for sorting: higher first.
pub fn score(p: &ColorProfile, key: &str) -> f32 {
    if let Some(i) = HUES.iter().position(|(k, _)| *k == key) {
        return p.hues[i];
    }
    match key {
        "dark" => 1.0 - p.lightness,
        "light" => p.lightness,
        "muted" => 1.0 - p.colorful,
        _ => 0.0,
    }
}

/// Whether `p` is `key` enough to be shown for it.
pub fn matches(p: &ColorProfile, key: &str) -> bool {
    if HUES.iter().any(|(k, _)| *k == key) {
        return score(p, key) >= HUE_SHARE;
    }
    match key {
        "dark" => p.lightness <= DARK,
        "light" => p.lightness >= LIGHT,
        "muted" => p.colorful <= MUTED,
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
        let cache_file = cache_dir.join("colors-v3.json");
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
        assert!(blue.colorful > 0.99);

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

        // Skin isn't a color here; pink hair (~345°) is pink, not red.
        assert!(!profile(&solid(240, 200, 180, 10)).hues.iter().any(|s| *s > 0.0));
        assert!(matches(&profile(&solid(240, 120, 160, 10)), "pink"));
        // Red wraps around 0°: 352° and 5° are both red.
        assert!(matches(&profile(&solid(220, 20, 50, 10)), "red"));
        assert!(matches(&profile(&solid(220, 40, 20, 10)), "red"));
        assert!(score(&blue, "blue") > score(&p, "blue"));
    }
}
