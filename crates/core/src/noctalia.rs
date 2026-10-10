//! Noctalia settings readers for color generation and competing wallpaper detection.

use std::path::PathBuf;

/// Noctalia v5 keeps its settings under the XDG state directory.
pub fn settings_path() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/state")))?;
    Some(base.join("noctalia").join("settings.toml"))
}

/// The `enabled` key of the `[wallpaper]` table, if the file says. Only that
/// table counts: sub-tables (`[wallpaper.default]`) and other tables are
/// skipped. A tiny reader on purpose, not a TOML parser: it only needs this.
pub fn wallpaper_enabled(settings_toml: &str) -> Option<bool> {
    let mut in_wallpaper = false;
    for line in settings_toml.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_wallpaper = line == "[wallpaper]";
            continue;
        }
        if !in_wallpaper {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        if key.trim() == "enabled" {
            let value = value.split('#').next().unwrap_or("").trim();
            return match value {
                "true" => Some(true),
                "false" => Some(false),
                _ => None,
            };
        }
    }
    None
}

/// Whether Noctalia's wallpaper is on, from its settings file. `None` when
/// the file can't be read or doesn't say (Noctalia's default then applies).
/// The generator scheme Noctalia uses for wallpaper colors
/// (`[theme] wallpaper_scheme`, "m3-rainbow"), if the file says.
pub fn wallpaper_scheme(settings_toml: &str) -> Option<String> {
    let mut in_theme = false;
    for line in settings_toml.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_theme = line == "[theme]";
            continue;
        }
        if in_theme
            && let Some((key, value)) = line.split_once('=')
            && key.trim() == "wallpaper_scheme"
        {
            let v = value.trim().trim_matches('"');
            let ok = !v.is_empty() && v.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_');
            return ok.then(|| v.to_string());
        }
    }
    None
}

pub fn wallpaper_scheme_on_disk() -> Option<String> {
    wallpaper_scheme(&std::fs::read_to_string(settings_path()?).ok()?)
}

pub fn wallpaper_enabled_on_disk() -> Option<bool> {
    let text = std::fs::read_to_string(settings_path()?).ok()?;
    wallpaper_enabled(&text)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn color_scheme_and_wallpaper_conflicts_read_only_their_own_tables() {
        let text = "[theme]\nwallpaper_scheme = \"m3-rainbow\"\n[other]\nenabled = false\n[wallpaper]\nenabled = true # competing renderer\n[wallpaper.default]\nenabled = false\n";
        assert_eq!(wallpaper_enabled(text), Some(true));
        assert_eq!(wallpaper_scheme(text).as_deref(), Some("m3-rainbow"));
        assert_eq!(wallpaper_scheme("[theme]\nwallpaper_scheme = \"../bad\""), None);
        assert_eq!(wallpaper_enabled("[wallpaper.default]\nenabled = true"), None);
    }
}
