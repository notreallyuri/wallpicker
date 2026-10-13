//! Codec adapters. Returned buffers belong to the caller; only codec state is
//! retained here. Never accumulate frames or resize to output dimensions.
use std::io::BufReader;
use std::time::Duration;

use super::{FileIdentity, Format, Frame, Kind, Result, Source, context, invalid, probe};
use image::{AnimationDecoder, ImageDecoder};

type Reader = BufReader<probe::Input>;

enum Decoder {
    Gif(image::Frames<'static>),
    WebP(Box<super::webp::WebP>),
    Still(Option<Reader>),
}

/// A fused, one-pass stream. On error it releases the decoder and yields that
/// error once. To loop, open a new stream from the Source (finite loop metadata
/// does not constrain the caller). No worker or clock exists in this layer.
pub struct Frames {
    source: Source,
    decoder: Option<Decoder>,
    decoded: u32,
}

impl Frames {
    pub(super) fn open(source: Source) -> Result<Self> {
        let input = probe::Input::open(&source.path, &source.identity)?;
        let decoder = match source.info.format {
            Format::Gif => {
                let mut decoder =
                    image::codecs::gif::GifDecoder::new(BufReader::new(input)).map_err(|e| context(&source.path, e))?;
                decoder
                    .set_limits(source.limits.image())
                    .map_err(|e| context(&source.path, e))?;
                if decoder.dimensions() != source.info.dimensions() {
                    return Err(invalid("GIF dimensions changed"));
                }
                Decoder::Gif(decoder.into_frames())
            }
            Format::WebP => Decoder::WebP(Box::new(super::webp::WebP::open(input, &source)?)),
            _ => Decoder::Still(Some(BufReader::new(input))),
        };
        Ok(Self {
            source,
            decoder: Some(decoder),
            decoded: 0,
        })
    }

    fn decode(&mut self) -> Result<Frame> {
        let source = &self.source;
        if FileIdentity::read(&source.path)? != source.identity {
            return Err(invalid("media changed since inspection"));
        }
        let decoder = self.decoder.as_mut().expect("active stream");
        let (pixels, delay) = match decoder {
            Decoder::Gif(frames) => {
                let frame = frames
                    .next()
                    .ok_or_else(|| invalid("GIF ended before its inspected frame count"))?
                    .map_err(|e| invalid(e.to_string()))?;
                let delay = Duration::from(frame.delay());
                (frame.into_buffer(), Some(delay))
            }
            Decoder::WebP(decoder) => {
                let frame = decoder.frame()?;
                (frame.pixels, frame.delay)
            }
            Decoder::Still(reader) => {
                let mut reader = image::ImageReader::with_format(
                    reader.take().expect("one still frame"),
                    probe::image_format(source.info.format),
                );
                reader.limits(source.limits.image());
                (reader.decode().map_err(|e| invalid(e.to_string()))?.into_rgba8(), None)
            }
        };
        if pixels.dimensions() != source.info.dimensions() {
            return Err(invalid("decoded frame dimensions changed"));
        }
        if FileIdentity::read(&source.path)? != source.identity {
            return Err(invalid("media changed during decoding"));
        }
        Ok(Frame {
            pixels,
            delay: if source.info.kind == Kind::Static { None } else { delay },
        })
    }
}

impl Iterator for Frames {
    type Item = Result<Frame>;

    fn next(&mut self) -> Option<Self::Item> {
        self.decoder.as_ref()?;
        let result = self.decode().map_err(|e| context(&self.source.path, e));
        self.decoded += 1;
        if result.is_err() || self.decoded == self.source.info.frame_count {
            self.decoder = None;
        }
        Some(result)
    }
}

impl std::iter::FusedIterator for Frames {}
