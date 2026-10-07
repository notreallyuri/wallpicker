//! Unsplash (unsplash.com): photography. A free Access Key is needed (sent as
//! `Authorization: Client-ID <key>`; 50 requests an hour for a new key).
//! With search text it's Unsplash's search; without, the Wallpapers topic;
//! `user:<name>` is that photographer's photos. Its guidelines ask for the
//! photographer and Unsplash to be credited (with links) and for each
//! download to be reported (`download_url`). No networking here.

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::online::{Details, OnlineTag, OnlineWallpaper, Page, Person, Provider};

pub const API: &str = "https://api.unsplash.com";
/// Results per page (Unsplash allows up to 30).
pub const PER_PAGE: u32 = 24;
/// Added to links to Unsplash, as its guidelines ask.
const UTM: &str = "utm_source=wallpicker&utm_medium=referral";

/// Order of the results: by relevance to the search (the topic's most
/// popular without one), or the newest first.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Order {
    #[default]
    Relevant,
    Latest,
}

/// The filters as a search uses them; `query` is the search text.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct UnsplashQuery {
    pub query: String,
    pub order: Order,
    /// "landscape", "portrait", "squarish"; "" any.
    pub orientation: String,
    /// One of [`COLORS`]; "" any.
    pub color: String,
    /// "low" (Unsplash's default) or "high" (stricter).
    pub content_filter: String,
    pub page: u32,
}

impl Default for UnsplashQuery {
    fn default() -> Self {
        UnsplashQuery {
            query: String::new(),
            order: Order::Relevant,
            orientation: "landscape".into(),
            color: String::new(),
            content_filter: "low".into(),
            page: 1,
        }
    }
}

pub const ORDERS: [(&str, &str); 2] = [("relevant", "Relevant (popular without a search)"), ("latest", "Latest")];
pub const ORIENTATIONS: [(&str, &str); 4] = [("", "Any"), ("landscape", "Landscape"), ("portrait", "Portrait"), ("squarish", "Square")];
pub const COLORS: [(&str, &str); 12] = [
    ("", "Any color"),
    ("black_and_white", "Black and white"),
    ("black", "Black"),
    ("white", "White"),
    ("yellow", "Yellow"),
    ("orange", "Orange"),
    ("red", "Red"),
    ("purple", "Purple"),
    ("magenta", "Magenta"),
    ("green", "Green"),
    ("teal", "Teal"),
    ("blue", "Blue"),
];
pub const CONTENT_FILTERS: [(&str, &str); 2] = [("low", "Unsplash's default"), ("high", "Stricter")];

impl UnsplashQuery {
    /// The filters worth remembering: no search text, page 1.
    pub fn filters_only(mut self) -> UnsplashQuery {
        self.query.clear();
        self.page = 1;
        self
    }

    /// How many filters differ from the defaults (the Filters button).
    pub fn active_filters(&self) -> usize {
        let d = UnsplashQuery::default();
        [self.order != d.order, self.orientation != d.orientation, !self.color.is_empty(), self.content_filter != d.content_filter]
            .into_iter()
            .filter(|on| *on)
            .count()
    }

    /// Where to ask, and with what: (URL, query parameters). A known value
    /// only goes out (a color or orientation Unsplash doesn't know is left
    /// out rather than sent).
    pub fn request(&self) -> (String, Vec<(&'static str, String)>) {
        let known = |list: &[(&str, &str)], v: &str| !v.is_empty() && list.iter().any(|(k, _)| *k == v);
        let mut params = vec![("page", self.page.max(1).to_string()), ("per_page", PER_PAGE.to_string())];
        if known(&ORIENTATIONS, &self.orientation) {
            params.push(("orientation", self.orientation.clone()));
        }
        let q = self.query.trim();
        if let Some(user) = q.strip_prefix("user:").map(str::trim).filter(|u| is_valid_username(u)) {
            params.push(("order_by", if self.order == Order::Latest { "latest" } else { "popular" }.into()));
            return (format!("{API}/users/{user}/photos"), params);
        }
        if q.is_empty() {
            params.push(("order_by", if self.order == Order::Latest { "latest" } else { "popular" }.into()));
            return (format!("{API}/topics/wallpapers/photos"), params);
        }
        params.push(("query", q.to_string()));
        params.push(("order_by", if self.order == Order::Latest { "latest" } else { "relevant" }.into()));
        if known(&COLORS, &self.color) {
            params.push(("color", self.color.clone()));
        }
        if known(&CONTENT_FILTERS, &self.content_filter) {
            params.push(("content_filter", self.content_filter.clone()));
        }
        (format!("{API}/search/photos"), params)
    }
}

/// Unsplash usernames: letters, digits and underscores.
fn is_valid_username(name: &str) -> bool {
    !name.is_empty() && name.len() <= 64 && name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
}

/// The API's address that reports a download of `id`, as the guidelines
/// ask (with the Access Key).
pub fn download_url(id: &str) -> String {
    format!("{API}/photos/{id}/download")
}

fn with_utm(url: &str) -> String {
    let joiner = if url.contains('?') { '&' } else { '?' };
    format!("{url}{joiner}{UTM}")
}

/// Unsplash's own page, credited beside the photographer.
pub fn home_url() -> String {
    with_utm("https://unsplash.com/")
}

#[derive(Deserialize)]
struct Photo {
    id: String,
    #[serde(default)]
    width: u32,
    #[serde(default)]
    height: u32,
    #[serde(default)]
    likes: u64,
    urls: Urls,
    links: Links,
    user: User,
    #[serde(default)]
    tags: Vec<Tag>,
}

#[derive(Deserialize)]
struct Urls {
    full: String,
    regular: String,
    small: String,
}

#[derive(Deserialize)]
struct Links {
    html: String,
}

#[derive(Deserialize)]
struct User {
    name: String,
    username: String,
    links: Links,
}

#[derive(Deserialize)]
struct Tag {
    title: String,
}

#[derive(Deserialize)]
struct Search {
    #[serde(default)]
    total: u64,
    #[serde(default)]
    total_pages: u64,
    results: Vec<Photo>,
}

impl From<Photo> for OnlineWallpaper {
    fn from(p: Photo) -> OnlineWallpaper {
        let tags = p.tags.into_iter().map(|t| OnlineTag { query: t.title.clone(), name: t.title, category: String::new(), is_artist: false }).collect();
        OnlineWallpaper {
            provider: Provider::Unsplash,
            page_url: with_utm(&p.links.html),
            thumb: p.urls.small,
            preview: p.urls.regular,
            full: p.urls.full,
            width: p.width,
            height: p.height,
            file_size: 0,
            extension: "jpg".into(),
            purity: "sfw".into(),
            category: "photo".into(),
            likes: p.likes,
            views: None,
            details: Some(Details {
                // The photographer ("Photo by"; "More from" lists their photos).
                uploader: Some(Person { name: p.user.name, query: Some(format!("user:{}", p.user.username)) }),
                // The photographer's page, credited with Unsplash.
                source: with_utm(&p.user.links.html),
                tags,
            }),
            id: p.id,
        }
    }
}

/// A search's answer (`/search/photos`).
pub fn parse_search(json: &str, page: u32) -> Result<Page> {
    let s: Search = serde_json::from_str(json).map_err(|e| Error::Invalid(format!("Unsplash's answer: {e}")))?;
    Ok(Page { items: s.results.into_iter().map(Into::into).collect(), page: u64::from(page.max(1)), last_page: s.total_pages.max(1), total: s.total, seed: None })
}

/// A list's answer (the topic, a photographer): `total` is the X-Total
/// header, when it came.
pub fn parse_list(json: &str, page: u32, total: Option<u64>) -> Result<Page> {
    let photos: Vec<Photo> = serde_json::from_str(json).map_err(|e| Error::Invalid(format!("Unsplash's answer: {e}")))?;
    let page = u64::from(page.max(1));
    let full = photos.len() as u64 >= u64::from(PER_PAGE);
    let last_page = match total {
        Some(t) => t.div_ceil(u64::from(PER_PAGE)).max(1),
        // Unknown: there's a next page as long as this one is full.
        None => page + u64::from(full),
    };
    Ok(Page { items: photos.into_iter().map(Into::into).collect(), page, last_page, total: total.unwrap_or(0), seed: None })
}

#[cfg(test)]
mod tests {
    use super::*;

    const PHOTO: &str = r#"{"id":"Dwu85P9SOIk","width":6000,"height":4000,"likes":250,
        "urls":{"raw":"https://images.unsplash.com/photo-1?ixid=x","full":"https://images.unsplash.com/photo-1?ixid=x&fm=jpg&q=85",
                "regular":"https://images.unsplash.com/photo-1?w=1080","small":"https://images.unsplash.com/photo-1?w=400","thumb":"t"},
        "links":{"html":"https://unsplash.com/photos/Dwu85P9SOIk","download_location":"https://api.unsplash.com/photos/Dwu85P9SOIk/download?ixid=x"},
        "user":{"name":"Ann Smith","username":"ann_smith","links":{"html":"https://unsplash.com/@ann_smith"}},
        "tags":[{"type":"search","title":"mountain"},{"type":"search","title":"sky"}]}"#;

    #[test]
    fn a_search_becomes_a_page_with_the_photographer_credited() {
        let page = parse_search(&format!(r#"{{"total":120,"total_pages":5,"results":[{PHOTO}]}}"#), 2).unwrap();
        assert_eq!((page.page, page.last_page, page.total), (2, 5, 120));
        let w = &page.items[0];
        assert_eq!(w.provider, Provider::Unsplash);
        assert_eq!(w.key(), "unsplash:Dwu85P9SOIk");
        assert_eq!(w.full, "https://images.unsplash.com/photo-1?ixid=x&fm=jpg&q=85");
        assert_eq!(w.thumb, "https://images.unsplash.com/photo-1?w=400");
        assert_eq!(w.page_url, "https://unsplash.com/photos/Dwu85P9SOIk?utm_source=wallpicker&utm_medium=referral");
        assert!(w.is_allowed_download_url());
        let d = w.details.as_ref().unwrap();
        assert_eq!(d.uploader.as_ref().unwrap().query.as_deref(), Some("user:ann_smith"));
        assert_eq!(d.source, "https://unsplash.com/@ann_smith?utm_source=wallpicker&utm_medium=referral");
        assert_eq!(d.uploader.as_ref().unwrap().name, "Ann Smith");
        assert_eq!(d.tags.iter().map(|t| t.name.as_str()).collect::<Vec<_>>(), ["mountain", "sky"]);
        assert_eq!(w.file_name().unwrap(), "unsplash-Dwu85P9SOIk.jpg");
    }

    #[test]
    fn a_list_knows_its_last_page_from_the_total_or_how_full_it_is() {
        let one = format!("[{PHOTO}]");
        assert_eq!(parse_list(&one, 1, Some(100)).unwrap().last_page, 5);
        assert_eq!(parse_list(&one, 3, None).unwrap().last_page, 3);
        let full = format!("[{}]", vec![PHOTO; 24].join(","));
        assert_eq!(parse_list(&full, 3, None).unwrap().last_page, 4);
    }

    #[test]
    fn requests_go_where_the_query_says() {
        let topic = UnsplashQuery::default().request();
        assert_eq!(topic.0, "https://api.unsplash.com/topics/wallpapers/photos");
        assert!(topic.1.contains(&("order_by", "popular".into())));
        assert!(topic.1.contains(&("orientation", "landscape".into())));

        let q = UnsplashQuery { query: "forest".into(), color: "green".into(), order: Order::Latest, ..Default::default() };
        let (url, params) = q.request();
        assert_eq!(url, "https://api.unsplash.com/search/photos");
        for p in [("query", "forest"), ("color", "green"), ("order_by", "latest"), ("content_filter", "low")] {
            assert!(params.contains(&(p.0, p.1.into())), "{p:?}");
        }

        let user = UnsplashQuery { query: "user:ann_smith".into(), ..Default::default() }.request();
        assert_eq!(user.0, "https://api.unsplash.com/users/ann_smith/photos");

        // Unknown values aren't sent; a bad username is just a search.
        let odd = UnsplashQuery { query: "user:../x".into(), color: "plaid".into(), orientation: "round".into(), ..Default::default() }.request();
        assert_eq!(odd.0, "https://api.unsplash.com/search/photos");
        assert!(!odd.1.iter().any(|(k, _)| *k == "color" || *k == "orientation"));
    }

    #[test]
    fn filters_count_what_differs_from_the_defaults() {
        assert_eq!(UnsplashQuery::default().active_filters(), 0);
        let q = UnsplashQuery { orientation: String::new(), color: "blue".into(), ..Default::default() };
        assert_eq!(q.active_filters(), 2);
        assert_eq!(UnsplashQuery { query: "x".into(), page: 4, ..Default::default() }.filters_only(), UnsplashQuery::default());
    }
}
