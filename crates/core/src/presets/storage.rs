//! Atomic persistence. A separate file lock serializes edits across CLI/picker processes.

use super::{Collection, Preset, capture, validation};
use crate::{
    Error, Result,
    store::{Store, atomic_write},
};
use std::{fs, path::PathBuf};

impl Store {
    pub fn presets_path(&self) -> PathBuf {
        self.dirs().data.join("presets.json")
    }

    /// A missing collection is empty; malformed or unsupported data is an error.
    pub fn load_presets(&self) -> Result<Vec<Preset>> {
        Ok(self.preset_collection()?.items)
    }

    /// Save a new snapshot. Does not change current wallpapers or settings.
    pub fn save_current_preset(&self, name: &str) -> Result<Preset> {
        let name = validation::name(name)?;
        self.edit_presets(|saved| {
            validation::unique_name(&saved.items, &name, None)?;
            let id = format!("p{}", saved.next_id);
            let preset = self.capture_preset(id, name)?;
            saved.next_id = saved
                .next_id
                .checked_add(1)
                .ok_or_else(|| Error::Invalid("preset IDs exhausted".into()))?;
            saved.items.push(preset.clone());
            Ok(preset)
        })
    }

    /// Explicitly replace a snapshot, retaining its name and stable ID.
    pub fn update_current_preset(&self, id: &str) -> Result<Preset> {
        self.edit_presets(|saved| {
            let index = find(saved, id)?;
            let preset = self.capture_preset(id.into(), saved.items[index].name.clone())?;
            saved.items[index] = preset.clone();
            Ok(preset)
        })
    }

    pub fn rename_preset(&self, id: &str, name: &str) -> Result<Preset> {
        let name = validation::name(name)?;
        self.edit_presets(|saved| {
            let index = find(saved, id)?;
            validation::unique_name(&saved.items, &name, Some(id))?;
            saved.items[index].name = name;
            Ok(saved.items[index].clone())
        })
    }

    pub fn delete_preset(&self, id: &str) -> Result<()> {
        self.edit_presets(|saved| {
            let index = find(saved, id)?;
            saved.items.remove(index);
            Ok(())
        })
    }

    fn capture_preset(&self, id: String, name: String) -> Result<Preset> {
        let state = self.load_state()?;
        if state.current.values().any(|p| p.starts_with(&self.dirs().cache)) {
            return Err(Error::Invalid(
                "presets require original wallpapers, not generated cache files".into(),
            ));
        }
        capture::capture(id, name, &state, &self.load_settings()?)
    }

    pub fn preset_collection(&self) -> Result<Collection> {
        let saved = match fs::read(self.presets_path()) {
            Ok(bytes) => serde_json::from_slice(&bytes)?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Collection::default(),
            Err(e) => return Err(e.into()),
        };
        validation::collection(&saved)?;
        Ok(saved)
    }

    fn edit_presets<T>(&self, change: impl FnOnce(&mut Collection) -> Result<T>) -> Result<T> {
        fs::create_dir_all(&self.dirs().data)?;
        let lock = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(self.dirs().data.join("presets.lock"))?;
        lock.lock()?;
        let mut saved = self.preset_collection()?;
        let result = change(&mut saved)?;
        validation::collection(&saved)?;
        let bytes = serde_json::to_vec_pretty(&saved)?;
        self.snapshot_catalog()?;
        atomic_write(&self.presets_path(), &bytes, false)?;
        // The stable lock file is kept; dropping the handle releases its lock.
        Ok(result)
    }
}

fn find(saved: &Collection, id: &str) -> Result<usize> {
    saved
        .items
        .iter()
        .position(|p| p.id == id)
        .ok_or_else(|| Error::Invalid(format!("no preset with ID {id:?}")))
}
