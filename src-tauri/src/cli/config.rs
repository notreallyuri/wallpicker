//! `wallpicker filters` and `wallpicker settings`: read them as JSON, change
//! them with `set key=value...`. The same files as the app's Filters dialog
//! and Settings, saved with the same checks.

use serde::Serialize;
use serde::de::DeserializeOwned;

use wallpicker_core::moebooru::{MoebooruQuery, MoebooruSort, Ratings};
use wallpicker_core::online::{MIN_RESOLUTIONS, OnlineFilters, Provider};
use wallpicker_core::settings::{self, Settings, WallpaperBackend};
use wallpicker_core::store::{Dirs, Store};
use wallpicker_core::wallhaven::{self, SearchQuery, Sorting};
use wallpicker_core::cmd::SystemRunner;
use wallpicker_core::hyprpaper;
use wallpicker_core::noctalia;

use super::{Choice, choices, fail, print_json, usage_for};

// --------------------------------------------------------------- filters ----

#[derive(Serialize)]
struct FiltersOutput {
    wallhaven: WallhavenOut,
    konachan: MoebooruOut,
    yandere: MoebooruOut,
    options: FilterOptions,
    /// NSFW Wallhaven results need an API key (set in the app).
    api_key_set: bool,
    /// "net" is safe-for-work only, so its other ratings find nothing.
    konachan_site: &'static str,
}

#[derive(Serialize)]
struct WallhavenOut {
    sort: &'static str,
    /// Toplist period; only used when sorting by Top.
    range: &'static str,
    /// "" for any, or WIDTHxHEIGHT.
    atleast: String,
    /// Exact resolutions and aspect ratios; empty for any.
    resolutions: Vec<String>,
    ratios: Vec<String>,
    general: bool,
    anime: bool,
    people: bool,
    sfw: bool,
    sketchy: bool,
    nsfw: bool,
    /// How many filters differ from the defaults.
    active: usize,
}

#[derive(Serialize)]
struct MoebooruOut {
    sort: &'static str,
    atleast: String,
    safe: bool,
    questionable: bool,
    explicit: bool,
    active: usize,
}

#[derive(Serialize)]
struct FilterOptions {
    wallhaven_sorts: Vec<Choice>,
    moebooru_sorts: Vec<Choice>,
    top_ranges: Vec<Choice>,
    ratios: Vec<Choice>,
    resolutions: Vec<Choice>,
    min_resolutions: Vec<Choice>,
}

fn wallhaven_out(q: &SearchQuery) -> WallhavenOut {
    WallhavenOut {
        sort: q.sorting.as_str(),
        range: q.top_range.unwrap_or(wallhaven::TopRange::OneMonth).as_str(),
        atleast: q.atleast.clone().unwrap_or_default(),
        resolutions: q.resolutions.clone(),
        ratios: q.ratios.clone(),
        general: q.categories.general,
        anime: q.categories.anime,
        people: q.categories.people,
        sfw: q.purity.sfw,
        sketchy: q.purity.sketchy,
        nsfw: q.purity.nsfw,
        active: q.active_filters(),
    }
}

fn moebooru_out(q: &MoebooruQuery) -> MoebooruOut {
    MoebooruOut {
        sort: q.sorting.as_str(),
        atleast: q.atleast.clone().unwrap_or_default(),
        safe: q.ratings.safe,
        questionable: q.ratings.questionable,
        explicit: q.ratings.explicit,
        active: q.active_filters(),
    }
}

const FILTERS_USAGE: &str = "usage: wallpicker filters [--json]\n       \
     wallpicker filters <wallhaven|konachan|yandere> set KEY=VALUE...\n       \
     wallpicker filters <wallhaven|konachan|yandere> reset\n\
     wallhaven keys: sort range atleast resolutions ratios general anime people sfw sketchy nsfw\n\
     (resolutions and ratios: comma-separated, e.g. resolutions=1920x1080,2560x1440 ratios=16x9,4x3; empty for any)\n\
     konachan/yandere keys: sort atleast safe questionable explicit";

/// `wallpicker filters [--json]`, `filters <provider> set KEY=VALUE...`,
/// `filters <provider> reset`.
pub fn filters(args: &[String]) -> i32 {
    let store = Store::new(Dirs::from_env());
    let mut saved = match store.load_online_filters() {
        Ok(f) => f,
        Err(e) => return fail(&e),
    };
    match args {
        [] => return print_filters(&store, &saved),
        [flag] if flag == "--json" => return print_filters(&store, &saved),
        [provider, action, pairs @ ..] => {
            let Some(provider) = Provider::from_str(provider) else {
                return usage_for("filters", &format!("unknown provider {provider}"), FILTERS_USAGE);
            };
            let changed = match (action.as_str(), pairs) {
                ("reset", []) => {
                    reset(&mut saved, provider);
                    Ok(())
                }
                ("set", pairs) if !pairs.is_empty() => set_filters(&mut saved, provider, pairs),
                _ => return usage_for("filters", "use set KEY=VALUE... or reset", FILTERS_USAGE),
            };
            if let Err(why) = changed {
                return usage_for("filters", &why, FILTERS_USAGE);
            }
            if let Err(e) = store.save_online_filters(&saved) {
                return fail(&e);
            }
            print_filters(&store, &saved)
        }
        _ => usage_for("filters", "which provider, and set or reset?", FILTERS_USAGE),
    }
}

fn print_filters(store: &Store, f: &OnlineFilters) -> i32 {
    let settings = store.load_settings().unwrap_or_default();
    print_json(&FiltersOutput {
        wallhaven: wallhaven_out(&f.wallhaven),
        konachan: moebooru_out(&f.konachan),
        yandere: moebooru_out(&f.yandere),
        options: FilterOptions {
            wallhaven_sorts: Sorting::ALL
                .iter()
                .map(|s| Choice { value: s.as_str(), label: s.label() })
                .collect(),
            moebooru_sorts: MoebooruSort::ALL
                .iter()
                .map(|s| Choice { value: s.as_str(), label: s.label() })
                .collect(),
            top_ranges: wallhaven::TOP_RANGES
                .iter()
                .map(|(r, label)| Choice { value: r.as_str(), label })
                .collect(),
            ratios: choices(&wallhaven::RATIOS),
            resolutions: choices(&wallhaven::RESOLUTIONS),
            min_resolutions: choices(&MIN_RESOLUTIONS),
        },
        api_key_set: settings.wallhaven_api_key.is_some(),
        konachan_site: settings.konachan_site.as_str(),
    })
}

fn reset(f: &mut OnlineFilters, provider: Provider) {
    match provider {
        Provider::Wallhaven => f.wallhaven = SearchQuery::default(),
        Provider::Konachan => f.konachan = MoebooruQuery::default(),
        Provider::Yandere => f.yandere = MoebooruQuery::default(),
    }
}

/// Apply every `KEY=VALUE`, or none if any is wrong.
fn set_filters(f: &mut OnlineFilters, provider: Provider, pairs: &[String]) -> Result<(), String> {
    let mut next = f.clone();
    for pair in pairs {
        let (key, value) = split_pair(pair)?;
        match provider {
            Provider::Wallhaven => set_wallhaven(&mut next.wallhaven, key, value)?,
            Provider::Konachan => set_moebooru(&mut next.konachan, key, value)?,
            Provider::Yandere => set_moebooru(&mut next.yandere, key, value)?,
        }
    }
    *f = next;
    Ok(())
}

fn set_wallhaven(q: &mut SearchQuery, key: &str, value: &str) -> Result<(), String> {
    match key {
        "sort" => q.sorting = Sorting::from_str(value).ok_or_else(|| bad(key, value))?,
        "range" => {
            q.top_range = Some(
                wallhaven::TOP_RANGES
                    .iter()
                    .map(|(r, _)| *r)
                    .find(|r| r.as_str() == value)
                    .ok_or_else(|| bad(key, value))?,
            );
        }
        "atleast" => q.atleast = resolution(key, value)?,
        "ratios" => q.ratios = list(key, value, wallhaven::is_valid_ratio)?,
        "resolutions" => q.resolutions = list(key, value, wallhaven::is_valid_resolution)?,
        "general" => q.categories.general = boolean(key, value)?,
        "anime" => q.categories.anime = boolean(key, value)?,
        "people" => q.categories.people = boolean(key, value)?,
        "sfw" => q.purity.sfw = boolean(key, value)?,
        "sketchy" => q.purity.sketchy = boolean(key, value)?,
        "nsfw" => q.purity.nsfw = boolean(key, value)?,
        _ => return Err(format!("{key} isn't a Wallhaven filter")),
    }
    Ok(())
}

fn set_moebooru(q: &mut MoebooruQuery, key: &str, value: &str) -> Result<(), String> {
    let r: &mut Ratings = &mut q.ratings;
    match key {
        "sort" => q.sorting = MoebooruSort::from_str(value).ok_or_else(|| bad(key, value))?,
        "atleast" => q.atleast = resolution(key, value)?,
        "safe" => r.safe = boolean(key, value)?,
        "questionable" => r.questionable = boolean(key, value)?,
        "explicit" => r.explicit = boolean(key, value)?,
        _ => return Err(format!("{key} isn't a Konachan/yande.re filter")),
    }
    Ok(())
}

// -------------------------------------------------------------- settings ----

#[derive(Serialize)]
struct SettingsOutput {
    folders: Vec<String>,
    download_dir: String,
    recursive: bool,
    backend: &'static str,
    /// "" is hyprpaper's default.
    fit_mode: &'static str,
    /// Monitor name -> its own fit, over `fit_mode`.
    monitor_fits: std::collections::BTreeMap<String, &'static str>,
    slideshow: settings::Slideshow,
    /// awww's transition (the awww backend).
    transition: settings::Transition,
    /// Other programs drawing wallpapers besides the backend (they fight).
    other_wallpaper_programs: Vec<String>,
    close_to_tray: bool,
    thumb_width: u32,
    konachan_site: &'static str,
    /// The key itself is never printed; it's set in the app.
    api_key_set: bool,
    /// With the Noctalia backend, Noctalia's own wallpaper must be on.
    noctalia_wallpaper_enabled: Option<bool>,
    options: SettingsOptions,
}

#[derive(Serialize)]
struct SettingsOptions {
    backends: Vec<Choice>,
    fit_modes: Vec<Choice>,
    transitions: Vec<Choice>,
    konachan_sites: Vec<Choice>,
}

const SETTINGS_USAGE: &str = "usage: wallpicker settings [--json]\n       \
     wallpicker settings set KEY=VALUE...\n\
     keys: folders (one per line) download_dir recursive backend fit_mode thumb_width konachan_site close_to_tray\n\
     monitor_fit.NAME (a monitor's own fit; empty to use fit_mode)\n\
     slideshow.enabled slideshow.minutes slideshow.favorites slideshow.tags (comma-separated)\n\
     slideshow.any slideshow.folder slideshow.monitors (comma-separated; empty: all) slideshow.each\n\
     transition.kind transition.duration_ms transition.fps (the awww backend)\n\
     The Wallhaven API key is set in the app.";

/// `wallpicker settings [--json]`, `settings set KEY=VALUE...`.
pub fn settings(args: &[String]) -> i32 {
    let store = Store::new(Dirs::from_env());
    let mut s = match store.load_settings() {
        Ok(s) => s,
        Err(e) => return fail(&e),
    };
    match args {
        [] => print_settings(&s),
        [flag] if flag == "--json" => print_settings(&s),
        [action, pairs @ ..] if action == "set" && !pairs.is_empty() => {
            let backend_before = s.wallpaper_backend;
            if let Err(why) = set_settings(&mut s, pairs) {
                return usage_for("settings", &why, SETTINGS_USAGE);
            }
            let before = hyprpaper::Fits::load(&store);
            if let Err(e) = store.save_settings(&s) {
                return fail(&e);
            }
            // A new backend shows what each monitor had; a fit that changed
            // shows right away where it applies.
            let switched = s.wallpaper_backend != backend_before;
            let after = hyprpaper::Fits::load(&store);
            if s.wallpaper_backend != WallpaperBackend::Noctalia
                && let Err(e) =
                    hyprpaper::reapply(&SystemRunner, &store, &|m, p| switched || before.of(m, p) != after.of(m, p))
            {
                eprintln!("wallpicker: could not re-apply the wallpapers: {e}");
            }
            match store.load_settings() {
                Ok(saved) => print_settings(&saved),
                Err(e) => fail(&e),
            }
        }
        _ => usage_for("settings", "use set KEY=VALUE...", SETTINGS_USAGE),
    }
}

fn print_settings(s: &Settings) -> i32 {
    let noctalia_wallpaper_enabled = (s.wallpaper_backend == WallpaperBackend::Noctalia)
        .then(noctalia::wallpaper_enabled_on_disk)
        .flatten();
    print_json(&SettingsOutput {
        folders: s.folders.iter().map(|p| p.display().to_string()).collect(),
        download_dir: s.download_dir.display().to_string(),
        recursive: s.recursive,
        backend: s.wallpaper_backend.as_str(),
        fit_mode: s.fit_mode.map_or("", |f| f.as_str()),
        monitor_fits: s.monitor_fits.iter().map(|(m, f)| (m.clone(), f.as_str())).collect(),
        slideshow: s.slideshow.clone(),
        transition: s.transition.clone(),
        other_wallpaper_programs: hyprpaper::conflicts(&wallpicker_core::cmd::SystemRunner, s.wallpaper_backend),
        close_to_tray: s.close_to_tray,
        thumb_width: s.thumb_width,
        konachan_site: s.konachan_site.as_str(),
        api_key_set: s.wallhaven_api_key.is_some(),
        noctalia_wallpaper_enabled,
        options: SettingsOptions {
            backends: choices(&settings::BACKENDS),
            fit_modes: choices(&settings::FIT_MODES),
            transitions: choices(&settings::TRANSITIONS),
            konachan_sites: choices(&settings::KONACHAN_SITES),
        },
    })
}

/// Apply every `KEY=VALUE`, or none if any is wrong.
fn set_settings(s: &mut Settings, pairs: &[String]) -> Result<(), String> {
    let home = std::env::var_os("HOME").map(std::path::PathBuf::from);
    // Like the app: `~` is fine, relative paths aren't.
    let path = |key: &str, value: &str| {
        let p = settings::expand_tilde(std::path::Path::new(value.trim()), home.as_deref());
        if p.has_root() {
            Ok(p)
        } else {
            Err(format!("{key}: {value:?} isn't an absolute path (~ is fine)"))
        }
    };
    let mut next = s.clone();
    for pair in pairs {
        let (key, value) = split_pair(pair)?;
        match key {
            "folders" => {
                let folders = value
                    .lines()
                    .map(str::trim)
                    .filter(|l| !l.is_empty())
                    .map(|l| path(key, l))
                    .collect::<Result<Vec<_>, _>>()?;
                if folders.is_empty() {
                    return Err("folders: at least one folder".into());
                }
                next.folders = folders;
            }
            "download_dir" => next.download_dir = path(key, value)?,
            "recursive" => next.recursive = boolean(key, value)?,
            "close_to_tray" => next.close_to_tray = boolean(key, value)?,
            "backend" => next.wallpaper_backend = enum_value(key, value)?,
            "fit_mode" => next.fit_mode = if value.is_empty() { None } else { Some(enum_value(key, value)?) },
            "thumb_width" => {
                next.thumb_width = value
                    .parse::<u32>()
                    .ok()
                    .filter(|w| (120..=1024).contains(w))
                    .ok_or_else(|| format!("thumb_width: a number from 120 to 1024, not {value:?}"))?;
            }
            "konachan_site" => next.konachan_site = enum_value(key, value)?,
            "transition.kind" => {
                if !settings::TRANSITIONS.iter().any(|(k, _)| *k == value) {
                    let known: Vec<&str> = settings::TRANSITIONS.iter().map(|(k, _)| *k).collect();
                    return Err(format!("{key}: {value}? one of {}", known.join(", ")));
                }
                next.transition.kind = value.to_string();
            }
            "transition.duration_ms" => {
                next.transition.duration_ms = value.parse().map_err(|_| format!("{key}: a number of milliseconds"))?;
            }
            "transition.fps" => {
                next.transition.fps = value.parse().map_err(|_| format!("{key}: a number from 1 to 255"))?;
            }
            "slideshow.enabled" => next.slideshow.enabled = boolean(key, value)?,
            "slideshow.minutes" => {
                next.slideshow.minutes = value
                    .parse::<u32>()
                    .ok()
                    .filter(|m| (1..=24 * 60).contains(m))
                    .ok_or_else(|| format!("{key}: minutes from 1 to 1440, not {value:?}"))?;
            }
            "slideshow.favorites" => next.slideshow.favorites = boolean(key, value)?,
            "slideshow.any" => next.slideshow.any = boolean(key, value)?,
            "slideshow.each" => next.slideshow.each = boolean(key, value)?,
            "slideshow.tags" => {
                next.slideshow.tags = value
                    .split(',')
                    .map(str::trim)
                    .filter(|t| !t.is_empty())
                    .map(|t| wallpicker_core::catalog::normalize_tag(t).ok_or_else(|| format!("{key}: {t:?} isn't a tag")))
                    .collect::<Result<_, _>>()?;
            }
            "slideshow.folder" => {
                next.slideshow.folder = if value.trim().is_empty() { None } else { Some(path(key, value)?) };
            }
            "slideshow.monitors" => {
                next.slideshow.monitors = value.split(',').map(str::trim).filter(|m| !m.is_empty()).map(String::from).collect();
            }
            k if k.starts_with("monitor_fit.") => {
                let monitor = &k["monitor_fit.".len()..];
                if monitor.is_empty() || monitor.len() > 64 || monitor.chars().any(|c| c.is_control() || c.is_whitespace()) {
                    return Err(format!("{k}: not a monitor name"));
                }
                if value.is_empty() {
                    next.monitor_fits.remove(monitor);
                } else {
                    next.monitor_fits.insert(monitor.to_string(), enum_value(key, value)?);
                }
            }
            "wallhaven_api_key" => {
                return Err("set the Wallhaven API key in the app (Settings → Wallhaven), so it never sits on a command line".into());
            }
            _ => return Err(format!("{key} isn't a setting")),
        }
    }
    *s = next;
    Ok(())
}

// --------------------------------------------------------------- values ----

fn split_pair(pair: &str) -> Result<(&str, &str), String> {
    pair.split_once('=').ok_or_else(|| format!("{pair:?}: expected KEY=VALUE"))
}

fn bad(key: &str, value: &str) -> String {
    format!("{key}: {value:?} isn't one of its values")
}

fn boolean(key: &str, value: &str) -> Result<bool, String> {
    match value {
        "true" | "on" | "yes" | "1" => Ok(true),
        "false" | "off" | "no" | "0" => Ok(false),
        _ => Err(format!("{key}: {value:?} isn't true or false")),
    }
}

/// A comma-separated list ("" for none), each item checked by `valid`, in
/// order, without repeats.
fn list(key: &str, value: &str, valid: fn(&str) -> bool) -> Result<Vec<String>, String> {
    let mut out: Vec<String> = Vec::new();
    for item in value.split(',').map(str::trim).filter(|i| !i.is_empty()) {
        if !valid(item) {
            return Err(format!("{key}: {item:?} isn't valid (WIDTHxHEIGHT, comma-separated)"));
        }
        if !out.iter().any(|o| o == item) {
            out.push(item.to_string());
        }
    }
    Ok(out)
}

/// "" for any, else `<width>x<height>`.
fn resolution(key: &str, value: &str) -> Result<Option<String>, String> {
    match value {
        "" => Ok(None),
        v if wallhaven::is_valid_resolution(v) => Ok(Some(v.to_string())),
        _ => Err(format!("{key}: {value:?} isn't WIDTHxHEIGHT")),
    }
}

/// An enum by its saved name ("noctalia", "cover", "com").
fn enum_value<T: DeserializeOwned>(key: &str, value: &str) -> Result<T, String> {
    serde_json::from_value(serde_json::Value::String(value.to_string())).map_err(|_| bad(key, value))
}
