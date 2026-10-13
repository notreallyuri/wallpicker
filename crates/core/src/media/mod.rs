//! Content-aware local media, independent of rendering and desktop state.
//!
//! [`Source::open`] inspects the container without collecting decoded frames.
//! [`Source::representative`] and [`Source::frames`] use the same composited first
//! frame. Streams make one pass; open a fresh stream to repeat, resetting disposal
//! state. Callers own scheduling, cancellation between frames and queue budgets.
//!
//! Input, dimensions, frame counts and output bytes are checked. Codec allocation
//! limits are **best effort**, not a hard process-memory bound (see [`Limits`]).

mod decode;
mod identity;
mod limits;
mod probe;
mod probe_gif;
mod probe_webp;
mod webp;
mod webp_reader;

#[cfg(test)]
mod fixture;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod webp_tests;

use std::path::{Path, PathBuf};
use std::time::Duration;

pub use decode::Frames;
pub use identity::{FileIdentity, MetadataCache};
pub use image::RgbaImage;
pub use limits::Limits;

use crate::{Error, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Jpeg,
    Png,
    Gif,
    WebP,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Static,
    Animated,
}

/// Normalized total plays specified by the file. Wallpaper playback may ignore
/// this and repeat indefinitely. An absent GIF loop extension means one play;
/// GIF's repeat count excludes the first play, whereas WebP's includes it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoopCount {
    Infinite,
    Finite(u32),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Info {
    pub format: Format,
    pub kind: Kind,
    pub width: u32,
    pub height: u32,
    pub frame_count: u32,
    pub loop_count: LoopCount,
}

impl Info {
    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }
}

/// A complete source-sized, straight-alpha RGBA canvas. Transparent pixels stay
/// transparent; the renderer/thumbnailer decides how to flatten them.
pub struct Frame {
    pub pixels: RgbaImage,
    /// Original encoded delay, including zero. Still images have no delay.
    pub delay: Option<Duration>,
}

impl Frame {
    /// Proposed image-playback policy: tiny/zero delays become 10 ms. This does
    /// not cap presentation rate; the scheduler may skip uploads independently.
    pub fn playback_delay(&self) -> Option<Duration> {
        self.delay.map(|d| d.max(Duration::from_millis(10)))
    }
}

/// Inspected file plus the identity and limits under which it was inspected.
/// Opening a stream refuses a replaced/modified file; call `open` again to refresh.
#[derive(Debug, Clone)]
pub struct Source {
    path: PathBuf,
    identity: FileIdentity,
    info: Info,
    limits: Limits,
}

impl Source {
    pub fn open(path: &Path, limits: Limits) -> Result<Self> {
        let identity = FileIdentity::read(path)?;
        limits.file(identity.len)?;
        let mut input = probe::Input::open(path, &identity)?;
        let info = probe::inspect(&mut input, limits).map_err(|e| context(path, e))?;
        input.verify(path, &identity)?;
        Ok(Self {
            path: path.to_owned(),
            identity,
            info,
            limits,
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
    pub fn identity(&self) -> &FileIdentity {
        &self.identity
    }
    pub fn info(&self) -> &Info {
        &self.info
    }
    pub fn limits(&self) -> Limits {
        self.limits
    }

    /// One pass, with at most one returned frame decoded per call. Dropping the
    /// stream closes its file and releases the decoder's canvas/scratch buffers.
    pub fn frames(&self) -> Result<Frames> {
        Frames::open(self.clone())
    }

    pub fn representative(&self) -> Result<RgbaImage> {
        self.frames()?
            .next()
            .ok_or_else(|| invalid("media has no frames"))?
            .map(|f| f.pixels)
    }
}

fn invalid(message: impl Into<String>) -> Error {
    Error::Image(message.into())
}
fn context(path: &Path, error: impl std::fmt::Display) -> Error {
    invalid(format!("{}: {error}", path.display()))
}
