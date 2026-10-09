//! Wallpapers shown adjusted (mirrored now; cropped for framing later): no
//! backend can do it, so wallpicker makes the changed copy once, caches it,
//! and applies that file instead. The state and the history keep the
//! original's path; only what's handed to the backend differs.

use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use fast_image_resize::{FilterType, ResizeAlg, ResizeOptions, Resizer};
use image::DynamicImage;

use crate::settings::{Adjust, State};
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

/// Whether `adjust` needs a changed copy (the fit is the backend's).
fn changes_pixels(adjust: &Adjust) -> bool {
    !adjust.flip.is_none() || adjust.frame.is_some() || adjust.span.is_some()
}

/// Where the copy of `src` with `adjust` is (or would be): its name says
/// what was done, so a cached one is found again.
fn copy_path(cache: &Path, src: &Path, adjust: &Adjust) -> Result<PathBuf> {
    let meta = fs::metadata(src)?;
    let mtime = meta.modified().ok().and_then(|t| t.duration_since(UNIX_EPOCH).ok()).map_or(0, |d| d.as_secs());
    let png = src.extension().is_some_and(|e| e.eq_ignore_ascii_case("png"));
    let frame = match (adjust.span, adjust.frame) {
        (Some(c), _) => format!("-s{}-{}-{}x{}-{}x{}", c.x, c.y, c.width, c.height, c.out_width, c.out_height),
        (None, Some(f)) => format!("-z{}x{}y{}-{}x{}", f.zoom, f.x, f.y, f.width, f.height),
        (None, None) => String::new(),
    };
    Ok(dir(cache).join(format!(
        "{}-{mtime}-{}-{}{frame}.{}",
        stem(src),
        meta.len(),
        adjust.flip.as_str(),
        if png { "png" } else { "jpg" }
    )))
}

/// The file to show for `src` with `adjust`: `src` itself when nothing
/// changes its pixels, else a cached copy (made now if needed): framed (cut
/// and scaled to the monitor's size), then mirrored. PNG stays lossless,
/// other formats become high-quality JPEG.
pub fn shown_file(cache: &Path, src: &Path, adjust: &Adjust) -> Result<PathBuf> {
    if !changes_pixels(adjust) {
        return Ok(src.to_path_buf());
    }
    let out = copy_path(cache, src, adjust)?;
    let png = out.extension().is_some_and(|e| e == "png");
    if out.is_file() {
        return Ok(out);
    }
    let mut img = image::open(src).map_err(|e| Error::Image(format!("{}: {e}", src.display())))?;
    if let Some(cut) = adjust.span {
        let (iw, ih) = (img.width(), img.height());
        let (x, y) = (cut.x.min(iw.saturating_sub(1)), cut.y.min(ih.saturating_sub(1)));
        let (w, h) = (cut.width.clamp(1, iw - x), cut.height.clamp(1, ih - y));
        img = cut_and_scale(&img, (x, y, w, h), cut.out_width.max(1), cut.out_height.max(1));
    } else if let Some(frame) = adjust.frame {
        let (x, y, w, h) = frame.rect(img.width(), img.height());
        img = cut_and_scale(&img, (x, y, w, h), frame.width, frame.height);
    }
    let flip = adjust.flip;
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

/// `img`'s `(x, y, w, h)` scaled to `width`×`height`, its pixel type kept
/// (alpha, 16 bits): Lanczos3 with SIMD, as compose.rs (an 8K picture's
/// cut in a tenth of the time); image's own for the types
/// fast_image_resize doesn't take (floats).
fn cut_and_scale(img: &DynamicImage, (x, y, w, h): (u32, u32, u32, u32), width: u32, height: u32) -> DynamicImage {
    let mut out = DynamicImage::new(width, height, img.color());
    let options = ResizeOptions::new()
        .resize_alg(ResizeAlg::Convolution(FilterType::Lanczos3))
        .crop(f64::from(x), f64::from(y), f64::from(w), f64::from(h));
    match Resizer::new().resize(img, &mut out, &options) {
        Ok(()) => out,
        Err(_) => img.crop_imm(x, y, w, h).resize_exact(width, height, image::imageops::FilterType::Lanczos3),
    }
}

/// Drop the copies no monitor shows any more.
pub fn prune(cache: &Path, state: &State) {
    let keep: Vec<PathBuf> = state
        .current
        .iter()
        .filter_map(|(m, src)| {
            let adjust = state.adjust_for(m, src);
            changes_pixels(&adjust).then(|| copy_path(cache, src, &adjust).ok()).flatten()
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
    use crate::settings::Flip;

    #[test]
    fn frames_to_the_monitor_size() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("b.png");
        // Left half red, right half blue, 400×300.
        let mut img = image::RgbImage::new(400, 300);
        for (x, _, p) in img.enumerate_pixels_mut() {
            *p = if x < 200 { image::Rgb([255, 0, 0]) } else { image::Rgb([0, 0, 255]) };
        }
        img.save(&src).unwrap();
        let cache = tmp.path().join("cache");
        // Zoomed into the right quarter: all blue, at the monitor's size.
        let frame = crate::settings::Frame { zoom: 400, x: 900, y: 500, width: 160, height: 90 };
        let out = shown_file(&cache, &src, &Adjust { frame: Some(frame), ..Default::default() }).unwrap();
        let out = image::open(&out).unwrap().to_rgb8();
        assert_eq!(out.dimensions(), (160, 90));
        assert_eq!(out.get_pixel(0, 45), &image::Rgb([0, 0, 255]));
        assert_eq!(out.get_pixel(159, 45), &image::Rgb([0, 0, 255]));
    }

    #[test]
    fn scaling_keeps_the_pixel_type() {
        // 16 bits with alpha: the fast path, the type kept.
        let rgba16 = DynamicImage::ImageRgba16(image::ImageBuffer::from_pixel(40, 20, image::Rgba([0, 0, 65535, 32768])));
        let out = cut_and_scale(&rgba16, (10, 0, 20, 20), 8, 8);
        assert_eq!((out.width(), out.height(), out.color()), (8, 8, image::ColorType::Rgba16));
        let p = out.to_rgba16().get_pixel(4, 4).0;
        assert!(p[2] > 65000 && p[3].abs_diff(32768) < 300, "{p:?}");
        // Floats: image's own scaling.
        let float = DynamicImage::ImageRgb32F(image::ImageBuffer::from_pixel(40, 20, image::Rgb([1.0, 0.0, 0.0])));
        let out = cut_and_scale(&float, (0, 0, 40, 20), 10, 5);
        assert_eq!((out.width(), out.height(), out.color()), (10, 5, image::ColorType::Rgb32F));
    }

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

        let with = |h, v| Adjust { flip: Flip { h, v }, ..Default::default() };
        assert_eq!(shown_file(&cache, &src, &with(false, false)).unwrap(), src);
        let h = shown_file(&cache, &src, &with(true, false)).unwrap();
        let out = image::open(&h).unwrap().to_rgb8();
        assert_eq!(out.get_pixel(0, 0), &image::Rgb([0, 0, 255]));
        assert_eq!(out.get_pixel(3, 0), &image::Rgb([255, 0, 0]));
        // Cached: the same file again.
        assert_eq!(shown_file(&cache, &src, &with(true, false)).unwrap(), h);
        let hv = shown_file(&cache, &src, &with(true, true)).unwrap();
        // Pruning keeps what a monitor shows, drops the rest.
        let mut state = State::default();
        state.current.insert("DP-1".into(), src.clone());
        state.adjust.insert("DP-1".into(), with(true, true));
        prune(&cache, &state);
        assert!(hv.is_file() && !h.is_file());
        state.adjust.clear();
        prune(&cache, &state);
        assert!(!hv.is_file());
    }
}
