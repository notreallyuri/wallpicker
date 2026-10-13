//! What wallpicker's own renderer (the wallpicker backend) puts on a
//! monitor: the picture fitted into the output's pixels, then as `wl_shm`'s
//! XRGB8888 (each pixel B, G, R, X in memory).

use std::path::Path;

use fast_image_resize::{FilterType, ResizeAlg, ResizeOptions, Resizer};
use image::Rgb;
/// A decoded picture, as [`open`] gives it.
pub use image::RgbImage;

use crate::error::{Error, Result};
use crate::settings::FitMode;

/// The picture at `path`, decoded (what [`compose`] fits; the renderer keeps
/// it a moment, for the next monitor).
pub fn open(path: &Path) -> Result<RgbImage> {
    let img = image::ImageReader::open(path)?
        .with_guessed_format()?
        .decode()
        .map_err(|e| Error::Image(format!("{}: {e}", path.display())))?;
    Ok(img.into_rgb8())
}

/// `src` as `width`×`height` pixels, fitted as `fit` says (none: Cover, the
/// renderer's default). Contain's bars are black; Tile repeats the picture
/// at its own size from the top left. A picture already the output's size
/// (framed and spanned copies are) is used as it is.
pub fn compose(src: &RgbImage, width: u32, height: u32, fit: Option<FitMode>) -> RgbImage {
    let (iw, ih) = src.dimensions();
    if (iw, ih) == (width, height) {
        return src.clone();
    }
    if iw == 0 || ih == 0 {
        return RgbImage::new(width, height);
    }
    match fit.unwrap_or(FitMode::Cover) {
        FitMode::Fill => resize(src, width, height, false),
        FitMode::Cover => resize(src, width, height, true),
        FitMode::Contain => {
            // u64: 8K × 8K overflows u32.
            let wider = u64::from(iw) * u64::from(height) > u64::from(ih) * u64::from(width);
            let scaled = |a: u32, b: u32, c: u32| (u64::from(a) * u64::from(b) / u64::from(c)).max(1) as u32;
            let (sw, sh) = if wider {
                (width, scaled(ih, width, iw))
            } else {
                (scaled(iw, height, ih), height)
            };
            let mut out = RgbImage::from_pixel(width, height, Rgb([0, 0, 0]));
            let picture = resize(src, sw, sh, false);
            image::imageops::replace(
                &mut out,
                &picture,
                i64::from((width - sw) / 2),
                i64::from((height - sh) / 2),
            );
            out
        }
        FitMode::Tile => RgbImage::from_fn(width, height, |x, y| *src.get_pixel(x % iw, y % ih)),
    }
}

/// Lanczos3, as `transform.rs` scales framed and spanned copies, but with
/// SIMD: a 4K picture to 1080p in 15 ms instead of 230 (fast_image_resize).
/// `middle`: cut the middle at the output's shape first (Cover).
fn resize(src: &RgbImage, width: u32, height: u32, middle: bool) -> RgbImage {
    let mut out = RgbImage::new(width, height);
    let mut options = ResizeOptions::new().resize_alg(ResizeAlg::Convolution(FilterType::Lanczos3));
    if middle {
        options = options.fit_into_destination(Some((0.5, 0.5)));
    }
    // Both RGB, neither empty: nothing for it to refuse.
    Resizer::new()
        .resize(src, &mut out, &options)
        .expect("resizing RGB to RGB");
    out
}

/// `img`'s pixels as XRGB8888, row after row (the stride is width × 4).
pub fn xrgb(img: &RgbImage) -> Vec<u8> {
    let mut out = Vec::with_capacity(img.as_raw().len() / 3 * 4);
    for p in img.pixels() {
        let [r, g, b] = p.0;
        out.extend_from_slice(&[b, g, r, 0xff]);
    }
    out
}

/// A straight-alpha frame composited onto black, as [`geometry`] expects it.
pub fn flatten(src: &image::RgbaImage) -> RgbImage {
    RgbImage::from_fn(src.width(), src.height(), |x, y| {
        let [r, g, b, a] = src.get_pixel(x, y).0;
        [r, g, b]
            .map(|c| ((u16::from(c) * u16::from(a) + 127) / 255) as u8)
            .into()
    })
}

/// Apply portable geometry to an already decoded representative still. Alpha
/// must be composited onto black before calling this RGB-only fitting path.
pub fn geometry(src: &RgbImage, geometry: crate::render::Geometry, size: (u32, u32)) -> Result<Vec<u8>> {
    use crate::render::View;
    geometry.validate_for_source(src.dimensions())?;
    if src.width() == 0
        || src.height() == 0
        || size.0 == 0
        || size.1 == 0
        || size.0 > 16384
        || size.1 > 16384
        || u64::from(size.0) * u64::from(size.1) * 4 > 64 * 1024 * 1024
    {
        return Err(Error::Image("invalid representative dimensions".into()));
    }
    let region = match geometry.view {
        View::Full => None,
        View::Region { x, y, width, height } => Some((x, y, width, height)),
        View::Frame { .. } => Some(geometry.adjust(size).frame.unwrap().rect(src.width(), src.height())),
    };
    let mut image = if let Some((x, y, w, h)) = region {
        // Crop before filtering so a standalone cut cannot sample adjacent
        // source pixels outside its selected region.
        let cropped = image::imageops::crop_imm(src, x, y, w, h).to_image();
        compose(&cropped, size.0, size.1, Some(FitMode::Fill))
    } else {
        src.clone()
    };
    if geometry.flip.h {
        image::imageops::flip_horizontal_in_place(&mut image);
    }
    if geometry.flip.v {
        image::imageops::flip_vertical_in_place(&mut image);
    }
    Ok(xrgb(&compose(&image, size.0, size.1, geometry.fit)))
}

#[cfg(test)]
mod tests {
    use super::*;

    const RED: Rgb<u8> = Rgb([255, 0, 0]);
    const GREEN: Rgb<u8> = Rgb([0, 255, 0]);
    const BLUE: Rgb<u8> = Rgb([0, 0, 255]);
    const BLACK: Rgb<u8> = Rgb([0, 0, 0]);

    /// Within a few levels: Lanczos3 rings a little near hard edges, and
    /// samples just outside a cut.
    #[track_caller]
    fn near(p: &Rgb<u8>, want: Rgb<u8>) {
        let close = p.0.iter().zip(want.0).all(|(a, b)| a.abs_diff(b) <= 3);
        assert!(close, "{p:?} isn't {want:?}");
    }

    /// 300×100: red, green and blue thirds, left to right.
    fn thirds() -> RgbImage {
        RgbImage::from_fn(300, 100, |x, _| [RED, GREEN, BLUE][(x / 100) as usize])
    }

    #[test]
    fn cover_keeps_the_middle() {
        // A square output out of a 3:1 picture: the green third, all of it.
        let out = compose(&thirds(), 50, 50, None);
        assert_eq!(out.dimensions(), (50, 50));
        near(out.get_pixel(2, 25), GREEN);
        near(out.get_pixel(47, 25), GREEN);
        assert_eq!(compose(&thirds(), 50, 50, Some(FitMode::Cover)), out);
    }

    #[test]
    fn contain_has_black_bars() {
        // 3:1 into 60×60: 60×20 in the middle, black above and below.
        let out = compose(&thirds(), 60, 60, Some(FitMode::Contain));
        near(out.get_pixel(30, 5), BLACK);
        near(out.get_pixel(30, 54), BLACK);
        near(out.get_pixel(5, 30), RED);
        near(out.get_pixel(30, 30), GREEN);
        near(out.get_pixel(54, 30), BLUE);
    }

    #[test]
    fn fill_stretches_everything_in() {
        let out = compose(&thirds(), 30, 90, Some(FitMode::Fill));
        assert_eq!(out.dimensions(), (30, 90));
        near(out.get_pixel(2, 45), RED);
        near(out.get_pixel(15, 45), GREEN);
        near(out.get_pixel(27, 45), BLUE);
    }

    #[test]
    fn tile_repeats_at_its_own_size() {
        let out = compose(&thirds(), 700, 150, Some(FitMode::Tile));
        near(out.get_pixel(50, 10), RED);
        near(out.get_pixel(350, 10), RED);
        near(out.get_pixel(650, 120), RED);
        near(out.get_pixel(450, 120), GREEN);
    }

    #[test]
    fn the_outputs_size_is_used_as_it_is() {
        let img = thirds();
        assert_eq!(compose(&img, 300, 100, Some(FitMode::Contain)), img);
    }

    #[test]
    fn xrgb_is_bgrx_in_memory() {
        let img = RgbImage::from_pixel(2, 1, Rgb([1, 2, 3]));
        assert_eq!(xrgb(&img), vec![3, 2, 1, 255, 3, 2, 1, 255]);
    }
}
