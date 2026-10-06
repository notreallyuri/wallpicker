use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// How hyprpaper should fit an image to the monitor.
///
/// This is passed as a third argument to `hyprctl hyprpaper wallpaper`, which
/// only newer hyprpaper versions understand. Leave it unset to use whatever
/// default your hyprpaper has.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FitMode {
    Cover,
    Contain,
    Tile,
    Fill,
}

impl FitMode {
    pub fn as_str(self) -> &'static str {
        match self {
            FitMode::Cover => "cover",
            FitMode::Contain => "contain",
            FitMode::Tile => "tile",
            FitMode::Fill => "fill",
        }
    }
}

/// What draws the wallpaper. wallpicker applies through it and records what
/// it applied either way.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WallpaperBackend {
    /// hyprpaper, through `hyprctl hyprpaper`. Fit mode is wallpicker's.
    #[default]
    Hyprpaper,
    /// Noctalia's wallpaper manager, through `noctalia msg wallpaper-set`.
    /// It keeps the wallpapers itself; fill mode and transitions are set in
    /// Noctalia's settings.
    Noctalia,
    /// awww (swww before it was renamed), through `awww img`, with
    /// wallpicker's fit and a transition ([`Transition`]). Its daemon keeps
    /// the wallpapers across restarts; wallpicker starts it when needed.
    #[serde(alias = "swww")]
    Awww,
}

impl WallpaperBackend {
    pub fn as_str(self) -> &'static str {
        match self {
            WallpaperBackend::Hyprpaper => "hyprpaper",
            WallpaperBackend::Noctalia => "noctalia",
            WallpaperBackend::Awww => "awww",
        }
    }
}

/// Which Konachan to use: the safe-for-work mirror or the full site. Same
/// posts and ids; only the ratings differ.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum KonachanSite {
    /// konachan.net: safe-for-work only.
    #[default]
    Net,
    /// konachan.com: all ratings.
    Com,
}

impl KonachanSite {
    pub fn as_str(self) -> &'static str {
        match self {
            KonachanSite::Net => "net",
            KonachanSite::Com => "com",
        }
    }
}

/// User-editable configuration, stored in `config.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Folders scanned for local wallpapers.
    pub folders: Vec<PathBuf>,
    /// Where Wallhaven downloads are saved.
    pub download_dir: PathBuf,
    /// Scan folders recursively.
    pub recursive: bool,
    /// Optional Wallhaven API key (needed for NSFW results and your account
    /// settings). Sent as a header, never in the URL.
    pub wallhaven_api_key: Option<String>,
    /// Your Wallhaven username: opening your collections needs it (the API
    /// doesn't say whose key it is).
    pub wallhaven_username: Option<String>,
    pub wallpaper_backend: WallpaperBackend,
    /// The fit for every monitor (None: the backend's default).
    pub fit_mode: Option<FitMode>,
    /// A monitor's own fit (by name), over `fit_mode`. A wallpaper's own fit
    /// (in the catalog) wins over both; see [`Settings::fit_for`].
    pub monitor_fits: BTreeMap<String, FitMode>,
    pub konachan_site: KonachanSite,
    pub slideshow: Slideshow,
    /// Closing the window hides it to the tray (the app keeps running).
    pub close_to_tray: bool,
    /// The app's colors come from the current wallpaper (through Noctalia's
    /// palette generator, when it's installed).
    pub theme_from_wallpaper: bool,
    /// The monitor whose wallpaper the colors come from; None: the first one.
    pub theme_monitor: Option<String>,
    /// awww's transition (the awww backend).
    pub transition: Transition,
    /// Noctalia's color scheme before "follow the wallpaper" was turned on
    /// ("community Garnet"), to go back to.
    pub noctalia_previous_scheme: Option<String>,
    /// Width in pixels of generated thumbnails.
    pub thumb_width: u32,
}

impl Default for Settings {
    fn default() -> Self {
        let home = std::env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."));
        let base = home.join("Pictures").join("Wallpapers");
        Settings {
            folders: vec![base.clone()],
            download_dir: base.join("wallhaven"),
            recursive: true,
            wallhaven_api_key: None,
            wallhaven_username: None,
            wallpaper_backend: WallpaperBackend::default(),
            fit_mode: None,
            monitor_fits: BTreeMap::new(),
            konachan_site: KonachanSite::default(),
            slideshow: Slideshow::default(),
            close_to_tray: true,
            theme_from_wallpaper: true,
            theme_monitor: None,
            transition: Transition::default(),
            noctalia_previous_scheme: None,
            thumb_width: 360,
        }
    }
}

impl Settings {
    /// The fit for a wallpaper on `monitor`: the wallpaper's own (`wallpaper`,
    /// from the catalog), else the monitor's, else the general one. None
    /// leaves it to hyprpaper.
    pub fn fit_for(&self, monitor: &str, wallpaper: Option<FitMode>) -> Option<FitMode> {
        wallpaper.or_else(|| self.monitor_fits.get(monitor).copied()).or(self.fit_mode)
    }
}

/// Turn a leading `~` or `~/` into `home`. Other paths are returned unchanged.
pub fn expand_tilde(path: &std::path::Path, home: Option<&std::path::Path>) -> PathBuf {
    let Some(home) = home else {
        return path.to_path_buf();
    };
    match path.strip_prefix("~") {
        Ok(rest) => home.join(rest),
        Err(_) => path.to_path_buf(),
    }
}

impl Settings {
    /// Clamp values that would break the app if edited by hand, and expand `~`.
    pub fn sanitized(self) -> Self {
        let home = std::env::var_os("HOME").map(PathBuf::from);
        self.sanitized_with_home(home.as_deref())
    }

    pub fn sanitized_with_home(mut self, home: Option<&std::path::Path>) -> Self {
        self.folders = self
            .folders
            .iter()
            .map(|p| expand_tilde(p, home))
            .filter(|p| !p.as_os_str().is_empty())
            .collect();
        self.folders.dedup();
        self.download_dir = expand_tilde(&self.download_dir, home);
        self.thumb_width = self.thumb_width.clamp(120, 1024);
        self.wallhaven_api_key = self
            .wallhaven_api_key
            .map(|k| k.trim().to_string())
            .filter(|k| !k.is_empty());
        // Trimmed; one with characters a username can't have is dropped.
        self.wallhaven_username = self
            .wallhaven_username
            .map(|u| u.trim().to_string())
            .filter(|u| crate::wallhaven::is_valid_username(u));
        self.theme_monitor = self.theme_monitor.map(|m| m.trim().to_string()).filter(|m| !m.is_empty());
        self.transition = self.transition.sanitized();
        self
    }
}

/// Fit modes with labels, as `backend` does them: "" is its own default
/// (awww's crops, which is Cover), and awww can't tile (it centers).
pub fn fit_modes(backend: WallpaperBackend) -> [(&'static str, &'static str); 5] {
    let (default, tile) = match backend {
        WallpaperBackend::Awww => ("awww's default (Cover)", "Centered (awww can't tile)"),
        WallpaperBackend::Hyprpaper | WallpaperBackend::Noctalia => ("hyprpaper's default", "Tile"),
    };
    [("", default), ("cover", "Cover"), ("contain", "Contain"), ("tile", tile), ("fill", "Fill")]
}

pub const BACKENDS: [(&str, &str); 3] =
    [("hyprpaper", "hyprpaper"), ("awww", "awww (swww)"), ("noctalia", "Noctalia")];

/// awww's transitions, with labels. ("any" and "center" are grow from a
/// random point and from the middle.)
pub const TRANSITIONS: [(&str, &str); 14] = [
    ("fade", "Fade"),
    ("simple", "Simple fade"),
    ("wipe", "Wipe"),
    ("wave", "Wave"),
    ("grow", "Grow (a circle)"),
    ("center", "Grow from the middle"),
    ("any", "Grow from anywhere"),
    ("outer", "Shrink (a circle)"),
    ("left", "Slide from the left"),
    ("right", "Slide from the right"),
    ("top", "Slide from the top"),
    ("bottom", "Slide from the bottom"),
    ("random", "A different one each time"),
    ("none", "None (instant)"),
];

/// How awww changes to a new wallpaper.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Transition {
    /// One of [`TRANSITIONS`].
    pub kind: String,
    /// How long it takes (awww ignores this for "simple").
    pub duration_ms: u32,
    /// Frames per second (awww allows up to 255).
    pub fps: u32,
}

impl Default for Transition {
    fn default() -> Self {
        Transition { kind: "fade".into(), duration_ms: 1000, fps: 60 }
    }
}

impl Transition {
    /// A known kind, a duration up to 10 s and 1 to 255 fps.
    pub fn sanitized(self) -> Self {
        let kind = if TRANSITIONS.iter().any(|(k, _)| *k == self.kind) { self.kind } else { Transition::default().kind };
        Transition { kind, duration_ms: self.duration_ms.min(10_000), fps: self.fps.clamp(1, 255) }
    }
}

pub const KONACHAN_SITES: [(&str, &str); 2] = [
    ("net", "konachan.net (safe-for-work only)"),
    ("com", "konachan.com (all ratings)"),
];

/// What is currently applied, so it can be restored after a reboot.
/// Stored in `state.json`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct State {
    /// Monitor name (e.g. "DP-1") -> wallpaper path.
    pub current: BTreeMap<String, PathBuf>,
    /// When the slideshow last changed the wallpaper (seconds since the Unix
    /// epoch; 0: never).
    pub slideshow_changed: u64,
    /// What each monitor showed before, oldest first (see
    /// [`HISTORY_LEN`]): `wallpicker previous` goes back through it.
    pub history: BTreeMap<String, Vec<PathBuf>>,
}

/// Wallpapers remembered per monitor for going back.
pub const HISTORY_LEN: usize = 30;

/// Change the wallpaper every few minutes, picking at random like
/// `wallpicker random`. Something has to call `wallpicker slideshow tick`
/// (the Noctalia plugin does, every minute) or run `wallpicker slideshow run`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Slideshow {
    pub enabled: bool,
    /// Minutes between changes (at least 1).
    pub minutes: u32,
    /// Only favorites.
    pub favorites: bool,
    /// Only wallpapers with these tags: all of them, or (`any`) any one.
    pub tags: Vec<String>,
    pub any: bool,
    /// Only wallpapers inside this folder.
    pub folder: Option<PathBuf>,
    /// The monitors to change; empty: all of them.
    pub monitors: Vec<String>,
    /// A different wallpaper on each monitor (else the same one on all).
    pub each: bool,
}

impl Default for Slideshow {
    fn default() -> Self {
        Slideshow {
            enabled: false,
            minutes: 30,
            favorites: false,
            tags: Vec::new(),
            any: false,
            folder: None,
            monitors: Vec::new(),
            each: true,
        }
    }
}

impl Slideshow {
    /// Seconds until the next change (0: now), `now` and the last change
    /// in seconds since the epoch. A last change in the future (the clock
    /// went back) counts as long ago.
    pub fn due_in(&self, now: u64, last: u64) -> u64 {
        let every = u64::from(self.minutes.max(1)) * 60;
        if last > now {
            return 0;
        }
        every.saturating_sub(now - last)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_fields_fall_back_to_defaults() {
        let s: Settings = serde_json::from_str(r#"{"recursive": false}"#).unwrap();
        assert!(!s.recursive);
        assert_eq!(s.thumb_width, 360);
        assert!(s.wallhaven_api_key.is_none());
    }

    #[test]
    fn sanitize_clamps_and_trims() {
        let s = Settings {
            thumb_width: 5,
            wallhaven_api_key: Some("   ".into()),
            ..Settings::default()
        }
        .sanitized();
        assert_eq!(s.thumb_width, 120);
        assert_eq!(s.wallhaven_api_key, None);

        let s = Settings {
            thumb_width: 99_999,
            wallhaven_api_key: Some("  abc  ".into()),
            ..Settings::default()
        }
        .sanitized();
        assert_eq!(s.thumb_width, 1024);
        assert_eq!(s.wallhaven_api_key.as_deref(), Some("abc"));
    }

    #[test]
    fn tilde_is_expanded() {
        let home = std::path::Path::new("/home/u");
        let s = Settings {
            folders: vec![
                PathBuf::from("~/Pictures"),
                PathBuf::from("/abs"),
                PathBuf::from("~notme/x"),
            ],
            download_dir: PathBuf::from("~"),
            ..Settings::default()
        }
        .sanitized_with_home(Some(home));
        assert_eq!(
            s.folders,
            vec![
                PathBuf::from("/home/u/Pictures"),
                PathBuf::from("/abs"),
                PathBuf::from("~notme/x"),
            ]
        );
        assert_eq!(s.download_dir, PathBuf::from("/home/u"));

        // No HOME: leave paths alone.
        assert_eq!(
            expand_tilde(std::path::Path::new("~/a"), None),
            PathBuf::from("~/a")
        );
    }

    #[test]
    fn slideshow_timing() {
        let s = Slideshow { minutes: 10, ..Slideshow::default() };
        assert_eq!(s.due_in(1_000, 0), 0, "never changed: now");
        assert_eq!(s.due_in(1_000, 1_000), 600);
        assert_eq!(s.due_in(1_000 + 599, 1_000), 1);
        assert_eq!(s.due_in(1_000 + 600, 1_000), 0);
        assert_eq!(s.due_in(500, 1_000), 0, "clock went back");
        assert_eq!(Slideshow { minutes: 0, ..s }.due_in(60, 0), 0);
        // Old config files (no slideshow) load with it off.
        let old: Settings = serde_json::from_str("{}").unwrap();
        assert!(!old.slideshow.enabled);
        assert_eq!(old.slideshow.minutes, 30);
    }

    #[test]
    fn fit_comes_from_the_wallpaper_then_the_monitor_then_the_default() {
        let mut s = Settings { fit_mode: Some(FitMode::Cover), ..Settings::default() };
        s.monitor_fits.insert("DP-3".into(), FitMode::Contain);
        assert_eq!(s.fit_for("DP-1", None), Some(FitMode::Cover));
        assert_eq!(s.fit_for("DP-3", None), Some(FitMode::Contain));
        assert_eq!(s.fit_for("DP-3", Some(FitMode::Tile)), Some(FitMode::Tile));
        assert_eq!(Settings::default().fit_for("DP-1", None), None);
        // Old config files (no monitor_fits) still load.
        let old: Settings = serde_json::from_str(r#"{"fit_mode": "fill"}"#).unwrap();
        assert!(old.monitor_fits.is_empty());
        assert_eq!(old.fit_for("DP-2", None), Some(FitMode::Fill));
    }

    #[test]
    fn fit_mode_round_trips_lowercase() {
        assert_eq!(serde_json::to_string(&FitMode::Cover).unwrap(), "\"cover\"");
        let m: FitMode = serde_json::from_str("\"contain\"").unwrap();
        assert_eq!(m.as_str(), "contain");
    }
}
