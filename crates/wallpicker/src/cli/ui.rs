//! The picker: the Quickshell interface (quickshell/), run as `qs -p <its
//! folder>` with this binary as its backend (WALLPICKER_BIN, for `wallpicker
//! serve`). `wallpicker` (or `wallpicker ui`) opens it; `--toggle` closes it
//! when it's open (a keybind) or shows it again while it's still loaded;
//! `--open PLACE [IMAGE]` opens it at a tab or a
//! dialog (the Noctalia plugin hands bigger jobs over that way), or moves
//! the open one there.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use super::fail;

/// Where the interface is: WALLPICKER_UI, a checkout this binary was built
/// in (development), or the installed copy.
fn ui_dir() -> Option<PathBuf> {
    let has_shell = |d: &PathBuf| d.join("shell.qml").is_file();
    if let Some(d) = std::env::var_os("WALLPICKER_UI").map(PathBuf::from) {
        return has_shell(&d).then_some(d);
    }
    if let Ok(exe) = std::env::current_exe() {
        for dir in exe.ancestors().skip(1) {
            let d = dir.join("quickshell");
            if has_shell(&d) {
                return Some(d);
            }
        }
    }
    let data = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")))?;
    Some(data.join("wallpicker/ui")).filter(has_shell)
}

/// Whether it's open: its IPC answers.
/// Settings that cut the open picker's memory (measured on the desktop
/// 2026-10-07: ~260 MB -> ~220 MB, interface and backend together):
/// glibc's allocator hands freed memory back instead of keeping a pool per
/// thread (`serve` inherits it: 25 -> 6 MB), and Qt draws through Vulkan,
/// where there's a driver for it, instead of OpenGL (whose AMD driver keeps
/// LLVM's compiler around). Anything already set in the environment wins.
fn lighter(qs: &mut Command) {
    let unset = |key: &str| std::env::var_os(key).is_none();
    for (key, value) in [("MALLOC_ARENA_MAX", "2"), ("MALLOC_MMAP_THRESHOLD_", "1048576"), ("MALLOC_TRIM_THRESHOLD_", "131072")] {
        if unset(key) {
            qs.env(key, value);
        }
    }
    if unset("QSG_RHI_BACKEND") && has_vulkan_driver() {
        qs.env("QSG_RHI_BACKEND", "vulkan");
    }
}

/// A Vulkan driver is installed (an ICD manifest where the loader looks).
fn has_vulkan_driver() -> bool {
    ["/usr/share/vulkan/icd.d", "/etc/vulkan/icd.d"].iter().any(|d| {
        std::fs::read_dir(d).is_ok_and(|mut e| e.any(|f| f.is_ok_and(|f| f.path().extension().is_some_and(|x| x == "json"))))
    })
}

fn running(dir: &PathBuf) -> bool {
    Command::new("qs")
        .arg("-p")
        .arg(dir)
        .args(["ipc", "show"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

pub fn ui(args: &[String]) -> i32 {
    let toggle = args.iter().any(|a| a == "--toggle");
    let cwd = std::env::current_dir().unwrap_or_default();
    let place = match parse_open(args, &cwd) {
        Ok(place) => place,
        Err(e) => {
            eprintln!("wallpicker: {e}");
            return 2;
        }
    };
    let Some(dir) = ui_dir() else {
        return fail(&"the interface isn't installed (scripts/install-local.sh puts it in ~/.local/share/wallpicker/ui)");
    };
    if running(&dir) {
        let call = |args: &[&str]| {
            Command::new("qs")
                .arg("-p")
                .arg(&dir)
                .args(["ipc", "call", "wallpicker"])
                .args(args)
                .stdout(Stdio::null())
                .status()
        };
        // Running: open, or closed but still loaded (Settings → Performance).
        if let Some(p) = &place {
            let _ = call(&["open", &p.view, p.path.as_deref().unwrap_or("")]);
        } else if toggle {
            let _ = call(&["toggle"]);
        } else {
            let _ = call(&["reopen"]);
        }
        return 0;
    }
    let exe = std::env::current_exe().map(|p| p.into_os_string()).unwrap_or_else(|_| "wallpicker".into());
    let mut qs = Command::new("qs");
    qs.arg("-p").arg(&dir).args(["--no-duplicate", "--daemonize"]).env("WALLPICKER_BIN", exe);
    // Where to open, read once it's up (shell.qml).
    if let Some(p) = &place {
        qs.env("WALLPICKER_OPEN", format!("{}\n{}", p.view, p.path.as_deref().unwrap_or("")));
    }
    lighter(&mut qs);
    match qs
        .stdin(Stdio::null())
        // Quickshell keeps its own log (qs log); a launch says nothing.
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
    {
        Ok(s) if s.success() => 0,
        Ok(s) => fail(&format!("quickshell exited with {s}")),
        Err(e) => fail(&format!("could not start quickshell (qs): {e}")),
    }
}

/// Where `--open` can go.
const PLACES: [&str; 6] = ["local", "online", "collections", "settings", "duplicates", "tags"];

#[derive(Debug, PartialEq, Eq)]
struct Place {
    view: String,
    /// The image, for "tags" (absolute).
    path: Option<String>,
}

/// The place `args` ask for (None: no `--open`). A relative image path is
/// taken from `cwd`, where `wallpicker` was started.
fn parse_open(args: &[String], cwd: &Path) -> Result<Option<Place>, String> {
    let Some(i) = args.iter().position(|a| a == "--open") else {
        return Ok(None);
    };
    let Some(view) = args.get(i + 1) else {
        return Err("--open needs a place: local, online, collections, settings, duplicates or tags <image>".into());
    };
    if !PLACES.contains(&view.as_str()) {
        return Err(format!("--open {view}? use local, online, collections, settings, duplicates or tags <image>"));
    }
    let path = if view == "tags" {
        let image = args.get(i + 2).ok_or("--open tags needs the image")?;
        Some(cwd.join(image).to_string_lossy().into_owned())
    } else {
        None
    };
    Ok(Some(Place { view: view.clone(), path }))
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
        assert_eq!(parse_open(&args(&["--toggle"]), cwd), Ok(None));
        assert_eq!(parse_open(&args(&["--open", "settings"]), cwd), Ok(Some(Place { view: "settings".into(), path: None })));
        assert_eq!(
            parse_open(&args(&["--open", "tags", "pics/a.png"]), cwd),
            Ok(Some(Place { view: "tags".into(), path: Some("/home/u/pics/a.png".into()) }))
        );
        assert_eq!(parse_open(&args(&["--open", "tags", "/w/b.jpg"]), cwd).unwrap().unwrap().path.as_deref(), Some("/w/b.jpg"));
        assert!(parse_open(&args(&["--open"]), cwd).is_err());
        assert!(parse_open(&args(&["--open", "nowhere"]), cwd).is_err());
        assert!(parse_open(&args(&["--open", "tags"]), cwd).is_err());
    }
}
