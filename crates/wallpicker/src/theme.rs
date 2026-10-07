//! Colors from the wallpaper. The app's palette comes from the wallpaper on
//! the first monitor, through Noctalia's own generator (`noctalia theme
//! <image> --both --scheme <its wallpaper scheme>`), so it matches Noctalia
//! when Noctalia's colors follow the wallpaper too (noctalia_colors.rs).

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::Command;

use serde::{Deserialize, Serialize};
use wallpicker_core::cmd::SystemRunner;
use wallpicker_core::hyprpaper;
use wallpicker_core::noctalia;
use wallpicker_core::store::Store;

pub use crate::noctalia_colors::{NoctaliaColors, noctalia_colors, set_noctalia_follow};

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
pub(crate) fn source_image(store: &Store, monitor: Option<&str>) -> Option<PathBuf> {
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
