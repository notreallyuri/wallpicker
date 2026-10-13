use super::*;
use crate::{
    Result,
    presets::{Collection, validation},
    store::{Store, atomic_write},
};
use std::fs;

impl Store {
    pub fn export_backup(&self, created: String) -> Result<Backup> {
        let settings = self.load_settings()?;
        let size = |p: &Path| fs::metadata(p).ok().filter(|m| m.is_file()).map(|m| m.len());
        let mut backup = export(&self.load_catalog()?, &settings.folders, created, &size);
        backup.include_presets(self.preset_collection()?, &size)?;
        Ok(backup)
    }

    pub fn import_backup(
        &self,
        backup: &Backup,
        folders: &[PathBuf],
        library: &[(PathBuf, u64)],
        replace: bool,
    ) -> Result<ImportReport> {
        backup.validate()?;
        let _lock = self.backup_lock()?;
        let mut catalog = self.load_catalog()?;
        let current = self.preset_collection()?;
        let mut report = import(&mut catalog, backup, folders, library, replace);
        let presets = backup.import_presets(&current, folders, library, replace, &mut report)?;
        self.snapshot_catalog()?;
        self.write_backup_data(&catalog, presets.as_ref())?;
        Ok(report)
    }

    pub(crate) fn backup_lock(&self) -> Result<fs::File> {
        fs::create_dir_all(&self.dirs().data)?;
        let lock = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(self.dirs().data.join("presets.lock"))?;
        lock.lock()?;
        Ok(lock)
    }

    pub(crate) fn write_backup_data(&self, catalog: &Catalog, presets: Option<&Collection>) -> Result<()> {
        let catalog_path = self.dirs().data.join("library.json");
        let catalog_bytes = serde_json::to_vec_pretty(catalog)?;
        let preset_bytes = presets
            .map(|p| {
                validation::collection(p)?;
                Ok::<_, crate::Error>(serde_json::to_vec_pretty(p)?)
            })
            .transpose()?;
        // Serialize and validate both first. Roll back the library if the second write fails.
        let old = match fs::read(&catalog_path) {
            Ok(bytes) => Some(bytes),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
            Err(e) => return Err(e.into()),
        };
        atomic_write(&catalog_path, &catalog_bytes, false)?;
        if let Some(bytes) = preset_bytes {
            if let Err(error) = atomic_write(&self.presets_path(), &bytes, false) {
                if let Some(old) = old {
                    atomic_write(&catalog_path, &old, false)?;
                } else {
                    fs::remove_file(&catalog_path)?;
                }
                return Err(error);
            }
        }
        Ok(())
    }
}
