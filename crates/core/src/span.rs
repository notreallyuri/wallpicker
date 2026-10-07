//! One picture spanned across the monitors: it covers the whole layout (as
//! Cover would one screen, centered) and each monitor shows its own part, at
//! its own resolution. The layout is Hyprland's: positions in logical pixels
//! (a monitor's pixels / its scale), rotated monitors turned. A bezel gap
//! (logical pixels) leaves out the part of the picture behind the frames
//! between neighbouring monitors, so lines carry on straight across them.
//! The span's look zooms and moves it like a monitor's frame (a frame over
//! the whole layout) and mirrors it as a whole.

use crate::monitors::Monitor;
use crate::settings::{Cut, Frame, SpanLook};

/// A monitor's place in the layout (logical pixels) and its resolution as
/// shown (pixels, after rotation).
struct Place {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    out_w: u32,
    out_h: u32,
}

fn place(m: &Monitor) -> Place {
    let turned = m.transform % 2 == 1;
    let (out_w, out_h) = if turned { (m.height, m.width) } else { (m.width, m.height) };
    let scale = if m.scale > 0.0 { m.scale } else { 1.0 };
    Place { x: f64::from(m.x), y: f64::from(m.y), w: f64::from(out_w) / scale, h: f64::from(out_h) / scale, out_w, out_h }
}

/// Each monitor's part of a `width`×`height` picture spanned across
/// `monitors`, in their order; `gap` logical pixels between neighbours.
/// Mirrored, a cut is from the other side of the picture (the monitor then
/// mirrors its part: `look.flip` on each).
pub fn cuts(monitors: &[Monitor], width: u32, height: u32, gap: u32, look: &SpanLook) -> Vec<(String, Cut)> {
    if monitors.is_empty() || width == 0 || height == 0 {
        return Vec::new();
    }
    let places: Vec<Place> = monitors.iter().map(place).collect();
    // The gap pushes a monitor right (down) once for each monitor wholly to
    // its left (above): the picture behind the bezels isn't shown.
    let gap = f64::from(gap);
    let shifted: Vec<(f64, f64)> = places
        .iter()
        .map(|p| {
            let left = places.iter().filter(|o| o.x + o.w <= p.x + 0.5).count() as f64;
            let above = places.iter().filter(|o| o.y + o.h <= p.y + 0.5).count() as f64;
            (p.x + left * gap, p.y + above * gap)
        })
        .collect();
    let min_x = shifted.iter().map(|s| s.0).fold(f64::INFINITY, f64::min);
    let min_y = shifted.iter().map(|s| s.1).fold(f64::INFINITY, f64::min);
    let max_x = places.iter().zip(&shifted).map(|(p, s)| s.0 + p.w).fold(f64::NEG_INFINITY, f64::max);
    let max_y = places.iter().zip(&shifted).map(|(p, s)| s.1 + p.h).fold(f64::NEG_INFINITY, f64::max);
    let (layout_w, layout_h) = (max_x - min_x, max_y - min_y);
    let (iw, ih) = (f64::from(width), f64::from(height));
    // The part of the picture over the whole layout: a frame in the
    // layout's shape (zoom 100, centered: Cover).
    let frame = Frame { zoom: look.zoom, x: look.x, y: look.y, width: layout_w.round().max(1.0) as u32, height: layout_h.round().max(1.0) as u32 };
    let (rl, rt, rw, _) = frame.rect(width, height);
    // Picture pixels per layout pixel.
    let s = f64::from(rw) / layout_w;
    monitors
        .iter()
        .zip(places.iter().zip(&shifted))
        .map(|(m, (p, &(sx, sy)))| {
            let (mut rx, mut ry) = (sx - min_x, sy - min_y);
            if look.flip.h {
                rx = layout_w - rx - p.w;
            }
            if look.flip.v {
                ry = layout_h - ry - p.h;
            }
            let left = (f64::from(rl) + rx * s).round().clamp(0.0, iw - 1.0);
            let top = (f64::from(rt) + ry * s).round().clamp(0.0, ih - 1.0);
            let w = (p.w * s).round().clamp(1.0, iw - left);
            let h = (p.h * s).round().clamp(1.0, ih - top);
            let cut = Cut { x: left as u32, y: top as u32, width: w as u32, height: h as u32, out_width: p.out_w, out_height: p.out_h };
            (m.name.clone(), cut)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn monitor(name: &str, width: u32, height: u32, x: i32, y: i32, scale: f64, transform: i32) -> Monitor {
        Monitor {
            name: name.into(),
            description: String::new(),
            width,
            height,
            x,
            y,
            focused: false,
            disabled: false,
            scale,
            transform,
            active_workspace: None,
        }
    }

    fn cut(x: u32, y: u32, width: u32, height: u32, out_width: u32, out_height: u32) -> Cut {
        Cut { x, y, width, height, out_width, out_height }
    }

    #[test]
    fn side_by_side_halves_a_wide_picture() {
        let ms = [monitor("DP-1", 1920, 1080, 0, 0, 1.0, 0), monitor("DP-2", 1920, 1080, 1920, 0, 1.0, 0)];
        let c = cuts(&ms, 3840, 1080, 0, &SpanLook::default());
        assert_eq!(c[0], ("DP-1".into(), cut(0, 0, 1920, 1080, 1920, 1080)));
        assert_eq!(c[1], ("DP-2".into(), cut(1920, 0, 1920, 1080, 1920, 1080)));
    }

    #[test]
    fn a_narrower_picture_is_covered_and_centered() {
        // A 16:9 picture over a 32:9 layout: scaled to the width, the
        // middle band kept.
        let ms = [monitor("DP-1", 1920, 1080, 0, 0, 1.0, 0), monitor("DP-2", 1920, 1080, 1920, 0, 1.0, 0)];
        let c = cuts(&ms, 3840, 2160, 0, &SpanLook::default());
        assert_eq!(c[0].1, cut(0, 540, 1920, 1080, 1920, 1080));
        assert_eq!(c[1].1, cut(1920, 540, 1920, 1080, 1920, 1080));
    }

    #[test]
    fn mixed_resolutions_keep_their_place_and_size() {
        // QHD at the left, FHD at the right, lower and centered.
        let ms = [monitor("DP-1", 2560, 1440, 0, 0, 1.0, 0), monitor("DP-2", 1920, 1080, 2560, 180, 1.0, 0)];
        let c = cuts(&ms, 4480, 1440, 0, &SpanLook::default());
        assert_eq!(c[0].1, cut(0, 0, 2560, 1440, 2560, 1440));
        assert_eq!(c[1].1, cut(2560, 180, 1920, 1080, 1920, 1080));
    }

    #[test]
    fn a_scaled_monitor_takes_its_logical_size_and_keeps_its_pixels() {
        // A 4K monitor at scale 2 is 1920x1080 in the layout.
        let ms = [monitor("DP-1", 3840, 2160, 0, 0, 2.0, 0), monitor("DP-2", 1920, 1080, 1920, 0, 1.0, 0)];
        let c = cuts(&ms, 3840, 1080, 0, &SpanLook::default());
        assert_eq!(c[0].1, cut(0, 0, 1920, 1080, 3840, 2160));
        assert_eq!(c[1].1, cut(1920, 0, 1920, 1080, 1920, 1080));
    }

    #[test]
    fn a_turned_monitor_is_tall() {
        let ms = [monitor("DP-1", 1920, 1080, 0, 0, 1.0, 0), monitor("DP-2", 1920, 1080, 1920, 0, 1.0, 1)];
        let c = cuts(&ms, 3000, 1920, 0, &SpanLook::default());
        // The layout is 3000x1920 (1080 wide, 1920 tall at the right).
        assert_eq!(c[1].1, cut(1920, 0, 1080, 1920, 1080, 1920));
        assert_eq!(c[0].1.out_width, 1920);
    }

    #[test]
    fn a_bezel_gap_skips_the_picture_behind_the_frames() {
        let ms = [monitor("DP-1", 1920, 1080, 0, 0, 1.0, 0), monitor("DP-2", 1920, 1080, 1920, 0, 1.0, 0)];
        let c = cuts(&ms, 3880, 1080, 40, &SpanLook::default());
        assert_eq!(c[0].1, cut(0, 0, 1920, 1080, 1920, 1080));
        assert_eq!(c[1].1, cut(1960, 0, 1920, 1080, 1920, 1080));
    }

    #[test]
    fn zoomed_in_it_shows_less_of_the_picture_around_the_center() {
        let ms = [monitor("DP-1", 1920, 1080, 0, 0, 1.0, 0), monitor("DP-2", 1920, 1080, 1920, 0, 1.0, 0)];
        let look = SpanLook { zoom: 200, ..SpanLook::default() };
        let c = cuts(&ms, 3840, 1080, 0, &look);
        // Half the picture, centered: 960..2880 across both monitors.
        assert_eq!(c[0].1, cut(960, 270, 960, 540, 1920, 1080));
        assert_eq!(c[1].1, cut(1920, 270, 960, 540, 1920, 1080));
        // Moved right as far as it goes.
        let right = SpanLook { zoom: 200, x: 1000, ..SpanLook::default() };
        assert_eq!(cuts(&ms, 3840, 1080, 0, &right)[1].1.x, 2880);
    }

    #[test]
    fn mirrored_each_monitor_takes_the_other_side() {
        let ms = [monitor("DP-1", 1920, 1080, 0, 0, 1.0, 0), monitor("DP-2", 1920, 1080, 1920, 0, 1.0, 0)];
        let look = SpanLook { flip: crate::settings::Flip { h: true, v: false }, ..SpanLook::default() };
        let c = cuts(&ms, 3840, 1080, 0, &look);
        // DP-1 (left) shows the right half, mirrored by its own flip.
        assert_eq!(c[0].1.x, 1920);
        assert_eq!(c[1].1.x, 0);
    }

    #[test]
    fn nothing_to_cut_for_nothing() {
        assert!(cuts(&[], 100, 100, 0, &SpanLook::default()).is_empty());
        assert!(cuts(&[monitor("DP-1", 1920, 1080, 0, 0, 1.0, 0)], 0, 0, 0, &SpanLook::default()).is_empty());
    }
}
