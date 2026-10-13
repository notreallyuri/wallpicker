//! Representative stills after hardware EGL initialization fails. Conversion and
//! decoder destruction run on the caller; output fitting runs on image workers.
use super::{PendingShow, Picture, Transition, image::Want, state::State};
use crate::preparation::Handoff;
use std::sync::{Arc, Mutex, mpsc};
use wallpicker_core::{
    compose::{self, RgbImage},
    media::FileIdentity,
    render::{Batch, Content},
};

#[derive(Clone)]
pub(super) struct Budget(Arc<Pool>);
struct Pool {
    limit: u64,
    used: Mutex<u64>,
}
struct Reservation {
    budget: Budget,
    bytes: u64,
}
impl Budget {
    pub(super) fn new(limit: u64) -> Self {
        Self(Arc::new(Pool {
            limit,
            used: Mutex::new(0),
        }))
    }
    pub(super) fn used(&self) -> u64 {
        *self.0.used.lock().unwrap()
    }
    fn reserve(&self, bytes: u64) -> Result<Reservation, String> {
        let mut used = self.0.used.lock().unwrap();
        if bytes > self.0.limit.saturating_sub(*used) {
            return Err("representative still budget exhausted".into());
        }
        *used += bytes;
        Ok(Reservation {
            budget: self.clone(),
            bytes,
        })
    }
}
impl Drop for Reservation {
    fn drop(&mut self) {
        *self.budget.0.used.lock().unwrap() -= self.bytes;
    }
}

#[derive(Clone)]
pub(super) struct Still(Arc<Image>);
struct Image {
    pixels: RgbImage,
    reason: String,
    _reservation: Reservation,
}
impl Still {
    fn from_pixels(
        pixels: &wallpicker_core::media::RgbaImage,
        reason: String,
        budget: &Budget,
    ) -> Result<Self, String> {
        let reservation = budget.reserve(u64::from(pixels.width()) * u64::from(pixels.height()) * 3)?;
        Ok(Self(Arc::new(Image {
            pixels: compose::flatten(pixels),
            reason,
            _reservation: reservation,
        })))
    }
    pub(super) fn reason(&self) -> &str {
        &self.0.reason
    }
    pub(super) fn render(&self, picture: &Picture, size: (u32, u32)) -> Result<(Vec<u8>, Hold), String> {
        if size.0 == 0 || size.1 == 0 || size.0 > 16384 || size.1 > 16384 {
            return Err("representative output exceeds image limits".into());
        }
        let output_bytes = u64::from(size.0) * u64::from(size.1) * 4;
        if output_bytes > 64 * 1024 * 1024 {
            return Err("representative output exceeds image limits".into());
        }
        // Logical pixel workspace allowance for source copies, fitting and XRGB.
        // This is not a bound on allocator overhead or the resize library's RSS.
        let bytes = self.0.pixels.as_raw().len() as u64 * 2 + output_bytes * 4;
        let hold = Hold {
            _reservation: self.0._reservation.budget.reserve(bytes)?,
        };
        let pixels = compose::geometry(&self.0.pixels, picture.picture.geometry, size).map_err(|e| e.to_string())?;
        Ok((pixels, hold))
    }
}
// Keep fitting/output reservation until the result has been submitted or dropped.
pub(super) struct Hold {
    _reservation: Reservation,
}

pub(super) struct Offer {
    pub reason: String,
    pub budget: Budget,
}
pub(super) struct Prepared {
    batch: Batch,
    groups: Vec<Option<usize>>,
    stills: Vec<Still>,
}
impl Offer {
    pub(super) fn prepare(self, handoff: Handoff) -> Result<Prepared, String> {
        let mut stills = Vec::new();
        for session in handoff.sessions {
            let pixels = &session.first().frame().pixels;
            stills.push(Still::from_pixels(pixels, self.reason.clone(), &self.budget)?);
            // Session drop cancels and joins the decoder on this caller thread.
        }
        for output in &handoff.batch.outputs {
            if let Content::Animated { identity } = &output.picture.content
                && FileIdentity::read(&output.picture.source).map_err(|e| e.to_string())? != *identity
            {
                return Err("media changed before representative still handoff".into());
            }
        }
        Ok(Prepared {
            batch: handoff.batch,
            groups: handoff.groups,
            stills,
        })
    }
}

impl State {
    pub(super) fn accept_fallback(&mut self, prepared: Prepared) -> Vec<PendingShow> {
        let mut answers = Vec::new();
        for (index, output) in prepared.batch.outputs.iter().enumerate() {
            let (reply, answer) = mpsc::channel();
            answers.push((output.monitor.clone(), answer));
            let picture = prepared.batch.selection(output.picture.clone());
            let (want, transition) = match prepared.groups[index] {
                Some(group) => (Want::Still(picture, prepared.stills[group].clone()), None),
                None => (
                    Want::File(picture),
                    Transition::from_setting(
                        &prepared.batch.transition.kind,
                        prepared.batch.transition.duration_ms.min(10_000),
                    ),
                ),
            };
            self.show_want(&output.monitor, want, transition, reply);
        }
        answers
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wallpicker_core::{
        media::RgbaImage,
        render::{Geometry, View},
        settings::{FitMode, Flip},
    };

    #[test]
    fn representative_alpha_geometry_and_shared_reservations() {
        let pixels = RgbaImage::from_raw(
            2,
            2,
            vec![255, 0, 0, 0, 0, 255, 0, 255, 0, 0, 255, 128, 255, 255, 255, 255],
        )
        .unwrap();
        let budget = Budget::new(4096);
        let still = Still::from_pixels(&pixels, "test EGL failure".into(), &budget).unwrap();
        let other = still.clone();
        assert_eq!(budget.used(), 12);
        let mut picture: Picture = (std::path::PathBuf::from("/not-opened.gif"), None).into();
        let (rendered, hold) = still.render(&picture, (2, 2)).unwrap();
        assert_eq!(
            rendered,
            vec![0, 0, 0, 255, 0, 255, 0, 255, 128, 0, 0, 255, 255, 255, 255, 255]
        );
        assert!(budget.used() > 12);
        drop(hold);
        picture.picture.geometry = Geometry {
            view: View::Region {
                x: 1,
                y: 0,
                width: 1,
                height: 1,
            },
            fit: Some(FitMode::Tile),
            flip: Flip { h: true, v: true },
        };
        let (rendered, hold) = still.render(&picture, (4, 3)).unwrap();
        assert_eq!(rendered, [0, 255, 0, 255].repeat(12));
        drop(hold);
        assert!(still.render(&picture, (16384, 16384)).is_err());
        assert!(still.render(&picture, (u32::MAX, u32::MAX)).is_err());
        drop(still);
        assert_eq!(budget.used(), 12);
        drop(other);
        assert_eq!(budget.used(), 0);
    }

    #[test]
    fn representative_admission_and_fitting_fail_before_allocation() {
        let pixels = RgbaImage::new(2, 2);
        assert!(Still::from_pixels(&pixels, "test".into(), &Budget::new(11)).is_err());
        let budget = Budget::new(12);
        let still = Still::from_pixels(&pixels, "test".into(), &budget).unwrap();
        let picture: Picture = (std::path::PathBuf::from("/not-opened.gif"), None).into();
        assert!(still.render(&picture, (2, 2)).is_err());
        assert_eq!(budget.used(), 12);
        drop(still);
        assert_eq!(budget.used(), 0);
    }
}
