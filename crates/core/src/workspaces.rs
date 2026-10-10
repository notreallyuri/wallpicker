//! A wallpaper per Hyprland workspace (Settings → Workspaces). Hyprland
//! announces workspace changes on its event socket; on each, the monitors'
//! workspaces are read again (`hyprctl monitors -j`), and a monitor that now
//! shows a workspace with a wallpaper gets it. Special workspaces (scratch-
//! pads) float over a normal one and are left out.

use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::cmd::Runner;
use crate::error::Result;
use crate::hyprpaper::{Backend, Fits};
use crate::store::Store;

/// Hyprland's event socket (`.socket2.sock`), if Hyprland is running.
pub fn socket_path() -> Option<PathBuf> {
    let sig = std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE")?;
    let runtime = std::env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from)?;
    let p = runtime.join("hypr").join(sig).join(".socket2.sock");
    p.exists().then_some(p)
}

/// The events after which a monitor may show another workspace.
pub fn is_workspace_event(line: &str) -> bool {
    let name = line.split(">>").next().unwrap_or("");
    matches!(
        name,
        "workspace" | "workspacev2" | "focusedmon" | "focusedmonv2" | "moveworkspace" | "moveworkspacev2" | "monitoradded" | "monitoraddedv2"
    )
}

/// Each monitor's (normal) workspace now.
fn workspaces_now(runner: &dyn Runner) -> Result<HashMap<String, String>> {
    Ok(crate::monitors::list(runner)?
        .into_iter()
        .filter_map(|m| {
            let ws = m.active_workspace?.name;
            (!ws.is_empty() && !ws.starts_with("special")).then_some((m.name, ws))
        })
        .collect())
}

/// Put each monitor's workspace wallpaper on it where its workspace changed
/// since `last` (which is updated). Returns (monitor, wallpaper) applied.
pub fn sync(runner: &dyn Runner, store: &Store, last: &mut HashMap<String, String>) -> Result<Vec<(String, PathBuf)>> {
    let now = workspaces_now(runner)?;
    let settings = store.load_settings()?;
    let mut applied = Vec::new();
    if settings.workspaces.enabled {
        let state = store.load_state()?;
        let backend = Backend::from_settings(store);
        let fits = Fits::load(store);
        for (monitor, ws) in &now {
            if last.get(monitor) == Some(ws) {
                continue;
            }
            let Some(path) = settings.workspaces.wallpapers.get(ws) else {
                continue;
            };
            if !path.is_file() || state.current.get(monitor) == Some(path) {
                continue;
            }
            match backend.apply(runner, monitor, path, fits.of(monitor, path)) {
                Ok(()) => {
                    store.show_for_workspace(monitor, path)?;
                    applied.push((monitor.clone(), path.clone()));
                }
                Err(e) => eprintln!("wallpicker: workspace {ws} on {monitor}: {e}"),
            }
        }
    }
    *last = now;
    Ok(applied)
}

/// Wallpapers just put on `monitors` by hand (applied, Previous): with
/// "remember" on, what each shows now becomes the wallpaper of its workspace.
/// Failing here never fails the apply; it's said and skipped.
pub fn remember_shown(runner: &dyn Runner, store: &Store, monitors: &[String]) {
    let Ok(state) = store.load_state() else {
        return;
    };
    for m in monitors {
        if let Some(path) = state.current.get(m)
            && let Err(e) = remember(runner, store, m, path)
        {
            eprintln!("wallpicker: could not remember {m}'s workspace wallpaper: {e}");
        }
    }
}

/// A wallpaper put on `monitor` by hand: with "remember" on, it becomes the
/// wallpaper of the workspace the monitor shows.
pub fn remember(runner: &dyn Runner, store: &Store, monitor: &str, path: &Path) -> Result<()> {
    let mut settings = store.load_settings()?;
    if !(settings.workspaces.enabled && settings.workspaces.remember) {
        return Ok(());
    }
    let Some(ws) = workspaces_now(runner)?.remove(monitor) else {
        return Ok(());
    };
    if settings.workspaces.wallpapers.get(&ws).map(PathBuf::as_path) != Some(path) {
        settings.workspaces.wallpapers.insert(ws, path.to_path_buf());
        store.save_settings(&settings)?;
    }
    Ok(())
}

/// A workspace for Settings: its name, the monitor showing it (if one is),
/// and its wallpaper (if it has one).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct WorkspaceRow {
    pub name: String,
    pub monitor: Option<String>,
    pub open: bool,
    pub wallpaper: Option<PathBuf>,
}

/// Hyprland's workspaces now and those with a wallpaper, in workspace order
/// (numbers first, by number).
pub fn rows(runner: &dyn Runner, store: &Store) -> Result<Vec<WorkspaceRow>> {
    #[derive(serde::Deserialize)]
    struct Ws {
        name: String,
        #[serde(default)]
        monitor: String,
    }
    let settings = store.load_settings()?;
    let shown = workspaces_now(runner).unwrap_or_default();
    let open: Vec<Ws> = runner
        .run("hyprctl", &["workspaces", "-j"])
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    let mut rows: Vec<WorkspaceRow> = open
        .into_iter()
        .filter(|w| !w.name.is_empty() && !w.name.starts_with("special"))
        .map(|w| WorkspaceRow {
            monitor: shown.iter().find(|(_, ws)| **ws == w.name).map(|(m, _)| m.clone()).or(Some(w.monitor).filter(|m| !m.is_empty())),
            wallpaper: settings.workspaces.wallpapers.get(&w.name).cloned(),
            open: true,
            name: w.name,
        })
        .collect();
    for (name, path) in &settings.workspaces.wallpapers {
        if !rows.iter().any(|r| r.name == *name) {
            rows.push(WorkspaceRow {
                name: name.clone(),
                monitor: None,
                open: false,
                wallpaper: Some(path.clone()),
            });
        }
    }
    rows.sort_by(|a, b| match (a.name.parse::<i64>(), b.name.parse::<i64>()) {
        (Ok(x), Ok(y)) => x.cmp(&y),
        (Ok(_), Err(_)) => std::cmp::Ordering::Less,
        (Err(_), Ok(_)) => std::cmp::Ordering::Greater,
        _ => a.name.cmp(&b.name),
    });
    Ok(rows)
}

/// Set (or with None, clear) a workspace's wallpaper.
pub fn assign(store: &Store, workspace: &str, path: Option<PathBuf>) -> Result<()> {
    let mut settings = store.load_settings()?;
    match path {
        Some(p) => settings.workspaces.wallpapers.insert(workspace.to_string(), p),
        None => settings.workspaces.wallpapers.remove(workspace),
    };
    store.save_settings(&settings)
}

/// One listener per session (the app's and `wallpicker workspaces run`
/// would both apply): held while listening, a name only this user's
/// session sees.
fn claim() -> Option<UnixListener> {
    let runtime = std::env::var("XDG_RUNTIME_DIR").unwrap_or_default();
    let sig = std::env::var("HYPRLAND_INSTANCE_SIGNATURE").unwrap_or_default();
    claim_named(&format!("wallpicker-workspaces:{runtime}:{sig}"))
}

fn claim_named(name: &str) -> Option<UnixListener> {
    use std::os::linux::net::SocketAddrExt;
    let addr = std::os::unix::net::SocketAddr::from_abstract_name(name).ok()?;
    UnixListener::bind_addr(&addr).ok()
}

/// Follow Hyprland's events for as long as `keep_going` says, applying
/// workspace wallpapers; `applied` after each change. Reconnects when
/// Hyprland's socket goes away (a restart); waits while the feature is off,
/// or while another listener has it.
pub fn listen(runner: &dyn Runner, store: &Store, keep_going: &dyn Fn() -> bool, applied: &dyn Fn()) {
    let mut last = HashMap::new();
    let mut held = None;
    while keep_going() {
        if held.is_none() {
            held = claim();
            if held.is_none() {
                std::thread::sleep(Duration::from_secs(3));
                continue;
            }
        }
        let on = store.load_settings().is_ok_and(|s| s.workspaces.enabled);
        let socket = socket_path();
        let (true, Some(socket)) = (on, socket) else {
            last.clear();
            std::thread::sleep(Duration::from_secs(3));
            continue;
        };
        let Ok(stream) = UnixStream::connect(&socket) else {
            std::thread::sleep(Duration::from_secs(3));
            continue;
        };
        // Wake up now and then to notice the feature being turned off.
        let _ = stream.set_read_timeout(Some(Duration::from_secs(3)));
        if sync(runner, store, &mut last).is_ok_and(|a| !a.is_empty()) {
            applied();
        }
        let mut lines = BufReader::new(stream);
        let mut line = String::new();
        loop {
            line.clear();
            match lines.read_line(&mut line) {
                Ok(0) => break, // Hyprland went away.
                Ok(_) => {
                    if is_workspace_event(line.trim_end()) {
                        match sync(runner, store, &mut last) {
                            Ok(a) if !a.is_empty() => applied(),
                            Ok(_) => {}
                            Err(e) => eprintln!("wallpicker: workspaces: {e}"),
                        }
                    }
                }
                Err(e) if matches!(e.kind(), std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut) => {
                    if !keep_going() || !store.load_settings().is_ok_and(|s| s.workspaces.enabled) {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::Dirs;
    use std::cell::RefCell;

    /// Answers `hyprctl monitors -j` with the workspaces in `ws`; records
    /// the rest.
    struct Fake {
        ws: RefCell<Vec<(&'static str, &'static str)>>,
        calls: RefCell<Vec<String>>,
    }

    impl Runner for Fake {
        fn run(&self, program: &str, args: &[&str]) -> Result<String> {
            if args.first() == Some(&"monitors") {
                let list: Vec<String> = self
                    .ws
                    .borrow()
                    .iter()
                    .map(|(m, w)| {
                        format!(
                            r#"{{"name":"{m}","width":1920,"height":1080,"activeWorkspace":{{"id":1,"name":"{w}"}}}}"#
                        )
                    })
                    .collect();
                return Ok(format!("[{}]", list.join(",")));
            }
            self.calls.borrow_mut().push(format!("{program} {}", args.join(" ")));
            Ok(String::new())
        }
    }

    #[test]
    fn one_listener_at_a_time() {
        // Its own name: a running wallpicker holds the session's.
        let name = format!("wallpicker-workspaces-test:{}", std::process::id());
        let first = claim_named(&name).expect("free");
        assert!(claim_named(&name).is_none());
        drop(first);
        assert!(claim_named(&name).is_some());
    }

    #[test]
    fn events_that_matter() {
        assert!(is_workspace_event("workspacev2>>3,3"));
        assert!(is_workspace_event("focusedmon>>DP-2,5"));
        assert!(!is_workspace_event("activewindow>>kitty,~"));
        assert!(!is_workspace_event("openwindow>>abc,1,kitty,x"));
    }

    #[test]
    fn switching_applies_and_remembering_assigns() {
        let tmp = tempfile::tempdir().unwrap();
        let store = Store::new(Dirs::under(tmp.path()));
        let a = tmp.path().join("a.png");
        let b = tmp.path().join("b.png");
        image::RgbImage::new(4, 4).save(&a).unwrap();
        image::RgbImage::new(4, 4).save(&b).unwrap();
        let mut s = store.load_settings().unwrap();
        s.workspaces.enabled = true;
        s.workspaces.wallpapers.insert("3".into(), b.clone());
        store.save_settings(&s).unwrap();
        store.set_current("DP-1", &a).unwrap();

        let fake = Fake {
            ws: RefCell::new(vec![("DP-1", "1"), ("DP-2", "special:term")]),
            calls: RefCell::new(vec![]),
        };
        let mut last = HashMap::new();
        // Workspace 1 has no wallpaper: nothing happens.
        assert!(sync(&fake, &store, &mut last).unwrap().is_empty());
        // Over to 3: b goes on DP-1, without going into the history.
        fake.ws.borrow_mut()[0].1 = "3";
        assert_eq!(sync(&fake, &store, &mut last).unwrap(), vec![("DP-1".to_string(), b.clone())]);
        let state = store.load_state().unwrap();
        assert_eq!(state.current["DP-1"], b);
        assert!(state.history.get("DP-1").is_none_or(Vec::is_empty));
        // No change: no second apply.
        assert!(sync(&fake, &store, &mut last).unwrap().is_empty());

        // Back on 1, a wallpaper put there by hand is remembered for 1.
        fake.ws.borrow_mut()[0].1 = "1";
        remember(&fake, &store, "DP-1", &a).unwrap();
        assert_eq!(store.load_settings().unwrap().workspaces.wallpapers["1"], a);
    }
}
