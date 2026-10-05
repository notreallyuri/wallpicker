//! Favorites and tags for local wallpapers, stored in `library.json`.
//!
//! Plain data and rules only, so the UI can use them too. Entries are keyed by
//! the file's path; an entry that is neither a favorite nor tagged is dropped.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::settings::FitMode;

/// Longest tag accepted, in characters.
pub const MAX_TAG_LEN: usize = 64;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Entry {
    pub favorite: bool,
    /// In the order they were added.
    pub tags: Vec<String>,
    /// How to fit it on any monitor, over the monitor's and the general fit.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fit: Option<FitMode>,
}

impl Entry {
    fn is_empty(&self) -> bool {
        !self.favorite && self.tags.is_empty() && self.fit.is_none()
    }

    pub fn has_tag(&self, tag: &str) -> bool {
        self.tags.iter().any(|t| same_tag(t, tag))
    }
}

/// How a filter over several tags combines them.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TagMatch {
    /// The wallpaper has every selected tag.
    #[default]
    All,
    /// The wallpaper has at least one of them.
    Any,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Catalog {
    pub items: BTreeMap<PathBuf, Entry>,
}

/// Tags compare case-insensitively: "Dark" and "dark" are one tag.
fn same_tag(a: &str, b: &str) -> bool {
    a.to_lowercase() == b.to_lowercase()
}

/// Trim, collapse inner whitespace, and refuse empty, overlong or control
/// characters. Case is kept as typed.
pub fn normalize_tag(raw: &str) -> Option<String> {
    let tag = raw.split_whitespace().collect::<Vec<_>>().join(" ");
    let ok = !tag.is_empty()
        && tag.chars().count() <= MAX_TAG_LEN
        && !tag.chars().any(char::is_control);
    ok.then_some(tag)
}

impl Catalog {
    pub fn entry(&self, path: &Path) -> Option<&Entry> {
        self.items.get(path)
    }

    pub fn is_favorite(&self, path: &Path) -> bool {
        self.entry(path).is_some_and(|e| e.favorite)
    }

    pub fn tags(&self, path: &Path) -> &[String] {
        self.entry(path).map_or(&[], |e| e.tags.as_slice())
    }

    /// The wallpaper's own fit, if it has one.
    pub fn fit(&self, path: &Path) -> Option<FitMode> {
        self.entry(path).and_then(|e| e.fit)
    }

    /// Give `path` its own fit, or (None) go back to the monitor's.
    pub fn set_fit(&mut self, path: &Path, fit: Option<FitMode>) {
        self.items.entry(path.to_path_buf()).or_default().fit = fit;
        self.drop_if_empty(path);
    }

    pub fn set_favorite(&mut self, path: &Path, favorite: bool) {
        self.items.entry(path.to_path_buf()).or_default().favorite = favorite;
        self.drop_if_empty(path);
    }

    /// Add `raw` to `path`. If the tag already exists on another wallpaper
    /// with different capitalisation, that spelling is reused. Returns the
    /// tag as stored, or `None` if it isn't a valid tag.
    pub fn add_tag(&mut self, path: &Path, raw: &str) -> Option<String> {
        let tag = normalize_tag(raw)?;
        let tag = self.existing_spelling(&tag).unwrap_or(tag);
        let entry = self.items.entry(path.to_path_buf()).or_default();
        if !entry.has_tag(&tag) {
            entry.tags.push(tag.clone());
        }
        Some(tag)
    }

    /// Drop everything about `path` (it was deleted).
    pub fn forget(&mut self, path: &Path) {
        self.items.remove(path);
    }

    pub fn remove_tag(&mut self, path: &Path, tag: &str) {
        if let Some(entry) = self.items.get_mut(path) {
            entry.tags.retain(|t| !same_tag(t, tag));
        }
        self.drop_if_empty(path);
    }

    /// Does `path` pass a tag filter? No selected tags means everything does.
    pub fn matches(&self, path: &Path, selected: &[String], mode: TagMatch) -> bool {
        if selected.is_empty() {
            return true;
        }
        let Some(entry) = self.entry(path) else {
            return false;
        };
        match mode {
            TagMatch::All => selected.iter().all(|t| entry.has_tag(t)),
            TagMatch::Any => selected.iter().any(|t| entry.has_tag(t)),
        }
    }

    /// Every tag used by `paths`, with how many of them carry it, sorted by
    /// name. Only `paths` count, so tags of deleted files don't show up.
    pub fn tag_counts<'a>(&self, paths: impl IntoIterator<Item = &'a Path>) -> Vec<(String, usize)> {
        let mut counts: Vec<(String, usize)> = Vec::new();
        for path in paths {
            for tag in self.tags(path) {
                match counts.iter_mut().find(|(t, _)| same_tag(t, tag)) {
                    Some((_, n)) => *n += 1,
                    None => counts.push((tag.clone(), 1)),
                }
            }
        }
        counts.sort_by_key(|(t, _)| t.to_lowercase());
        counts
    }

    fn existing_spelling(&self, tag: &str) -> Option<String> {
        self.items
            .values()
            .flat_map(|e| e.tags.iter())
            .find(|t| same_tag(t, tag))
            .cloned()
    }

    fn drop_if_empty(&mut self, path: &Path) {
        if self.items.get(path).is_some_and(Entry::is_empty) {
            self.items.remove(path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(s: &str) -> &Path {
        Path::new(s)
    }

    #[test]
    fn tags_are_cleaned_up_or_refused() {
        assert_eq!(normalize_tag("  dark   mode "), Some("dark mode".into()));
        assert_eq!(normalize_tag("Mitsu"), Some("Mitsu".into()));
        assert_eq!(normalize_tag("   "), None);
        assert_eq!(normalize_tag("a\u{7}b"), None);
        assert_eq!(normalize_tag(&"x".repeat(MAX_TAG_LEN)), Some("x".repeat(MAX_TAG_LEN)));
        assert_eq!(normalize_tag(&"x".repeat(MAX_TAG_LEN + 1)), None);
    }

    #[test]
    fn tags_ignore_case_and_reuse_the_first_spelling() {
        let mut c = Catalog::default();
        assert_eq!(c.add_tag(p("/a.png"), "Dark").as_deref(), Some("Dark"));
        assert_eq!(c.add_tag(p("/b.png"), "dark").as_deref(), Some("Dark"));
        // Same tag twice on one file is one tag.
        c.add_tag(p("/a.png"), "DARK");
        assert_eq!(c.tags(p("/a.png")), ["Dark"]);
        assert_eq!(c.tags(p("/b.png")), ["Dark"]);
        assert_eq!(c.add_tag(p("/a.png"), "  "), None);
    }

    #[test]
    fn empty_entries_are_dropped() {
        let mut c = Catalog::default();
        c.set_favorite(p("/a.png"), true);
        c.add_tag(p("/a.png"), "x");
        c.set_favorite(p("/a.png"), false);
        assert!(c.entry(p("/a.png")).is_some());
        c.remove_tag(p("/a.png"), "X");
        assert!(c.items.is_empty());

        // Unfavoriting something never favorited leaves nothing behind.
        c.set_favorite(p("/b.png"), false);
        assert!(c.items.is_empty());
    }

    #[test]
    fn all_and_any_filters() {
        let mut c = Catalog::default();
        c.add_tag(p("/both.png"), "dark");
        c.add_tag(p("/both.png"), "anime");
        c.add_tag(p("/dark.png"), "dark");
        c.set_favorite(p("/plain.png"), true);
        let sel = vec!["Dark".to_string(), "anime".to_string()];

        assert!(c.matches(p("/both.png"), &sel, TagMatch::All));
        assert!(!c.matches(p("/dark.png"), &sel, TagMatch::All));
        assert!(c.matches(p("/dark.png"), &sel, TagMatch::Any));
        assert!(!c.matches(p("/plain.png"), &sel, TagMatch::Any));
        assert!(!c.matches(p("/unknown.png"), &sel, TagMatch::Any));
        // Nothing selected: no filtering.
        assert!(c.matches(p("/unknown.png"), &[], TagMatch::All));
    }

    #[test]
    fn counts_only_cover_the_given_files() {
        let mut c = Catalog::default();
        c.add_tag(p("/a.png"), "dark");
        c.add_tag(p("/b.png"), "Dark");
        c.add_tag(p("/b.png"), "anime");
        c.add_tag(p("/gone.png"), "old");
        let counts = c.tag_counts([p("/a.png"), p("/b.png")]);
        assert_eq!(counts, vec![("anime".into(), 1), ("dark".into(), 2)]);
    }

    #[test]
    fn round_trips_through_json() {
        let mut c = Catalog::default();
        c.set_favorite(p("/a.png"), true);
        c.add_tag(p("/a.png"), "Mitsu");
        let json = serde_json::to_string(&c).unwrap();
        assert_eq!(serde_json::from_str::<Catalog>(&json).unwrap(), c);
        assert_eq!(serde_json::from_str::<Catalog>("{}").unwrap(), Catalog::default());
    }
}
