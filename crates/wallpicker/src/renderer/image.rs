//! Worker-side decoding, fitting, and the short-lived decoded-image cache.

use super::{Picture, Saved};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};
use wallpicker_core::compose::{self, RgbImage};
use wallpicker_core::media::{FileIdentity, Limits, Source};
use wallpicker_core::render::Content;

/// The last picture decoded (its file, as modified then), for the next
/// monitor showing it.
pub(super) type Recent = Arc<Mutex<Option<(PathBuf, Option<SystemTime>, Arc<RgbImage>)>>>;

/// How long [`Recent`] keeps it: a decoded 8K picture is 130 MB.
pub(super) const RECENT_FOR: Duration = Duration::from_secs(3);

pub(super) struct Made {
    pub(super) picture: Picture,
    /// Its size in pixels.
    pub(super) size: (u32, u32),
    pub(super) pixels: Vec<u8>,
    pub(super) still: Option<super::fallback::Still>,
    pub(super) _hold: Option<super::fallback::Hold>,
}

/// What a screen shows: the saved picture (looked up when drawn), or a file.
#[derive(Clone)]
pub(super) enum Want {
    Saved,
    File(Picture),
    Still(Picture, super::fallback::Still),
}

impl Want {
    pub(super) fn picture(&self) -> Option<&Picture> {
        match self {
            Self::Saved => None,
            Self::File(p) | Self::Still(p, _) => Some(p),
        }
    }
}

/// Big allocations mapped on their own, so they go back to the system when
/// freed: a frame's tens of MB (decoded, fitted, copied) would otherwise
/// stay with glibc's allocator, in the arena of the worker that made it,
/// for as long as the daemon runs (85 MB after a few changes, against 7).
pub(super) fn map_big_allocations() {
    const M_MMAP_THRESHOLD: std::ffi::c_int = -3;
    unsafe extern "C" {
        fn mallopt(param: std::ffi::c_int, value: std::ffi::c_int) -> std::ffi::c_int;
    }
    // SAFETY: glibc's; a tuning knob, safe to set at any time.
    unsafe {
        mallopt(M_MMAP_THRESHOLD, 1 << 20);
    }
}

/// A worker's job: the picture (`want`, the saved one looked up now) read
/// (or the one just read again) and fitted to `size`.
pub(super) fn make(
    name: &str,
    want: Want,
    size: (u32, u32),
    saved: &Saved,
    recent: &Recent,
) -> Result<Option<Made>, String> {
    let picture = match want {
        Want::File(p) => p,
        Want::Still(picture, still) => {
            let (pixels, hold) = still.render(&picture, size)?;
            return Ok(Some(Made {
                picture,
                size,
                pixels,
                still: Some(still),
                _hold: Some(hold),
            }));
        }
        Want::Saved => match saved(name) {
            Some(p) => p,
            None => return Ok(None),
        },
    };
    if let Content::Animated { identity } = &picture.picture.content {
        // Not played (playback off, or no hardware for it): the first frame.
        let src = representative(&picture.picture.source, identity, recent)?;
        let pixels = compose::geometry(&src, picture.picture.geometry, size).map_err(|e| e.to_string())?;
        return Ok(Some(Made {
            picture,
            size,
            pixels,
            still: None,
            _hold: None,
        }));
    }
    let (file, fit) = picture.picture.static_file().map_err(|e| e.to_string())?;
    let src = if matches!(picture.picture.content,
        Content::Static { prepared_size: Some(prepared), .. } if prepared != size)
    {
        let image = wallpicker_core::transform::render(&picture.picture.source, &picture.picture.geometry.adjust(size))
            .map_err(|e| e.to_string())?;
        Arc::new(image.into_rgb8())
    } else {
        decoded(file, recent)?
    };
    let pixels = compose::xrgb(&compose::compose(&src, size.0, size.1, fit));
    Ok(Some(Made {
        picture,
        size,
        pixels,
        still: None,
        _hold: None,
    }))
}

/// The picture at `path`: the recent one if it's that file, unchanged since.
fn decoded(path: &Path, recent: &Recent) -> Result<Arc<RgbImage>, String> {
    cached(path, recent, || compose::open(path).map_err(|e| e.to_string()))
}

/// An animation's first frame, flattened onto black; only if the file is
/// still the one the request was prepared from.
fn representative(path: &Path, identity: &FileIdentity, recent: &Recent) -> Result<Arc<RgbImage>, String> {
    cached(path, recent, || {
        let source = Source::open(path, Limits::default()).map_err(|e| e.to_string())?;
        if source.identity() != identity {
            return Err(format!("media changed since request preparation: {}", path.display()));
        }
        Ok(compose::flatten(&source.representative().map_err(|e| e.to_string())?))
    })
}

fn cached(path: &Path, recent: &Recent, read: impl FnOnce() -> Result<RgbImage, String>) -> Result<Arc<RgbImage>, String> {
    let modified = std::fs::metadata(path).and_then(|m| m.modified()).ok();
    let lock = || recent.lock().unwrap_or_else(|e| e.into_inner());
    if let Some((p, m, src)) = lock().as_ref()
        && p == path
        && *m == modified
    {
        return Ok(src.clone());
    }
    let src = Arc::new(read()?);
    *lock() = Some((path.to_path_buf(), modified, src.clone()));
    Ok(src)
}

#[cfg(test)]
mod tests {
    use super::*;
    use wallpicker_core::{
        render::{Batch, Intent, Preparation},
        settings::{Adjust, FitMode, Flip, Frame},
    };

    #[test]
    fn resize_rebuilds_portable_framing_from_original_source() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("gradient.png");
        RgbImage::from_fn(16, 8, |x, y| [x as u8 * 16, y as u8 * 16, 0].into())
            .save(&source)
            .unwrap();
        let adjust = Adjust {
            frame: Some(Frame {
                zoom: 100,
                x: 500,
                y: 500,
                width: 8,
                height: 4,
            }),
            ..Default::default()
        };
        let picture = Preparation::default()
            .picture(&dir.path().join("cache"), &source, &adjust, None)
            .unwrap();
        let batch = Batch::new(Intent::Apply, Default::default(), vec![]);
        let selected = batch.selection(picture);
        let saved: Saved = Arc::new(|_| None);
        let made = make("DP-1", Want::File(selected), (4, 8), &saved, &Arc::default())
            .unwrap()
            .unwrap();
        assert_eq!(made.size, (4, 8));
        assert_eq!(made.picture.picture.source, source);
        // New aspect ratio crops source columns 6..10. Stretching the original
        // prepared 8×4 frame would instead start near column 0.
        assert!((90..=110).contains(&made.pixels[2]), "red={}", made.pixels[2]);
    }

    /// Left half red, right half blue, then all green.
    fn animation(path: &Path) {
        let mut bytes = Vec::new();
        {
            let palette = [255, 0, 0, 0, 0, 255, 0, 255, 0];
            let mut encoder = gif::Encoder::new(&mut bytes, 16, 8, &palette).unwrap();
            let first = (0..16 * 8).map(|i| u8::from(i % 16 >= 8)).collect();
            for buffer in [first, vec![2; 16 * 8]] {
                encoder
                    .write_frame(&gif::Frame {
                        width: 16,
                        height: 8,
                        delay: 10,
                        buffer: std::borrow::Cow::Owned(buffer),
                        ..Default::default()
                    })
                    .unwrap();
            }
        }
        std::fs::write(path, bytes).unwrap();
    }

    #[test]
    fn unplayed_animation_is_drawn_as_its_first_frame_with_its_geometry() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("moving.gif");
        animation(&source);
        let adjust = Adjust {
            flip: Flip { h: true, v: false },
            ..Default::default()
        };
        let picture = Preparation::default()
            .picture(&dir.path().join("cache"), &source, &adjust, Some(FitMode::Fill))
            .unwrap();
        let selected = Batch::new(Intent::Apply, Default::default(), vec![]).selection(picture);
        let saved: Saved = Arc::new(|_| None);
        let made = make("DP-1", Want::File(selected.clone()), (16, 8), &saved, &Arc::default())
            .unwrap()
            .unwrap();
        // XRGB in memory is B, G, R, X. Flipped: blue on the left, red on the right.
        let pixel = |x: usize| &made.pixels[x * 4..x * 4 + 3];
        assert_eq!(pixel(0), [255, 0, 0]);
        assert_eq!(pixel(15), [0, 0, 255]);
        // A file replaced after preparation is not shown under the old identity.
        std::fs::write(&source, std::fs::read(&source).unwrap().repeat(2)).unwrap();
        let error = make("DP-1", Want::File(selected), (16, 8), &saved, &Arc::default()).err().unwrap();
        assert!(error.contains("changed"), "{error}");
    }
}
