//! Backups of the library and saved presets: one file to export and
//! import, and the daily snapshots kept automatically (see `Store`).
//!
//! A backup outlives the paths in it: on import each wallpaper is looked for
//! where it was, then at the same place inside a wallpaper folder (another
//! home, a renamed root), then by file name and size anywhere in the library.
//! What's found is merged into the library; what isn't is reported.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::catalog::{Catalog, Entry};

/// The file format's version, written as `wallpicker_backup`.
pub const FORMAT: u32 = 2;
mod dates;
mod presets;
mod storage;
pub use dates::{civil_date, date_and_time};
#[cfg(test)]
mod preset_tests;
#[cfg(test)]
mod tests;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Backup {
    pub wallpicker_backup: u32,
    /// When it was made, "2026-10-06 14:03" (UTC).
    #[serde(default)]
    pub created: String,
    /// The wallpaper folders then, for reference.
    #[serde(default)]
    pub folders: Vec<PathBuf>,
    pub items: Vec<Item>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub presets: Option<crate::presets::Collection>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub preset_images: Vec<Item>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Item {
    pub path: PathBuf,
    /// Its path inside the wallpaper folder it was in.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub relative: Option<PathBuf>,
    /// File size, to tell same-named files apart.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,
    #[serde(flatten)]
    pub entry: Entry,
}

/// What an import did.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportReport {
    /// Found where they were.
    pub same_place: usize,
    /// Found somewhere else (another folder root, or by name and size).
    pub moved: usize,
    /// Not in the library now: their file names.
    pub missing: Vec<String>,
    pub presets: Option<usize>,
    pub preset_images_moved: usize,
    pub preset_images_missing: Vec<String>,
}

/// A legacy library-only backup. Use `Store::export_backup` to include presets.
/// `size` gives a file's size (None if it's gone).
pub fn export(catalog: &Catalog, folders: &[PathBuf], created: String, size: &dyn Fn(&Path) -> Option<u64>) -> Backup {
    let items = catalog
        .items
        .iter()
        .map(|(path, entry)| Item {
            path: path.clone(),
            relative: folders
                .iter()
                .find_map(|f| path.strip_prefix(f).ok().map(Path::to_path_buf)),
            size: size(path),
            entry: entry.clone(),
        })
        .collect();
    Backup {
        wallpicker_backup: 1,
        created,
        folders: folders.to_vec(),
        items,
        presets: None,
        preset_images: Vec::new(),
    }
}

/// Merge `backup` into `catalog` (or, with `replace`, make the library
/// exactly what's found of it). `library` is every image in the wallpaper
/// folders now, with its size.
pub fn import(
    catalog: &mut Catalog,
    backup: &Backup,
    folders: &[PathBuf],
    library: &[(PathBuf, u64)],
    replace: bool,
) -> ImportReport {
    let present: HashSet<&Path> = library.iter().map(|(p, _)| p.as_path()).collect();
    let mut by_name: HashMap<&std::ffi::OsStr, Vec<&(PathBuf, u64)>> = HashMap::new();
    for item in library {
        if let Some(name) = item.0.file_name() {
            by_name.entry(name).or_default().push(item);
        }
    }
    if replace {
        *catalog = Catalog::default();
    }
    let mut report = ImportReport::default();
    for item in &backup.items {
        let found = if present.contains(item.path.as_path()) {
            report.same_place += 1;
            Some(item.path.clone())
        } else if let Some(p) = locate(item, folders, &present, &by_name) {
            report.moved += 1;
            Some(p)
        } else {
            None
        };
        match found {
            Some(path) => merge(catalog.items.entry(path).or_default(), &item.entry),
            None => report.missing.push(
                item.path
                    .file_name()
                    .map_or_else(|| item.path.display().to_string(), |n| n.to_string_lossy().into_owned()),
            ),
        }
    }
    // An entry with nothing left in it isn't kept (like the catalog's own edits).
    catalog.items.retain(|_, e| !e.is_empty());
    report
}

/// Where a wallpaper that isn't at its old path is now: the same place inside
/// one of the folders, else the one file with its name (and size).
fn locate(
    item: &Item,
    folders: &[PathBuf],
    present: &HashSet<&Path>,
    by_name: &HashMap<&std::ffi::OsStr, Vec<&(PathBuf, u64)>>,
) -> Option<PathBuf> {
    if let Some(rel) = &item.relative {
        for f in folders {
            let p = f.join(rel);
            if present.contains(p.as_path()) {
                return Some(p);
            }
        }
    }
    let candidates: Vec<_> = by_name
        .get(item.path.file_name()?)?
        .iter()
        .filter(|(_, size)| item.size.is_none_or(|s| s == *size))
        .collect();
    match candidates.as_slice() {
        [(p, _)] => Some(p.clone()),
        _ => None,
    }
}

/// Favorite if either is, every tag of both (theirs after), their metadata
/// where there's none.
fn merge(into: &mut Entry, from: &Entry) {
    into.favorite |= from.favorite;
    into.meta.fill_from(&from.meta);
    for t in &from.tags {
        if !into.has_tag(t) {
            into.tags.push(t.clone());
        }
    }
}
