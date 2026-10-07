//! Provider-neutral types for the online sources (Wallhaven, Konachan,
//! yande.re, Unsplash): what
//! the search results, the details dialog and the downloads work with.
//!
//! Each provider module turns its own API types into these.

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::moebooru::MoebooruQuery;
use crate::unsplash::UnsplashQuery;
use crate::wallhaven::{self, SearchQuery};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Provider {
    #[default]
    Wallhaven,
    Konachan,
    Yandere,
    Unsplash,
}

impl Provider {
    pub const ALL: [Provider; 4] = [Provider::Wallhaven, Provider::Konachan, Provider::Yandere, Provider::Unsplash];

    /// Also the prefix of downloaded file names.
    pub fn as_str(self) -> &'static str {
        match self {
            Provider::Wallhaven => "wallhaven",
            Provider::Konachan => "konachan",
            Provider::Yandere => "yandere",
            Provider::Unsplash => "unsplash",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Provider::Wallhaven => "Wallhaven",
            Provider::Konachan => "Konachan",
            Provider::Yandere => "yande.re",
            Provider::Unsplash => "Unsplash",
        }
    }

    pub fn from_str(s: &str) -> Option<Provider> {
        Provider::ALL.into_iter().find(|p| p.as_str() == s)
    }

    /// Full-size images are only downloaded from these, so a tampered
    /// response can't make the app fetch arbitrary URLs.
    fn download_prefixes(self) -> &'static [&'static str] {
        match self {
            Provider::Wallhaven => &["https://w.wallhaven.cc/"],
            // Both Konachans: which one is used is a setting.
            Provider::Konachan => &[
                "https://konachan.net/image/",
                "https://konachan.net/jpeg/",
                "https://konachan.com/image/",
                "https://konachan.com/jpeg/",
            ],
            Provider::Yandere => &["https://files.yande.re/image/", "https://files.yande.re/jpeg/"],
            Provider::Unsplash => &["https://images.unsplash.com/"],
        }
    }
}

/// Minimum sizes offered as filters ("" is any), with labels. Small ones
/// too: a wallpaper for a 720p or 4:3 screen is a fine find. Other sizes can
/// be typed in.
pub const MIN_RESOLUTIONS: [(&str, &str); 10] = [
    ("", "Any size"),
    ("1280x720", "≥ 1280×720"),
    ("1366x768", "≥ 1366×768"),
    ("1600x900", "≥ 1600×900"),
    ("1600x1200", "≥ 1600×1200"),
    ("1920x1080", "≥ 1920×1080"),
    ("2560x1080", "≥ 2560×1080"),
    ("2560x1440", "≥ 2560×1440"),
    ("3440x1440", "≥ 3440×1440"),
    ("3840x2160", "≥ 3840×2160"),
];

/// A search result from any provider.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct OnlineWallpaper {
    pub provider: Provider,
    /// The provider's own id.
    pub id: String,
    /// The wallpaper's page on the provider's website.
    pub page_url: String,
    /// Small image for the grid.
    pub thumb: String,
    /// Bigger image for the details dialog.
    pub preview: String,
    /// The full-size image, which is what gets downloaded.
    pub full: String,
    pub width: u32,
    pub height: u32,
    pub file_size: u64,
    /// "jpg", "png" or "webp".
    pub extension: String,
    /// "sfw", "sketchy" or "nsfw".
    pub purity: String,
    pub category: String,
    /// Favorites (Wallhaven) or score (Moebooru sites).
    pub likes: u64,
    pub views: Option<u64>,
    /// Set when the search result already carries the details (Moebooru);
    /// otherwise they're fetched when the details dialog opens.
    pub details: Option<Details>,
}

/// Uploader, source and tags of one wallpaper.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Details {
    pub uploader: Option<Person>,
    /// Where the image came from, often the artist's page. May be empty.
    pub source: String,
    pub tags: Vec<OnlineTag>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Person {
    pub name: String,
    /// The search for their uploads, if the provider has one.
    pub query: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct OnlineTag {
    pub name: String,
    /// The search that finds exactly this tag.
    pub query: String,
    /// Provider's tag category ("Artists", "Anime & Manga", ...), if known.
    pub category: String,
    pub is_artist: bool,
}

/// One page of search results.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Page {
    pub items: Vec<OnlineWallpaper>,
    pub page: u64,
    pub last_page: u64,
    pub total: u64,
    /// Keeps a random order stable across pages (Wallhaven).
    pub seed: Option<String>,
}

/// A search on one provider, with that provider's own filters.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "provider", content = "query", rename_all = "lowercase")]
pub enum OnlineQuery {
    Wallhaven(SearchQuery),
    Konachan(MoebooruQuery),
    Yandere(MoebooruQuery),
    Unsplash(UnsplashQuery),
}

impl OnlineQuery {
    pub fn provider(&self) -> Provider {
        match self {
            OnlineQuery::Wallhaven(_) => Provider::Wallhaven,
            OnlineQuery::Konachan(_) => Provider::Konachan,
            OnlineQuery::Yandere(_) => Provider::Yandere,
            OnlineQuery::Unsplash(_) => Provider::Unsplash,
        }
    }

    /// The same search on another page.
    pub fn at_page(mut self, page: u32, seed: Option<String>) -> OnlineQuery {
        match &mut self {
            OnlineQuery::Wallhaven(q) => {
                q.page = page;
                q.seed = seed;
            }
            OnlineQuery::Konachan(q) | OnlineQuery::Yandere(q) => q.page = page,
            OnlineQuery::Unsplash(q) => q.page = page,
        }
        self
    }
}

/// The filters of every provider, and which provider was last used. Saved
/// between sessions without search text or paging.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct OnlineFilters {
    pub provider: Provider,
    pub wallhaven: SearchQuery,
    pub konachan: MoebooruQuery,
    pub yandere: MoebooruQuery,
    pub unsplash: UnsplashQuery,
}

impl OnlineFilters {
    pub fn filters_only(self) -> OnlineFilters {
        OnlineFilters {
            provider: self.provider,
            wallhaven: self.wallhaven.filters_only(),
            konachan: self.konachan.filters_only(),
            yandere: self.yandere.filters_only(),
            unsplash: self.unsplash.filters_only(),
        }
    }
}

/// Wallhaven originals up to this size are what its details show.
pub const MAX_ORIGINAL_PREVIEW_BYTES: u64 = 24 * 1024 * 1024;

impl OnlineWallpaper {
    /// The sharpest image worth showing in its details. Wallhaven's
    /// `preview` is a ~300px thumbnail, so its original is used (when it
    /// isn't huge); Moebooru's is a ~1500px sample, plenty for a dialog.
    pub fn details_image(&self) -> &str {
        let fallback = if self.preview.is_empty() { &self.thumb } else { &self.preview };
        match self.provider {
            Provider::Wallhaven if self.file_size > 0 && self.file_size <= MAX_ORIGINAL_PREVIEW_BYTES => &self.full,
            _ => fallback,
        }
    }

    /// Unique across providers: "konachan:409210".
    pub fn key(&self) -> String {
        format!("{}:{}", self.provider.as_str(), self.id)
    }

    pub fn resolution(&self) -> String {
        format!("{}x{}", self.width, self.height)
    }

    /// `<provider>-<id>.<ext>`, or an error if the id looks unsafe (it ends
    /// up in a file name). [`key_from_file_name`] reads it back.
    pub fn file_name(&self) -> Result<String> {
        if !is_valid_id(&self.id) {
            return Err(Error::Invalid(format!("unsafe wallpaper id: {:?}", self.id)));
        }
        let ext = match self.extension.as_str() {
            "png" => "png",
            "webp" => "webp",
            _ => "jpg",
        };
        Ok(format!("{}-{}.{ext}", self.provider.as_str(), self.id))
    }

    pub fn is_allowed_download_url(&self) -> bool {
        self.provider
            .download_prefixes()
            .iter()
            .any(|p| self.full.starts_with(p))
            && !self
                .full
                .bytes()
                .any(|b| b.is_ascii_whitespace() || b.is_ascii_control())
    }
}

/// Provider ids are short strings of letters, digits, `-` and `_`
/// ("94x38z", "409210", Unsplash's "xg8z_KhSorQ"): safe in a file name
/// and a URL path.
pub fn is_valid_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 32 && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

/// The [`OnlineWallpaper::key`] of a downloaded file, from its name, so the
/// download folder tells which results are already on disk.
pub fn key_from_file_name(name: &str) -> Option<String> {
    let (stem, ext) = name.rsplit_once('.')?;
    if !["jpg", "png", "webp"].contains(&ext) {
        return None;
    }
    let (provider, id) = stem.split_once('-')?;
    let provider = Provider::from_str(provider)?;
    is_valid_id(id).then(|| format!("{}:{id}", provider.as_str()))
}

// ------------------------------------------------------------- wallhaven ----

impl From<wallhaven::Wallpaper> for OnlineWallpaper {
    fn from(w: wallhaven::Wallpaper) -> OnlineWallpaper {
        let thumb = if w.thumbs.large.is_empty() {
            w.thumbs.small.clone()
        } else {
            w.thumbs.large.clone()
        };
        OnlineWallpaper {
            provider: Provider::Wallhaven,
            extension: w.extension().to_string(),
            page_url: w.url,
            preview: thumb.clone(),
            thumb,
            full: w.path,
            width: w.dimension_x,
            height: w.dimension_y,
            file_size: w.file_size,
            purity: w.purity,
            category: w.category,
            likes: w.favorites,
            views: Some(w.views),
            id: w.id,
            details: None,
        }
    }
}

impl From<wallhaven::SearchResponse> for Page {
    fn from(r: wallhaven::SearchResponse) -> Page {
        Page {
            items: r.data.into_iter().map(OnlineWallpaper::from).collect(),
            page: r.meta.current_page.max(1),
            last_page: r.meta.last_page.max(1),
            total: r.meta.total,
            seed: r.meta.seed,
        }
    }
}

impl From<wallhaven::WallpaperInfo> for Details {
    fn from(info: wallhaven::WallpaperInfo) -> Details {
        Details {
            uploader: info.uploader.map(|u| Person {
                query: wallhaven::uploader_query(&u.username),
                name: u.username,
            }),
            source: info.source,
            tags: info
                .tags
                .into_iter()
                .map(|t| OnlineTag {
                    query: wallhaven::tag_query(t.id),
                    is_artist: t.is_artist(),
                    name: t.name,
                    category: t.category,
                })
                .collect(),
        }
    }
}

#[cfg(test)]
mod details_image_tests {
    use super::*;

    #[test]
    fn wallhaven_shows_its_original_unless_huge() {
        let w = OnlineWallpaper {
            provider: Provider::Wallhaven,
            thumb: "small".into(),
            preview: "lg".into(),
            full: "full".into(),
            file_size: 6_000_000,
            ..OnlineWallpaper::default()
        };
        assert_eq!(w.details_image(), "full");
        let huge = OnlineWallpaper { file_size: MAX_ORIGINAL_PREVIEW_BYTES + 1, ..w.clone() };
        assert_eq!(huge.details_image(), "lg");
        let unknown = OnlineWallpaper { file_size: 0, ..w.clone() };
        assert_eq!(unknown.details_image(), "lg");
        // Moebooru: the sample.
        let k = OnlineWallpaper { provider: Provider::Konachan, ..w.clone() };
        assert_eq!(k.details_image(), "lg");
        let bare = OnlineWallpaper { provider: Provider::Yandere, preview: String::new(), ..w };
        assert_eq!(bare.details_image(), "small");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wallpaper(provider: Provider, id: &str, full: &str) -> OnlineWallpaper {
        OnlineWallpaper {
            provider,
            id: id.into(),
            full: full.into(),
            extension: "png".into(),
            ..OnlineWallpaper::default()
        }
    }

    #[test]
    fn file_names_round_trip_through_keys() {
        let w = wallpaper(Provider::Konachan, "409210", "");
        assert_eq!(w.file_name().unwrap(), "konachan-409210.png");
        assert_eq!(key_from_file_name(&w.file_name().unwrap()), Some(w.key()));
        // Downloads from before there were providers keep working.
        assert_eq!(
            key_from_file_name("wallhaven-94x38z.jpg").as_deref(),
            Some("wallhaven:94x38z")
        );
        for bad in ["wallhaven-abc.part", "other-abc.jpg", "konachan-.jpg", "konachan-a b.jpg", "x.jpg"] {
            assert_eq!(key_from_file_name(bad), None, "{bad}");
        }
        for bad in ["", "../etc/passwd", "a/b", "a.b", "x y", "ü"] {
            assert!(!is_valid_id(bad), "{bad:?}");
            assert!(wallpaper(Provider::Konachan, bad, "").file_name().is_err(), "{bad:?}");
        }
    }

    #[test]
    fn downloads_only_from_each_providers_image_host() {
        let ok = [
            (Provider::Wallhaven, "https://w.wallhaven.cc/full/94/wallhaven-94x38z.jpg"),
            (Provider::Konachan, "https://konachan.com/image/abc/Konachan.com%20-%201.png"),
            (Provider::Konachan, "https://konachan.net/jpeg/abc/x.jpg"),
            (Provider::Konachan, "https://konachan.net/image/abc/x.png"),
            (Provider::Yandere, "https://files.yande.re/image/abc/yande.re%201.png"),
            (Provider::Yandere, "https://files.yande.re/jpeg/abc/x.jpg"),
        ];
        for (p, url) in ok {
            assert!(wallpaper(p, "1", url).is_allowed_download_url(), "{url}");
        }
        let bad = [
            (Provider::Wallhaven, "https://konachan.net/image/abc/x.png"),
            (Provider::Wallhaven, "http://w.wallhaven.cc/full/94/x.jpg"),
            (Provider::Wallhaven, "https://evil.example/w.wallhaven.cc/x.jpg"),
            (Provider::Wallhaven, "https://w.wallhaven.cc.evil.example/x.jpg"),
            (Provider::Wallhaven, "https://w.wallhaven.cc@evil.example/x.jpg"),
            (Provider::Wallhaven, "https://w.wallhaven.cc/x.jpg\r\nHost: evil"),
            (Provider::Wallhaven, "file:///etc/passwd"),
            (Provider::Wallhaven, ""),
            (Provider::Konachan, "https://konachan.net.evil.example/image/x.png"),
            (Provider::Konachan, "https://w.wallhaven.cc/full/x.jpg"),
            (Provider::Konachan, "https://konachan.net/post/show/1"),
            (Provider::Yandere, "https://konachan.com/image/abc/x.png"),
            (Provider::Yandere, "https://assets.yande.re/data/preview/a.jpg"),
            (Provider::Yandere, "https://files.yande.re.evil.example/image/x.png"),
            (Provider::Konachan, "https://konachan.net/image/a b.png"),
        ];
        for (p, url) in bad {
            assert!(!wallpaper(p, "1", url).is_allowed_download_url(), "{url}");
        }
    }

    #[test]
    fn queries_are_tagged_by_provider() {
        let q = OnlineQuery::Konachan(MoebooruQuery::default());
        let json = serde_json::to_string(&q).unwrap();
        assert!(json.contains(r#""provider":"konachan""#));
        assert_eq!(serde_json::from_str::<OnlineQuery>(&json).unwrap(), q);
        assert_eq!(q.at_page(3, None).provider(), Provider::Konachan);
    }

    #[test]
    fn wallhaven_results_and_details_convert() {
        let w = wallhaven::Wallpaper {
            id: "94x38z".into(),
            favorites: 7,
            dimension_x: 1920,
            dimension_y: 1080,
            file_type: "image/png".into(),
            path: "https://w.wallhaven.cc/full/94/wallhaven-94x38z.png".into(),
            ..wallhaven::Wallpaper::default()
        };
        let o = OnlineWallpaper::from(w);
        assert_eq!(o.key(), "wallhaven:94x38z");
        assert_eq!(o.resolution(), "1920x1080");
        assert_eq!(o.extension, "png");
        assert_eq!(o.likes, 7);
        assert!(o.is_allowed_download_url());

        let info = wallhaven::parse_info_response(
            r#"{"data": {"id": "x", "uploader": {"username": "someone"},
                "tags": [{"id": 42, "name": "Mitsu", "category": "Artists"}]}}"#,
        )
        .unwrap();
        let d = Details::from(info);
        assert_eq!(d.uploader.unwrap().query.as_deref(), Some("@someone"));
        assert_eq!(d.tags[0].query, "id:42");
        assert!(d.tags[0].is_artist);
    }
}
