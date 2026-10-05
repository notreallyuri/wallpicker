//! `wallpicker --open WHERE [IMAGE]`: show the window at a place in it (a
//! tab, Settings, Duplicates, an image's tags), for the Noctalia panel to
//! hand bigger jobs over. Unlike `--toggle` it never hides the window.
//!
//! The place waits here until the UI takes it (`take_open_request`): at
//! startup the UI asks once it's loaded, and a running app is nudged with
//! an `open-request` event.

use std::path::Path;
use std::sync::Mutex;

use serde::Serialize;
use tauri::{AppHandle, Emitter};

/// Where `--open` can go.
pub const PLACES: [&str; 6] = ["local", "online", "collections", "settings", "duplicates", "tags"];

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Place {
    pub view: String,
    /// The image, for "tags" (absolute).
    pub path: Option<String>,
}

static PENDING: Mutex<Option<Place>> = Mutex::new(None);

/// The place `argv` asks for (None: no `--open`). A relative image path is
/// taken from `cwd`, the directory `wallpicker` was started in.
pub fn parse(argv: &[String], cwd: &Path) -> Result<Option<Place>, String> {
    let Some(i) = argv.iter().position(|a| a == "--open") else {
        return Ok(None);
    };
    let Some(view) = argv.get(i + 1) else {
        return Err("--open needs a place: local, online, collections, settings, duplicates or tags <image>".into());
    };
    if !PLACES.contains(&view.as_str()) {
        return Err(format!("--open {view}? use local, online, collections, settings, duplicates or tags <image>"));
    }
    let path = if view == "tags" {
        let image = argv.get(i + 2).ok_or("--open tags needs the image")?;
        Some(cwd.join(image).to_string_lossy().into_owned())
    } else {
        None
    };
    Ok(Some(Place { view: view.clone(), path }))
}

/// Keep `place` for the UI to take.
pub fn set_pending(place: Place) {
    if let Ok(mut p) = PENDING.lock() {
        *p = Some(place);
    }
}

pub fn take() -> Option<Place> {
    PENDING.lock().ok()?.take()
}

/// A running app asked to go to `place`: show the window there.
pub fn request(app: &AppHandle, place: Place) {
    set_pending(place);
    crate::tray::show(app);
    let _ = app.emit("open-request", ());
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(a: &[&str]) -> Vec<String> {
        a.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn parses_places() {
        let cwd = Path::new("/home/u");
        assert_eq!(parse(&args(&["wallpicker", "--toggle"]), cwd), Ok(None));
        assert_eq!(
            parse(&args(&["wallpicker", "--open", "settings"]), cwd),
            Ok(Some(Place { view: "settings".into(), path: None }))
        );
        assert_eq!(
            parse(&args(&["wallpicker", "--open", "tags", "pics/a.png"]), cwd),
            Ok(Some(Place { view: "tags".into(), path: Some("/home/u/pics/a.png".into()) }))
        );
        assert_eq!(
            parse(&args(&["wallpicker", "--open", "tags", "/w/b.jpg"]), cwd).unwrap().unwrap().path.as_deref(),
            Some("/w/b.jpg")
        );
        assert!(parse(&args(&["wallpicker", "--open"]), cwd).is_err());
        assert!(parse(&args(&["wallpicker", "--open", "nowhere"]), cwd).is_err());
        assert!(parse(&args(&["wallpicker", "--open", "tags"]), cwd).is_err());
    }
}
