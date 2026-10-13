//! Present one ANMF payload as a standalone extended WebP, without copying its
//! compressed bytes. The synthesized header is only 30 bytes; seeks are local
//! to this view so the patch decoder cannot read another animation frame.
use super::{Result, probe::Input, probe_webp::chunk};
use std::io::{self, Read, Seek, SeekFrom};

pub(super) struct Patch<'a> {
    input: &'a mut Input,
    prefix: [u8; 30],
    start: u64,
    len: u64,
    position: u64,
}

impl<'a> Patch<'a> {
    pub(super) fn new(input: &'a mut Input, len: u32, width: u32, height: u32) -> Result<Self> {
        let start = input.position();
        let end = start + u64::from(len);
        let mut alpha = false;
        while input.position() < end {
            let (tag, _, next) = chunk(input, end)?;
            alpha |= tag == *b"ALPH" || tag == *b"VP8L";
            input.seek(SeekFrom::Start(next))?;
        }
        input.seek(SeekFrom::Start(start))?;
        let riff_len = len
            .checked_add(22)
            .ok_or_else(|| io::Error::other("WebP patch too large"))?;
        let mut prefix = [0; 30];
        prefix[..4].copy_from_slice(b"RIFF");
        prefix[4..8].copy_from_slice(&riff_len.to_le_bytes());
        prefix[8..16].copy_from_slice(b"WEBPVP8X");
        prefix[16..20].copy_from_slice(&10u32.to_le_bytes());
        // Opaque VP8 must not advertise a missing ALPH chunk. The adapter
        // expands its RGB output to RGBA before compositing onto the canvas.
        prefix[20] = if alpha { 0x10 } else { 0 };
        prefix[24..27].copy_from_slice(&(width - 1).to_le_bytes()[..3]);
        prefix[27..30].copy_from_slice(&(height - 1).to_le_bytes()[..3]);
        Ok(Self {
            start: input.position(),
            input,
            prefix,
            len: u64::from(len) + 30,
            position: 0,
        })
    }
}

impl Read for Patch<'_> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let count = buf.len().min((self.len - self.position) as usize);
        let read = if self.position < 30 {
            let count = count.min(30 - self.position as usize);
            buf[..count].copy_from_slice(&self.prefix[self.position as usize..self.position as usize + count]);
            count
        } else {
            self.input.read(&mut buf[..count])?
        };
        self.position += read as u64;
        Ok(read)
    }
}

impl Seek for Patch<'_> {
    fn seek(&mut self, from: SeekFrom) -> io::Result<u64> {
        let p = match from {
            SeekFrom::Start(p) => i128::from(p),
            SeekFrom::Current(p) => i128::from(self.position) + i128::from(p),
            SeekFrom::End(p) => i128::from(self.len) + i128::from(p),
        };
        if p < 0 || p > i128::from(self.len) {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "WebP patch seek outside frame",
            ));
        }
        self.input
            .seek(SeekFrom::Start(self.start + (p as u64).saturating_sub(30)))?;
        self.position = p as u64;
        Ok(self.position)
    }
}
