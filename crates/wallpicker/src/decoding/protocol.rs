//! Private pull protocol: one request byte permits exactly one source frame.
//! Fixed headers and validated lengths bound allocations in the parent process.
use serde::{Deserialize, Serialize};
use std::io::{self, Read, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use wallpicker_core::media::{FileIdentity, Frame, Limits, RgbaImage, Source};

pub(super) const NEXT: u8 = 1;
const FRAME: u8 = 1;
const ERROR: u8 = 2;
const MAX_REQUEST: usize = 64 * 1024;
const MAX_ERROR: usize = 4096;

#[derive(Serialize, Deserialize)]
pub(super) struct Request {
    pub path: PathBuf,
    pub identity: FileIdentity,
    pub limits: Limits,
}

pub(super) fn request(source: &Source) -> io::Result<Vec<u8>> {
    let data = serde_json::to_vec(&Request {
        path: source.path().into(),
        identity: source.identity().clone(),
        limits: source.limits(),
    })?;
    if data.len() > MAX_REQUEST {
        return Err(io::Error::other("decoder request too large"));
    }
    let mut packet = Vec::with_capacity(4 + data.len());
    packet.extend_from_slice(&(data.len() as u32).to_le_bytes());
    packet.extend_from_slice(&data);
    Ok(packet)
}

pub(super) fn read_request(input: &mut impl Read) -> io::Result<Request> {
    let mut size = [0; 4];
    input.read_exact(&mut size)?;
    let len = u32::from_le_bytes(size) as usize;
    if len > MAX_REQUEST {
        return Err(io::Error::other("decoder request too large"));
    }
    let mut data = vec![0; len];
    input.read_exact(&mut data)?;
    Ok(serde_json::from_slice(&data)?)
}

pub(super) fn write_frame(output: &mut impl Write, frame: &Frame) -> io::Result<()> {
    output.write_all(&[FRAME])?;
    output.write_all(&frame.pixels.width().to_le_bytes())?;
    output.write_all(&frame.pixels.height().to_le_bytes())?;
    let nanos = frame.delay.map_or(u64::MAX, |d| d.as_nanos() as u64);
    output.write_all(&nanos.to_le_bytes())?;
    output.write_all(frame.pixels.as_raw())?;
    output.flush()
}

pub(super) fn write_error(output: &mut impl Write, error: impl std::fmt::Display) -> io::Result<()> {
    let message = error.to_string();
    let bytes = &message.as_bytes()[..message.len().min(MAX_ERROR)];
    output.write_all(&[ERROR])?;
    output.write_all(&(bytes.len() as u32).to_le_bytes())?;
    output.write_all(bytes)?;
    output.flush()
}

fn read(stream: &mut UnixStream, mut data: &mut [u8], deadline: Instant) -> io::Result<()> {
    while !data.is_empty() {
        let left = deadline.saturating_duration_since(Instant::now());
        if left.is_zero() {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "decoder frame deadline exceeded",
            ));
        }
        stream.set_read_timeout(Some(left))?;
        match stream.read(data) {
            Ok(0) => {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "decoder exited or disconnected",
                ));
            }
            Ok(n) => data = &mut data[n..],
            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(e),
        }
    }
    Ok(())
}

pub(super) fn read_frame(stream: &mut UnixStream, source: &Source, deadline: Instant) -> io::Result<Frame> {
    let mut tag = [0];
    read(stream, &mut tag, deadline)?;
    if tag[0] == ERROR {
        let mut len = [0; 4];
        read(stream, &mut len, deadline)?;
        let len = u32::from_le_bytes(len) as usize;
        if len > MAX_ERROR {
            return Err(io::Error::other("oversized decoder error"));
        }
        let mut bytes = vec![0; len];
        read(stream, &mut bytes, deadline)?;
        return Err(io::Error::other(String::from_utf8_lossy(&bytes).into_owned()));
    }
    if tag[0] != FRAME {
        return Err(io::Error::other("invalid decoder response"));
    }
    let mut header = [0; 16];
    read(stream, &mut header, deadline)?;
    let width = u32::from_le_bytes(header[..4].try_into().unwrap());
    let height = u32::from_le_bytes(header[4..8].try_into().unwrap());
    let nanos = u64::from_le_bytes(header[8..].try_into().unwrap());
    if (width, height) != source.info().dimensions() {
        return Err(io::Error::other("decoder frame dimensions changed"));
    }
    let len = super::frame_bytes(source)?;
    let mut bytes = Vec::new();
    bytes.try_reserve_exact(len).map_err(io::Error::other)?;
    bytes.resize(len, 0);
    read(stream, &mut bytes, deadline)?;
    if FileIdentity::read(source.path()).map_err(io::Error::other)? != *source.identity() {
        return Err(io::Error::other("media changed during decoding"));
    }
    Ok(Frame {
        pixels: RgbaImage::from_raw(width, height, bytes).ok_or_else(|| io::Error::other("invalid RGBA frame"))?,
        delay: (nanos != u64::MAX).then(|| Duration::from_nanos(nanos)),
    })
}
