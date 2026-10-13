//! WebP patch decoding and straight-alpha source compositing.
//!
//! image-webp 0.2.4's animation compositor clears the entire canvas when a
//! full-canvas blended frame follows a disposed partial frame. Decode patches
//! with its still decoder and implement the small ANMF compositor here instead.
//! Semantics: https://developers.google.com/speed/webp/docs/riff_container#animation
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::time::Duration;

use super::{
    Frame, Limits, Result, RgbaImage, Source, invalid,
    probe::Input,
    probe_webp::{chunk, u24},
    webp_reader::Patch,
};

pub(super) struct WebP {
    input: Input,
    limits: Limits,
    size: (u32, u32),
    canvas: Option<RgbaImage>,
    /// Rectangle disposed before the next frame, when requested.
    dispose: Option<(u32, u32, u32, u32)>,
    next: u64,
}

impl WebP {
    pub(super) fn open(mut input: Input, source: &Source) -> Result<Self> {
        let header = input.bytes::<21>()?;
        let animated = &header[12..16] == b"VP8X" && header[20] & 2 != 0;
        input.rewind()?;
        let size = source.info.dimensions();
        let canvas = if animated {
            let bytes = source.limits.dimensions(size.0, size.1)?;
            // Our canvas, patch and returned canvas. Codec scratch is additional
            // and only best-effort limited, as documented by Limits.
            if (bytes as u64)
                .checked_mul(3)
                .is_none_or(|n| n > source.limits.decoder_bytes)
            {
                return Err(invalid("WebP compositing buffer budget exceeded"));
            }
            Some(buffer(size, source.limits)?)
        } else {
            None
        };
        Ok(Self {
            input,
            limits: source.limits,
            size,
            canvas,
            dispose: None,
            next: 12,
        })
    }

    pub(super) fn frame(&mut self) -> Result<Frame> {
        let Some(canvas) = self.canvas.as_mut() else {
            return Ok(Frame {
                pixels: still(BufReader::new(&mut self.input), self.size, self.limits)?,
                delay: None,
            });
        };
        self.input.seek(SeekFrom::Start(self.next))?;
        let end = self.input.len();
        while self.input.position() < end {
            let (tag, size, next) = chunk(&mut self.input, end)?;
            if tag != *b"ANMF" {
                self.input.seek(SeekFrom::Start(next))?;
                continue;
            }
            if size < 16 {
                return Err(invalid("truncated WebP frame header"));
            }
            let h = self.input.bytes::<16>()?;
            let (x, y) = (u24(&h[..3]) * 2, u24(&h[3..6]) * 2);
            let (width, height) = (u24(&h[6..9]) + 1, u24(&h[9..12]) + 1);
            self.limits.dimensions(width, height)?;
            if x + width > self.size.0 || y + height > self.size.1 {
                return Err(invalid("WebP patch outside canvas"));
            }
            let reader = Patch::new(&mut self.input, size - 16, width, height)?;
            let patch = still(BufReader::new(reader), (width, height), self.limits)?;
            if let Some((dx, dy, dw, dh)) = self.dispose.take() {
                for row in dy..dy + dh {
                    for col in dx..dx + dw {
                        canvas.put_pixel(col, row, image::Rgba([0; 4]));
                    }
                }
            }
            for (px, py, pixel) in patch.enumerate_pixels() {
                let dst = canvas.get_pixel_mut(x + px, y + py);
                dst.0 = if h[15] & 2 == 0 { over(pixel.0, dst.0) } else { pixel.0 };
            }
            self.dispose = (h[15] & 1 != 0).then_some((x, y, width, height));
            self.next = next;
            let mut pixels = buffer(self.size, self.limits)?;
            pixels.as_mut().copy_from_slice(canvas.as_raw());
            return Ok(Frame {
                pixels,
                delay: Some(Duration::from_millis(u64::from(u24(&h[12..15])))),
            });
        }
        Err(invalid("WebP ended before its inspected frame count"))
    }
}

fn buffer((width, height): (u32, u32), limits: Limits) -> Result<RgbaImage> {
    let len = limits.dimensions(width, height)?;
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(len)
        .map_err(|_| invalid("cannot allocate media frame"))?;
    bytes.resize(len, 0);
    RgbaImage::from_raw(width, height, bytes).ok_or_else(|| invalid("invalid media frame buffer"))
}

fn still(reader: impl BufRead + Seek, size: (u32, u32), limits: Limits) -> Result<RgbaImage> {
    let mut decoder = image_webp::WebPDecoder::new(reader).map_err(|e| invalid(e.to_string()))?;
    decoder.set_memory_limit(usize::try_from(limits.decoder_bytes).unwrap_or(usize::MAX));
    if decoder.dimensions() != size || decoder.is_animated() {
        return Err(invalid("WebP patch dimensions changed"));
    }
    let mut pixels = buffer(size, limits)?;
    let alpha = decoder.has_alpha();
    let bytes = pixels.as_mut();
    let len = if alpha { bytes.len() } else { bytes.len() / 4 * 3 };
    decoder
        .read_image(&mut bytes[..len])
        .map_err(|e| invalid(e.to_string()))?;
    if !alpha {
        for i in (0..bytes.len() / 4).rev() {
            let rgb = [bytes[i * 3], bytes[i * 3 + 1], bytes[i * 3 + 2], 255];
            bytes[i * 4..i * 4 + 4].copy_from_slice(&rgb);
        }
    }
    Ok(pixels)
}

/// Source-over with straight alpha, rounded to nearest. Keep full precision
/// until division, so opaque colors are not darkened by reciprocal truncation.
fn over(src: [u8; 4], dst: [u8; 4]) -> [u8; 4] {
    let a = u32::from(src[3]);
    if a == 0 {
        return dst;
    }
    if a == 255 || dst[3] == 0 {
        return src;
    }
    let b = u32::from(dst[3]) * (255 - a);
    let alpha = a * 255 + b;
    let mut out = [0; 4];
    for i in 0..3 {
        out[i] = ((u32::from(src[i]) * a * 255 + u32::from(dst[i]) * b + alpha / 2) / alpha) as u8;
    }
    out[3] = ((alpha + 127) / 255) as u8;
    out
}

#[cfg(test)]
mod tests {
    use super::over;
    #[test]
    fn straight_alpha_blend_keeps_transparency_and_opaque_endpoints() {
        assert_eq!(over([1, 2, 3, 0], [4, 5, 6, 127]), [4, 5, 6, 127]);
        assert_eq!(over([1, 2, 3, 255], [4, 5, 6, 127]), [1, 2, 3, 255]);
        assert_eq!(over([255, 0, 0, 128], [0, 0, 255, 128]), [170, 0, 85, 192]);
        assert_eq!(over([1, 2, 3, 128], [255, 255, 255, 0]), [1, 2, 3, 128]);
    }
}
