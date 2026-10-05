//! Noctalia (v5) as the wallpaper backend.
//!
//! Applying is `noctalia msg wallpaper-set <monitor> <path>`. Noctalia saves
//! it in its own settings and restores it on start, so nothing else is needed
//! after a reboot. Its fill mode and transitions are global settings of its
//! own; wallpicker's fit mode doesn't apply here.

use std::path::{Path, PathBuf};

use crate::cmd::Runner;
use crate::error::{Error, Result};

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
pub fn wallpaper_enabled_on_disk() -> Option<bool> {
    let text = std::fs::read_to_string(settings_path()?).ok()?;
    wallpaper_enabled(&text)
}

/// Set `path` as `monitor`'s wallpaper. `enabled` is what Noctalia's settings
/// say about its wallpaper: when it's off, Noctalia would save the path but
/// draw nothing, so that's reported instead of looking like it worked.
pub fn apply(runner: &dyn Runner, monitor: &str, path: &Path, enabled: Option<bool>) -> Result<()> {
    if !path.is_file() {
        return Err(Error::Invalid(format!("not a file: {}", path.display())));
    }
    let p = path
        .to_str()
        .ok_or_else(|| Error::Invalid("path is not valid UTF-8".into()))?;
    if !path.is_absolute() {
        return Err(Error::Invalid(format!("path must be absolute: {p}")));
    }
    if monitor.is_empty() || monitor.starts_with('-') || monitor.chars().any(|c| c.is_control()) {
        return Err(Error::Invalid(format!("invalid monitor name: {monitor:?}")));
    }
    if enabled == Some(false) {
        return Err(Error::Invalid(
            "Noctalia's wallpaper is turned off. Turn it on in Noctalia's settings \
             (Wallpaper), or switch wallpicker back to hyprpaper (Settings → General)."
                .into(),
        ));
    }
    runner.run("noctalia", &["msg", "wallpaper-set", monitor, p])?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    struct Mock {
        calls: RefCell<Vec<String>>,
        reply: Result<String>,
    }

    impl Runner for Mock {
        fn run(&self, program: &str, args: &[&str]) -> Result<String> {
            self.calls
                .borrow_mut()
                .push(format!("{program} {}", args.join(" ")));
            match &self.reply {
                Ok(s) => Ok(s.clone()),
                Err(e) => Err(Error::Invalid(e.to_string())),
            }
        }
    }

    fn mock(reply: Result<String>) -> Mock {
        Mock {
            calls: RefCell::new(Vec::new()),
            reply,
        }
    }

    #[test]
    fn reads_only_the_wallpaper_tables_own_enabled() {
        let toml = r#"
[bar]
enabled = true

[wallpaper]
directory = "/home/u/Pictures"
enabled = false # turned off

    [wallpaper.default]
    enabled = true
"#;
        assert_eq!(wallpaper_enabled(toml), Some(false));
        assert_eq!(wallpaper_enabled("[wallpaper]\nenabled = true\n"), Some(true));
        // Only a sub-table says it: the table itself doesn't.
        assert_eq!(wallpaper_enabled("[wallpaper.default]\nenabled = true\n"), None);
        assert_eq!(wallpaper_enabled("[bar]\nenabled = false\n"), None);
        assert_eq!(wallpaper_enabled(""), None);
    }

    #[test]
    fn sets_through_noctalia_msg() {
        let tmp = tempfile::tempdir().unwrap();
        let img = tmp.path().join("a.png");
        std::fs::write(&img, b"x").unwrap();

        let m = mock(Ok(String::new()));
        apply(&m, "DP-1", &img, Some(true)).unwrap();
        assert_eq!(
            m.calls.borrow().as_slice(),
            [format!("noctalia msg wallpaper-set DP-1 {}", img.display())]
        );

        // Unknown either way (no settings file): still tried.
        let m = mock(Ok(String::new()));
        apply(&m, "DP-1", &img, None).unwrap();
        assert_eq!(m.calls.borrow().len(), 1);
    }

    #[test]
    fn refuses_instead_of_silently_doing_nothing() {
        let tmp = tempfile::tempdir().unwrap();
        let img = tmp.path().join("a.png");
        std::fs::write(&img, b"x").unwrap();

        // Wallpaper turned off in Noctalia: say so, don't call it.
        let m = mock(Ok(String::new()));
        let err = apply(&m, "DP-1", &img, Some(false)).unwrap_err().to_string();
        assert!(err.contains("turned off"));
        assert!(m.calls.borrow().is_empty());

        // Bad input never reaches noctalia.
        let m = mock(Ok(String::new()));
        assert!(apply(&m, "", &img, None).is_err());
        assert!(apply(&m, "--help", &img, None).is_err());
        assert!(apply(&m, "DP-1", &tmp.path().join("gone.png"), None).is_err());
        assert!(m.calls.borrow().is_empty());

        // Noctalia's own error comes through.
        let m = mock(Err(Error::Invalid(r#"error: unknown output "DP-9""#.into())));
        let err = apply(&m, "DP-9", &img, None).unwrap_err().to_string();
        assert!(err.contains("unknown output"));
    }
}
