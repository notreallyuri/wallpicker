use super::*;
use crate::{
    Error, Result,
    presets::{Collection, validation},
};

impl Backup {
    pub fn validate(&self) -> Result<()> {
        if !(1..=FORMAT).contains(&self.wallpicker_backup) {
            return Err(Error::Invalid("unsupported backup version".into()));
        }
        if (self.wallpicker_backup == 2) != self.presets.is_some() {
            return Err(Error::Invalid(
                "backup preset section does not match its version".into(),
            ));
        }
        if let Some(saved) = &self.presets {
            validation::collection(saved)?;
        }
        Ok(())
    }

    pub fn include_presets(&mut self, saved: Collection, size: &dyn Fn(&Path) -> Option<u64>) -> Result<()> {
        validation::collection(&saved)?;
        let paths: std::collections::BTreeSet<_> = saved
            .items
            .iter()
            .flat_map(|p| p.monitors.values().map(|m| m.wallpaper.clone()))
            .collect();
        self.preset_images = paths
            .into_iter()
            .map(|path| Item {
                relative: self
                    .folders
                    .iter()
                    .find_map(|f| path.strip_prefix(f).ok().map(Path::to_path_buf)),
                size: size(&path),
                path,
                entry: Entry::default(),
            })
            .collect();
        self.presets = Some(saved);
        self.wallpicker_backup = FORMAT;
        Ok(())
    }

    pub(crate) fn import_presets(
        &self,
        current: &Collection,
        folders: &[PathBuf],
        library: &[(PathBuf, u64)],
        replace: bool,
        report: &mut ImportReport,
    ) -> Result<Option<Collection>> {
        self.validate()?;
        let Some(incoming) = &self.presets else {
            return Ok(None);
        };
        let mut incoming = incoming.clone();
        let present: HashSet<&Path> = library.iter().map(|(p, _)| p.as_path()).collect();
        let mut by_name = HashMap::new();
        for image in library {
            if let Some(name) = image.0.file_name() {
                by_name.entry(name).or_insert_with(Vec::new).push(image);
            }
        }
        let paths: std::collections::BTreeSet<_> = incoming
            .items
            .iter()
            .flat_map(|p| p.monitors.values().map(|m| m.wallpaper.clone()))
            .collect();
        let mut relocated = HashMap::new();
        for path in paths {
            if present.contains(path.as_path()) {
                continue;
            }
            let fallback = Item {
                path: path.clone(),
                relative: None,
                size: None,
                entry: Entry::default(),
            };
            let source = self.preset_images.iter().find(|i| i.path == path).unwrap_or(&fallback);
            if let Some(found) = locate(source, folders, &present, &by_name) {
                report.preset_images_moved += 1;
                relocated.insert(path, found);
            } else {
                report.preset_images_missing.push(path.display().to_string());
            }
        }
        for preset in &mut incoming.items {
            for monitor in preset.monitors.values_mut() {
                if let Some(path) = relocated.get(&monitor.wallpaper) {
                    monitor.wallpaper = path.clone();
                }
            }
            if let Some(span) = &mut preset.span {
                if let Some(path) = relocated.get(&span.wallpaper) {
                    span.wallpaper = path.clone();
                }
            }
        }
        report.presets = Some(incoming.items.len());
        let mut result = if replace {
            Collection::default()
        } else {
            current.clone()
        };
        result.next_id = current.next_id.max(incoming.next_id);
        for preset in incoming.items {
            if let Some(existing) = result.items.iter().find(|p| p.id == preset.id) {
                if existing == &preset {
                    continue;
                }
                return Err(Error::Invalid(format!(
                    "preset ID {} conflicts; use Replace to restore this backup",
                    preset.id
                )));
            }
            validation::unique_name(&result.items, &preset.name, None)?;
            result.items.push(preset);
        }
        validation::collection(&result)?;
        Ok(Some(result))
    }
}
