//! Backups of the library (favorites and tags): one file to export and
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
pub const FORMAT: u32 = 1;

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
}

/// A backup of `catalog`. `size` gives a file's size (None if it's gone).
pub fn export(catalog: &Catalog, folders: &[PathBuf], created: String, size: &dyn Fn(&Path) -> Option<u64>) -> Backup {
    let items = catalog
        .items
        .iter()
        .map(|(path, entry)| Item {
            path: path.clone(),
            relative: folders.iter().find_map(|f| path.strip_prefix(f).ok().map(Path::to_path_buf)),
            size: size(path),
            entry: entry.clone(),
        })
        .collect();
    Backup { wallpicker_backup: FORMAT, created, folders: folders.to_vec(), items }
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
            None => report
                .missing
                .push(item.path.file_name().map_or_else(|| item.path.display().to_string(), |n| n.to_string_lossy().into_owned())),
        }
    }
    // An entry with nothing left in it isn't kept (like the catalog's own edits).
    catalog.items.retain(|_, e| e.favorite || !e.tags.is_empty());
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

/// Favorite if either is, every tag of both (theirs after).
fn merge(into: &mut Entry, from: &Entry) {
    into.favorite |= from.favorite;
    for t in &from.tags {
        if !into.has_tag(t) {
            into.tags.push(t.clone());
        }
    }
}

/// The date of `secs` since the Unix epoch (UTC): (year, month, day).
pub fn civil_date(secs: u64) -> (i64, u32, u32) {
    // Howard Hinnant's days-to-civil.
    let z = (secs / 86_400) as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let year = yoe + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

/// "2026-10-06" and "2026-10-06 14:03" for `secs` (UTC).
pub fn date_and_time(secs: u64) -> (String, String) {
    let (y, m, d) = civil_date(secs);
    let date = format!("{y:04}-{m:02}-{d:02}");
    let time = format!("{date} {:02}:{:02}", (secs / 3600) % 24, (secs / 60) % 60);
    (date, time)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(favorite: bool, tags: &[&str]) -> Entry {
        Entry { favorite, tags: tags.iter().map(|t| t.to_string()).collect() }
    }

    #[test]
    fn dates() {
        assert_eq!(civil_date(0), (1970, 1, 1));
        // 2026-10-06 14:03:00 UTC
        assert_eq!(date_and_time(1_791_295_380), ("2026-10-06".into(), "2026-10-06 14:03".into()));
        assert_eq!(civil_date(951_782_400), (2000, 2, 29));
    }

    #[test]
    fn export_then_import_on_another_machine() {
        let mut old = Catalog::default();
        old.items.insert("/home/a/wpp/x/one.jpg".into(), entry(true, &["Mitsu"]));
        old.items.insert("/home/a/wpp/two.png".into(), entry(false, &["dark"]));
        old.items.insert("/home/a/wpp/three.jpg".into(), entry(true, &[]));
        old.items.insert("/home/a/wpp/gone.jpg".into(), entry(true, &[]));
        let sizes = |p: &Path| Some(if p.ends_with("three.jpg") { 300 } else { 100 });
        let backup = export(&old, &["/home/a/wpp".into()], "2026-10-06 14:03".into(), &sizes);
        assert_eq!(backup.items[0].relative.as_deref(), Some(Path::new("gone.jpg")));

        // New home, a different root; three.jpg moved to another subfolder.
        let folders = vec![PathBuf::from("/home/b/Pictures")];
        let library = vec![
            ("/home/b/Pictures/x/one.jpg".into(), 100),
            ("/home/b/Pictures/two.png".into(), 100),
            ("/home/b/Pictures/elsewhere/three.jpg".into(), 300),
            ("/home/b/Pictures/other/three.jpg".into(), 999),
        ];
        let mut now = Catalog::default();
        now.items.insert("/home/b/Pictures/two.png".into(), entry(false, &["Dark", "night"]));
        let report = import(&mut now, &backup, &folders, &library, false);
        assert_eq!(report, ImportReport { same_place: 0, moved: 3, missing: vec!["gone.jpg".into()] });
        assert_eq!(now.items[Path::new("/home/b/Pictures/x/one.jpg")], entry(true, &["Mitsu"]));
        // Merged: tags once (case-insensitive).
        assert_eq!(now.items[Path::new("/home/b/Pictures/two.png")], entry(false, &["Dark", "night"]));
        // By name, told apart by size.
        assert!(now.items[Path::new("/home/b/Pictures/elsewhere/three.jpg")].favorite);
    }

    #[test]
    fn replace_keeps_only_the_backup() {
        let mut old = Catalog::default();
        old.items.insert("/w/a.jpg".into(), entry(true, &[]));
        let backup = export(&old, &[], String::new(), &|_| None);
        let mut now = Catalog::default();
        now.items.insert("/w/b.jpg".into(), entry(true, &[]));
        let library = vec![("/w/a.jpg".into(), 1), ("/w/b.jpg".into(), 1)];
        let report = import(&mut now, &backup, &[], &library, true);
        assert_eq!(report.same_place, 1);
        assert_eq!(now.items.keys().collect::<Vec<_>>(), vec![Path::new("/w/a.jpg")]);
    }
}
