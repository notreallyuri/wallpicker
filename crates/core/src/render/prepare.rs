//! Preflight without renderer calls or state changes. Only static sources are
//! baked into cache files; animated sources retain original geometry and identity.
use super::{Content, Geometry, Picture};
use crate::{
    Error, Result, media,
    settings::{Adjust, FitMode},
};
use std::{
    collections::BTreeMap,
    fs::File,
    io::BufReader,
    path::{Path, PathBuf},
};

#[derive(Clone)]
struct Inspected {
    dimensions: (u32, u32),
    animated: Option<media::FileIdentity>,
}

/// Static sources are checked from their headers: the renderer decodes them
/// anyway and reports a broken file per output. [`Preparation::verifying`]
/// decodes them completely, for callers that must not apply anything unless
/// every image is readable (presets).
#[derive(Default)]
pub struct Preparation {
    sources: BTreeMap<PathBuf, Inspected>,
    verify: bool,
}

impl Preparation {
    pub fn verifying() -> Self {
        Self {
            verify: true,
            ..Self::default()
        }
    }

    pub fn is_animated(&mut self, path: &Path) -> Result<bool> {
        self.inspect(path)?;
        Ok(self.sources[path].animated.is_some())
    }
    /// Animation limits apply only after detecting animated content.
    pub fn inspect(&mut self, path: &Path) -> Result<(u32, u32)> {
        if let Some(info) = self.sources.get(path) {
            return Ok(info.dimensions);
        }
        let reader = image::ImageReader::open(path)?.with_guessed_format()?;
        let animation = match reader.format() {
            Some(image::ImageFormat::WebP) => image::codecs::webp::WebPDecoder::new(BufReader::new(File::open(path)?))
                .map_err(|e| Error::Image(e.to_string()))?
                .has_animation(),
            Some(image::ImageFormat::Gif) => true,
            Some(image::ImageFormat::Png | image::ImageFormat::Jpeg) => false,
            _ => return Err(Error::Invalid(format!("unsupported media: {}", path.display()))),
        };
        let info = if animation {
            let source = media::Source::open(path, media::Limits::default())?;
            let animated = (source.info().kind == media::Kind::Animated).then(|| source.identity().clone());
            // Animated frames are decoded only by bounded, isolated workers;
            // metadata and geometry alone are safe to read here.
            if animated.is_none() && self.verify {
                source.representative()?;
            }
            Inspected {
                dimensions: source.info().dimensions(),
                animated,
            }
        } else {
            let failed = |e: image::ImageError| Error::Image(format!("{}: {e}", path.display()));
            let dimensions = if self.verify {
                let decoded = reader.decode().map_err(failed)?;
                (decoded.width(), decoded.height())
            } else {
                reader.into_dimensions().map_err(failed)?
            };
            Inspected {
                dimensions,
                animated: None,
            }
        };
        let dimensions = info.dimensions;
        self.sources.insert(path.to_owned(), info);
        Ok(dimensions)
    }

    pub fn picture(&mut self, cache: &Path, path: &Path, adjust: &Adjust, fit: Option<FitMode>) -> Result<Picture> {
        let dimensions = self.inspect(path)?;
        let geometry = Geometry::from_adjust(adjust, fit);
        geometry.validate_for_source(dimensions)?;
        let prepared_size = adjust
            .span
            .map(|c| (c.out_width, c.out_height))
            .or_else(|| adjust.frame.map(|f| (f.width, f.height)));
        if prepared_size.is_some_and(|(w, h)| w == 0 || h == 0 || w > 16384 || h > 16384) {
            return Err(Error::Invalid("invalid target dimensions".into()));
        }
        let content = if let Some(identity) = &self.sources[path].animated {
            Content::Animated {
                identity: identity.clone(),
            }
        } else {
            Content::Static {
                path: crate::transform::shown_file(cache, path, adjust)?,
                fit: if prepared_size.is_some() { None } else { fit },
                prepared_size,
            }
        };
        Ok(Picture {
            source: path.to_owned(),
            geometry,
            content,
        })
    }
}
