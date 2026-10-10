//! Move the incoming image across the output, clipping outside its bounds.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Left,
    Right,
    Top,
    Bottom,
}

impl Direction {
    pub(super) fn vector(self) -> (f32, f32) {
        match self {
            Self::Left => (1.0, 0.0),
            Self::Right => (-1.0, 0.0),
            Self::Top => (0.0, 1.0),
            Self::Bottom => (0.0, -1.0),
        }
    }
}

pub(super) const SHADER: &str = include_str!("slide.frag");
