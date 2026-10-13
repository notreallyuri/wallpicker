//! Validate every selected source/geometry before admitting any decoder worker.
use std::path::PathBuf;
use wallpicker_core::{
    media::{FileIdentity, Kind, Limits, Source},
    render::{Batch, Content, Preparation},
};

pub(super) struct Inspection {
    pub sources: Vec<Source>,
    pub groups: Vec<Option<usize>>,
    pub files: Vec<(PathBuf, FileIdentity)>,
}

impl Inspection {
    pub(super) fn new(batch: &Batch, limits: Limits) -> Result<Self, String> {
        batch.validate().map_err(|e| e.to_string())?;
        let mut inspected = Self {
            sources: vec![],
            groups: vec![],
            files: vec![],
        };
        let mut static_sources = Preparation::default();
        for output in &batch.outputs {
            let picture = &output.picture;
            let identity = inspected.track(&picture.source)?;
            let (dimensions, group) = match &picture.content {
                Content::Animated { identity: expected } => {
                    if identity != *expected {
                        return Err("media changed since request preparation".into());
                    }
                    let existing = inspected.sources.iter().position(|s| s.identity() == expected);
                    let group = match existing {
                        Some(group) => group,
                        None => {
                            let source = Source::open(&picture.source, limits).map_err(|e| e.to_string())?;
                            if source.identity() != expected {
                                return Err("media changed during batch inspection".into());
                            }
                            if source.info().kind != Kind::Animated {
                                return Err("animated request refers to a static source".into());
                            }
                            inspected.sources.push(source);
                            inspected.sources.len() - 1
                        }
                    };
                    (inspected.sources[group].info().dimensions(), Some(group))
                }
                Content::Static { path, .. } => {
                    inspected.track(path)?;
                    let dimensions = static_sources.inspect(&picture.source).map_err(|e| e.to_string())?;
                    for file in [picture.source.as_path(), path.as_path()] {
                        if static_sources.is_animated(file).map_err(|e| e.to_string())? {
                            return Err("animated media requires an animated render request".into());
                        }
                    }
                    (dimensions, None)
                }
            };
            picture
                .geometry
                .validate_for_source(dimensions)
                .map_err(|e| e.to_string())?;
            inspected.groups.push(group);
        }
        inspected.revalidate()?;
        Ok(inspected)
    }

    fn track(&mut self, path: &std::path::Path) -> Result<FileIdentity, String> {
        if let Some((_, identity)) = self.files.iter().find(|(p, _)| p == path) {
            return Ok(identity.clone());
        }
        let identity = FileIdentity::read(path).map_err(|e| e.to_string())?;
        self.files.push((path.to_owned(), identity.clone()));
        Ok(identity)
    }

    pub(super) fn revalidate(&self) -> Result<(), String> {
        for (path, expected) in &self.files {
            if FileIdentity::read(path).map_err(|e| e.to_string())? != *expected {
                return Err(format!("media changed before render handoff: {}", path.display()));
            }
        }
        Ok(())
    }
}
