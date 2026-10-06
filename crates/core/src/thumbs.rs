//! Cached JPEG thumbnails for the local grid.
//!
//! The grid would otherwise load multi-megabyte originals just to draw a
//! 360-pixel tile. A thumbnail is keyed on the source path, its size and
//! modification time, and the requested width, so editing or replacing a file
//! produces a fresh one.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use crate::error::{Error, Result};

/// FNV-1a. Not cryptographic; only used to name cache files, and unlike
/// `DefaultHasher` its output doesn't change between Rust versions.
fn fnv1a(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for b in bytes {
        hash ^= u64::from(*b);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

/// Where the thumbnail for this exact file state lives.
pub fn thumb_path(cache_dir: &Path, src: &Path, mtime: u64, size: u64, width: u32) -> PathBuf {
    let key = format!("{}|{mtime}|{size}|{width}", src.to_string_lossy());
    cache_dir
        .join("thumbs")
        .join(format!("{:016x}.jpg", fnv1a(key.as_bytes())))
}

/// Return the cached thumbnail for `src`, creating it if needed.
pub fn ensure_thumb(cache_dir: &Path, src: &Path, width: u32) -> Result<PathBuf> {
    let meta = fs::metadata(src)?;
    if !meta.is_file() {
        return Err(Error::Invalid(format!("not a file: {}", src.display())));
    }
    let mtime = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let out = thumb_path(cache_dir, src, mtime, meta.len(), width);
    if out.is_file() {
        return Ok(out);
    }

    let img = image::open(src).map_err(|e| Error::Image(format!("{}: {e}", src.display())))?;
    let rgb = img.thumbnail(width, width).to_rgb8();

    fs::create_dir_all(out.parent().unwrap_or(cache_dir))?;
    // Write elsewhere first so a concurrent reader never sees a partial file.
    let tmp = out.with_extension("jpg.tmp");
    rgb.save_with_format(&tmp, image::ImageFormat::Jpeg)
        .map_err(|e| Error::Image(e.to_string()))?;
    fs::rename(&tmp, &out)?;
    Ok(out)
}

/// `src` fitted inside `max`×`max` (a cached JPEG, as [`ensure_thumb`]), or
/// `src` itself when it already fits: never made bigger.
pub fn fit_within(cache_dir: &Path, src: &Path, max: u32) -> Result<PathBuf> {
    let (w, h) = image::image_dimensions(src).map_err(|e| Error::Image(format!("{}: {e}", src.display())))?;
    if w <= max && h <= max {
        return Ok(src.to_path_buf());
    }
    ensure_thumb(cache_dir, src, max)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_png(path: &Path, w: u32, h: u32) {
        image::RgbImage::from_pixel(w, h, image::Rgb([200, 80, 40]))
            .save(path)
            .unwrap();
    }

    #[test]
    fn creates_a_smaller_cached_jpeg() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("big.png");
        write_png(&src, 1600, 900);
        let cache = tmp.path().join("cache");

        let thumb = ensure_thumb(&cache, &src, 320).unwrap();
        assert!(thumb.is_file());
        assert_eq!(thumb.extension().unwrap(), "jpg");

        let (w, h) = image::image_dimensions(&thumb).unwrap();
        assert_eq!(w, 320);
        assert_eq!(h, 180);
    }

    #[test]
    fn fit_within_shrinks_big_and_keeps_small() {
        let tmp = tempfile::tempdir().unwrap();
        let cache = tmp.path().join("cache");
        let big = tmp.path().join("big.png");
        write_png(&big, 2400, 1350);
        let fitted = fit_within(&cache, &big, 1600).unwrap();
        assert_eq!(image::image_dimensions(&fitted).unwrap(), (1600, 900));
        let small = tmp.path().join("small.png");
        write_png(&small, 800, 600);
        assert_eq!(fit_within(&cache, &small, 1600).unwrap(), small);
    }

    #[test]
    fn second_call_reuses_the_cache() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("a.png");
        write_png(&src, 800, 600);
        let cache = tmp.path().join("cache");

        let first = ensure_thumb(&cache, &src, 200).unwrap();
        let stamp = fs::metadata(&first).unwrap().modified().unwrap();
        let second = ensure_thumb(&cache, &src, 200).unwrap();
        assert_eq!(first, second);
        assert_eq!(fs::metadata(&second).unwrap().modified().unwrap(), stamp);
    }

    #[test]
    fn different_widths_get_different_files() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("a.png");
        write_png(&src, 800, 600);
        let cache = tmp.path().join("cache");
        let a = ensure_thumb(&cache, &src, 200).unwrap();
        let b = ensure_thumb(&cache, &src, 300).unwrap();
        assert_ne!(a, b);
    }

    #[test]
    fn cache_key_changes_with_file_state() {
        let c = Path::new("/c");
        let s = Path::new("/a.png");
        assert_ne!(thumb_path(c, s, 1, 10, 100), thumb_path(c, s, 2, 10, 100));
        assert_ne!(thumb_path(c, s, 1, 10, 100), thumb_path(c, s, 1, 11, 100));
        assert_eq!(thumb_path(c, s, 1, 10, 100), thumb_path(c, s, 1, 10, 100));
    }

    #[test]
    fn corrupt_images_and_missing_files_are_errors() {
        let tmp = tempfile::tempdir().unwrap();
        let cache = tmp.path().join("cache");

        let bad = tmp.path().join("bad.png");
        fs::write(&bad, b"this is not a png").unwrap();
        assert!(matches!(
            ensure_thumb(&cache, &bad, 200),
            Err(Error::Image(_))
        ));

        assert!(ensure_thumb(&cache, &tmp.path().join("nope.png"), 200).is_err());
        assert!(ensure_thumb(&cache, tmp.path(), 200).is_err());
    }
}
