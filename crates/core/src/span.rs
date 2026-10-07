//! One picture spanned across some monitors: it covers their arrangement
//! (as Cover would one screen) and each monitor shows its own part, at its
//! own resolution. The arrangement is wallpicker's own canvas, in
//! millimetres: each monitor at its physical size, so lines carry on
//! straight in the real world across different sizes and densities, and
//! the gaps between monitors (their bezels) leave out the picture behind
//! them. Its first version comes from Hyprland's layout. The span's look
//! zooms and moves the picture like a monitor's frame (a frame over the
//! whole arrangement) and mirrors it as a whole.

use std::collections::BTreeMap;

use crate::monitors::Monitor;
use crate::settings::{Cut, Frame, SpanLook, SpanPoint, SpanRect};

/// Millimetres per logical pixel when a monitor doesn't say its size
/// (96 dpi).
const MM_PER_PX: f64 = 25.4 / 96.0;

/// A monitor on the canvas: its place and size (mm) and its resolution as
/// shown (pixels, after rotation).
#[derive(Debug, Clone, PartialEq)]
pub struct Placed {
    pub name: String,
    pub rect: SpanRect,
    pub out_width: u32,
    pub out_height: u32,
}

fn turned(m: &Monitor) -> bool {
    m.transform % 2 == 1
}

/// The resolution as shown (turned monitors are tall).
fn shown_size(m: &Monitor) -> (u32, u32) {
    if turned(m) { (m.height, m.width) } else { (m.width, m.height) }
}

/// Its size in logical pixels (Hyprland's layout).
fn logical_size(m: &Monitor) -> (f64, f64) {
    let (w, h) = shown_size(m);
    let scale = if m.scale > 0.0 { m.scale } else { 1.0 };
    (f64::from(w) / scale, f64::from(h) / scale)
}

/// Its physical size as it stands (mm): what it reports, else its logical
/// size at 96 dpi.
pub fn physical_size(m: &Monitor) -> (f64, f64) {
    if m.physical_width > 0 && m.physical_height > 0 {
        let (w, h) = (f64::from(m.physical_width), f64::from(m.physical_height));
        if turned(m) { (h, w) } else { (w, h) }
    } else {
        let (w, h) = logical_size(m);
        (w * MM_PER_PX, h * MM_PER_PX)
    }
}

/// The canvas from Hyprland's layout: a monitor that touches one already
/// placed goes against it, `bezel` mm apart, its offset along the edge
/// kept in proportion; others where their logical position puts them. The
/// top left at (0, 0).
pub fn default_layout(monitors: &[Monitor], bezel: u32) -> BTreeMap<String, SpanPoint> {
    let bezel = f64::from(bezel);
    let mut order: Vec<&Monitor> = monitors.iter().collect();
    order.sort_by_key(|m| (m.x, m.y));
    let near = |a: f64, b: f64| (a - b).abs() < 2.0;
    let overlaps = |a0: f64, a1: f64, b0: f64, b1: f64| a0 < b1 && b0 < a1;
    let mut placed: Vec<(&Monitor, SpanRect)> = Vec::new();
    for m in order {
        let (lw, lh) = logical_size(m);
        let (pw, ph) = physical_size(m);
        let (mx, my) = (f64::from(m.x), f64::from(m.y));
        let mut at = None;
        for (n, r) in &placed {
            let (nw, nh) = logical_size(n);
            let (nx, ny) = (f64::from(n.x), f64::from(n.y));
            // Millimetres per logical pixel along the neighbour's edges.
            let (kx, ky) = (r.w / nw.max(1.0), r.h / nh.max(1.0));
            if overlaps(my, my + lh, ny, ny + nh) {
                let y = r.y + (my - ny) * ky;
                if near(mx, nx + nw) {
                    at = Some((r.x + r.w + bezel, y));
                } else if near(mx + lw, nx) {
                    at = Some((r.x - pw - bezel, y));
                }
            }
            if at.is_none() && overlaps(mx, mx + lw, nx, nx + nw) {
                let x = r.x + (mx - nx) * kx;
                if near(my, ny + nh) {
                    at = Some((x, r.y + r.h + bezel));
                } else if near(my + lh, ny) {
                    at = Some((x, r.y - ph - bezel));
                }
            }
            if at.is_some() {
                break;
            }
        }
        let (x, y) = at.unwrap_or((mx * pw / lw.max(1.0), my * ph / lh.max(1.0)));
        placed.push((m, SpanRect { x, y, w: pw, h: ph }));
    }
    let min_x = placed.iter().map(|(_, r)| r.x).fold(f64::INFINITY, f64::min);
    let min_y = placed.iter().map(|(_, r)| r.y).fold(f64::INFINITY, f64::min);
    placed.into_iter().map(|(m, r)| (m.name.clone(), SpanPoint { x: r.x - min_x, y: r.y - min_y })).collect()
}

/// Every monitor on the canvas: where `saved` puts it, else where
/// [`default_layout`] does.
pub fn arrangement(monitors: &[Monitor], saved: &BTreeMap<String, SpanPoint>, bezel: u32) -> Vec<Placed> {
    let fallback = default_layout(monitors, bezel);
    monitors
        .iter()
        .map(|m| {
            let at = saved.get(&m.name).or_else(|| fallback.get(&m.name)).copied().unwrap_or_default();
            let (w, h) = physical_size(m);
            let (out_width, out_height) = shown_size(m);
            Placed { name: m.name.clone(), rect: SpanRect { x: at.x, y: at.y, w, h }, out_width, out_height }
        })
        .collect()
}

/// Each monitor's part of a `width`×`height` picture spanned across
/// `placed`, in their order. Mirrored, a cut is from the other side of the
/// picture (the monitor then mirrors its part: `look.flip` on each).
pub fn cuts(placed: &[Placed], width: u32, height: u32, look: &SpanLook) -> Vec<(String, Cut)> {
    if placed.is_empty() || width == 0 || height == 0 {
        return Vec::new();
    }
    let min_x = placed.iter().map(|p| p.rect.x).fold(f64::INFINITY, f64::min);
    let min_y = placed.iter().map(|p| p.rect.y).fold(f64::INFINITY, f64::min);
    let max_x = placed.iter().map(|p| p.rect.x + p.rect.w).fold(f64::NEG_INFINITY, f64::max);
    let max_y = placed.iter().map(|p| p.rect.y + p.rect.h).fold(f64::NEG_INFINITY, f64::max);
    let (span_w, span_h) = ((max_x - min_x).max(1.0), (max_y - min_y).max(1.0));
    let (iw, ih) = (f64::from(width), f64::from(height));
    // The part of the picture over the whole arrangement: a frame in its
    // shape (tenths of mm, for the proportions); zoom 100, centered: Cover.
    let frame = Frame { zoom: look.zoom, x: look.x, y: look.y, width: (span_w * 10.0).round() as u32, height: (span_h * 10.0).round() as u32 };
    let (rl, rt, rw, _) = frame.rect(width, height);
    // Picture pixels per millimetre.
    let s = f64::from(rw) / span_w;
    placed
        .iter()
        .map(|p| {
            let (mut rx, mut ry) = (p.rect.x - min_x, p.rect.y - min_y);
            if look.flip.h {
                rx = span_w - rx - p.rect.w;
            }
            if look.flip.v {
                ry = span_h - ry - p.rect.h;
            }
            let left = (f64::from(rl) + rx * s).round().clamp(0.0, iw - 1.0);
            let top = (f64::from(rt) + ry * s).round().clamp(0.0, ih - 1.0);
            let w = (p.rect.w * s).round().clamp(1.0, iw - left);
            let h = (p.rect.h * s).round().clamp(1.0, ih - top);
            let cut = Cut { x: left as u32, y: top as u32, width: w as u32, height: h as u32, out_width: p.out_width, out_height: p.out_height };
            (p.name.clone(), cut)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::Flip;

    /// A monitor reporting its physical size (0: unknown).
    fn monitor(name: &str, (width, height): (u32, u32), (x, y): (i32, i32), (pw, ph): (u32, u32)) -> Monitor {
        Monitor {
            name: name.into(),
            description: String::new(),
            width,
            height,
            x,
            y,
            focused: false,
            disabled: false,
            scale: 1.0,
            transform: 0,
            physical_width: pw,
            physical_height: ph,
            active_workspace: None,
        }
    }

    fn placed(name: &str, (x, y, w, h): (f64, f64, f64, f64), out: (u32, u32)) -> Placed {
        Placed { name: name.into(), rect: SpanRect { x, y, w, h }, out_width: out.0, out_height: out.1 }
    }

    fn cut(x: u32, y: u32, width: u32, height: u32, out_width: u32, out_height: u32) -> Cut {
        Cut { x, y, width, height, out_width, out_height }
    }

    #[test]
    fn hyprland_side_by_side_becomes_touching_on_the_canvas() {
        let ms = [monitor("DP-1", (1920, 1080), (0, 0), (600, 340)), monitor("DP-2", (1920, 1080), (1920, 0), (600, 340))];
        let l = default_layout(&ms, 0);
        assert_eq!(l["DP-1"], SpanPoint { x: 0.0, y: 0.0 });
        assert_eq!(l["DP-2"], SpanPoint { x: 600.0, y: 0.0 });
        // With bezels, that far apart.
        assert_eq!(default_layout(&ms, 20)["DP-2"].x, 620.0);
    }

    #[test]
    fn different_densities_touch_by_their_real_size() {
        // A 27" QHD at the left, a 24" FHD right of it, 180 logical px
        // lower in Hyprland.
        let ms = [monitor("DP-1", (2560, 1440), (0, 0), (597, 336)), monitor("DP-2", (1920, 1080), (2560, 180), (531, 299))];
        let l = default_layout(&ms, 0);
        assert_eq!(l["DP-2"].x, 597.0);
        // 180 of the QHD's 1440 logical px is 42 of its 336 mm.
        assert!((l["DP-2"].y - 42.0).abs() < 0.01);
    }

    #[test]
    fn a_monitor_above_goes_above() {
        // M3 above M1, M2 right of M1.
        let ms = [
            monitor("M1", (1920, 1080), (0, 0), (600, 340)),
            monitor("M2", (1920, 1080), (1920, 0), (600, 340)),
            monitor("M3", (1920, 1080), (0, -1080), (600, 340)),
        ];
        let l = default_layout(&ms, 0);
        assert_eq!(l["M3"], SpanPoint { x: 0.0, y: 0.0 });
        assert_eq!(l["M1"], SpanPoint { x: 0.0, y: 340.0 });
        assert_eq!(l["M2"], SpanPoint { x: 600.0, y: 340.0 });
    }

    #[test]
    fn an_unknown_size_is_taken_at_96_dpi() {
        let m = monitor("X", (1920, 1080), (0, 0), (0, 0));
        let (w, h) = physical_size(&m);
        assert!((w - 508.0).abs() < 0.01 && (h - 285.75).abs() < 0.01);
    }

    #[test]
    fn saved_places_win_over_hyprland() {
        let ms = [monitor("DP-1", (1920, 1080), (0, 0), (600, 340)), monitor("DP-2", (1920, 1080), (1920, 0), (600, 340))];
        let saved = BTreeMap::from([("DP-2".to_string(), SpanPoint { x: 610.0, y: 50.0 })]);
        let a = arrangement(&ms, &saved, 0);
        assert_eq!(a[1].rect, SpanRect { x: 610.0, y: 50.0, w: 600.0, h: 340.0 });
        assert_eq!(a[0].rect.x, 0.0);
    }

    #[test]
    fn side_by_side_halves_a_wide_picture() {
        let p = [placed("DP-1", (0.0, 0.0, 600.0, 340.0), (1920, 1080)), placed("DP-2", (600.0, 0.0, 600.0, 340.0), (1920, 1080))];
        let c = cuts(&p, 3840, 1088, &SpanLook::default());
        assert_eq!(c[0], ("DP-1".into(), cut(0, 0, 1920, 1088, 1920, 1080)));
        assert_eq!(c[1], ("DP-2".into(), cut(1920, 0, 1920, 1088, 1920, 1080)));
    }

    #[test]
    fn a_narrower_picture_is_covered_and_centered() {
        // 32:9 of monitors over a 16:9 picture: its middle band.
        let p = [placed("DP-1", (0.0, 0.0, 600.0, 337.5), (1920, 1080)), placed("DP-2", (600.0, 0.0, 600.0, 337.5), (1920, 1080))];
        let c = cuts(&p, 3840, 2160, &SpanLook::default());
        assert_eq!(c[0].1, cut(0, 540, 1920, 1080, 1920, 1080));
        assert_eq!(c[1].1, cut(1920, 540, 1920, 1080, 1920, 1080));
    }

    #[test]
    fn a_smaller_monitor_gets_its_real_share() {
        // Same resolution, but DP-2 is half as wide in the real world: it
        // gets half as much of the picture, at its own resolution.
        let p = [placed("DP-1", (0.0, 0.0, 600.0, 300.0), (1920, 1080)), placed("DP-2", (600.0, 0.0, 300.0, 150.0), (1920, 1080))];
        let c = cuts(&p, 900, 300, &SpanLook::default());
        assert_eq!(c[0].1, cut(0, 0, 600, 300, 1920, 1080));
        assert_eq!(c[1].1, cut(600, 0, 300, 150, 1920, 1080));
    }

    #[test]
    fn a_bezel_gap_skips_the_picture_behind_the_frames() {
        let p = [placed("DP-1", (0.0, 0.0, 600.0, 340.0), (1920, 1080)), placed("DP-2", (620.0, 0.0, 600.0, 340.0), (1920, 1080))];
        let c = cuts(&p, 1220, 340, &SpanLook::default());
        assert_eq!((c[0].1.x, c[1].1.x), (0, 620));
    }

    #[test]
    fn zoomed_in_it_shows_less_of_the_picture_around_the_center() {
        let p = [placed("DP-1", (0.0, 0.0, 600.0, 337.5), (1920, 1080)), placed("DP-2", (600.0, 0.0, 600.0, 337.5), (1920, 1080))];
        let look = SpanLook { zoom: 200, ..SpanLook::default() };
        let c = cuts(&p, 3840, 1080, &look);
        assert_eq!(c[0].1, cut(960, 270, 960, 540, 1920, 1080));
        assert_eq!(c[1].1, cut(1920, 270, 960, 540, 1920, 1080));
        let right = SpanLook { zoom: 200, x: 1000, ..SpanLook::default() };
        assert_eq!(cuts(&p, 3840, 1080, &right)[1].1.x, 2880);
    }

    #[test]
    fn mirrored_each_monitor_takes_the_other_side() {
        let p = [placed("DP-1", (0.0, 0.0, 600.0, 337.5), (1920, 1080)), placed("DP-2", (600.0, 0.0, 600.0, 337.5), (1920, 1080))];
        let look = SpanLook { flip: Flip { h: true, v: false }, ..SpanLook::default() };
        let c = cuts(&p, 3840, 1080, &look);
        assert_eq!((c[0].1.x, c[1].1.x), (1920, 0));
    }

    #[test]
    fn nothing_to_cut_for_nothing() {
        assert!(cuts(&[], 100, 100, &SpanLook::default()).is_empty());
        assert!(cuts(&[placed("DP-1", (0.0, 0.0, 1.0, 1.0), (1, 1))], 0, 0, &SpanLook::default()).is_empty());
    }
}
