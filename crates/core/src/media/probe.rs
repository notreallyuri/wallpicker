//! Bounded, seekable file access shared by probes and codecs. Seeking beyond
//! the inspected file is an error, including truncated chunks skipped by codecs.
use std::fs::File;
use std::io::{self, BufReader, Read, Seek, SeekFrom};
use std::path::Path;

use super::{FileIdentity, Format, Info, Kind, Limits, LoopCount, Result, invalid};
use image::ImageDecoder;

pub(super) struct Input {
    file: BufReader<File>,
    len: u64,
    position: u64,
}

impl Input {
    pub(super) fn open(path: &Path, identity: &FileIdentity) -> Result<Self> {
        let input = Self {
            file: BufReader::new(File::open(path)?),
            len: identity.len,
            position: 0,
        };
        input.verify(path, identity)?;
        Ok(input)
    }

    pub(super) fn verify(&self, path: &Path, identity: &FileIdentity) -> Result<()> {
        if FileIdentity::from_metadata(&self.file.get_ref().metadata()?)? != *identity
            || FileIdentity::read(path)? != *identity
        {
            return Err(invalid("media changed since inspection"));
        }
        Ok(())
    }

    pub(super) fn len(&self) -> u64 {
        self.len
    }
    pub(super) fn position(&self) -> u64 {
        self.position
    }
    pub(super) fn bytes<const N: usize>(&mut self) -> Result<[u8; N]> {
        let mut bytes = [0; N];
        self.read_exact(&mut bytes)?;
        Ok(bytes)
    }
    pub(super) fn skip(&mut self, count: u64) -> Result<()> {
        let end = self
            .position
            .checked_add(count)
            .ok_or_else(|| invalid("chunk size overflow"))?;
        self.seek(SeekFrom::Start(end))?;
        Ok(())
    }
}

impl Read for Input {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let count = buf
            .len()
            .min((self.len - self.position).min(usize::MAX as u64) as usize);
        let read = self.file.read(&mut buf[..count])?;
        self.position += read as u64;
        Ok(read)
    }
}

impl Seek for Input {
    fn seek(&mut self, from: SeekFrom) -> io::Result<u64> {
        let position = match from {
            SeekFrom::Start(p) => i128::from(p),
            SeekFrom::End(p) => i128::from(self.len) + i128::from(p),
            SeekFrom::Current(p) => i128::from(self.position) + i128::from(p),
        };
        if position < 0 || position > i128::from(self.len) {
            return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "media seek outside file"));
        }
        // Small metadata skips stay inside the read buffer instead of causing
        // one filesystem seek per GIF sub-block.
        if let Ok(delta) = i64::try_from(position - i128::from(self.position)) {
            self.file.seek_relative(delta)?;
        } else {
            self.file.seek(SeekFrom::Start(position as u64))?;
        }
        self.position = position as u64;
        Ok(self.position)
    }
}

pub(super) fn inspect(input: &mut Input, limits: Limits) -> Result<Info> {
    let signature = input.bytes::<12>()?;
    input.rewind()?;
    let format = match image::guess_format(&signature).map_err(|e| invalid(e.to_string()))? {
        image::ImageFormat::Gif => return super::probe_gif::inspect(input, limits),
        image::ImageFormat::WebP => return super::probe_webp::inspect(input, limits),
        image::ImageFormat::Png => Format::Png,
        image::ImageFormat::Jpeg => Format::Jpeg,
        _ => return Err(invalid("unsupported media format")),
    };
    let mut reader = image::ImageReader::with_format(BufReader::new(input), image_format(format));
    reader.limits(limits.image());
    let decoder = reader.into_decoder().map_err(|e| invalid(e.to_string()))?;
    let (width, height) = decoder.dimensions();
    limits.dimensions(width, height)?;
    limits.frames(1)?;
    Ok(Info {
        format,
        kind: Kind::Static,
        width,
        height,
        frame_count: 1,
        loop_count: LoopCount::Finite(1),
    })
}

pub(super) fn image_format(format: Format) -> image::ImageFormat {
    match format {
        Format::Jpeg => image::ImageFormat::Jpeg,
        Format::Png => image::ImageFormat::Png,
        Format::Gif => image::ImageFormat::Gif,
        Format::WebP => image::ImageFormat::WebP,
    }
}
