use crate::settings::{Adjust, Cut, FitMode, Flip, Frame};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Geometry {
    pub fit: Option<FitMode>,
    pub flip: Flip,
    pub view: View,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum View {
    #[default]
    Full,
    Frame {
        zoom: u16,
        x: u16,
        y: u16,
    },
    Region {
        x: u32,
        y: u32,
        width: u32,
        height: u32,
    },
}

impl Geometry {
    pub fn validate_for_source(&self, dimensions: (u32, u32)) -> crate::Result<()> {
        super::validate::geometry(self, Some(dimensions))
    }

    pub fn from_adjust(adjust: &Adjust, fit: Option<FitMode>) -> Self {
        let view = if let Some(c) = adjust.span {
            View::Region {
                x: c.x,
                y: c.y,
                width: c.width,
                height: c.height,
            }
        } else if let Some(f) = adjust.frame {
            View::Frame {
                zoom: f.zoom,
                x: f.x,
                y: f.y,
            }
        } else {
            View::Full
        };
        Self {
            fit,
            flip: adjust.flip,
            view,
        }
    }

    /// Resolve portable framing/source cuts against the current output size.
    /// Source cut or frame precedes flips, matching static transform preparation.
    pub fn adjust(self, (width, height): (u32, u32)) -> Adjust {
        let mut adjust = Adjust {
            fit: self.fit,
            flip: self.flip,
            ..Default::default()
        };
        match self.view {
            View::Full => {}
            View::Frame { zoom, x, y } => {
                adjust.frame = Some(Frame {
                    zoom,
                    x,
                    y,
                    width,
                    height,
                })
            }
            View::Region {
                x,
                y,
                width: w,
                height: h,
            } => {
                adjust.span = Some(Cut {
                    x,
                    y,
                    width: w,
                    height: h,
                    out_width: width,
                    out_height: height,
                })
            }
        }
        adjust
    }
}
