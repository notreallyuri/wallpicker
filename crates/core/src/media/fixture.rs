//! Tiny deterministic containers generated in tests; no external tooling/assets.
use std::borrow::Cow;
use std::path::Path;

use super::{Limits, Source};

pub(super) const RED: [u8; 4] = [255, 0, 0, 255];
pub(super) const GREEN: [u8; 4] = [0, 255, 0, 255];
pub(super) const BLUE: [u8; 4] = [0, 0, 255, 255];
pub(super) const CLEAR: [u8; 4] = [0, 0, 0, 0];

pub(super) fn source(dir: &Path, data: &[u8]) -> Source {
    let path = dir.join("misleading.jpg");
    std::fs::write(&path, data).unwrap();
    Source::open(&path, Limits::default()).unwrap()
}

pub(super) struct Gif {
    pub x: u16,
    pub pixels: Vec<u8>,
    pub disposal: gif::DisposalMethod,
    pub delay: u16,
}

pub(super) fn gif(width: u16, frames: &[Gif], repeat: Option<gif::Repeat>) -> Vec<u8> {
    let mut bytes = Vec::new();
    {
        let mut encoder = gif::Encoder::new(&mut bytes, width, 1, &[0, 0, 0, 255, 0, 0, 0, 255, 0, 0, 0, 255]).unwrap();
        if let Some(repeat) = repeat {
            encoder.set_repeat(repeat).unwrap();
        }
        for frame in frames {
            encoder
                .write_frame(&gif::Frame {
                    left: frame.x,
                    width: frame.pixels.len() as u16,
                    height: 1,
                    buffer: Cow::Borrowed(&frame.pixels),
                    transparent: Some(0),
                    dispose: frame.disposal,
                    delay: frame.delay,
                    ..Default::default()
                })
                .unwrap();
        }
    }
    bytes
}

pub(super) fn still_webp(pixels: &[[u8; 4]]) -> Vec<u8> {
    let mut bytes = Vec::new();
    image::codecs::webp::WebPEncoder::new_lossless(&mut bytes)
        .encode(
            pixels.as_flattened(),
            pixels.len() as u32,
            1,
            image::ExtendedColorType::Rgba8,
        )
        .unwrap();
    bytes
}

pub(super) struct WebP {
    pub x: u32,
    pub pixels: Vec<[u8; 4]>,
    pub delay: u32,
    pub blend: bool,
    pub dispose: bool,
}

pub(super) fn chunk(out: &mut Vec<u8>, tag: &[u8; 4], data: &[u8]) {
    out.extend_from_slice(tag);
    out.extend_from_slice(&(data.len() as u32).to_le_bytes());
    out.extend_from_slice(data);
    if data.len() % 2 == 1 {
        out.push(0);
    }
}

pub(super) fn webp(width: u32, frames: &[WebP], repeat: u16) -> Vec<u8> {
    let mut body = b"WEBP".to_vec();
    let mut vp8x = vec![0x12, 0, 0, 0];
    vp8x.extend_from_slice(&(width - 1).to_le_bytes()[..3]);
    vp8x.extend_from_slice(&[0, 0, 0]);
    chunk(&mut body, b"VP8X", &vp8x);
    let mut anim = vec![0; 4];
    anim.extend_from_slice(&repeat.to_le_bytes());
    chunk(&mut body, b"ANIM", &anim);
    for f in frames {
        assert_eq!(f.x % 2, 0);
        let mut anmf = Vec::new();
        anmf.extend_from_slice(&(f.x / 2).to_le_bytes()[..3]);
        anmf.extend_from_slice(&[0, 0, 0]);
        anmf.extend_from_slice(&(f.pixels.len() as u32 - 1).to_le_bytes()[..3]);
        anmf.extend_from_slice(&[0, 0, 0]);
        anmf.extend_from_slice(&f.delay.to_le_bytes()[..3]);
        anmf.push(u8::from(f.dispose) | (u8::from(!f.blend) << 1));
        let encoded = still_webp(&f.pixels);
        anmf.extend_from_slice(&encoded[12..]);
        chunk(&mut body, b"ANMF", &anmf);
    }
    let mut bytes = b"RIFF".to_vec();
    bytes.extend_from_slice(&(body.len() as u32).to_le_bytes());
    bytes.extend_from_slice(&body);
    bytes
}
