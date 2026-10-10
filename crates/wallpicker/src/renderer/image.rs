//! Worker-side decoding, fitting, and the short-lived decoded-image cache.

use super::{Picture, Saved};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};
use wallpicker_core::compose::{self, RgbImage};

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
}

/// What a screen shows: the saved picture (looked up when drawn), or a file.
#[derive(Clone)]
pub(super) enum Want {
    Saved,
    File(Picture),
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
        Want::Saved => match saved(name) {
            Some(p) => p,
            None => return Ok(None),
        },
    };
    let src = decoded(&picture.0, recent)?;
    let pixels = compose::xrgb(&compose::compose(&src, size.0, size.1, picture.1));
    Ok(Some(Made {
        picture,
        size,
        pixels,
    }))
}

/// The picture at `path`: the recent one if it's that file, unchanged since.
fn decoded(path: &std::path::Path, recent: &Recent) -> Result<Arc<RgbImage>, String> {
    let modified = std::fs::metadata(path).and_then(|m| m.modified()).ok();
    let lock = || recent.lock().unwrap_or_else(|e| e.into_inner());
    if let Some((p, m, src)) = lock().as_ref()
        && p == path
        && *m == modified
    {
        return Ok(src.clone());
    }
    let src = Arc::new(compose::open(path).map_err(|e| e.to_string())?);
    *lock() = Some((path.to_path_buf(), modified, src.clone()));
    Ok(src)
}
