use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

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
    /// A monitor's own fit (by name), over `fit_mode`. An adjusted fit (the
    /// Adjust dialog, until the wallpaper changes) wins over both; see
    /// [`Settings::fit_for`].
    pub monitor_fits: BTreeMap<String, FitMode>,
    pub konachan_site: KonachanSite,
    pub slideshow: Slideshow,
    /// A wallpaper per Hyprland workspace.
    pub workspaces: Workspaces,
    /// Closing the window hides it to the tray (the app keeps running).
    pub close_to_tray: bool,
    /// The app's colors come from the current wallpaper (through Noctalia's
    /// palette generator, when it's installed).
    pub theme_from_wallpaper: bool,
    /// The monitor whose wallpaper the colors come from; None: the first one.
    pub theme_monitor: Option<String>,
    /// awww's transition (the awww backend).
    pub transition: Transition,
    /// Online details show the original image (else one fitted to 1600 px,
    /// which costs a fraction of the memory: an 8K original decoded is 130 MB).
    pub full_previews: bool,
    /// Where the library's daily snapshots go (None: wallpicker's data
    /// folder). A synced folder makes them off-machine backups.
    pub snapshot_dir: Option<PathBuf>,
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
            workspaces: Workspaces::default(),
            close_to_tray: true,
            theme_from_wallpaper: true,
            theme_monitor: None,
            transition: Transition::default(),
            snapshot_dir: None,
            full_previews: false,
            noctalia_previous_scheme: None,
            thumb_width: 360,
        }
    }
}

impl Settings {
    /// The fit on `monitor`: its adjusted one (`adjusted`, until the
    /// wallpaper changes), else the monitor's, else the general one. None
    /// leaves it to the backend.
    pub fn fit_for(&self, monitor: &str, adjusted: Option<FitMode>) -> Option<FitMode> {
        adjusted.or_else(|| self.monitor_fits.get(monitor).copied()).or(self.fit_mode)
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
        self.snapshot_dir = self
            .snapshot_dir
            .map(|p| expand_tilde(&p, home))
            .filter(|p| !p.as_os_str().is_empty());
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
        self.slideshow.color = known_color(self.slideshow.color.take());
        self.slideshow.night = std::mem::take(&mut self.slideshow.night).sanitized();
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
    /// "day" or "night": which set the slideshow last picked from, to change
    /// right away when that turns ("" before the first time).
    pub slideshow_period: String,
    /// What each monitor showed before, oldest first (see
    /// [`HISTORY_LEN`]): `wallpicker previous` goes back through it.
    pub history: BTreeMap<String, Vec<PathBuf>>,
    /// How each monitor's current wallpaper is adjusted (mirrored...).
    /// Temporary: another wallpaper on that monitor clears it.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub adjust: BTreeMap<String, Adjust>,
}

impl State {
    /// `monitor`'s adjustments, when it still shows `path`.
    pub fn adjust_for(&self, monitor: &str, path: &Path) -> Adjust {
        match self.current.get(monitor) {
            Some(p) if p == path => self.adjust.get(monitor).copied().unwrap_or_default(),
            _ => Adjust::default(),
        }
    }
}

/// Mirroring: left and right swapped (`h`), top and bottom (`v`), or both.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(default)]
pub struct Flip {
    pub h: bool,
    pub v: bool,
}

impl Flip {
    pub fn is_none(&self) -> bool {
        !self.h && !self.v
    }

    /// "h", "v", "hv" or "" (file names, the command line).
    pub fn as_str(&self) -> &'static str {
        match (self.h, self.v) {
            (true, true) => "hv",
            (true, false) => "h",
            (false, true) => "v",
            (false, false) => "",
        }
    }
}

/// A monitor's adjustments to the wallpaper it shows (Adjust dialog). No
/// backend can do these: wallpicker makes the changed copy (`transform`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Adjust {
    pub flip: Flip,
    /// Its own fit on this monitor, over the monitor's and the general one.
    /// Not used while it's framed (the frame is the monitor's exact size).
    pub fit: Option<FitMode>,
    /// The part of the picture shown, cut to the monitor's size.
    pub frame: Option<Frame>,
    /// Its part of a picture spanned across the monitors (`span.rs`): wins
    /// over the frame and the fit.
    pub span: Option<Cut>,
}

impl Adjust {
    pub fn is_none(&self) -> bool {
        self.flip.is_none() && self.fit.is_none() && self.frame.is_none() && self.span.is_none()
    }
}

/// An exact part of a picture (`x`, `y`, `width`×`height` in its pixels),
/// scaled to `out_width`×`out_height` (the monitor's resolution).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cut {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub out_width: u32,
    pub out_height: u32,
}

/// A frame on a picture: the largest area in the monitor's proportions that
/// fits in it (as Cover would show), made `zoom`% smaller around a center
/// (`x`, `y`, ‰ of the picture's width and height), and scaled to the
/// monitor's resolution (`width`×`height`). Integers, so it stays Eq/Copy
/// and names a cached file exactly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Frame {
    /// 100 (the whole Cover area) to 400 (a quarter of its width).
    pub zoom: u16,
    pub x: u16,
    pub y: u16,
    pub width: u32,
    pub height: u32,
}

impl Frame {
    /// The area cut from a `w`×`h` picture: (left, top, width, height) in
    /// pixels, kept inside the picture.
    pub fn rect(&self, w: u32, h: u32) -> (u32, u32, u32, u32) {
        let (w, h) = (f64::from(w.max(1)), f64::from(h.max(1)));
        let aspect = f64::from(self.width.max(1)) / f64::from(self.height.max(1));
        // Zoom 100: the widest area with the monitor's shape.
        let (mut cw, mut ch) = if w / h > aspect { (h * aspect, h) } else { (w, w / aspect) };
        let zoom = f64::from(self.zoom.clamp(100, 400)) / 100.0;
        cw /= zoom;
        ch /= zoom;
        let cx = f64::from(self.x.min(1000)) / 1000.0 * w;
        let cy = f64::from(self.y.min(1000)) / 1000.0 * h;
        let left = (cx - cw / 2.0).clamp(0.0, w - cw);
        let top = (cy - ch / 2.0).clamp(0.0, h - ch);
        (left.round() as u32, top.round() as u32, cw.round().max(1.0) as u32, ch.round().max(1.0) as u32)
    }

    /// Valid values: zoom in range, center in the picture, a real size.
    pub fn sanitized(self) -> Option<Frame> {
        (self.width > 0 && self.height > 0 && self.width <= 16384 && self.height <= 16384).then_some(Frame {
            zoom: self.zoom.clamp(100, 400),
            x: self.x.min(1000),
            y: self.y.min(1000),
            ..self
        })
    }
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
    /// Only wallpapers with this color (a hue, "dark", "light" or "muted";
    /// see `colors`).
    pub color: Option<String>,
    /// Another set for the night (its own picks, in a time window).
    pub night: NightSet,
}

/// A wallpaper per workspace: when a monitor switches to a workspace that
/// has one, it goes on that monitor. `remember`: a wallpaper put on a monitor
/// by hand becomes the wallpaper of the workspace it's showing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Workspaces {
    pub enabled: bool,
    pub remember: bool,
    /// Workspace name ("3", "web") -> wallpaper.
    pub wallpapers: BTreeMap<String, PathBuf>,
}

impl Default for Workspaces {
    fn default() -> Self {
        Workspaces { enabled: false, remember: true, wallpapers: BTreeMap::new() }
    }
}

/// The slideshow's night: between `from` and `to` (local time, "HH:MM"; the
/// window may cross midnight) it picks from these instead, and it changes
/// right away when day turns to night and back.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct NightSet {
    pub enabled: bool,
    pub from: String,
    pub to: String,
    pub favorites: bool,
    pub tags: Vec<String>,
    pub any: bool,
    pub folder: Option<PathBuf>,
    pub color: Option<String>,
}

impl Default for NightSet {
    fn default() -> Self {
        NightSet {
            enabled: false,
            from: "20:00".into(),
            to: "07:00".into(),
            favorites: false,
            tags: Vec::new(),
            any: false,
            folder: None,
            color: Some("dark".into()),
        }
    }
}

/// Minutes since midnight of "HH:MM" (or "H:MM").
pub fn parse_hhmm(text: &str) -> Option<u32> {
    let (h, m) = text.trim().split_once(':')?;
    let (h, m): (u32, u32) = (h.parse().ok()?, m.parse().ok()?);
    (h < 24 && m < 60 && text.trim().len() <= 5).then_some(h * 60 + m)
}

impl NightSet {
    /// Whether `minute` (since local midnight) is in the night. Off, or a
    /// window that starts and ends at the same time, is never night.
    pub fn is_night(&self, minute: u32) -> bool {
        let (Some(from), Some(to)) = (parse_hhmm(&self.from), parse_hhmm(&self.to)) else { return false };
        if !self.enabled || from == to {
            return false;
        }
        if from < to { (from..to).contains(&minute) } else { minute >= from || minute < to }
    }

    /// Times that parse, and a known color, or the defaults.
    pub fn sanitized(mut self) -> Self {
        let d = NightSet::default();
        if parse_hhmm(&self.from).is_none() {
            self.from = d.from;
        }
        if parse_hhmm(&self.to).is_none() {
            self.to = d.to;
        }
        self.color = known_color(self.color);
        self
    }
}

/// A color the filter knows (`colors::HUES` or `TONES`), else None.
pub fn known_color(color: Option<String>) -> Option<String> {
    color.filter(|c| {
        crate::colors::HUES.iter().chain(crate::colors::TONES.iter()).any(|(k, _)| k == c)
    })
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
            color: None,
            night: NightSet::default(),
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
    fn frame_rects() {
        let f = |zoom, x, y| Frame { zoom, x, y, width: 1920, height: 1080 };
        // A 4:3 picture on a 16:9 monitor: full width, a band of its height.
        assert_eq!(f(100, 500, 500).rect(4000, 3000), (0, 375, 4000, 2250));
        // Twice as close, around the center.
        assert_eq!(f(200, 500, 500).rect(4000, 3000), (1000, 938, 2000, 1125));
        // A center near the edge stays inside the picture.
        assert_eq!(f(200, 0, 1000).rect(4000, 3000), (0, 1875, 2000, 1125));
        // A wide picture: full height.
        assert_eq!(f(100, 500, 500).rect(7680, 2160).3, 2160);
        assert_eq!(f(900, 2000, 0).sanitized().unwrap().zoom, 400);
        assert!(Frame { width: 0, ..f(100, 0, 0) }.sanitized().is_none());
    }

    #[test]
    fn night_windows() {
        let night = |from: &str, to: &str| NightSet { enabled: true, from: from.into(), to: to.into(), ..NightSet::default() };
        let n = night("20:00", "07:00");
        assert!(n.is_night(22 * 60) && n.is_night(3 * 60) && n.is_night(20 * 60));
        assert!(!n.is_night(7 * 60) && !n.is_night(12 * 60));
        let afternoon = night("13:30", "18:00");
        assert!(afternoon.is_night(14 * 60) && !afternoon.is_night(18 * 60) && !afternoon.is_night(9 * 60));
        assert!(!night("10:00", "10:00").is_night(10 * 60));
        assert!(!NightSet { enabled: false, ..n.clone() }.is_night(22 * 60));
        assert_eq!(parse_hhmm("7:05"), Some(425));
        assert_eq!(parse_hhmm("24:00"), None);
        assert_eq!(parse_hhmm("ab"), None);
        let fixed = NightSet { from: "25:00".into(), color: Some("plaid".into()), ..n }.sanitized();
        assert_eq!((fixed.from.as_str(), fixed.color), ("20:00", None));
    }

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
