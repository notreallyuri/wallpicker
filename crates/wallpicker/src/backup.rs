//! Library backups, shared by the app and `wallpicker backup`: export to one
//! file, import (merged, or replacing), and the daily snapshots the store
//! keeps on its own.

use std::path::{Path, PathBuf};

use serde::Serialize;
use wallpicker_core::backup::{self, Backup, ImportReport};
use wallpicker_core::library;
use wallpicker_core::settings::expand_tilde;
use wallpicker_core::store::Store;

fn home() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// Where an export goes by default: `~/wallpicker-backup-2026-10-06.json`.
pub fn default_export_path() -> PathBuf {
    let (date, _) = backup::date_and_time(now_secs());
    home().unwrap_or_default().join(format!("wallpicker-backup-{date}.json"))
}

/// `~` expanded; relative paths are taken from the home folder.
fn resolve(path: &str) -> PathBuf {
    let p = expand_tilde(Path::new(path.trim()), home().as_deref());
    if p.is_absolute() { p } else { home().unwrap_or_default().join(p) }
}

#[derive(Debug, Serialize)]
pub struct Exported {
    pub path: PathBuf,
    pub items: usize,
}

/// Write the library to `path` (None: [`default_export_path`]).
pub fn export(store: &Store, path: Option<&str>) -> Result<Exported, String> {
    let settings = store.load_settings().map_err(|e| e.to_string())?;
    let catalog = store.load_catalog().map_err(|e| e.to_string())?;
    let (_, created) = backup::date_and_time(now_secs());
    let b = backup::export(&catalog, &settings.folders, created, &|p| std::fs::metadata(p).ok().map(|m| m.len()));
    let target = path.filter(|p| !p.trim().is_empty()).map_or_else(default_export_path, resolve);
    if target.is_dir() {
        return Err(format!("{} is a folder; give a file name", target.display()));
    }
    let text = serde_json::to_string_pretty(&b).map_err(|e| e.to_string())?;
    std::fs::write(&target, text).map_err(|e| format!("Could not write {}: {e}", target.display()))?;
    Ok(Exported { path: target, items: b.items.len() })
}

/// Read `path` and merge it into the library (or, with `replace`, make the
/// library what's found of it). Today's snapshot is taken first, as for any
/// change.
pub fn import(store: &Store, path: &str, replace: bool) -> Result<ImportReport, String> {
    let file = resolve(path);
    let text = std::fs::read_to_string(&file).map_err(|e| format!("Could not read {}: {e}", file.display()))?;
    let b: Backup = serde_json::from_str(&text).map_err(|_| format!("{} isn't a wallpicker backup", file.display()))?;
    if b.wallpicker_backup > backup::FORMAT {
        return Err("That backup is from a newer wallpicker.".into());
    }
    let settings = store.load_settings().map_err(|e| e.to_string())?;
    let images: Vec<(PathBuf, u64)> =
        library::scan(&settings.folders, settings.recursive).into_iter().map(|i| (i.path, i.size)).collect();
    let mut report = ImportReport::default();
    store
        .update_catalog(|c| report = backup::import(c, &b, &settings.folders, &images, replace))
        .map_err(|e| e.to_string())?;
    Ok(report)
}
