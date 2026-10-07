//! Colors from the wallpaper. The app's palette comes from the wallpaper on
//! the first monitor, through Noctalia's own generator (`noctalia theme
//! <image> --both --scheme <its wallpaper scheme>`), so it matches Noctalia
//! when Noctalia's colors follow the wallpaper too. That's a switch here as
//! well: it sets Noctalia's color source to the wallpaper and remembers the
//! scheme it had, to go back to.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::Command;

use serde::{Deserialize, Serialize};
use wallpicker_core::cmd::SystemRunner;
use wallpicker_core::hyprpaper;
use wallpicker_core::noctalia;
use wallpicker_core::store::Store;

/// Material 3 colors (name → "#rrggbb") for both modes, and where they came from.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Palette {
    pub image: PathBuf,
    pub scheme: String,
    pub dark: BTreeMap<String, String>,
    pub light: BTreeMap<String, String>,
}

#[derive(Serialize, Deserialize)]
struct Cached {
    image: PathBuf,
    modified: u64,
    scheme: String,
    palette: Palette,
}

fn is_hex(v: &str) -> bool {
    v.len() == 7 && v.starts_with('#') && v[1..].bytes().all(|b| b.is_ascii_hexdigit())
}

fn only_colors(v: &serde_json::Value) -> BTreeMap<String, String> {
    v.as_object()
        .map(|o| {
            o.iter()
                .filter_map(|(k, v)| v.as_str().filter(|s| is_hex(s)).map(|s| (k.clone(), s.to_string())))
                .collect()
        })
        .unwrap_or_default()
}

/// The wallpaper the palette comes from: the chosen monitor's, else the
/// first connected monitor's, else any monitor's that still exists.
fn source_image(store: &Store, monitor: Option<&str>) -> Option<PathBuf> {
    let state = store.load_state().ok()?;
    let monitors = hyprpaper::list_monitors(&SystemRunner).unwrap_or_default();
    monitor
        .and_then(|m| state.current.get(m))
        .into_iter()
        .chain(monitors.iter().filter_map(|m| state.current.get(&m.name)))
        .chain(state.current.values())
        .find(|p| p.is_file())
        .cloned()
}

/// The palette for the current wallpaper, or None (turned off, nothing
/// applied yet, or no Noctalia to generate it). Cached per image and scheme.
pub fn palette(store: &Store) -> Result<Option<Palette>, String> {
    let settings = store.load_settings().map_err(|e| e.to_string())?;
    if !settings.theme_from_wallpaper {
        return Ok(None);
    }
    let Some(image) = source_image(store, settings.theme_monitor.as_deref()) else { return Ok(None) };
    let scheme = noctalia::wallpaper_scheme_on_disk().unwrap_or_else(|| "m3-tonal-spot".into());
    let modified = image
        .metadata()
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map_or(0, |d| d.as_secs());
    let cache_file = store.dirs().cache.join("palette.json");
    if let Some(c) = std::fs::read_to_string(&cache_file).ok().and_then(|t| serde_json::from_str::<Cached>(&t).ok())
        && c.image == image
        && c.modified == modified
        && c.scheme == scheme
    {
        return Ok(Some(c.palette));
    }
    let out = match Command::new("noctalia").arg("theme").arg(&image).args(["--both", "--scheme", &scheme]).output() {
        Ok(o) if o.status.success() => o.stdout,
        // No Noctalia, or it couldn't read the image: the app keeps its own colors.
        _ => return Ok(None),
    };
    let json: serde_json::Value = serde_json::from_slice(&out).map_err(|e| format!("Noctalia's palette: {e}"))?;
    let palette = Palette { image: image.clone(), scheme: scheme.clone(), dark: only_colors(&json["dark"]), light: only_colors(&json["light"]) };
    if palette.dark.is_empty() || palette.light.is_empty() {
        return Ok(None);
    }
    let cached = Cached { image, modified, scheme, palette: palette.clone() };
    if let Ok(text) = serde_json::to_string(&cached) {
        let _ = std::fs::write(cache_file, text);
    }
    Ok(Some(palette))
}

/// Noctalia's color source and selection ("community Garnet"), if it runs.
fn noctalia_scheme() -> Option<String> {
    let out = Command::new("noctalia").args(["msg", "color-scheme-get"]).output().ok()?;
    let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
    (out.status.success() && !s.is_empty()).then_some(s)
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

pub fn noctalia_colors() -> NoctaliaColors {
    let scheme = noctalia_scheme();
    NoctaliaColors {
        available: scheme.is_some(),
        following: scheme.as_deref().is_some_and(|s| s.starts_with("wallpaper")),
        scheme,
    }
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
    if follow {
        if !now.starts_with("wallpaper") {
            settings.noctalia_previous_scheme = Some(now);
            store.save_settings(&settings).map_err(|e| e.to_string())?;
        }
        let scheme = noctalia::wallpaper_scheme_on_disk().unwrap_or_else(|| "m3-tonal-spot".into());
        set_scheme(&["wallpaper", &scheme])?;
    } else if now.starts_with("wallpaper") {
        let previous = settings.noctalia_previous_scheme.clone().ok_or(
            "There's no earlier Noctalia scheme to go back to; pick one in Noctalia's settings.",
        )?;
        let parts: Vec<&str> = previous.split_whitespace().collect();
        set_scheme(&parts)?;
        settings.noctalia_previous_scheme = None;
        store.save_settings(&settings).map_err(|e| e.to_string())?;
    }
    Ok(noctalia_colors())
}
