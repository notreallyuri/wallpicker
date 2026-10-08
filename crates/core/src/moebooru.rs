//! Moebooru sites (Konachan, yande.re): one API, one module. Each site is a
//! [`Site`], which only knows its addresses; everything else is shared.
//! Konachan comes in two: konachan.net (safe-for-work only) and konachan.com
//! (all ratings), the same posts with the same ids.
//!
//! Searches are tag queries; sorting, minimum size and ratings are metatags
//! added to them (`order:score`, `width:>=1920`, `rating:s`). `post.json` has
//! no result count, so the total comes from `post.xml`, whose root element
//! carries it. Asked as API version 2, `post.json` also says what kind each
//! tag is (artist, character, ...), which is how artists are found.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::online::{Details, OnlineTag, OnlineWallpaper, Page, Person, Provider};
use crate::settings::KonachanSite;
use crate::wallhaven::is_valid_resolution;

pub const PER_PAGE: u32 = 24;

/// Largest original file used for a download. Bigger posts (yande.re has
/// 100 MB scans) are downloaded as the site's full-resolution JPEG instead.
pub const MAX_ORIGINAL_BYTES: u64 = 64 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Site {
    /// konachan.net: only safe-for-work posts.
    KonachanNet,
    /// konachan.com: all ratings.
    KonachanCom,
    Yandere,
}

impl From<KonachanSite> for Site {
    fn from(s: KonachanSite) -> Site {
        match s {
            KonachanSite::Net => Site::KonachanNet,
            KonachanSite::Com => Site::KonachanCom,
        }
    }
}

impl Site {
    pub fn provider(self) -> Provider {
        match self {
            Site::KonachanNet | Site::KonachanCom => Provider::Konachan,
            Site::Yandere => Provider::Yandere,
        }
    }

    fn base(self) -> &'static str {
        match self {
            Site::KonachanNet => "https://konachan.net",
            Site::KonachanCom => "https://konachan.com",
            Site::Yandere => "https://yande.re",
        }
    }

    pub fn posts_json_url(self) -> String {
        format!("{}/post.json", self.base())
    }

    /// Tag lookup: `name` is a pattern (`*mitsu*`).
    pub fn tags_json_url(self) -> String {
        format!("{}/tag.json", self.base())
    }

    pub fn posts_xml_url(self) -> String {
        format!("{}/post.xml", self.base())
    }

    pub fn post_page(self, id: u64) -> String {
        format!("{}/post/show/{id}", self.base())
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MoebooruSort {
    /// Newest first (Moebooru's default order).
    #[default]
    Latest,
    Score,
    Random,
}

impl MoebooruSort {
    /// In the order a sort menu lists them.
    pub const ALL: [MoebooruSort; 3] = [MoebooruSort::Latest, MoebooruSort::Score, MoebooruSort::Random];

    pub fn label(self) -> &'static str {
        match self {
            MoebooruSort::Latest => "Latest",
            MoebooruSort::Score => "Score",
            MoebooruSort::Random => "Random",
        }
    }

    pub fn from_str(s: &str) -> Option<MoebooruSort> {
        MoebooruSort::ALL.into_iter().find(|v| v.as_str() == s)
    }

    pub fn as_str(self) -> &'static str {
        match self {
            MoebooruSort::Latest => "latest",
            MoebooruSort::Score => "score",
            MoebooruSort::Random => "random",
        }
    }

    fn metatag(self) -> Option<&'static str> {
        match self {
            MoebooruSort::Latest => None,
            MoebooruSort::Score => Some("order:score"),
            MoebooruSort::Random => Some("order:random"),
        }
    }
}

/// Which ratings to show. Moebooru rates every post Safe, Questionable or
/// Explicit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Ratings {
    pub safe: bool,
    pub questionable: bool,
    pub explicit: bool,
}

impl Default for Ratings {
    /// Safe only, so nothing else shows unless it's asked for.
    fn default() -> Self {
        Ratings {
            safe: true,
            questionable: false,
            explicit: false,
        }
    }
}

impl Ratings {
    /// One metatag for any combination: `rating:x` for one rating,
    /// `-rating:x` to leave one out, nothing for all three. None ticked means
    /// Safe, like Wallhaven's purity.
    fn metatag(self) -> Option<&'static str> {
        match (self.safe, self.questionable, self.explicit) {
            (true, true, true) => None,
            (true, true, false) => Some("-rating:e"),
            (true, false, true) => Some("-rating:q"),
            (false, true, true) => Some("-rating:s"),
            (false, true, false) => Some("rating:q"),
            (false, false, true) => Some("rating:e"),
            (true, false, false) | (false, false, false) => Some("rating:s"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct MoebooruQuery {
    /// Tags, space-separated, as typed (Moebooru syntax: `-tag` excludes).
    pub tags: String,
    pub sorting: MoebooruSort,
    /// "WIDTHxHEIGHT".
    pub atleast: Option<String>,
    pub ratings: Ratings,
    pub page: u32,
}

impl Default for MoebooruQuery {
    fn default() -> Self {
        MoebooruQuery {
            tags: String::new(),
            sorting: MoebooruSort::default(),
            atleast: None,
            ratings: Ratings::default(),
            page: 1,
        }
    }
}

impl MoebooruQuery {
    pub fn filters_only(mut self) -> MoebooruQuery {
        self.tags.clear();
        self.page = 1;
        self
    }

    /// How many filters differ from the defaults (for the Filters button).
    pub fn active_filters(&self) -> usize {
        usize::from(self.sorting != MoebooruSort::default())
            + usize::from(self.atleast.as_deref().is_some_and(|s| !s.is_empty()))
            + usize::from(self.ratings != Ratings::default())
    }

    /// The full tag query: what was typed plus the sorting, rating and size
    /// metatags.
    pub fn tag_query(&self) -> String {
        let mut parts: Vec<String> = split_tags(&self.tags)
            .into_iter()
            .filter(|t| !t.chars().any(char::is_control))
            .collect();
        parts.extend(self.sorting.metatag().map(str::to_string));
        parts.extend(self.ratings.metatag().map(str::to_string));
        if let Some((w, h)) = self
            .atleast
            .as_deref()
            .map(str::trim)
            .filter(|s| is_valid_resolution(s))
            .and_then(|s| s.split_once('x'))
        {
            parts.push(format!("width:>={w}"));
            parts.push(format!("height:>={h}"));
        }
        parts.join(" ")
    }

    /// Parameters for `post.json`: API version 2 with the tags' kinds.
    pub fn to_params(&self) -> Vec<(&'static str, String)> {
        vec![
            ("tags", self.tag_query()),
            ("limit", PER_PAGE.to_string()),
            ("page", self.page.max(1).to_string()),
            ("api_version", "2".to_string()),
            ("include_tags", "1".to_string()),
        ]
    }

    /// Parameters for `post.xml`, asked only for the count.
    pub fn count_params(&self) -> Vec<(&'static str, String)> {
        vec![("tags", self.tag_query()), ("limit", "1".to_string())]
    }
}

/// A post as `post.json` returns it (only the fields used).
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct Post {
    pub id: u64,
    /// Space-separated, words joined with `_`.
    pub tags: String,
    /// The uploader.
    pub author: String,
    pub source: String,
    pub score: i64,
    pub file_size: u64,
    pub file_ext: String,
    pub file_url: String,
    /// Full resolution as JPEG (the original when that already is one).
    pub jpeg_url: String,
    pub jpeg_file_size: u64,
    pub preview_url: String,
    pub sample_url: String,
    pub width: u32,
    pub height: u32,
    /// "s", "q" or "e".
    pub rating: String,
}

/// Tag (as written in posts, `long_hair`) -> its kind: "artist",
/// "character", "copyright", "circle", "general", "faults".
pub type TagTypes = HashMap<String, String>;

/// `post.json`'s answer: version 2's `{ "posts": [...], "tags": {...} }`, or
/// version 1's bare list (no kinds then).
#[derive(Deserialize)]
#[serde(untagged)]
enum PostsAnswer {
    V2 {
        posts: Vec<Post>,
        #[serde(default)]
        tags: TagTypes,
    },
    V1(Vec<Post>),
}

pub fn parse_posts(json: &str) -> Result<(Vec<Post>, TagTypes)> {
    Ok(match serde_json::from_str(json)? {
        PostsAnswer::V2 { posts, tags } => (posts, tags),
        PostsAnswer::V1(posts) => (posts, TagTypes::new()),
    })
}

// ------------------------------------------------------------ suggestions ----

/// A site's tag, as `tag.json` lists it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TagSuggestion {
    /// As searched: `mitsu_(mitsu_art)`.
    pub name: String,
    /// Posts with it.
    pub count: u64,
    /// "artist", "character", "copyright", "circle", "faults" or "general".
    pub kind: String,
    /// What it replaces in the search: the last word, or every plain word
    /// (when they were tried as one tag: "flandre scarlet").
    pub replaces_all: bool,
}

#[derive(Deserialize)]
struct RawTag {
    name: String,
    #[serde(default)]
    count: u64,
    #[serde(rename = "type", default)]
    kind: i64,
}

fn kind_name(kind: i64) -> &'static str {
    match kind {
        1 => "artist",
        3 => "copyright",
        4 => "character",
        5 => "circle",
        6 => "faults",
        _ => "general",
    }
}

/// Is `tag` one to look up: not a metatag (`order:score`), an exclusion
/// (`-tag`) or an either-or (`~tag`)?
fn is_plain(tag: &str) -> bool {
    !tag.contains(':') && !tag.starts_with('-') && !tag.starts_with('~')
}

/// The search's tags. With a comma, each comma-separated piece is one tag,
/// its spaces made underscores ("flandre scarlet, long hair" →
/// `flandre_scarlet`, `long_hair`); without, spaces separate tags, as on
/// the sites themselves.
pub fn split_tags(text: &str) -> Vec<String> {
    if text.contains(',') {
        text.split(',')
            .map(|p| p.split_whitespace().collect::<Vec<_>>().join("_"))
            .filter(|p| !p.is_empty())
            .collect()
    } else {
        text.split_whitespace().map(str::to_string).collect()
    }
}

/// The plain tags of a search (see [`is_plain`]).
pub fn plain_words(text: &str) -> Vec<String> {
    split_tags(text).into_iter().filter(|t| is_plain(t)).collect()
}

/// Patterns to look up for `text`, with whether a hit replaces every plain
/// tag: several words tried as one tag first (`*flandre*scarlet*`), then
/// the last tag anywhere in a tag (`*mitsu*`). Empty for nothing to ask.
pub fn suggestion_patterns(text: &str) -> Vec<(String, bool)> {
    let words = plain_words(text);
    let clean = |w: &str| w.replace(['*', '?'], "");
    let mut out = Vec::new();
    // Space-separated words that may have meant one tag (with commas the
    // pieces already are tags).
    if words.len() > 1 && !text.contains(',') {
        let joined: Vec<String> = words.iter().map(|w| clean(w)).filter(|w| !w.is_empty()).collect();
        out.push((format!("*{}*", joined.join("*")), true));
    }
    if let Some(last) = words.last().map(|w| clean(w)).filter(|w| w.chars().count() >= 2) {
        out.push((format!("*{last}*"), false));
    }
    out
}

/// Parse `tag.json`; tags no post has are left out.
pub fn parse_tags(json: &str, replaces_all: bool) -> Result<Vec<TagSuggestion>> {
    let raw: Vec<RawTag> = serde_json::from_str(json)?;
    Ok(raw
        .into_iter()
        .filter(|t| t.count > 0)
        .map(|t| TagSuggestion { name: t.name, count: t.count, kind: kind_name(t.kind).to_string(), replaces_all })
        .collect())
}

/// `text` with `pick` in: replacing every plain tag, or just the last one.
/// Metatags and exclusions stay where they are, and so does the style: with
/// commas, the result keeps them.
pub fn apply_suggestion(text: &str, pick: &TagSuggestion) -> String {
    let commas = text.contains(',');
    let pieces: Vec<String> = if commas {
        text.split(',').map(|p| p.trim().to_string()).filter(|p| !p.is_empty()).collect()
    } else {
        text.split_whitespace().map(str::to_string).collect()
    };
    let as_tag = |p: &str| p.split_whitespace().collect::<Vec<_>>().join("_");
    let last_plain = pieces.iter().rposition(|p| is_plain(&as_tag(p)));
    let mut placed = false;
    let mut out: Vec<String> = Vec::new();
    for (i, p) in pieces.iter().enumerate() {
        let plain = is_plain(&as_tag(p));
        let replace = if pick.replaces_all { plain } else { Some(i) == last_plain };
        if replace {
            if !placed {
                out.push(pick.name.clone());
                placed = true;
            }
        } else {
            out.push(p.clone());
        }
    }
    if !placed {
        out.push(pick.name.clone());
    }
    out.join(if commas { ", " } else { " " })
}

/// A tag kind as a category label, like Wallhaven's.
fn kind_label(kind: &str) -> &'static str {
    match kind {
        "artist" => "Artists",
        "character" => "Characters",
        "copyright" => "Copyright",
        "circle" => "Circles",
        "faults" => "Faults",
        "general" => "General",
        _ => "",
    }
}

/// The `count` attribute of `post.xml`'s root element.
pub fn parse_count(xml: &str) -> Option<u64> {
    let root = &xml[xml.find("<posts")?..];
    let root = &root[..root.find('>')?];
    let rest = &root[root.find("count=\"")? + "count=\"".len()..];
    rest[..rest.find('"')?].parse().ok()
}

/// Uploader names that are safe to put in a `user:` search.
fn is_plain_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.'))
}

impl Post {
    /// `types` comes with the search (see [`parse_posts`]); empty, no tag is
    /// marked as an artist.
    pub fn into_online(self, site: Site, types: &TagTypes) -> OnlineWallpaper {
        let p = self;
        let purity = match p.rating.as_str() {
            "s" => "sfw",
            "q" => "sketchy",
            _ => "nsfw",
        };
        // Originals over the size cap are downloaded as the full-resolution
        // JPEG instead (same pixels, much smaller than a huge PNG scan).
        let (full, file_size, ext) =
            if p.file_size > MAX_ORIGINAL_BYTES && !p.jpeg_url.is_empty() {
                (p.jpeg_url, p.jpeg_file_size, "jpg".to_string())
            } else {
                (p.file_url, p.file_size, p.file_ext.to_ascii_lowercase())
            };
        let extension = match ext.as_str() {
            "png" => "png",
            "webp" => "webp",
            _ => "jpg",
        };
        let details = Details {
            uploader: Some(&p.author).filter(|a| !a.is_empty()).map(|a| Person {
                name: a.clone(),
                query: is_plain_name(a).then(|| format!("user:{a}")),
            }),
            source: p.source.clone(),
            tags: p
                .tags
                .split_whitespace()
                .map(|t| {
                    let kind = types.get(t).map_or("", String::as_str);
                    OnlineTag {
                        name: t.replace('_', " "),
                        query: t.to_string(),
                        category: kind_label(kind).to_string(),
                        is_artist: kind == "artist",
                    }
                })
                .collect(),
        };
        OnlineWallpaper {
            provider: site.provider(),
            id: p.id.to_string(),
            page_url: site.post_page(p.id),
            thumb: p.preview_url,
            preview: if p.sample_url.is_empty() {
                full.clone()
            } else {
                p.sample_url
            },
            full,
            width: p.width,
            height: p.height,
            file_size,
            extension: extension.to_string(),
            purity: purity.to_string(),
            category: "anime".to_string(),
            likes: p.score.max(0) as u64,
            views: None,
            details: Some(details),
        }
    }
}

/// A page of results. `total` comes from `post.xml`; without it the pager
/// only knows that a full page probably has a next one.
pub fn page(site: Site, posts: Vec<Post>, types: &TagTypes, page: u32, total: Option<u64>) -> Page {
    let full_page = posts.len() as u32 >= PER_PAGE;
    let page = u64::from(page.max(1));
    let (total, last_page) = match total {
        Some(t) => (t, t.div_ceil(u64::from(PER_PAGE)).max(1)),
        None => (0, if full_page { page + 1 } else { page }),
    };
    Page {
        items: posts.into_iter().map(|p| p.into_online(site, types)).collect(),
        page,
        last_page,
        total,
        seed: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get<'a>(p: &'a [(&'static str, String)], k: &str) -> Option<&'a str> {
        p.iter().find(|(key, _)| *key == k).map(|(_, v)| v.as_str())
    }

    #[test]
    fn query_adds_sorting_rating_and_size_metatags() {
        let q = MoebooruQuery::default();
        assert_eq!(get(&q.to_params(), "tags"), Some("rating:s"));
        assert_eq!(get(&q.to_params(), "limit"), Some("24"));
        assert_eq!(get(&q.to_params(), "page"), Some("1"));

        let q = MoebooruQuery {
            tags: "  landscape   -rain ".into(),
            sorting: MoebooruSort::Score,
            atleast: Some("2560x1440".into()),
            ratings: Ratings {
                safe: true,
                questionable: true,
                explicit: true,
            },
            page: 3,
        };
        assert_eq!(
            q.tag_query(),
            "landscape -rain order:score width:>=2560 height:>=1440"
        );
        assert_eq!(get(&q.to_params(), "page"), Some("3"));
        assert_eq!(get(&q.count_params(), "limit"), Some("1"));

        // A bad size is dropped rather than sent.
        let q = MoebooruQuery {
            atleast: Some("big".into()),
            ..MoebooruQuery::default()
        };
        assert_eq!(q.tag_query(), "rating:s");
    }

    #[test]
    fn every_rating_combination_is_one_metatag() {
        let r = |safe, questionable, explicit| {
            Ratings {
                safe,
                questionable,
                explicit,
            }
            .metatag()
        };
        assert_eq!(r(true, false, false), Some("rating:s"));
        assert_eq!(r(false, true, false), Some("rating:q"));
        assert_eq!(r(false, false, true), Some("rating:e"));
        assert_eq!(r(true, true, false), Some("-rating:e"));
        assert_eq!(r(true, false, true), Some("-rating:q"));
        assert_eq!(r(false, true, true), Some("-rating:s"));
        assert_eq!(r(true, true, true), None);
        // Nothing ticked: Safe, like Wallhaven's purity.
        assert_eq!(r(false, false, false), Some("rating:s"));
        // Saved filters from before ratings existed get the Safe default.
        let q: MoebooruQuery = serde_json::from_str(r#"{"sorting":"score"}"#).unwrap();
        assert_eq!(q.ratings, Ratings::default());
    }

    #[test]
    fn active_filters_ignore_tags_and_paging() {
        let q = MoebooruQuery {
            tags: "night".into(),
            page: 4,
            ..MoebooruQuery::default()
        };
        assert_eq!(q.active_filters(), 0);
        let q = MoebooruQuery {
            sorting: MoebooruSort::Random,
            atleast: Some("1920x1080".into()),
            ratings: Ratings {
                explicit: true,
                ..Ratings::default()
            },
            ..MoebooruQuery::default()
        };
        assert_eq!(q.active_filters(), 3);
    }

    #[test]
    fn count_comes_from_the_xml_root() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<posts count="1682" offset="0">
  <post id="1" tags="a" count="9"/>"#;
        assert_eq!(parse_count(xml), Some(1682));
        assert_eq!(parse_count("<posts offset=\"0\">"), None);
        assert_eq!(parse_count("nope"), None);
    }

    #[test]
    fn posts_convert_per_site() {
        let json = r#"[{"id": 409210, "tags": "long_hair night original", "author": "Battlequeen",
            "source": "https://www.pixiv.net/x", "score": 19, "file_size": 1786700, "file_ext": "png",
            "file_url": "https://konachan.com/image/abc/Konachan.com%20-%20409210.png",
            "jpeg_url": "https://konachan.com/jpeg/abc/Konachan.com%20-%20409210.jpg",
            "preview_url": "https://konachan.com/data/preview/ab/c.jpg",
            "sample_url": "https://konachan.com/sample/abc/s.jpg",
            "width": 2000, "height": 1125, "rating": "q", "unused": [1, 2]}]"#;
        let (posts, types) = parse_posts(json).unwrap();
        assert!(types.is_empty(), "version 1: no kinds");
        let none = TagTypes::new();
        let w = posts[0].clone().into_online(Site::KonachanCom, &none);
        assert_eq!(w.key(), "konachan:409210");
        assert_eq!(w.page_url, "https://konachan.com/post/show/409210");
        assert_eq!(w.extension, "png");
        assert_eq!(w.purity, "sketchy");
        assert_eq!(w.likes, 19);
        assert!(w.is_allowed_download_url());
        let d = w.details.unwrap();
        assert_eq!(d.uploader.unwrap().query.as_deref(), Some("user:Battlequeen"));
        assert_eq!(d.tags[0].name, "long hair");
        assert_eq!(d.tags[0].query, "long_hair");

        // The same post shape from yande.re.
        let mut p = posts[0].clone();
        p.file_url = "https://files.yande.re/image/abc/yande.re%20409210.png".into();
        let w = p.into_online(Site::Yandere, &none);
        assert_eq!(w.key(), "yandere:409210");
        assert_eq!(w.page_url, "https://yande.re/post/show/409210");
        assert!(w.is_allowed_download_url());
        // A konachan.com image isn't a valid yande.re download.
        let w = posts[0].clone().into_online(Site::Yandere, &none);
        assert!(!w.is_allowed_download_url());

        // konachan.net: same provider and key, its own page address.
        let mut p = posts[0].clone();
        p.file_url = "https://konachan.net/image/abc/x.png".into();
        let w = p.into_online(Site::KonachanNet, &none);
        assert_eq!(w.key(), "konachan:409210");
        assert_eq!(w.page_url, "https://konachan.net/post/show/409210");
        assert!(w.is_allowed_download_url());
    }

    #[test]
    fn version_2_marks_artists() {
        // As konachan.net and yande.re answer `api_version=2&include_tags=1`.
        let json = r#"{"posts": [{"id": 7, "tags": "2girls banjie flandre_scarlet moon",
            "file_url": "https://konachan.net/image/a/x.png", "file_ext": "png", "rating": "s"}],
            "tags": {"2girls": "general", "banjie": "artist", "flandre_scarlet": "character",
            "moon": "general"}}"#;
        let (posts, types) = parse_posts(json).unwrap();
        assert_eq!(types.get("banjie").map(String::as_str), Some("artist"));
        let w = posts[0].clone().into_online(Site::KonachanNet, &types);
        let tags = w.details.unwrap().tags;
        let artists: Vec<_> = tags.iter().filter(|t| t.is_artist).map(|t| (t.name.as_str(), t.query.as_str())).collect();
        assert_eq!(artists, [("banjie", "banjie")]);
        let flan = tags.iter().find(|t| t.query == "flandre_scarlet").unwrap();
        assert_eq!((flan.name.as_str(), flan.category.as_str(), flan.is_artist), ("flandre scarlet", "Characters", false));
        // A tag the map doesn't list: no kind, not an artist.
        let (posts, _) = parse_posts(r#"{"posts": [{"id": 1, "tags": "x"}]}"#).unwrap();
        let w = posts[0].clone().into_online(Site::Yandere, &TagTypes::new());
        assert!(!w.details.unwrap().tags[0].is_artist);
        // The request asks for version 2 with the kinds.
        let params = MoebooruQuery::default().to_params();
        assert!(params.contains(&("api_version", "2".to_string())));
        assert!(params.contains(&("include_tags", "1".to_string())));
    }

    #[test]
    fn suggestions_patterns_and_replacing() {
        assert_eq!(suggestion_patterns("mitsu"), [("*mitsu*".to_string(), false)]);
        assert_eq!(
            suggestion_patterns("flandre scarlet order:score -bad"),
            [("*flandre*scarlet*".to_string(), true), ("*scarlet*".to_string(), false)]
        );
        // One letter is too little to ask about; metatags alone, nothing.
        assert!(suggestion_patterns("a").is_empty());
        assert!(suggestion_patterns("order:score").is_empty());
        // Wildcards typed in don't double up.
        assert_eq!(suggestion_patterns("mi*tsu"), [("*mitsu*".to_string(), false)]);

        let json = r#"[{"id":1,"name":"mitsu_(mitsu_art)","count":406,"type":1,"ambiguous":false},
            {"id":2,"name":"stockings_long_hair","count":0,"type":0}]"#;
        let tags = parse_tags(json, false).unwrap();
        assert_eq!(tags.len(), 1, "unused tags are left out");
        assert_eq!((tags[0].name.as_str(), tags[0].count, tags[0].kind.as_str()), ("mitsu_(mitsu_art)", 406, "artist"));

        let last = TagSuggestion { name: "mitsu_(mitsu_art)".into(), count: 1, kind: "artist".into(), replaces_all: false };
        assert_eq!(apply_suggestion("night mitsu order:score", &last), "night mitsu_(mitsu_art) order:score");
        let all = TagSuggestion { name: "flandre_scarlet".into(), replaces_all: true, ..last.clone() };
        assert_eq!(apply_suggestion("-bad flandre scarlet", &all), "-bad flandre_scarlet");
        assert_eq!(apply_suggestion("", &last), "mitsu_(mitsu_art)");

        // Commas: each piece is a tag, spaces in it underscores.
        assert_eq!(split_tags("flandre scarlet, long hair ,night"), ["flandre_scarlet", "long_hair", "night"]);
        assert_eq!(split_tags("long_hair night"), ["long_hair", "night"]);
        let q = MoebooruQuery { tags: "flandre scarlet, -rating:e".into(), ..MoebooruQuery::default() };
        assert!(q.tag_query().starts_with("flandre_scarlet -rating:e"), "{}", q.tag_query());
        // No joined pattern with commas; the last piece, as a tag.
        assert_eq!(suggestion_patterns("night, mitsu art"), [("*mitsu_art*".to_string(), false)]);
        // Picking keeps the commas.
        assert_eq!(apply_suggestion("night, mitsu", &last), "night, mitsu_(mitsu_art)");
    }

    #[test]
    fn huge_originals_download_as_the_full_size_jpeg() {
        let p = Post {
            id: 1,
            file_size: MAX_ORIGINAL_BYTES + 1,
            file_ext: "png".into(),
            file_url: "https://files.yande.re/image/a/huge.png".into(),
            jpeg_url: "https://files.yande.re/jpeg/a/huge.jpg".into(),
            jpeg_file_size: 9_000_000,
            ..Post::default()
        };
        let w = p.into_online(Site::Yandere, &TagTypes::new());
        assert_eq!(w.full, "https://files.yande.re/jpeg/a/huge.jpg");
        assert_eq!(w.extension, "jpg");
        assert_eq!(w.file_size, 9_000_000);
        assert!(w.is_allowed_download_url());
    }

    #[test]
    fn pages_from_the_count_or_from_a_full_page() {
        let posts = |n| vec![Post::default(); n];
        let t = TagTypes::new();
        let p = page(Site::KonachanNet, posts(24), &t, 2, Some(1682));
        assert_eq!((p.page, p.last_page, p.total), (2, 71, 1682));
        // No count: a full page probably has a next one, a short one doesn't.
        assert_eq!(page(Site::KonachanCom, posts(24), &t, 2, None).last_page, 3);
        assert_eq!(page(Site::Yandere, posts(5), &t, 2, None).last_page, 2);
        assert_eq!(page(Site::Yandere, posts(0), &t, 1, Some(0)).last_page, 1);
    }
}
