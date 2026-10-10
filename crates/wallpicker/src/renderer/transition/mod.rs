//! Effect selection and elapsed-time progress; effect shaders live separately.

mod circle;
mod slide;
mod wave;
mod wipe;

use slide::Direction;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Effect {
    Fade,
    Wipe,
    Slide(Direction),
    Circle { origin: (u16, u16), shrinking: bool },
    Wave,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Transition {
    pub effect: Effect,
    pub duration: Duration,
}

impl Transition {
    /// Unsupported saved kinds keep their previous Fade fallback.
    pub fn from_setting(kind: &str, duration_ms: u32) -> Option<Self> {
        if kind == "none" || duration_ms == 0 {
            return None;
        }
        let effect = match kind {
            "wipe" => Effect::Wipe,
            "left" => Effect::Slide(Direction::Left),
            "right" => Effect::Slide(Direction::Right),
            "top" => Effect::Slide(Direction::Top),
            "bottom" => Effect::Slide(Direction::Bottom),
            "grow" | "center" => Effect::Circle {
                origin: (500, 500),
                shrinking: false,
            },
            "any" => Effect::Circle {
                origin: (random(1001) as u16, random(1001) as u16),
                shrinking: false,
            },
            "outer" => Effect::Circle {
                origin: (500, 500),
                shrinking: true,
            },
            "wave" => Effect::Wave,
            "random" => match random(9) {
                0 => Effect::Fade,
                1 => Effect::Wipe,
                2 => Effect::Slide(Direction::Left),
                3 => Effect::Slide(Direction::Right),
                4 => Effect::Slide(Direction::Top),
                5 => Effect::Slide(Direction::Bottom),
                6 => Effect::Circle {
                    origin: (500, 500),
                    shrinking: false,
                },
                7 => Effect::Circle {
                    origin: (500, 500),
                    shrinking: true,
                },
                _ => Effect::Wave,
            },
            _ => Effect::Fade,
        };
        Some(Self {
            effect,
            duration: Duration::from_millis(duration_ms.into()),
        })
    }

    pub fn progress(&self, elapsed: Duration) -> f64 {
        if self.duration.is_zero() {
            return 1.0;
        }
        (elapsed.as_secs_f64() / self.duration.as_secs_f64()).clamp(0.0, 1.0)
    }
}

impl Effect {
    pub(in crate::renderer) fn shader(self) -> Option<&'static str> {
        match self {
            Self::Fade => None,
            Self::Wipe => Some(wipe::SHADER),
            Self::Slide(_) => Some(slide::SHADER),
            Self::Circle {
                shrinking: false, ..
            } => Some(circle::GROW),
            Self::Circle {
                shrinking: true, ..
            } => Some(circle::SHRINK),
            Self::Wave => Some(wave::SHADER),
        }
    }

    pub(in crate::renderer) fn parameters(self) -> (f32, f32) {
        match self {
            Self::Slide(direction) => direction.vector(),
            Self::Circle { origin: (x, y), .. } => (x as f32 / 1000.0, y as f32 / 1000.0),
            _ => (0.0, 0.0),
        }
    }
}

/// Session-local variation; no cryptographic guarantees are needed for effects.
fn random(limit: u64) -> u64 {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let mut n = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64
        ^ COUNTER
            .fetch_add(1, Ordering::Relaxed)
            .wrapping_mul(0x9e3779b97f4a7c15);
    n ^= n >> 30;
    n = n.wrapping_mul(0xbf58476d1ce4e5b9);
    n ^= n >> 27;
    n = n.wrapping_mul(0x94d049bb133111eb);
    (n ^ (n >> 31)) % limit
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_select_implemented_effects() {
        for kind in [
            "wipe", "left", "right", "top", "bottom", "grow", "center", "any", "outer", "wave",
        ] {
            assert!(
                Transition::from_setting(kind, 800)
                    .unwrap()
                    .effect
                    .shader()
                    .is_some(),
                "{kind}"
            );
        }
        for kind in ["fade", "simple", "unknown"] {
            assert_eq!(
                Transition::from_setting(kind, 800).unwrap().effect,
                Effect::Fade
            );
        }
        assert!(Transition::from_setting("none", 800).is_none());
        assert!(Transition::from_setting("wipe", 0).is_none());
    }

    #[test]
    fn every_offered_choice_has_an_implementation() {
        use wallpicker_core::settings::{WallpaperBackend, transition_modes};
        for (kind, _) in transition_modes(WallpaperBackend::Wallpicker) {
            let transition = Transition::from_setting(kind, 1000);
            if kind == "none" {
                assert!(transition.is_none());
            } else {
                let effect = transition.unwrap().effect;
                assert!(
                    matches!(kind, "fade" | "random") || effect.shader().is_some(),
                    "{kind}"
                );
                assert_ne!(kind, "simple");
            }
        }
    }

    #[test]
    fn progress_reaches_exact_endpoints() {
        let t = Transition::from_setting("fade", 1000).unwrap();
        assert_eq!(t.progress(Duration::ZERO), 0.0);
        assert_eq!(t.progress(Duration::from_millis(250)), 0.25);
        assert_eq!(t.progress(Duration::from_secs(3)), 1.0);
        assert_eq!(
            Transition {
                duration: Duration::ZERO,
                ..t
            }
            .progress(Duration::ZERO),
            1.0
        );
    }

    #[test]
    fn directions_and_origins_are_bounded() {
        assert_eq!(Effect::Slide(Direction::Left).parameters(), (1.0, 0.0));
        assert_eq!(Effect::Slide(Direction::Right).parameters(), (-1.0, 0.0));
        assert_eq!(Effect::Slide(Direction::Top).parameters(), (0.0, 1.0));
        assert_eq!(Effect::Slide(Direction::Bottom).parameters(), (0.0, -1.0));
        for _ in 0..100 {
            let (x, y) = Transition::from_setting("any", 800)
                .unwrap()
                .effect
                .parameters();
            assert!((0.0..=1.0).contains(&x) && (0.0..=1.0).contains(&y));
        }
    }
}
