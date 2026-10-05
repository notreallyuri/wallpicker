//! Finding the same wallpaper more than once: exact copies (same bytes,
//! maybe another name) and similar ones (the same picture at another size or
//! saved again), from a 64-bit fingerprint of each image's thumbnail (a
//! "difference hash": is each pixel brighter than its right neighbour, on a
//! 9×8 grey version). Each group comes best first: the suggested one to keep.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// How alike a group's wallpapers are.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Likeness {
    /// Byte-for-byte the same file.
    Exact,
    /// The same picture by its fingerprint (another size, re-saved, ...).
    Similar,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DuplicateGroup {
    pub likeness: Likeness,
    /// Best first (largest, then favorite, then biggest file, then oldest):
    /// the suggested one to keep.
    pub items: Vec<DuplicateItem>,
}

/// One copy, with what helps pick the one to keep.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DuplicateItem {
    pub path: PathBuf,
    pub width: u32,
    pub height: u32,
    /// File size in bytes.
    pub size: u64,
}

impl DuplicateGroup {
    pub fn paths(&self) -> impl Iterator<Item = &PathBuf> {
        self.items.iter().map(|i| &i.path)
    }
}

/// A picture's fingerprint, from its thumbnail: a 256-bit difference hash
/// of a 17×16 grey version (is each pixel brighter than its right
/// neighbour?), and the average colours of a 4×4 grid. The hash alone
/// confuses pictures with the same layout (one figure on a white
/// background); the colours tell them apart.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Print {
    pub bits: [u64; 4],
    /// 16 cells × RGB.
    pub colors: Vec<u8>,
}

/// Same picture: at most this many of the 256 hash bits differ...
/// (Measured on a real library: re-saved and half-size copies came out 0
/// bits and 0 colour apart; the closest different pictures, the same
/// artist's figure on white, 14 bits and 9 colour apart.)
pub const SIMILAR_BITS: u32 = 6;
/// ...the grid's colours differ by at most this much on average...
pub const SIMILAR_COLOR: u32 = 4;
/// ...and the shapes (width / height) by at most this fraction.
pub const SIMILAR_ASPECT: f64 = 0.03;

/// The difference hash of a 17×16 grey image (row by row, 272 values).
pub fn dhash(gray17x16: &[u8]) -> [u64; 4] {
    let mut out = [0u64; 4];
    for y in 0..16 {
        for x in 0..16 {
            let bit = gray17x16[y * 17 + x] > gray17x16[y * 17 + x + 1];
            let n = y * 16 + x;
            out[n / 64] |= u64::from(bit) << (63 - n % 64);
        }
    }
    out
}

pub fn hamming(a: &[u64; 4], b: &[u64; 4]) -> u32 {
    a.iter().zip(b).map(|(x, y)| (x ^ y).count_ones()).sum()
}

/// Average difference between two colour grids (0..=255).
pub fn color_distance(a: &[u8], b: &[u8]) -> u32 {
    if a.len() != b.len() || a.is_empty() {
        return u32::MAX;
    }
    let total: u32 = a.iter().zip(b).map(|(x, y)| u32::from(x.abs_diff(*y))).sum();
    total / a.len() as u32
}

/// A hash that says nothing (a flat image: all black wallpapers would
/// otherwise "match").
fn is_blank(bits: &[u64; 4]) -> bool {
    let ones: u32 = bits.iter().map(|b| b.count_ones()).sum();
    !(8..=248).contains(&ones)
}

/// The same picture? Both fingerprints, and the shapes (width / height, 0 if
/// unknown).
pub fn looks_same(a: &Print, b: &Print, aspect_a: f64, aspect_b: f64) -> bool {
    let shapes = aspect_a > 0.0 && aspect_b > 0.0 && (aspect_a / aspect_b - 1.0).abs() <= SIMILAR_ASPECT;
    shapes
        && !is_blank(&a.bits)
        && !is_blank(&b.bits)
        && hamming(&a.bits, &b.bits) <= SIMILAR_BITS
        && color_distance(&a.colors, &b.colors) <= SIMILAR_COLOR
}

/// Groups among `n` items: `exact[i]` is a content key (None: unknown),
/// `prints[i]` a fingerprint and `aspects[i]` the shape. Exact copies are
/// grouped first; then each picture joins the first group whose first
/// picture it looks like (compared with that one only, so look-alikes can't
/// chain a whole style together). Index lists, unsorted.
pub fn group(exact: &[Option<u64>], prints: &[Option<Print>], aspects: &[f64]) -> Vec<(Likeness, Vec<usize>)> {
    let n = exact.len();
    // Exact copies: one entry each, by content key.
    let mut entries: Vec<Vec<usize>> = Vec::new();
    let mut by_key: std::collections::HashMap<u64, usize> = Default::default();
    for i in 0..n {
        match exact[i].and_then(|k| by_key.get(&k).copied()) {
            Some(e) => entries[e].push(i),
            None => {
                if let Some(k) = exact[i] {
                    by_key.insert(k, entries.len());
                }
                entries.push(vec![i]);
            }
        }
    }
    // Then by look, entry by entry, against each group's first picture.
    let mut groups: Vec<(Vec<usize>, bool)> = Vec::new(); // (members, joined by look)
    for entry in entries {
        let me = entry[0];
        let home = groups.iter().position(|(g, _)| {
            let seed = g[0];
            matches!((&prints[me], &prints[seed]), (Some(a), Some(b)) if looks_same(a, b, aspects[me], aspects[seed]))
        });
        match home {
            Some(h) => {
                groups[h].0.extend(entry);
                groups[h].1 = true;
            }
            None => groups.push((entry, false)),
        }
    }
    groups
        .into_iter()
        .filter(|(g, _)| g.len() > 1)
        .map(|(g, by_look)| (if by_look { Likeness::Similar } else { Likeness::Exact }, g))
        .collect()
}

#[cfg(feature = "native")]
pub use native::find;

#[cfg(feature = "native")]
mod native {
    use std::collections::{BTreeMap, HashMap};
    use std::hash::{Hash, Hasher};
    use std::path::{Path, PathBuf};

    use serde::{Deserialize, Serialize};

    use super::{DuplicateGroup, DuplicateItem, Print, dhash, group};
    use crate::catalog::Catalog;
    use crate::library::LocalImage;

    /// Cached fingerprints, by path, valid while size and time match.
    #[derive(Default, Serialize, Deserialize)]
    struct PrintCache {
        items: BTreeMap<PathBuf, CachedPrint>,
    }

    #[derive(Clone, Serialize, Deserialize)]
    struct CachedPrint {
        size: u64,
        modified: u64,
        print: Print,
    }

    fn content_key(path: &Path) -> Option<u64> {
        let bytes = std::fs::read(path).ok()?;
        // A fixed-key hasher: the same bytes always give the same key.
        let mut h = std::collections::hash_map::DefaultHasher::new();
        bytes.hash(&mut h);
        Some(h.finish())
    }

    fn fingerprint(cache_dir: &Path, path: &Path, thumb_width: u32) -> Option<Print> {
        // From the (cached) thumbnail: decoding a 7680×4320 original for a
        // few hundred pixels would be slow.
        let thumb = crate::thumbs::ensure_thumb(cache_dir, path, thumb_width).ok()?;
        let img = image::open(thumb).ok()?;
        let grey = image::imageops::resize(&img.to_luma8(), 17, 16, image::imageops::FilterType::Triangle);
        let colors = image::imageops::resize(&img.to_rgb8(), 4, 4, image::imageops::FilterType::Triangle);
        Some(Print { bits: dhash(grey.as_raw()), colors: colors.into_raw() })
    }

    /// The duplicate groups among `images`, best first in each.
    /// Fingerprints are cached in `cache_dir` (with the thumbnails).
    pub fn find(images: &[LocalImage], catalog: &Catalog, cache_dir: &Path, thumb_width: u32) -> Vec<DuplicateGroup> {
        // Same bytes needs the same size: only those files are read whole.
        let mut by_size: HashMap<u64, usize> = HashMap::new();
        for i in images {
            *by_size.entry(i.size).or_default() += 1;
        }
        let exact: Vec<Option<u64>> = images
            .iter()
            .map(|i| if by_size[&i.size] > 1 { content_key(&i.path) } else { None })
            .collect();

        let cache_file = cache_dir.join("fingerprints-v2.json");
        let mut cache: PrintCache = std::fs::read_to_string(&cache_file)
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default();
        let prints: Vec<Option<Print>> = images
            .iter()
            .map(|i| {
                if let Some(c) = cache.items.get(&i.path)
                    && c.size == i.size
                    && c.modified == i.modified
                {
                    return Some(c.print.clone());
                }
                let print = fingerprint(cache_dir, &i.path, thumb_width)?;
                cache.items.insert(i.path.clone(), CachedPrint { size: i.size, modified: i.modified, print: print.clone() });
                Some(print)
            })
            .collect();
        let aspects: Vec<f64> = images
            .iter()
            .map(|i| if i.height > 0 { f64::from(i.width) / f64::from(i.height) } else { 0.0 })
            .collect();
        // Forget files that are gone; best effort.
        cache.items.retain(|p, _| images.iter().any(|i| &i.path == p));
        if let Ok(text) = serde_json::to_string(&cache) {
            let _ = std::fs::create_dir_all(cache_dir);
            let _ = std::fs::write(&cache_file, text);
        }

        let mut out: Vec<DuplicateGroup> = group(&exact, &prints, &aspects)
            .into_iter()
            .map(|(likeness, mut idx)| {
                idx.sort_by(|&a, &b| {
                    let (a, b) = (&images[a], &images[b]);
                    let pixels = |i: &LocalImage| u64::from(i.width) * u64::from(i.height);
                    pixels(b)
                        .cmp(&pixels(a))
                        .then(catalog.is_favorite(&b.path).cmp(&catalog.is_favorite(&a.path)))
                        .then(b.size.cmp(&a.size))
                        .then(a.modified.cmp(&b.modified))
                        .then(a.path.cmp(&b.path))
                });
                let items = idx
                    .into_iter()
                    .map(|i| {
                        let im = &images[i];
                        DuplicateItem { path: im.path.clone(), width: im.width, height: im.height, size: im.size }
                    })
                    .collect();
                DuplicateGroup { likeness, items }
            })
            .collect();
        // Exact copies first (the safest to clear), then by first path.
        out.sort_by(|a, b| (a.likeness as u8).cmp(&(b.likeness as u8)).then(a.items[0].path.cmp(&b.items[0].path)));
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn print(bits: [u64; 4], color: u8) -> Print {
        Print { bits, colors: vec![color; 48] }
    }

    #[test]
    fn dhash_compares_neighbours() {
        let falling: Vec<u8> = (0..16).flat_map(|_| (0..17).rev().map(|x| x as u8 * 10)).collect();
        assert_eq!(dhash(&falling), [u64::MAX; 4]);
        let rising: Vec<u8> = (0..16).flat_map(|_| (0..17).map(|x| x as u8 * 10)).collect();
        assert_eq!(dhash(&rising), [0; 4]);
        assert_eq!(hamming(&[0b1011, 0, 0, 1], &[0b0001, 0, 0, 0]), 3);
        assert_eq!(color_distance(&[10, 20], &[20, 0]), 15);
    }

    #[test]
    fn looks_same_needs_hash_colours_and_shape() {
        let a = [0x0f0f_3c3c_5a5a_9696u64, 0x1234_5678_9abc_def0, 0xffff_0000_ffff_0000, 0x0ff0_0ff0_f00f_f00f];
        let near = [a[0] ^ 0b111, a[1], a[2], a[3]];
        let far = [!a[0], !a[1], a[2], a[3]];
        let w = 16.0 / 9.0;
        assert!(looks_same(&print(a, 100), &print(near, 104), w, w));
        assert!(!looks_same(&print(a, 100), &print(far, 100), w, w), "different layout");
        assert!(!looks_same(&print(a, 100), &print(near, 160), w, w), "different colours");
        assert!(!looks_same(&print(a, 100), &print(a, 100), w, 4.0 / 3.0), "different shape");
        assert!(!looks_same(&print([0; 4], 0), &print([0; 4], 0), w, w), "blank says nothing");
    }

    #[test]
    fn groups_exact_copies_and_lookalikes_without_chaining() {
        let a = [0x0f0f_3c3c_5a5a_9696u64, 0x1234_5678_9abc_def0, 0xffff_0000_ffff_0000, 0x0ff0_0ff0_f00f_f00f];
        let step = |n: u32, b: [u64; 4]| [b[0] ^ ((1u64 << n) - 1), b[1], b[2], b[3]];
        let w = 16.0 / 9.0;
        // 0,1 same bytes; 2 looks like 0 (4 bits); 3 looks like 2 (5 bits) but
        // not 0 (9 bits): no chain;
        // 4,5 same bytes, no fingerprint.
        let exact = [Some(1), Some(1), Some(2), Some(3), Some(9), Some(9)];
        let prints = [
            Some(print(a, 100)),
            Some(print(a, 100)),
            Some(print(step(4, a), 100)),
            Some(print(step(9, a), 100)),
            None,
            None,
        ];
        let mut g = group(&exact, &prints, &[w; 6]);
        g.sort_by_key(|(_, idx)| idx[0]);
        assert_eq!(g, vec![(Likeness::Similar, vec![0, 1, 2]), (Likeness::Exact, vec![4, 5])]);
    }

    #[cfg(feature = "native")]
    #[test]
    fn finds_copies_and_resized_versions_best_first() {
        use crate::catalog::Catalog;
        use crate::library::LocalImage;
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path();
        // A picture with some structure (a diagonal gradient with a band).
        let pic = |w: u32, h: u32| {
            image::RgbImage::from_fn(w, h, |x, y| {
                let v = ((x * 255 / w) as u8).wrapping_add((y * 97 / h) as u8);
                let band = if (x * 8 / w) % 3 == 0 { 120 } else { 0 };
                image::Rgb([v, v.wrapping_add(band), 255 - v])
            })
        };
        let other = |w: u32, h: u32| image::RgbImage::from_fn(w, h, |x, y| image::Rgb([((y * 255 / h) as u8), (x % 7 * 30) as u8, 90]));
        pic(1920, 1080).save(dir.join("big.png")).unwrap();
        std::fs::copy(dir.join("big.png"), dir.join("copy.png")).unwrap();
        pic(960, 540).save(dir.join("small.png")).unwrap();
        other(1920, 1080).save(dir.join("other.png")).unwrap();
        let images: Vec<LocalImage> = crate::library::scan(&[dir.to_path_buf()], false);
        let mut catalog = Catalog::default();
        catalog.set_favorite(&dir.join("copy.png"), true);

        let cache = dir.join("cache");
        let groups = find(&images, &catalog, &cache, 120);
        assert_eq!(groups.len(), 1, "{groups:?}");
        let g = &groups[0];
        assert_eq!(g.likeness, Likeness::Similar);
        let names: Vec<_> = g.paths().map(|p| p.file_name().unwrap().to_str().unwrap()).collect();
        // Same size: the favorite first; the smaller version last.
        assert_eq!(names, ["copy.png", "big.png", "small.png"]);
        // The second run uses the cached fingerprints.
        assert!(cache.join("fingerprints-v2.json").is_file());
        assert_eq!(find(&images, &catalog, &cache, 120), groups);
    }
}
