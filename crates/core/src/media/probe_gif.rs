//! GIF block inspection without decoding or retaining compressed frame data.
use super::{Format, Info, Kind, Limits, LoopCount, Result, invalid, probe::Input};

pub(super) fn inspect(input: &mut Input, limits: Limits) -> Result<Info> {
    let signature = input.bytes::<6>()?;
    if signature != *b"GIF87a" && signature != *b"GIF89a" {
        return Err(invalid("invalid GIF signature"));
    }
    let header = input.bytes::<7>()?;
    let width = u32::from(u16::from_le_bytes([header[0], header[1]]));
    let height = u32::from(u16::from_le_bytes([header[2], header[3]]));
    limits.dimensions(width, height)?;
    palette(input, header[4])?;
    let mut frames = 0u32;
    let mut loop_count = LoopCount::Finite(1);
    loop {
        match input.bytes::<1>()?[0] {
            0x3b => break,
            0x2c => {
                let h = input.bytes::<9>()?;
                let word = |i| u32::from(u16::from_le_bytes([h[i], h[i + 1]]));
                let (x, y, w, hgt) = (word(0), word(2), word(4), word(6));
                limits.dimensions(w, hgt)?;
                if x + w > width || y + hgt > height {
                    return Err(invalid("GIF frame outside canvas"));
                }
                frames = frames
                    .checked_add(1)
                    .ok_or_else(|| invalid("GIF frame count overflow"))?;
                limits.frames(frames)?;
                palette(input, h[8])?;
                let code_size = input.bytes::<1>()?[0];
                if !(2..=8).contains(&code_size) {
                    return Err(invalid("invalid GIF LZW code size"));
                }
                blocks(input)?;
            }
            0x21 => match input.bytes::<1>()?[0] {
                0xf9 => {
                    let control = input.bytes::<6>()?;
                    if control[0] != 4 || control[5] != 0 || (control[1] >> 2) & 7 > 3 {
                        return Err(invalid("invalid GIF graphics control"));
                    }
                }
                0xff => {
                    let len = input.bytes::<1>()?[0];
                    if len != 11 {
                        return Err(invalid("invalid GIF application extension"));
                    }
                    let app = input.bytes::<11>()?;
                    if app == *b"NETSCAPE2.0" || app == *b"ANIMEXTS1.0" {
                        let repeat = input.bytes::<5>()?;
                        if repeat[0] != 3 || repeat[1] != 1 || repeat[4] != 0 {
                            return Err(invalid("invalid GIF loop extension"));
                        }
                        let n = u16::from_le_bytes([repeat[2], repeat[3]]);
                        loop_count = if n == 0 {
                            LoopCount::Infinite
                        } else {
                            LoopCount::Finite(u32::from(n) + 1)
                        };
                    } else {
                        blocks(input)?;
                    }
                }
                // Plain-text rendering is not supported by the image adapter.
                0x01 => return Err(invalid("GIF plain-text frames are unsupported")),
                0xfe => blocks(input)?,
                _ => return Err(invalid("unknown GIF extension")),
            },
            _ => return Err(invalid("invalid GIF block or missing trailer")),
        }
    }
    limits.frames(frames)?;
    Ok(Info {
        format: Format::Gif,
        kind: if frames > 1 { Kind::Animated } else { Kind::Static },
        width,
        height,
        frame_count: frames,
        loop_count,
    })
}

fn palette(input: &mut Input, flags: u8) -> Result<()> {
    if flags & 0x80 != 0 {
        input.skip(3 * (1u64 << ((flags & 7) + 1)))?;
    }
    Ok(())
}

fn blocks(input: &mut Input) -> Result<()> {
    loop {
        let count = input.bytes::<1>()?[0];
        if count == 0 {
            return Ok(());
        }
        input.skip(u64::from(count))?;
    }
}
