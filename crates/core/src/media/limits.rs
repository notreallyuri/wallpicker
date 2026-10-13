use super::{Result, invalid};

/// Foundation defaults, not measured playback budgets. These bound accepted
/// files/canvases and one pass's frame count. Callers must separately budget held
/// frames and concurrent streams. `decoder_bytes` is passed to codecs that
/// support it, but image-webp scratch allocations can exceed it; it is NOT a
/// strict RSS limit. Playback workers must enforce an outer process-memory limit.
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct Limits {
    pub encoded_bytes: u64,
    pub dimension: u32,
    pub frame_bytes: u64,
    pub frame_count: u32,
    pub decoder_bytes: u64,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            encoded_bytes: 64 * 1024 * 1024,
            dimension: 16_384,
            frame_bytes: 64 * 1024 * 1024,
            frame_count: 10_000,
            decoder_bytes: 256 * 1024 * 1024,
        }
    }
}

impl Limits {
    pub(super) fn file(self, bytes: u64) -> Result<()> {
        if bytes == 0 || bytes > self.encoded_bytes {
            return Err(invalid("empty media or encoded-file byte limit exceeded"));
        }
        Ok(())
    }

    pub(super) fn dimensions(self, width: u32, height: u32) -> Result<usize> {
        let bytes = u64::from(width)
            .checked_mul(u64::from(height))
            .and_then(|n| n.checked_mul(4));
        if width == 0 || height == 0 || width > self.dimension || height > self.dimension {
            return Err(invalid("media dimension limit exceeded or empty canvas"));
        }
        let bytes = bytes
            .filter(|n| *n <= self.frame_bytes && *n <= self.decoder_bytes)
            .ok_or_else(|| invalid("media frame byte limit exceeded"))?;
        usize::try_from(bytes).map_err(|_| invalid("media frame does not fit address space"))
    }

    pub(super) fn frames(self, count: u32) -> Result<()> {
        if count == 0 || count > self.frame_count {
            return Err(invalid("empty media or frame count limit exceeded"));
        }
        Ok(())
    }

    pub(super) fn image(self) -> image::Limits {
        let mut limits = image::Limits::default();
        limits.max_image_width = Some(self.dimension);
        limits.max_image_height = Some(self.dimension);
        limits.max_alloc = Some(self.decoder_bytes);
        limits
    }
}
