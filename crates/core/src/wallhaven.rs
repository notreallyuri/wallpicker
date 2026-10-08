//! Wallhaven API v1 types and query building.
//!
//! No networking happens here (that lives in `wallpicker`'s service layer), so everything
//! is testable offline. API reference: <https://wallhaven.cc/help/api>.
//!
//! Notes baked into the code below:
//! * NSFW results require an API key; without one the NSFW bit is dropped.
//! * The API key is sent as the `X-API-Key` header, not in the URL.
//! * Wallhaven's `meta.per_page` is sometimes a string and sometimes a number,
//!   and `meta.query` can be a string, an object or null, so both are parsed
//!   loosely.

use serde::{Deserialize, Deserializer, Serialize};

use crate::error::Result;

pub const API_SEARCH_URL: &str = "https://wallhaven.cc/api/v1/search";
/// One wallpaper's details: `API_WALLPAPER_URL` + id.
pub const API_WALLPAPER_URL: &str = "https://wallhaven.cc/api/v1/w/";

// ---------------------------------------------------------------- query ----

/// Toplist periods, with labels.
pub const TOP_RANGES: [(TopRange, &str); 7] = [
    (TopRange::OneDay, "Last day"),
    (TopRange::ThreeDays, "Last 3 days"),
    (TopRange::OneWeek, "Last week"),
    (TopRange::OneMonth, "Last month"),
    (TopRange::ThreeMonths, "Last 3 months"),
    (TopRange::SixMonths, "Last 6 months"),
    (TopRange::OneYear, "Last year"),
];

/// Aspect ratios offered as filters, with labels. Any number can be picked;
/// none means any ratio.
pub const RATIOS: [(&str, &str); 10] = [
    ("16x9", "16:9"),
    ("16x10", "16:10"),
    ("21x9", "21:9"),
    ("32x9", "32:9"),
    ("4x3", "4:3"),
    ("5x4", "5:4"),
    ("3x2", "3:2"),
    ("1x1", "1:1"),
    ("9x16", "9:16"),
    ("10x16", "10:16"),
];

/// Exact resolutions offered as filters (any number; none means any), with
/// labels. Other sizes can be typed in.
pub const RESOLUTIONS: [(&str, &str); 16] = [
    ("1280x720", "1280×720"),
    ("1280x1024", "1280×1024"),
    ("1366x768", "1366×768"),
    ("1440x900", "1440×900"),
    ("1600x900", "1600×900"),
    ("1600x1200", "1600×1200"),
    ("1680x1050", "1680×1050"),
    ("1920x1080", "1920×1080"),
    ("1920x1200", "1920×1200"),
    ("2560x1080", "2560×1080"),
    ("2560x1440", "2560×1440"),
    ("2560x1600", "2560×1600"),
    ("3440x1440", "3440×1440"),
    ("3840x1600", "3840×1600"),
    ("3840x2160", "3840×2160"),
    ("5120x1440", "5120×1440"),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Sorting {
    DateAdded,
    Relevance,
    Random,
    Views,
    Favorites,
    Toplist,
    Hot,
}

impl Sorting {
    /// In the order a sort menu lists them.
    pub const ALL: [Sorting; 7] = [
        Sorting::Toplist,
        Sorting::Hot,
        Sorting::DateAdded,
        Sorting::Relevance,
        Sorting::Views,
        Sorting::Favorites,
        Sorting::Random,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Sorting::Toplist => "Top",
            Sorting::Hot => "Hot",
            Sorting::DateAdded => "Latest",
            Sorting::Relevance => "Relevance",
            Sorting::Views => "Views",
            Sorting::Favorites => "Favorites",
            Sorting::Random => "Random",
        }
    }

    pub fn from_str(s: &str) -> Option<Sorting> {
        Sorting::ALL.into_iter().find(|v| v.as_str() == s)
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Sorting::DateAdded => "date_added",
            Sorting::Relevance => "relevance",
            Sorting::Random => "random",
            Sorting::Views => "views",
            Sorting::Favorites => "favorites",
            Sorting::Toplist => "toplist",
            Sorting::Hot => "hot",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Order {
    Desc,
    Asc,
}

impl Order {
    pub fn as_str(self) -> &'static str {
        match self {
            Order::Desc => "desc",
            Order::Asc => "asc",
        }
    }
}

/// Only used when sorting by toplist.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TopRange {
    #[serde(rename = "1d")]
    OneDay,
    #[serde(rename = "3d")]
    ThreeDays,
    #[serde(rename = "1w")]
    OneWeek,
    #[serde(rename = "1M")]
    OneMonth,
    #[serde(rename = "3M")]
    ThreeMonths,
    #[serde(rename = "6M")]
    SixMonths,
    #[serde(rename = "1y")]
    OneYear,
}

impl TopRange {
    pub fn as_str(self) -> &'static str {
        match self {
            TopRange::OneDay => "1d",
            TopRange::ThreeDays => "3d",
            TopRange::OneWeek => "1w",
            TopRange::OneMonth => "1M",
            TopRange::ThreeMonths => "3M",
            TopRange::SixMonths => "6M",
            TopRange::OneYear => "1y",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Categories {
    pub general: bool,
    pub anime: bool,
    pub people: bool,
}

impl Default for Categories {
    fn default() -> Self {
        Categories {
            general: true,
            anime: true,
            people: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Purity {
    pub sfw: bool,
    pub sketchy: bool,
    pub nsfw: bool,
}

impl Default for Purity {
    fn default() -> Self {
        Purity {
            sfw: true,
            sketchy: false,
            nsfw: false,
        }
    }
}

/// Everything the search form can express.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SearchQuery {
    pub q: String,
    pub categories: Categories,
    pub purity: Purity,
    pub sorting: Sorting,
    pub order: Order,
    pub top_range: Option<TopRange>,
    /// Minimum resolution, e.g. "1920x1080".
    pub atleast: Option<String>,
    /// Exact resolutions, e.g. ["1920x1080", "2560x1440"].
    pub resolutions: Vec<String>,
    /// Aspect ratios, e.g. ["16x9", "16x10"].
    pub ratios: Vec<String>,
    pub page: u32,
    /// Keeps `Random` pagination stable; comes back in `meta.seed`.
    pub seed: Option<String>,
}

impl Default for SearchQuery {
    fn default() -> Self {
        SearchQuery {
            q: String::new(),
            categories: Categories::default(),
            purity: Purity::default(),
            sorting: Sorting::DateAdded,
            order: Order::Desc,
            top_range: Some(TopRange::OneMonth),
            atleast: None,
            resolutions: Vec::new(),
            ratios: Vec::new(),
            page: 1,
            seed: None,
        }
    }
}

fn bit(b: bool) -> char {
    if b { '1' } else { '0' }
}

/// "1920x1080" -> true. Rejects anything that is not `<digits>x<digits>`.
pub fn is_valid_resolution(s: &str) -> bool {
    let Some((w, h)) = s.split_once('x') else {
        return false;
    };
    let ok = |p: &str| !p.is_empty() && p.len() <= 5 && p.bytes().all(|b| b.is_ascii_digit());
    ok(w) && ok(h)
}

/// "16x9" or "0.5x1": digits and dots on both sides of an `x`.
pub fn is_valid_ratio(s: &str) -> bool {
    let Some((w, h)) = s.split_once('x') else {
        return false;
    };
    let ok = |p: &str| {
        !p.is_empty()
            && p.len() <= 6
            && p.bytes().any(|b| b.is_ascii_digit())
            && p.bytes().all(|b| b.is_ascii_digit() || b == b'.')
    };
    ok(w) && ok(h)
}

impl SearchQuery {
    /// The part of a search worth remembering between sessions: the filters,
    /// without the search text or paging.
    pub fn filters_only(mut self) -> SearchQuery {
        self.q.clear();
        self.page = 1;
        self.seed = None;
        self
    }

    /// How many filters differ from the defaults (for the Filters button).
    /// The top range only counts when sorting by Top, where it applies.
    pub fn active_filters(&self) -> usize {
        let d = SearchQuery::default();
        [
            self.sorting != d.sorting,
            self.sorting == Sorting::Toplist && self.top_range != d.top_range,
            self.atleast.as_deref().is_some_and(|s| !s.is_empty()),
            !self.resolutions.is_empty(),
            !self.ratios.is_empty(),
            self.categories != d.categories,
            self.purity != d.purity,
        ]
        .into_iter()
        .filter(|on| *on)
        .count()
    }

    /// Build the URL query parameters.
    ///
    /// `has_api_key` decides whether the NSFW bit may be sent. Values that fail
    /// validation are dropped instead of forwarded, so a stray character in the
    /// form can't turn into a malformed request.
    pub fn to_params(&self, has_api_key: bool) -> Vec<(&'static str, String)> {
        let mut p: Vec<(&'static str, String)> = Vec::new();

        let q = self.q.trim();
        if !q.is_empty() {
            p.push(("q", q.to_string()));
        }

        let c = self.categories;
        let categories = if !(c.general || c.anime || c.people) {
            "111".to_string() // nothing ticked: search everything, like the site
        } else {
            [bit(c.general), bit(c.anime), bit(c.people)]
                .iter()
                .collect()
        };
        p.push(("categories", categories));

        let pu = self.purity;
        let nsfw = pu.nsfw && has_api_key;
        let purity = if !(pu.sfw || pu.sketchy || nsfw) {
            "100".to_string()
        } else {
            [bit(pu.sfw), bit(pu.sketchy), bit(nsfw)].iter().collect()
        };
        p.push(("purity", purity));

        p.push(("sorting", self.sorting.as_str().to_string()));
        p.push(("order", self.order.as_str().to_string()));

        if self.sorting == Sorting::Toplist
            && let Some(range) = self.top_range
        {
            p.push(("topRange", range.as_str().to_string()));
        }

        let resolutions: Vec<&str> = self
            .resolutions
            .iter()
            .map(|r| r.trim())
            .filter(|r| is_valid_resolution(r))
            .collect();
        // Wallhaven ignores `resolutions` when `atleast` is there too, so
        // picked exact sizes win over a minimum.
        if resolutions.is_empty()
            && let Some(at) = self.atleast.as_deref().map(str::trim)
            && is_valid_resolution(at)
        {
            p.push(("atleast", at.to_string()));
        }

        if !resolutions.is_empty() {
            p.push(("resolutions", resolutions.join(",")));
        }

        let ratios: Vec<&str> = self
            .ratios
            .iter()
            .map(|s| s.trim())
            .filter(|s| is_valid_ratio(s))
            .collect();
        if !ratios.is_empty() {
            p.push(("ratios", ratios.join(",")));
        }

        if self.page > 1 {
            p.push(("page", self.page.min(10_000).to_string()));
        }

        if self.sorting == Sorting::Random
            && let Some(seed) = self.seed.as_deref().map(str::trim)
            && !seed.is_empty()
            && seed.len() <= 16
            && seed.bytes().all(|b| b.is_ascii_alphanumeric())
        {
            p.push(("seed", seed.to_string()));
        }

        p
    }
}

// ------------------------------------------------------------- response ----

/// Accept `24` and `"24"` alike.
fn de_flexible_u64<'de, D>(d: D) -> std::result::Result<u64, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum N {
        Num(u64),
        Str(String),
    }
    match N::deserialize(d)? {
        N::Num(n) => Ok(n),
        N::Str(s) => s.trim().parse().map_err(serde::de::Error::custom),
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Thumbs {
    pub large: String,
    pub original: String,
    pub small: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Wallpaper {
    pub id: String,
    pub url: String,
    pub views: u64,
    pub favorites: u64,
    pub purity: String,
    pub category: String,
    pub dimension_x: u32,
    pub dimension_y: u32,
    pub resolution: String,
    pub file_size: u64,
    pub file_type: String,
    /// Direct link to the full-size image.
    pub path: String,
    pub thumbs: Thumbs,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Meta {
    #[serde(deserialize_with = "de_flexible_u64")]
    pub current_page: u64,
    #[serde(deserialize_with = "de_flexible_u64")]
    pub last_page: u64,
    #[serde(deserialize_with = "de_flexible_u64")]
    pub per_page: u64,
    #[serde(deserialize_with = "de_flexible_u64")]
    pub total: u64,
    pub seed: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SearchResponse {
    pub data: Vec<Wallpaper>,
    pub meta: Meta,
}

pub fn parse_search_response(json: &str) -> Result<SearchResponse> {
    Ok(serde_json::from_str(json)?)
}

// ------------------------------------------------------------ collections ----

/// Your collections (needs the API key; private ones included).
pub const API_COLLECTIONS_URL: &str = "https://wallhaven.cc/api/v1/collections";

/// One of your Wallhaven collections.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Collection {
    pub id: u64,
    pub label: String,
    /// How many wallpapers are in it.
    #[serde(deserialize_with = "de_flexible_u64", default)]
    pub count: u64,
    /// Wallhaven sends 0 or 1.
    #[serde(deserialize_with = "de_flexible_bool", default)]
    pub public: bool,
}

fn de_flexible_bool<'de, D: Deserializer<'de>>(d: D) -> std::result::Result<bool, D::Error> {
    Ok(match serde_json::Value::deserialize(d)? {
        serde_json::Value::Bool(b) => b,
        serde_json::Value::Number(n) => n.as_u64().unwrap_or(0) != 0,
        _ => false,
    })
}

#[derive(Deserialize)]
struct CollectionsResponse {
    #[serde(default)]
    data: Vec<Collection>,
}

pub fn parse_collections(json: &str) -> Result<Vec<Collection>> {
    Ok(serde_json::from_str::<CollectionsResponse>(json)?.data)
}

/// A Wallhaven username safe to put in a URL path.
pub fn is_valid_username(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && name.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.'))
}

/// The address of a page of a collection's wallpapers (the same shape as a
/// search: [`parse_search_response`]). None for a name that isn't one.
/// Every purity: they're your own picks (NSFW only comes with the API key).
pub fn collection_url(username: &str, id: u64, page: u32) -> Option<String> {
    is_valid_username(username)
        .then(|| format!("{API_COLLECTIONS_URL}/{username}/{id}?purity=111&page={}", page.max(1)))
}

/// Page numbers for a pager over `last` pages: the first, the last, and the
/// ones around `current`. `None` marks a gap ("…"); a gap of a single page
/// shows that page instead.
pub fn page_window(current: u64, last: u64) -> Vec<Option<u64>> {
    let last = last.max(1);
    let current = current.clamp(1, last);
    let mut pages = vec![1, current.saturating_sub(1), current, current + 1, last];
    pages.retain(|p| (1..=last).contains(p));
    pages.sort_unstable();
    pages.dedup();

    let mut out = Vec::new();
    let mut prev = 0;
    for p in pages {
        if prev != 0 && p - prev == 2 {
            out.push(Some(prev + 1));
        } else if prev != 0 && p - prev > 2 {
            out.push(None);
        }
        out.push(Some(p));
        prev = p;
    }
    out
}

// -------------------------------------------------------------- details ----

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Uploader {
    pub username: String,
    /// "User", "Moderator"...; "deleted" (as the name) for a removed account.
    pub group: String,
}

impl Uploader {
    pub fn is_deleted(&self) -> bool {
        self.group == "deleted"
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Tag {
    #[serde(deserialize_with = "de_flexible_u64")]
    pub id: u64,
    pub name: String,
    pub category: String,
    pub purity: String,
}

/// What `/api/v1/w/<id>` adds to a search result. Search results leave out
/// the uploader, source and tags.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct WallpaperInfo {
    pub id: String,
    pub uploader: Option<Uploader>,
    /// Where the image came from, often the artist's page. May be empty.
    pub source: String,
    pub tags: Vec<Tag>,
}

impl Tag {
    /// Wallhaven files artists as tags in the "Artists" category.
    pub fn is_artist(&self) -> bool {
        self.category == "Artists"
    }
}

pub fn parse_info_response(json: &str) -> Result<WallpaperInfo> {
    #[derive(Deserialize)]
    struct Envelope {
        data: WallpaperInfo,
    }
    Ok(serde_json::from_str::<Envelope>(json)?.data)
}

// The search box takes Wallhaven's query syntax as-is. These build the
// special forms from data we got back from Wallhaven, refusing anything that
// would read as different syntax.

/// Wallpapers with this exact tag.
pub fn tag_query(tag_id: u64) -> String {
    format!("id:{tag_id}")
}

/// Wallpapers uploaded by `username`.
pub fn uploader_query(username: &str) -> Option<String> {
    let ok = !username.is_empty()
        && username.len() <= 64
        && username
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.'));
    ok.then(|| format!("@{username}"))
}

/// Links the app will hand to the browser: http(s) only, nothing that could
/// be read as an option or split into several arguments.
pub fn is_openable_url(url: &str) -> bool {
    (url.starts_with("https://") || url.starts_with("http://"))
        && url.len() <= 2048
        && !url
            .bytes()
            .any(|b| b.is_ascii_whitespace() || b.is_ascii_control())
}

// ------------------------------------------------------------- download ----

// File names and allowed hosts live in `online`, shared by every provider.

impl Wallpaper {
    /// File extension to save the download under.
    pub fn extension(&self) -> &'static str {
        match self.file_type.as_str() {
            "image/png" => "png",
            "image/jpeg" | "image/jpg" => "jpg",
            "image/webp" => "webp",
            _ => match self
                .path
                .rsplit('.')
                .next()
                .map(|e| e.to_ascii_lowercase())
                .as_deref()
            {
                Some("png") => "png",
                Some("webp") => "webp",
                _ => "jpg",
            },
        }
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    fn get<'a>(p: &'a [(&'static str, String)], k: &str) -> Option<&'a str> {
        p.iter().find(|(key, _)| *key == k).map(|(_, v)| v.as_str())
    }

    #[test]
    fn default_query_params() {
        let p = SearchQuery::default().to_params(false);
        assert_eq!(get(&p, "q"), None);
        assert_eq!(get(&p, "categories"), Some("111"));
        assert_eq!(get(&p, "purity"), Some("100"));
        assert_eq!(get(&p, "sorting"), Some("date_added"));
        assert_eq!(get(&p, "order"), Some("desc"));
        // The range only applies to Top.
        assert_eq!(get(&p, "topRange"), None);
        assert_eq!(get(&p, "page"), None);
    }

    #[test]
    fn nsfw_needs_an_api_key() {
        let q = SearchQuery {
            purity: Purity {
                sfw: true,
                sketchy: true,
                nsfw: true,
            },
            ..SearchQuery::default()
        };
        assert_eq!(get(&q.to_params(false), "purity"), Some("110"));
        assert_eq!(get(&q.to_params(true), "purity"), Some("111"));

        let only_nsfw = SearchQuery {
            purity: Purity {
                sfw: false,
                sketchy: false,
                nsfw: true,
            },
            ..SearchQuery::default()
        };
        // Without a key nothing is left, so fall back to SFW only.
        assert_eq!(get(&only_nsfw.to_params(false), "purity"), Some("100"));
    }

    #[test]
    fn empty_category_selection_means_all() {
        let q = SearchQuery {
            categories: Categories {
                general: false,
                anime: false,
                people: false,
            },
            ..SearchQuery::default()
        };
        assert_eq!(get(&q.to_params(false), "categories"), Some("111"));

        let q = SearchQuery {
            categories: Categories {
                general: false,
                anime: true,
                people: false,
            },
            ..SearchQuery::default()
        };
        assert_eq!(get(&q.to_params(false), "categories"), Some("010"));
    }

    #[test]
    fn top_range_only_for_toplist() {
        let q = SearchQuery {
            sorting: Sorting::Relevance,
            ..SearchQuery::default()
        };
        assert_eq!(get(&q.to_params(false), "topRange"), None);
    }

    #[test]
    fn invalid_filters_are_dropped() {
        let q = SearchQuery {
            q: "  mountain lake  ".into(),
            atleast: Some("1920x1080&apikey=x".into()),
            resolutions: vec!["1920x1080".into(), "bad".into(), "2560x1440".into()],
            ratios: vec!["16x9".into(), "evil;".into(), "0.5x1".into()],
            ..SearchQuery::default()
        };
        let p = q.to_params(false);
        assert_eq!(get(&p, "q"), Some("mountain lake"));
        assert_eq!(get(&p, "atleast"), None);
        assert_eq!(get(&p, "resolutions"), Some("1920x1080,2560x1440"));
        assert_eq!(get(&p, "ratios"), Some("16x9,0.5x1"));
    }

    #[test]
    fn collections_parse_and_address() {
        // As Wallhaven answers (public is 0 or 1).
        let json = r#"{"data":[{"id":2117134,"label":"Default","views":0,"public":0,"count":16},
            {"id":2117347,"label":"goon","views":2,"public":1,"count":"11"}]}"#;
        let c = parse_collections(json).unwrap();
        assert_eq!(c.len(), 2);
        assert_eq!((c[0].label.as_str(), c[0].count, c[0].public), ("Default", 16, false));
        assert_eq!((c[1].count, c[1].public), (11, true));
        assert_eq!(
            collection_url("some_user-1.x", 2117134, 0).as_deref(),
            Some("https://wallhaven.cc/api/v1/collections/some_user-1.x/2117134?purity=111&page=1")
        );
        assert_eq!(collection_url("../x", 1, 1), None);
        assert_eq!(collection_url("a b", 1, 1), None);
        assert_eq!(collection_url("", 1, 1), None);
    }

    #[test]
    fn exact_resolutions_win_over_a_minimum() {
        let get = |p: &[(&str, String)], k: &str| p.iter().find(|(n, _)| *n == k).map(|(_, v)| v.clone());
        let q = SearchQuery {
            atleast: Some("1280x720".into()),
            resolutions: vec!["1920x1080".into(), "1024x768".into()],
            ..SearchQuery::default()
        };
        let p = q.to_params(false);
        assert_eq!(get(&p, "resolutions").as_deref(), Some("1920x1080,1024x768"));
        assert_eq!(get(&p, "atleast"), None);
        assert_eq!(q.active_filters(), 2);
        // Only invalid exact sizes: the minimum still applies.
        let q = SearchQuery { resolutions: vec!["big".into()], ..q };
        assert_eq!(get(&q.to_params(false), "atleast").as_deref(), Some("1280x720"));
    }

    #[test]
    fn valid_atleast_and_paging() {
        let q = SearchQuery {
            atleast: Some("2560x1440".into()),
            page: 3,
            ..SearchQuery::default()
        };
        let p = q.to_params(false);
        assert_eq!(get(&p, "atleast"), Some("2560x1440"));
        assert_eq!(get(&p, "page"), Some("3"));
    }

    #[test]
    fn seed_only_for_random_and_only_if_clean() {
        let q = SearchQuery {
            sorting: Sorting::Random,
            seed: Some("aB3xYz".into()),
            ..SearchQuery::default()
        };
        assert_eq!(get(&q.to_params(false), "seed"), Some("aB3xYz"));

        let q = SearchQuery {
            sorting: Sorting::Random,
            seed: Some("a b&c".into()),
            ..SearchQuery::default()
        };
        assert_eq!(get(&q.to_params(false), "seed"), None);

        let q = SearchQuery {
            sorting: Sorting::Hot,
            seed: Some("abc".into()),
            ..SearchQuery::default()
        };
        assert_eq!(get(&q.to_params(false), "seed"), None);
    }

    #[test]
    fn query_defaults_when_ui_sends_a_partial_object() {
        let q: SearchQuery = serde_json::from_str(r#"{"q":"forest","page":2}"#).unwrap();
        assert_eq!(q.q, "forest");
        assert_eq!(q.page, 2);
        assert_eq!(q.sorting, Sorting::DateAdded);
    }

    #[test]
    fn active_filters_count_what_differs_from_the_defaults() {
        assert_eq!(SearchQuery::default().active_filters(), 0);
        // Text and paging aren't filters.
        let q = SearchQuery {
            q: "forest".into(),
            page: 3,
            ..SearchQuery::default()
        };
        assert_eq!(q.active_filters(), 0);
        // A different range only counts with Top.
        let q = SearchQuery {
            top_range: Some(TopRange::OneYear),
            ..SearchQuery::default()
        };
        assert_eq!(q.active_filters(), 0);
        let q = SearchQuery {
            sorting: Sorting::Toplist,
            top_range: Some(TopRange::OneYear),
            atleast: Some("2560x1440".into()),
            ratios: vec!["16x9".into()],
            categories: Categories {
                people: false,
                ..Categories::default()
            },
            purity: Purity {
                sketchy: true,
                ..Purity::default()
            },
            ..SearchQuery::default()
        };
        assert_eq!(q.active_filters(), 6);
    }

    #[test]
    fn filters_only_drops_text_and_paging() {
        let q = SearchQuery {
            q: "forest".into(),
            page: 3,
            seed: Some("abc123".into()),
            sorting: Sorting::Views,
            atleast: Some("1920x1080".into()),
            ..SearchQuery::default()
        }
        .filters_only();
        assert_eq!(q.q, "");
        assert_eq!(q.page, 1);
        assert_eq!(q.seed, None);
        assert_eq!(q.sorting, Sorting::Views);
        assert_eq!(q.atleast.as_deref(), Some("1920x1080"));
    }

    #[test]
    fn parses_a_search_response() {
        // Shape follows the documented API; per_page is a string and `query`
        // an object, as the real API does in some cases.
        let json = r##"{
          "data": [{
            "id": "94x38z",
            "url": "https://wallhaven.cc/w/94x38z",
            "short_url": "http://whvn.cc/94x38z",
            "views": 12, "favorites": 3,
            "source": "", "purity": "sfw", "category": "anime",
            "dimension_x": 6742, "dimension_y": 3534,
            "resolution": "6742x3534", "ratio": "1.91",
            "file_size": 5070446, "file_type": "image/jpeg",
            "created_at": "2018-10-31 01:23:10",
            "colors": ["#000000"],
            "path": "https://w.wallhaven.cc/full/94/wallhaven-94x38z.jpg",
            "thumbs": {
              "large": "https://th.wallhaven.cc/lg/94/94x38z.jpg",
              "original": "https://th.wallhaven.cc/orig/94/94x38z.jpg",
              "small": "https://th.wallhaven.cc/small/94/94x38z.jpg"
            }
          }],
          "meta": {
            "current_page": 1, "last_page": 7, "per_page": "24",
            "total": 164, "query": {"id": 1, "tag": "anime"}, "seed": null
          }
        }"##;
        let r = parse_search_response(json).unwrap();
        assert_eq!(r.data.len(), 1);
        assert_eq!(r.data[0].id, "94x38z");
        assert_eq!(
            r.data[0].thumbs.large,
            "https://th.wallhaven.cc/lg/94/94x38z.jpg"
        );
        assert_eq!(r.meta.per_page, 24);
        assert_eq!(r.meta.last_page, 7);
        assert_eq!(r.meta.seed, None);
    }

    #[test]
    fn parses_wallpaper_details() {
        let json = r##"{"data": {
            "id": "94x38z",
            "uploader": {"username": "someuser", "group": "User", "avatar": {}},
            "source": "https://www.artstation.com/artwork/abc",
            "purity": "sfw",
            "tags": [
              {"id": 1, "name": "anime", "alias": "", "category_id": 1,
               "category": "Anime & Manga", "purity": "sfw",
               "created_at": "2015-01-16 02:06:45"},
              {"id": "42", "name": "Ilya Kuvshinov", "category": "Artists", "purity": "sfw"}
            ]
        }}"##;
        let info = parse_info_response(json).unwrap();
        assert_eq!(info.id, "94x38z");
        assert_eq!(info.uploader.unwrap().username, "someuser");
        assert_eq!(info.source, "https://www.artstation.com/artwork/abc");
        assert_eq!(info.tags.len(), 2);
        assert_eq!(info.tags[1].id, 42);
        assert_eq!(info.tags[1].name, "Ilya Kuvshinov");
        assert!(info.tags[1].is_artist());
        assert!(!info.tags[0].is_artist());

        // A removed account: named "deleted", in the group "deleted".
        let info = parse_info_response(r#"{"data": {"id": "x", "uploader": {"username": "deleted", "group": "deleted"}}}"#).unwrap();
        assert!(info.uploader.unwrap().is_deleted());

        // Deleted uploader / no tags.
        let info = parse_info_response(r#"{"data": {"id": "x", "uploader": null}}"#).unwrap();
        assert!(info.uploader.is_none());
        assert!(info.tags.is_empty());
    }

    #[test]
    fn special_queries_refuse_other_syntax() {
        assert_eq!(tag_query(37), "id:37");
        assert_eq!(uploader_query("Some_user.1-2").as_deref(), Some("@Some_user.1-2"));
        assert_eq!(uploader_query("a +nsfw"), None);
        assert_eq!(uploader_query(""), None);
    }

    #[test]
    fn only_plain_web_links_are_opened() {
        assert!(is_openable_url("https://www.pixiv.net/artworks/1"));
        assert!(is_openable_url("http://example.com/a?b=c"));
        assert!(!is_openable_url("file:///etc/passwd"));
        assert!(!is_openable_url("--help"));
        assert!(!is_openable_url("https://a.com/x y"));
        assert!(!is_openable_url("javascript:alert(1)"));
        assert!(!is_openable_url(""));
    }

    #[test]
    fn page_window_shows_ends_and_neighbours() {
        let w = |c, l| {
            page_window(c, l)
                .into_iter()
                .map(|p| p.map_or("…".to_string(), |n| n.to_string()))
                .collect::<Vec<_>>()
                .join(" ")
        };
        assert_eq!(w(1, 1), "1");
        assert_eq!(w(1, 0), "1");
        assert_eq!(w(1, 5), "1 2 … 5");
        assert_eq!(w(3, 5), "1 2 3 4 5");
        assert_eq!(w(4, 10), "1 2 3 4 5 … 10");
        assert_eq!(w(5, 10), "1 … 4 5 6 … 10");
        assert_eq!(w(10, 10), "1 … 9 10");
        assert_eq!(w(99, 10), "1 … 9 10");
        assert_eq!(w(5000, 22140), "1 … 4999 5000 5001 … 22140");
    }

    #[test]
    fn meta_accepts_numbers_strings_and_query_as_string() {
        let json = r#"{"data":[],"meta":{"current_page":"2","last_page":3,"per_page":24,"total":"50","query":"cats"}}"#;
        let r = parse_search_response(json).unwrap();
        assert_eq!(r.meta.current_page, 2);
        assert_eq!(r.meta.total, 50);
    }

    #[test]
    fn empty_object_parses_to_empty_response() {
        let r = parse_search_response("{}").unwrap();
        assert!(r.data.is_empty());
    }

    #[test]
    fn extensions() {
        let mut w = Wallpaper {
            file_type: "image/png".into(),
            ..Wallpaper::default()
        };
        assert_eq!(w.extension(), "png");

        w.file_type = String::new();
        w.path = "https://w.wallhaven.cc/full/94/wallhaven-94x38z.webp".into();
        assert_eq!(w.extension(), "webp");

        w.path = String::new();
        assert_eq!(w.extension(), "jpg");
    }
}
