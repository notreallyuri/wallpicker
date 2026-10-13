use std::borrow::Cow;
use std::path::Path;
use wallpicker_core::media::{Limits, RgbaImage, Source};

pub fn gif(path: &Path, width: u16, height: u16) -> Source {
    let mut bytes = Vec::new();
    {
        let mut encoder = gif::Encoder::new(&mut bytes, width, height, &[0, 0, 0, 255, 0, 0, 0, 255, 0]).unwrap();
        encoder.set_repeat(gif::Repeat::Finite(1)).unwrap();
        for (color, delay) in [(1, 0), (2, 7)] {
            encoder
                .write_frame(&gif::Frame {
                    width,
                    height,
                    buffer: Cow::Owned(vec![color; usize::from(width) * usize::from(height)]),
                    delay,
                    dispose: gif::DisposalMethod::Background,
                    ..Default::default()
                })
                .unwrap();
        }
    }
    std::fs::write(path, bytes).unwrap();
    Source::open(path, Limits::default()).unwrap()
}

fn chunk(bytes: &mut Vec<u8>, tag: &[u8; 4], data: &[u8]) {
    bytes.extend_from_slice(tag);
    bytes.extend_from_slice(&(data.len() as u32).to_le_bytes());
    bytes.extend_from_slice(data);
    if !data.len().is_multiple_of(2) {
        bytes.push(0);
    }
}

pub fn webp(path: &Path, width: u32, height: u32) -> Source {
    let mut body = b"WEBP".to_vec();
    let mut extended = vec![2, 0, 0, 0];
    extended.extend_from_slice(&(width - 1).to_le_bytes()[..3]);
    extended.extend_from_slice(&(height - 1).to_le_bytes()[..3]);
    chunk(&mut body, b"VP8X", &extended);
    chunk(&mut body, b"ANIM", &[0, 0, 0, 0, 1, 0]);
    for (color, delay) in [([255, 0, 0, 255], 0u32), ([0, 255, 0, 255], 70)] {
        let pixels = RgbaImage::from_raw(width, height, color.repeat((width * height) as usize)).unwrap();
        pixels.save(path).unwrap();
        let encoded = std::fs::read(path).unwrap();
        let mut frame = vec![0; 6];
        frame.extend_from_slice(&(width - 1).to_le_bytes()[..3]);
        frame.extend_from_slice(&(height - 1).to_le_bytes()[..3]);
        frame.extend_from_slice(&delay.to_le_bytes()[..3]);
        frame.push(2);
        frame.extend_from_slice(&encoded[12..]);
        chunk(&mut body, b"ANMF", &frame);
    }
    let mut bytes = b"RIFF".to_vec();
    bytes.extend_from_slice(&(body.len() as u32).to_le_bytes());
    bytes.extend_from_slice(&body);
    std::fs::write(path, bytes).unwrap();
    Source::open(path, Limits::default()).unwrap()
}
