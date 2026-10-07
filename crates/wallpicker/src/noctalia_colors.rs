//! Noctalia's own colors following the wallpaper. With the Noctalia backend
//! that's Noctalia's wallpaper mode: it draws the wallpaper, so it knows it.
//! With awww or hyprpaper it doesn't, so wallpicker hands it one: a custom
//! palette made from the wallpaper shown (`palettes/wallpicker.json` in
//! Noctalia's config, through Noctalia's own generator and scheme), switched
//! to, and switched to again when that wallpaper changes (`refresh`: Noctalia
//! doesn't watch the file). The scheme it had is remembered, to go back to.

use std::path::PathBuf;
use std::process::Command;

use serde::{Deserialize, Serialize};
use wallpicker_core::noctalia;
use wallpicker_core::settings::WallpaperBackend;
use wallpicker_core::store::Store;

/// The custom palette's name in Noctalia.
const CUSTOM: &str = "wallpicker";

/// Noctalia's color source and selection ("community Garnet"), if it runs.
fn noctalia_scheme() -> Option<String> {
    let out = Command::new("noctalia").args(["msg", "color-scheme-get"]).output().ok()?;
    let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
    (out.status.success() && !s.is_empty()).then_some(s)
}

/// Following the wallpaper shown: with the Noctalia backend, Noctalia's own
/// wallpaper mode; with another, only wallpicker's palette (Noctalia's mode
/// would follow a wallpaper of its own that isn't the one on screen).
fn follows(scheme: &str, backend: WallpaperBackend) -> bool {
    match backend {
        WallpaperBackend::Noctalia => scheme.starts_with("wallpaper"),
        _ => scheme == format!("custom {CUSTOM}"),
    }
}

fn backend_of(store: &Store) -> WallpaperBackend {
    store.load_settings().map(|s| s.wallpaper_backend).unwrap_or_default()
}

#[derive(Debug, Serialize)]
pub struct NoctaliaColors {
    /// Noctalia is running (its scheme could be read).
    pub available: bool,
    /// Its colors follow the wallpaper now.
    pub following: bool,
    /// What it is now ("community Garnet", "wallpaper m3-rainbow").
    pub scheme: Option<String>,
}

pub fn noctalia_colors(store: &Store) -> NoctaliaColors {
    let scheme = noctalia_scheme();
    let backend = backend_of(store);
    NoctaliaColors { available: scheme.is_some(), following: scheme.as_deref().is_some_and(|s| follows(s, backend)), scheme }
}

fn set_scheme(args: &[&str]) -> Result<(), String> {
    let out = Command::new("noctalia")
        .args(["msg", "color-scheme-set"])
        .args(args)
        .output()
        .map_err(|e| format!("Could not run noctalia: {e}"))?;
    if out.status.success() {
        Ok(())
    } else {
        Err(format!("noctalia: {}", String::from_utf8_lossy(&out.stderr).trim()))
    }
}

/// Make Noctalia's colors follow the wallpaper (remembering its scheme), or
/// go back to the scheme it had.
pub fn set_noctalia_follow(store: &Store, follow: bool) -> Result<NoctaliaColors, String> {
    let now = noctalia_scheme().ok_or("Noctalia isn't running.")?;
    let mut settings = store.load_settings().map_err(|e| e.to_string())?;
    let backend = settings.wallpaper_backend;
    if follow {
        if !follows(&now, backend) {
            settings.noctalia_previous_scheme = Some(now);
            store.save_settings(&settings).map_err(|e| e.to_string())?;
        }
        if settings.wallpaper_backend == WallpaperBackend::Noctalia {
            let scheme = noctalia::wallpaper_scheme_on_disk().unwrap_or_else(|| "m3-tonal-spot".into());
            set_scheme(&["wallpaper", &scheme])?;
        } else {
            write_palette(store, true)?;
            set_scheme(&["custom", CUSTOM])?;
        }
    } else if follows(&now, backend) {
        let previous = settings.noctalia_previous_scheme.clone().ok_or(
            "There's no earlier Noctalia scheme to go back to; pick one in Noctalia's settings.",
        )?;
        let parts: Vec<&str> = previous.split_whitespace().collect();
        set_scheme(&parts)?;
        settings.noctalia_previous_scheme = None;
        store.save_settings(&settings).map_err(|e| e.to_string())?;
    }
    Ok(noctalia_colors(store))
}

/// While Noctalia uses wallpicker's palette: make it again from the
/// wallpaper shown now and switch to it, if that wallpaper (or Noctalia's
/// scheme) changed since. Ok(true) when it did.
pub fn refresh(store: &Store) -> Result<bool, String> {
    if noctalia_scheme().as_deref() != Some(&format!("custom {CUSTOM}")) {
        return Ok(false);
    }
    if !write_palette(store, false)? {
        return Ok(false);
    }
    set_scheme(&["custom", CUSTOM])?;
    Ok(true)
}

/// What the palette was last made from (in wallpicker's cache).
#[derive(Serialize, Deserialize, PartialEq)]
struct Made {
    image: PathBuf,
    modified: u64,
    scheme: String,
}

/// Noctalia's palette for the wallpaper the colors come from (the monitor
/// chosen in Appearance, else the first), unless it's made already
/// (`force`: anyway). Ok(true) when it was written.
fn write_palette(store: &Store, force: bool) -> Result<bool, String> {
    let settings = store.load_settings().map_err(|e| e.to_string())?;
    let image = crate::theme::source_image(store, settings.theme_monitor.as_deref())
        .ok_or("No wallpaper to take the colors from yet.")?;
    let scheme = noctalia::wallpaper_scheme_on_disk().unwrap_or_else(|| "m3-tonal-spot".into());
    let modified = image
        .metadata()
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map_or(0, |d| d.as_secs());
    let made = Made { image: image.clone(), modified, scheme: scheme.clone() };
    let marker = store.dirs().cache.join("noctalia-palette.json");
    let file = palette_path().ok_or("Can't tell where Noctalia's config is.")?;
    let same = std::fs::read_to_string(&marker).ok().and_then(|t| serde_json::from_str::<Made>(&t).ok()).is_some_and(|m| m == made);
    if same && !force && file.is_file() {
        return Ok(false);
    }
    let out = Command::new("noctalia")
        .arg("theme")
        .arg(&image)
        .args(["--both", "--scheme", &scheme])
        .output()
        .map_err(|e| format!("Could not run noctalia: {e}"))?;
    if !out.status.success() {
        return Err(format!("noctalia theme: {}", String::from_utf8_lossy(&out.stderr).trim()));
    }
    let roles: serde_json::Value = serde_json::from_slice(&out.stdout).map_err(|e| format!("Noctalia's palette: {e}"))?;
    let palette = serde_json::json!({ "dark": noctalia_keys(&roles["dark"])?, "light": noctalia_keys(&roles["light"])? });
    if let Some(dir) = file.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let text = serde_json::to_string_pretty(&palette).map_err(|e| e.to_string())?;
    // Beside it, then renamed: Noctalia never reads half a file.
    let tmp = file.with_extension("json.part");
    std::fs::write(&tmp, text).and_then(|()| std::fs::rename(&tmp, &file)).map_err(|e| e.to_string())?;
    if let Ok(text) = serde_json::to_string(&made) {
        let _ = std::fs::write(marker, text);
    }
    Ok(true)
}

/// `~/.config/noctalia/palettes/wallpicker.json` (XDG_CONFIG_HOME honored).
fn palette_path() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))?;
    Some(base.join("noctalia").join("palettes").join(format!("{CUSTOM}.json")))
}

/// One mode of `noctalia theme`'s Material roles as Noctalia's palette keys
/// (the format of its community palettes).
fn noctalia_keys(roles: &serde_json::Value) -> Result<serde_json::Value, String> {
    const KEYS: [(&str, &str); 16] = [
        ("mPrimary", "primary"),
        ("mOnPrimary", "on_primary"),
        ("mSecondary", "secondary"),
        ("mOnSecondary", "on_secondary"),
        ("mTertiary", "tertiary"),
        ("mOnTertiary", "on_tertiary"),
        ("mError", "error"),
        ("mOnError", "on_error"),
        ("mSurface", "surface"),
        ("mOnSurface", "on_surface"),
        ("mSurfaceVariant", "surface_variant"),
        ("mOnSurfaceVariant", "on_surface_variant"),
        ("mOutline", "outline"),
        ("mShadow", "shadow"),
        ("mHover", "hover"),
        ("mOnHover", "on_hover"),
    ];
    const COLORS: [&str; 8] = ["black", "red", "green", "yellow", "blue", "magenta", "cyan", "white"];
    let get = |role: &str| -> Result<serde_json::Value, String> {
        roles.get(role).filter(|v| v.is_string()).cloned().ok_or(format!("Noctalia's palette has no {role}"))
    };
    let mut out = serde_json::Map::new();
    for (key, role) in KEYS {
        out.insert(key.into(), get(role)?);
    }
    let set = |prefix: &str| -> Result<serde_json::Value, String> {
        COLORS.iter().map(|c| Ok(((*c).to_string(), get(&format!("terminal_{prefix}_{c}"))?))).collect::<Result<serde_json::Map<_, _>, String>>().map(Into::into)
    };
    out.insert(
        "terminal".into(),
        serde_json::json!({
            "foreground": get("terminal_foreground")?,
            "background": get("terminal_background")?,
            "normal": set("normal")?,
            "bright": set("bright")?,
            "cursor": get("terminal_cursor")?,
            "cursorText": get("terminal_cursor_text")?,
            "selectionFg": get("terminal_selection_fg")?,
            "selectionBg": get("terminal_selection_bg")?,
        }),
    );
    Ok(out.into())
}
