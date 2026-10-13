//! Validate RIFF boundaries and encoded patch dimensions before codec allocation.
//! No compressed frame collection; unknown metadata chunks are skipped.
use super::{Format, Info, Kind, Limits, LoopCount, Result, invalid, probe::Input};

pub(super) fn u24(b: &[u8]) -> u32 {
    u32::from_le_bytes([b[0], b[1], b[2], 0])
}

pub(super) fn inspect(input: &mut Input, limits: Limits) -> Result<Info> {
    let h = input.bytes::<12>()?;
    let end = u64::from(u32::from_le_bytes(h[4..8].try_into().unwrap())) + 8;
    if &h[..4] != b"RIFF" || &h[8..] != b"WEBP" || end != input.len() {
        return Err(invalid("invalid or truncated WebP RIFF container"));
    }
    let mut canvas = None;
    let mut animated = false;
    let mut animation_header = false;
    let mut still = false;
    let mut frames = 0u32;
    let mut loop_count = LoopCount::Finite(1);
    while input.position() < end {
        let (tag, size, next) = chunk(input, end)?;
        match &tag {
            b"VP8X" => {
                if canvas.is_some() || input.position() != 20 || size != 10 {
                    return Err(invalid("invalid WebP extended header"));
                }
                let h = input.bytes::<10>()?;
                let dims = (u24(&h[4..7]) + 1, u24(&h[7..10]) + 1);
                limits.dimensions(dims.0, dims.1)?;
                canvas = Some(dims);
                animated = h[0] & 2 != 0;
            }
            b"ANIM" => {
                if !animated || animation_header || size != 6 {
                    return Err(invalid("invalid WebP animation header"));
                }
                let h = input.bytes::<6>()?;
                let count = u16::from_le_bytes([h[4], h[5]]);
                loop_count = if count == 0 {
                    LoopCount::Infinite
                } else {
                    LoopCount::Finite(u32::from(count))
                };
                animation_header = true;
            }
            b"ANMF" => {
                if !animation_header || size < 16 {
                    return Err(invalid("invalid WebP animation frame"));
                }
                let h = input.bytes::<16>()?;
                let (x, y) = (u24(&h[0..3]) * 2, u24(&h[3..6]) * 2);
                let dims = (u24(&h[6..9]) + 1, u24(&h[9..12]) + 1);
                limits.dimensions(dims.0, dims.1)?;
                let (w, hgt) = canvas.ok_or_else(|| invalid("WebP canvas missing"))?;
                if x + dims.0 > w || y + dims.1 > hgt {
                    return Err(invalid("WebP frame outside canvas"));
                }
                frames = frames
                    .checked_add(1)
                    .ok_or_else(|| invalid("WebP frame count overflow"))?;
                limits.frames(frames)?;
                let frame_end = input.position() + u64::from(size - 16);
                let mut image = false;
                let mut alpha = false;
                while input.position() < frame_end {
                    let (subtag, subsize, subnext) = chunk(input, frame_end)?;
                    if subtag == *b"VP8 " || subtag == *b"VP8L" {
                        if image || alpha && subtag == *b"VP8L" || dimensions(input, subtag, subsize)? != dims {
                            return Err(invalid("inconsistent WebP frame dimensions"));
                        }
                        image = true;
                    } else if subtag == *b"ALPH" {
                        if image || alpha {
                            return Err(invalid("WebP alpha chunk out of order"));
                        }
                        alpha = true;
                    }
                    input.skip(subnext - input.position())?;
                }
                if !image {
                    return Err(invalid("WebP frame has no image"));
                }
            }
            b"VP8 " | b"VP8L" => {
                if animated || still {
                    return Err(invalid("unexpected WebP still image"));
                }
                let dims = dimensions(input, tag, size)?;
                limits.dimensions(dims.0, dims.1)?;
                if canvas.is_some_and(|c| c != dims) {
                    return Err(invalid("inconsistent WebP canvas dimensions"));
                }
                canvas = Some(dims);
                still = true;
            }
            _ => {}
        }
        input.skip(next - input.position())?;
    }
    if animated && frames == 0 || !animated && !still {
        return Err(invalid("WebP has no frames"));
    }
    let frame_count = if animated { frames } else { 1 };
    limits.frames(frame_count)?;
    let (width, height) = canvas.ok_or_else(|| invalid("WebP canvas missing"))?;
    Ok(Info {
        format: Format::WebP,
        kind: if frame_count > 1 { Kind::Animated } else { Kind::Static },
        width,
        height,
        frame_count,
        loop_count,
    })
}

pub(super) fn chunk(input: &mut Input, end: u64) -> Result<([u8; 4], u32, u64)> {
    if end - input.position() < 8 {
        return Err(invalid("truncated WebP chunk header"));
    }
    let h = input.bytes::<8>()?;
    let size = u32::from_le_bytes(h[4..].try_into().unwrap());
    let next = input.position() + u64::from(size) + u64::from(size & 1);
    if next > end {
        return Err(invalid("WebP chunk outside container"));
    }
    Ok((h[..4].try_into().unwrap(), size, next))
}

fn dimensions(input: &mut Input, tag: [u8; 4], size: u32) -> Result<(u32, u32)> {
    if tag == *b"VP8L" {
        if size < 5 {
            return Err(invalid("truncated WebP lossless header"));
        }
        let h = input.bytes::<5>()?;
        let bits = u32::from_le_bytes(h[1..].try_into().unwrap());
        if h[0] != 0x2f || bits >> 29 != 0 {
            return Err(invalid("invalid WebP lossless header"));
        }
        Ok(((bits & 0x3fff) + 1, ((bits >> 14) & 0x3fff) + 1))
    } else {
        if size < 10 {
            return Err(invalid("truncated WebP lossy header"));
        }
        let h = input.bytes::<10>()?;
        if h[0] & 1 != 0 || h[3..6] != [0x9d, 1, 0x2a] {
            return Err(invalid("invalid WebP lossy header"));
        }
        Ok((
            u32::from(u16::from_le_bytes([h[6], h[7]]) & 0x3fff),
            u32::from(u16::from_le_bytes([h[8], h[9]]) & 0x3fff),
        ))
    }
}
