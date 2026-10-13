//! Source/destination rectangles in pixels. No decoded pixels are touched here.
use wallpicker_core::{
    render::{Geometry, View},
    settings::{FitMode, Frame},
};

#[derive(Debug, Clone, Copy)]
pub(super) struct Mapping {
    pub source: [f32; 4],
    pub destination: [f32; 4],
    pub flip: [f32; 2],
    pub tiled: bool,
}

impl Mapping {
    pub(super) fn new(source: (u32, u32), output: (u32, u32), geometry: Geometry) -> Result<Self, String> {
        super::dimensions(source)?;
        super::dimensions(output)?;
        geometry.validate_for_source(source).map_err(|e| e.to_string())?;
        let (iw, ih) = (f64::from(source.0), f64::from(source.1));
        let (ow, oh) = (f64::from(output.0), f64::from(output.1));
        let mut crop = [0., 0., iw, ih];
        let mut destination = [0., 0., ow, oh];
        let mut tiled = false;
        match geometry.view {
            View::Region { x, y, width, height } => crop = [x as f64, y as f64, width as f64, height as f64],
            View::Frame { zoom, x, y } => {
                let frame = Frame {
                    zoom,
                    x,
                    y,
                    width: output.0,
                    height: output.1,
                };
                let (x, y, w, h) = frame.rect(source.0, source.1);
                // Rounding a tiny crop at an edge can extend it by one pixel.
                let (x, y) = (x.min(source.0 - 1), y.min(source.1 - 1));
                crop = [
                    x as f64,
                    y as f64,
                    w.clamp(1, source.0 - x) as f64,
                    h.clamp(1, source.1 - y) as f64,
                ];
            }
            View::Full => match geometry.fit.unwrap_or(FitMode::Cover) {
                FitMode::Fill => {}
                FitMode::Cover => {
                    if iw * oh > ih * ow {
                        crop[2] = ih * ow / oh;
                        crop[0] = (iw - crop[2]) / 2.;
                    } else {
                        crop[3] = iw * oh / ow;
                        crop[1] = (ih - crop[3]) / 2.;
                    }
                }
                FitMode::Contain => {
                    // Match static compose's integer-sized, centered black bars.
                    if u64::from(source.0) * u64::from(output.1) > u64::from(source.1) * u64::from(output.0) {
                        let height = (u64::from(source.1) * u64::from(output.0) / u64::from(source.0)).max(1) as u32;
                        destination[1] = ((output.1 - height) / 2) as f64;
                        destination[3] = height as f64;
                    } else {
                        let width = (u64::from(source.0) * u64::from(output.1) / u64::from(source.1)).max(1) as u32;
                        destination[0] = ((output.0 - width) / 2) as f64;
                        destination[2] = width as f64;
                    }
                }
                FitMode::Tile => tiled = true,
            },
        }
        Ok(Self {
            source: crop.map(|n| n as f32),
            destination: destination.map(|n| n as f32),
            flip: [u8::from(geometry.flip.h) as f32, u8::from(geometry.flip.v) as f32],
            tiled,
        })
    }
}
