use super::{Batch, Content, Geometry, Report, VERSION, View};
use crate::{Error, Result};
use std::collections::BTreeSet;

pub(super) fn geometry(g: &Geometry, dimensions: Option<(u32, u32)>) -> Result<()> {
    let invalid = match g.view {
        View::Full => false,
        View::Frame { zoom, x, y } => !(100..=400).contains(&zoom) || x > 1000 || y > 1000,
        View::Region { x, y, width, height } => {
            width == 0
                || height == 0
                || x.checked_add(width).is_none()
                || y.checked_add(height).is_none()
                || dimensions.is_some_and(|(w, h)| {
                    u64::from(x) + u64::from(width) > u64::from(w) || u64::from(y) + u64::from(height) > u64::from(h)
                })
        }
    };
    if invalid {
        return Err(Error::Invalid("invalid source geometry".into()));
    }
    Ok(())
}

impl Batch {
    pub fn validate(&self) -> Result<()> {
        if self.version != VERSION {
            return Err(Error::Invalid(
                "unsupported render protocol; restart wallpicker daemon after updating".into(),
            ));
        }
        if self.application.is_empty() || self.application.len() > 128 || self.outputs.len() > 64 {
            return Err(Error::Invalid("invalid render batch".into()));
        }
        let mut monitors = BTreeSet::new();
        for output in &self.outputs {
            if output.monitor.is_empty() || !monitors.insert(&output.monitor) || !output.picture.source.is_absolute() {
                return Err(Error::Invalid(
                    "duplicate/invalid output or relative source in render batch".into(),
                ));
            }
            geometry(&output.picture.geometry, None)?;
            if let Content::Static {
                path, prepared_size, ..
            } = &output.picture.content
                && (!path.is_absolute()
                    || prepared_size.is_some_and(|(w, h)| w == 0 || h == 0 || w > 16384 || h > 16384))
            {
                return Err(Error::Invalid("invalid prepared static image".into()));
            }
        }
        Ok(())
    }
}

impl Report {
    /// Do not commit state from an incomplete, duplicated or foreign IPC reply.
    pub fn validate_for(&self, batch: &Batch) -> Result<()> {
        let expected: BTreeSet<_> = batch.outputs.iter().map(|o| o.monitor.as_str()).collect();
        let returned: Vec<_> = self
            .applied
            .iter()
            .map(String::as_str)
            .chain(self.failed.iter().map(|(m, _)| m.as_str()))
            .collect();
        if returned.len() != expected.len() || returned.into_iter().collect::<BTreeSet<_>>() != expected {
            return Err(Error::Invalid(
                "invalid renderer result: missing, duplicate or unexpected outputs".into(),
            ));
        }
        Ok(())
    }
}
