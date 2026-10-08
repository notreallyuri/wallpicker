//! Favorites, tags and metadata (where a wallpaper came from, its artists and
//! uploader) for local wallpapers, stored in `library.json`.
//!
//! Plain data and rules only, so the UI can use them too. Entries are keyed by
//! the file's path; an entry with nothing in it is dropped.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};


/// Longest tag accepted, in characters.
pub const MAX_TAG_LEN: usize = 64;

/// `library.json`'s format: 1 had favorites and tags, 2 adds the metadata.
/// Every save writes this one.
pub const VERSION: u32 = 2;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Entry {
    pub favorite: bool,
    /// In the order they were added.
    pub tags: Vec<String>,
    #[serde(flatten)]
    pub meta: Metadata,
}

impl Entry {
    pub fn is_empty(&self) -> bool {
        !self.favorite && self.tags.is_empty() && self.meta.is_empty()
    }

    pub fn has_tag(&self, tag: &str) -> bool {
        self.tags.iter().any(|t| same_tag(t, tag))
    }
}

/// Where a wallpaper came from and who made it, beside its tags: what a
/// download brings along (or what's added by hand, the artists).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Metadata {
    /// The provider and the id there, for a downloaded one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub origin: Option<Origin>,
    /// The provider's source link (often the artist's page).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uploader: Option<Person>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub artists: Vec<Person>,
}

impl Metadata {
    pub fn is_empty(&self) -> bool {
        self.origin.is_none() && self.source.is_none() && self.uploader.is_none() && self.artists.is_empty()
    }

    pub fn has_artist(&self, name: &str) -> bool {
        self.artists.iter().any(|a| same_tag(&a.name, name))
    }

    /// Each of `other`'s fields this one doesn't have (the artists: the ones
    /// it doesn't list), names cleaned like tags.
    pub fn fill_from(&mut self, other: &Metadata) {
        if self.origin.is_none() {
            self.origin = other.origin.clone();
        }
        if self.source.is_none() {
            self.source = other.source.clone();
        }
        if self.uploader.is_none() {
            self.uploader = other.uploader.as_ref().and_then(Person::cleaned);
        }
        for a in other.artists.iter().filter_map(Person::cleaned) {
            if !self.has_artist(&a.name) {
                self.artists.push(a);
            }
        }
    }
}

/// A downloaded wallpaper's provider ("wallhaven", "konachan", "yandere")
/// and id there. A string, not `online::Provider`: a provider taken out
/// mustn't make the library unreadable.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Origin {
    pub provider: String,
    pub id: String,
}

/// An artist or an uploader.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Person {
    pub name: String,
    /// The provider's search for exactly them (Wallhaven's `id:12345` for an
    /// artist, `@name` for an uploader). None when added by hand: filtered
    /// by, not searched for.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query: Option<String>,
}

impl Person {
    pub fn named(name: &str) -> Person {
        Person { name: name.to_string(), query: None }
    }

    /// The name cleaned like a tag; None if it isn't a valid one.
    fn cleaned(&self) -> Option<Person> {
        Some(Person { name: normalize_tag(&self.name)?, query: self.query.clone() })
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Catalog {
    /// The format it was read in (a file without one is 1).
    #[serde(default = "first_version")]
    pub version: u32,
    pub items: BTreeMap<PathBuf, Entry>,
}

impl Default for Catalog {
    fn default() -> Catalog {
        Catalog { version: VERSION, items: BTreeMap::new() }
    }
}

fn first_version() -> u32 {
    1
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

    pub fn metadata(&self, path: &Path) -> Option<&Metadata> {
        self.entry(path).map(|e| &e.meta)
    }

    /// Make `path`'s artists `artists`, in that order: names cleaned like
    /// tags (invalid ones left out), each once (ignoring case).
    pub fn set_artists(&mut self, path: &Path, artists: &[Person]) {
        let mut kept = Metadata::default();
        kept.fill_from(&Metadata { artists: artists.to_vec(), ..Metadata::default() });
        self.items.entry(path.to_path_buf()).or_default().meta.artists = kept.artists;
        self.drop_if_empty(path);
    }

    /// Give `path` what of `meta` it doesn't have yet ([`Metadata::fill_from`]).
    pub fn fill_metadata(&mut self, path: &Path, meta: &Metadata) {
        self.items.entry(path.to_path_buf()).or_default().meta.fill_from(meta);
        self.drop_if_empty(path);
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
    fn version_1_files_read_and_new_ones_are_current() {
        let old: Catalog = serde_json::from_str(r#"{"items": {"/a.png": {"favorite": true, "tags": ["x"]}}}"#).unwrap();
        assert_eq!(old.version, 1);
        assert_eq!(old.tags(p("/a.png")), ["x"]);
        assert_eq!(old.metadata(p("/a.png")), Some(&Metadata::default()));
        assert_eq!(Catalog::default().version, VERSION);
    }

    #[test]
    fn metadata_round_trips_and_stays_out_of_the_way() {
        let mut c = Catalog::default();
        let meta = Metadata {
            origin: Some(Origin { provider: "wallhaven".into(), id: "mlom5m".into() }),
            source: Some("https://example.com/a".into()),
            uploader: Some(Person { name: "jrmnt".into(), query: Some("@jrmnt".into()) }),
            artists: vec![Person { name: "Florent Lebrun".into(), query: Some("id:12".into()) }],
        };
        c.fill_metadata(p("/a.jpg"), &meta);
        c.set_favorite(p("/b.jpg"), true);
        let text = serde_json::to_string(&c).unwrap();
        assert_eq!(serde_json::from_str::<Catalog>(&text).unwrap(), c);
        // Flat in the entry; nothing written for what a wallpaper doesn't have.
        let json: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(json["items"]["/a.jpg"]["origin"]["id"], "mlom5m");
        assert_eq!(json["items"]["/a.jpg"]["artists"][0]["query"], "id:12");
        assert_eq!(json["items"]["/b.jpg"], serde_json::json!({"favorite": true, "tags": []}));
        // Only metadata is still an entry.
        assert!(c.entry(p("/a.jpg")).is_some());
    }

    #[test]
    fn metadata_fills_in_without_replacing() {
        let mut c = Catalog::default();
        let first = Metadata {
            source: Some("https://one".into()),
            artists: vec![Person::named("Mitsu")],
            ..Metadata::default()
        };
        c.fill_metadata(p("/a.jpg"), &first);
        c.fill_metadata(
            p("/a.jpg"),
            &Metadata {
                source: Some("https://two".into()),
                uploader: Some(Person::named("someone")),
                artists: vec![Person::named("mitsu"), Person::named("  Other   one "), Person::named(" ")],
                ..Metadata::default()
            },
        );
        let m = c.metadata(p("/a.jpg")).unwrap();
        assert_eq!(m.source.as_deref(), Some("https://one"));
        assert_eq!(m.uploader.as_ref().unwrap().name, "someone");
        let names: Vec<&str> = m.artists.iter().map(|a| a.name.as_str()).collect();
        assert_eq!(names, ["Mitsu", "Other one"]);
        assert!(m.has_artist("OTHER ONE"));
    }

    #[test]
    fn artists_are_set_cleaned_and_once() {
        let mut c = Catalog::default();
        c.set_artists(p("/a.jpg"), &[Person::named(" A  b "), Person::named("a B"), Person::named("C")]);
        let names: Vec<&str> = c.metadata(p("/a.jpg")).unwrap().artists.iter().map(|a| a.name.as_str()).collect();
        assert_eq!(names, ["A b", "C"]);
        // None left: the entry goes.
        c.set_artists(p("/a.jpg"), &[]);
        assert!(c.entry(p("/a.jpg")).is_none());
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
        // A file without a version is from before there were versions.
        assert_eq!(serde_json::from_str::<Catalog>("{}").unwrap(), Catalog { version: 1, ..Catalog::default() });
    }
}
