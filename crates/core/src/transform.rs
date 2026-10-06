//! Wallpapers shown adjusted (mirrored now; cropped for framing later): no
//! backend can do it, so wallpicker makes the changed copy once, caches it,
//! and applies that file instead. The state and the history keep the
//! original's path; only what's handed to the backend differs.

use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use crate::settings::{Flip, State};
use crate::error::{Error, Result};

fn dir(cache: &Path) -> PathBuf {
    cache.join("transformed")
}

/// A name for `src`'s copies: the same whatever change, so they're easy to
/// find (and drop) together.
fn stem(src: &Path) -> String {
    let mut h = DefaultHasher::new();
    src.hash(&mut h);
    format!("{:016x}", h.finish())
}

/// Where the copy of `src` with `flip` is (or would be).
fn copy_path(cache: &Path, src: &Path, flip: Flip) -> Result<PathBuf> {
    let meta = fs::metadata(src)?;
    let mtime = meta.modified().ok().and_then(|t| t.duration_since(UNIX_EPOCH).ok()).map_or(0, |d| d.as_secs());
    let png = src.extension().is_some_and(|e| e.eq_ignore_ascii_case("png"));
    Ok(dir(cache).join(format!("{}-{mtime}-{}-{}.{}", stem(src), meta.len(), flip.as_str(), if png { "png" } else { "jpg" })))
}

/// The file to show for `src` with `flip`: `src` itself when there's nothing
/// to change, else a cached mirrored copy (made now if needed: PNG stays
/// lossless, other formats become high-quality JPEG).
pub fn shown_file(cache: &Path, src: &Path, flip: Flip) -> Result<PathBuf> {
    if flip.is_none() {
        return Ok(src.to_path_buf());
    }
    let out = copy_path(cache, src, flip)?;
    let png = out.extension().is_some_and(|e| e == "png");
    if out.is_file() {
        return Ok(out);
    }
    let mut img = image::open(src).map_err(|e| Error::Image(format!("{}: {e}", src.display())))?;
    if flip.h {
        img = img.fliph();
    }
    if flip.v {
        img = img.flipv();
    }
    fs::create_dir_all(dir(cache))?;
    let tmp = out.with_extension("tmp");
    let saved = if png {
        img.save_with_format(&tmp, image::ImageFormat::Png)
    } else {
        fs::File::create(&tmp).map_err(image::ImageError::IoError).and_then(|f| {
            let mut enc = image::codecs::jpeg::JpegEncoder::new_with_quality(std::io::BufWriter::new(f), 95);
            enc.encode_image(&img.to_rgb8())
        })
    };
    saved.map_err(|e| Error::Image(e.to_string()))?;
    fs::rename(&tmp, &out)?;
    Ok(out)
}

/// Drop the copies no monitor shows any more.
pub fn prune(cache: &Path, state: &State) {
    let keep: Vec<PathBuf> = state
        .current
        .iter()
        .filter_map(|(m, src)| {
            let flip = state.adjust_for(m, src).flip;
            (!flip.is_none()).then(|| copy_path(cache, src, flip).ok()).flatten()
        })
        .collect();
    if let Ok(entries) = fs::read_dir(dir(cache)) {
        for e in entries.flatten() {
            if !keep.contains(&e.path()) {
                let _ = fs::remove_file(e.path());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mirrors_and_caches() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("a.png");
        // Left half red, right half blue.
        let mut img = image::RgbImage::new(4, 2);
        for (x, _, p) in img.enumerate_pixels_mut() {
            *p = if x < 2 { image::Rgb([255, 0, 0]) } else { image::Rgb([0, 0, 255]) };
        }
        img.save(&src).unwrap();
        let cache = tmp.path().join("cache");

        assert_eq!(shown_file(&cache, &src, Flip::default()).unwrap(), src);
        let h = shown_file(&cache, &src, Flip { h: true, v: false }).unwrap();
        let out = image::open(&h).unwrap().to_rgb8();
        assert_eq!(out.get_pixel(0, 0), &image::Rgb([0, 0, 255]));
        assert_eq!(out.get_pixel(3, 0), &image::Rgb([255, 0, 0]));
        // Cached: the same file again.
        assert_eq!(shown_file(&cache, &src, Flip { h: true, v: false }).unwrap(), h);
        let hv = shown_file(&cache, &src, Flip { h: true, v: true }).unwrap();
        // Pruning keeps what a monitor shows, drops the rest.
        let mut state = State::default();
        state.current.insert("DP-1".into(), src.clone());
        state.adjust.insert("DP-1".into(), crate::settings::Adjust { flip: Flip { h: true, v: true }, ..Default::default() });
        prune(&cache, &state);
        assert!(hv.is_file() && !h.is_file());
        state.adjust.clear();
        prune(&cache, &state);
        assert!(!hv.is_file());
    }
}
