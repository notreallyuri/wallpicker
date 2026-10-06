//! Drive hyprpaper through `hyprctl hyprpaper ...`.
//!
//! hyprpaper changed its control interface between releases, so applying a
//! wallpaper tries the direct form first:
//!
//! ```text
//! hyprctl hyprpaper wallpaper "DP-1,/path/to/img.png[,fit_mode]"
//! ```
//!
//! and, if that is refused, falls back to the older two-step form that first
//! has to load the image:
//!
//! ```text
//! hyprctl hyprpaper preload /path/to/img.png
//! hyprctl hyprpaper wallpaper "DP-1,/path/to/img.png"
//! ```
//!
//! If neither works the error carries hyprpaper's own reply, which is usually
//! enough to tell whether hyprpaper isn't running or doesn't know the command.

use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::Serialize;

use crate::cmd::Runner;
use crate::error::{Error, Result};
use crate::monitors::{Monitor, parse_monitors};
use crate::catalog::Catalog;
use crate::settings::{FitMode, Settings, WallpaperBackend};
use crate::store::Store;

/// hyprpaper answers "ok" on success; some versions print nothing.
fn is_ok(reply: &str) -> bool {
    let t = reply.trim();
    t.is_empty() || t.eq_ignore_ascii_case("ok")
}

fn path_str(path: &Path) -> Result<&str> {
    let s = path
        .to_str()
        .ok_or_else(|| Error::Invalid("path is not valid UTF-8".into()))?;
    if !path.is_absolute() {
        return Err(Error::Invalid(format!("path must be absolute: {s}")));
    }
    // Arguments are comma-separated, and newlines would corrupt the request.
    if s.contains(',') || s.contains('\n') || s.contains('\r') {
        return Err(Error::Invalid(format!(
            "paths containing commas or newlines are not supported by hyprpaper: {s:?}"
        )));
    }
    Ok(s)
}

/// The `monitor,path[,fit]` argument.
pub fn wallpaper_arg(monitor: &str, path: &Path, fit: Option<FitMode>) -> Result<String> {
    if monitor.is_empty() || monitor.contains(',') || monitor.chars().any(|c| c.is_control()) {
        return Err(Error::Invalid(format!("invalid monitor name: {monitor:?}")));
    }
    let p = path_str(path)?;
    Ok(match fit {
        Some(f) => format!("{monitor},{p},{}", f.as_str()),
        None => format!("{monitor},{p}"),
    })
}

/// The saved wallpapers as hyprpaper config (`wallpaper { ... }` blocks),
/// for a file the user's `hyprpaper.conf` sources. hyprpaper then shows them
/// whenever it starts, without wallpicker running.
///
/// In hyprlang `#` starts a comment (`##` is a literal one) and `$` expands
/// variables; a path with `$` or a line break can't be written safely, so it's
/// left out (with a comment saying so).
pub fn config_for(state: &crate::settings::State, fit_of: &dyn Fn(&str, &Path) -> Option<FitMode>) -> String {
    let mut out = String::from(
        "# Written by wallpicker: what it last applied to each monitor.\n\
         # It's rewritten on every change, so edit wallpapers in wallpicker instead.\n\
         # Loaded by `source = <this file>` in ~/.config/hypr/hyprpaper.conf.\n",
    );
    for (monitor, path) in &state.current {
        let fit = fit_of(monitor, path);
        let path = path.to_string_lossy();
        let unsafe_monitor = monitor.is_empty() || monitor.chars().any(|c| c.is_control() || c == '#' || c == '$');
        if unsafe_monitor || path.contains('$') || path.contains('\n') || path.contains('\r') {
            out.push_str(&format!("\n# skipped {monitor:?}: its name or path can't be written here\n"));
            continue;
        }
        out.push_str(&format!(
            "\nwallpaper {{\n    monitor = {monitor}\n    path = {}\n",
            path.replace('#', "##")
        ));
        if let Some(fit) = fit {
            out.push_str(&format!("    fit_mode = {}\n", fit.as_str()));
        }
        out.push_str("}\n");
    }
    out
}

/// Monitors Hyprland currently knows about.
pub fn list_monitors(runner: &dyn Runner) -> Result<Vec<Monitor>> {
    let json = runner.run("hyprctl", &["monitors", "-j"])?;
    parse_monitors(&json)
}

/// Set `path` as the wallpaper of one monitor.
pub fn apply(runner: &dyn Runner, monitor: &str, path: &Path, fit: Option<FitMode>) -> Result<()> {
    if !path.is_file() {
        return Err(Error::Invalid(format!("not a file: {}", path.display())));
    }
    let arg = wallpaper_arg(monitor, path, fit)?;

    // Direct form. A failure here (error exit or an error reply) just means we
    // try the older form next.
    let direct = match runner.run("hyprctl", &["hyprpaper", "wallpaper", &arg]) {
        Ok(reply) if is_ok(&reply) => return Ok(()),
        Ok(reply) => reply,
        Err(e) => e.to_string(),
    };

    apply_preloaded(runner, path, &arg).map_err(|e| {
        // Newer hyprpaper has no `preload`, so the direct form's reply is
        // often the useful one.
        let both = Error::Hyprpaper(format!("{direct}; older form: {e}"));
        explain_failure(both, runner.hyprpaper_socket().as_deref())
    })
}

/// The older two-step form: `preload`, then `wallpaper`.
fn apply_preloaded(runner: &dyn Runner, path: &Path, arg: &str) -> Result<()> {
    let p = path_str(path)?;
    let reply = runner.run("hyprctl", &["hyprpaper", "preload", p])?;
    if !is_ok(&reply) {
        return Err(Error::Hyprpaper(reply));
    }
    let reply = runner.run("hyprctl", &["hyprpaper", "wallpaper", arg])?;
    if !is_ok(&reply) {
        return Err(Error::Hyprpaper(reply));
    }
    // The old form keeps every preloaded image in memory. Best effort only.
    let _ = runner.run("hyprctl", &["hyprpaper", "unload", "unused"]);
    Ok(())
}

/// Where hyprpaper's IPC socket should be, if we're in a Hyprland session.
pub fn socket_path() -> Option<PathBuf> {
    let runtime = std::env::var_os("XDG_RUNTIME_DIR")?;
    let instance = std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE")?;
    Some(
        PathBuf::from(runtime)
            .join("hypr")
            .join(instance)
            .join(".hyprpaper.sock"),
    )
}

/// Without hyprpaper running, hyprctl answers every hyprpaper command with
/// "invalid hyprpaper request" or "connection refused", which don't say
/// what's wrong. If the Hyprland instance dir exists but nothing answers on
/// hyprpaper's socket, report that instead. The socket file alone isn't
/// enough: hyprpaper leaves it behind when it exits.
fn explain_failure(err: Error, socket: Option<&Path>) -> Error {
    match socket {
        Some(s) if s.parent().is_some_and(Path::is_dir) && UnixStream::connect(s).is_err() => {
            Error::HyprpaperNotRunning
        }
        _ => err,
    }
}

/// Apply one image to several monitors. Every monitor is attempted; the first
/// error (if any) is returned afterwards.
pub fn apply_to(
    runner: &dyn Runner,
    monitors: &[String],
    path: &Path,
    fit: Option<FitMode>,
) -> Result<()> {
    let mut first_err = None;
    for m in monitors {
        if let Err(e) = apply(runner, m, path, fit) {
            first_err.get_or_insert(e);
        }
    }
    match first_err {
        Some(e) => Err(e),
        None => Ok(()),
    }
}

/// Outcome of [`apply_and_remember`].
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
pub struct ApplyReport {
    /// Monitors that now show the image (and were saved to the state file).
    pub applied: Vec<String>,
    /// (monitor, error message)
    pub failed: Vec<(String, String)>,
}

/// Apply `path` to each monitor and record every success in the state file, so
/// [`restore`] can bring it back after a reboot. Monitors fail independently:
/// one bad output doesn't stop the others or lose their saved state.
pub fn apply_and_remember(
    runner: &dyn Runner,
    store: &Store,
    monitors: &[String],
    path: &Path,
) -> Result<ApplyReport> {
    apply_and_remember_with(runner, store, WallpaperBackend::Hyprpaper, monitors, path)
}

/// What decides each wallpaper's fit, read once: the settings (general and
/// per-monitor fit) and the catalog (each wallpaper's own).
pub struct Fits {
    settings: Settings,
    catalog: Catalog,
}

impl Fits {
    /// Files that can't be read count as defaults: a fit is never worth
    /// failing over.
    pub fn load(store: &Store) -> Fits {
        Fits {
            settings: store.load_settings().unwrap_or_default(),
            catalog: store.load_catalog().unwrap_or_default(),
        }
    }

    pub fn of(&self, monitor: &str, path: &Path) -> Option<FitMode> {
        self.settings.fit_for(monitor, self.catalog.fit(path))
    }
}

/// The chosen wallpaper backend and what it needs besides the image, read
/// once for a batch of monitors.
pub struct Backend {
    kind: WallpaperBackend,
    noctalia_enabled: Option<bool>,
    transition: crate::settings::Transition,
}

impl Backend {
    pub fn new(kind: WallpaperBackend, store: &Store) -> Backend {
        Backend {
            kind,
            noctalia_enabled: match kind {
                WallpaperBackend::Noctalia => crate::noctalia::wallpaper_enabled_on_disk(),
                _ => None,
            },
            transition: store.load_settings().map(|s| s.transition).unwrap_or_default(),
        }
    }

    /// The one in the settings.
    pub fn from_settings(store: &Store) -> Backend {
        Backend::new(store.load_settings().map(|s| s.wallpaper_backend).unwrap_or_default(), store)
    }

    /// Show `path` on `monitor`. With Noctalia, `fit` doesn't apply: its
    /// fill mode is a setting of its own.
    pub fn apply(&self, runner: &dyn Runner, monitor: &str, path: &Path, fit: Option<FitMode>) -> Result<()> {
        match self.kind {
            WallpaperBackend::Hyprpaper => apply(runner, monitor, path, fit),
            WallpaperBackend::Awww => crate::awww::apply(runner, monitor, path, fit, &self.transition),
            WallpaperBackend::Noctalia => crate::noctalia::apply(runner, monitor, path, self.noctalia_enabled),
        }
    }

    /// Whether wallpicker's fit modes mean anything to it.
    pub fn fits(&self) -> bool {
        self.kind != WallpaperBackend::Noctalia
    }
}

/// Other programs drawing wallpapers besides `backend`: two at once fight
/// over the screen (whichever drew last is on top). Names to show, like
/// "hyprpaper" or "Noctalia's wallpaper".
pub fn conflicts(runner: &dyn Runner, backend: WallpaperBackend) -> Vec<String> {
    const DAEMONS: [&str; 6] = ["hyprpaper", "awww-daemon", "swww-daemon", "swaybg", "mpvpaper", "wpaperd"];
    let mine: &[&str] = match backend {
        WallpaperBackend::Hyprpaper => &["hyprpaper"],
        WallpaperBackend::Awww => &["awww-daemon", "swww-daemon"],
        WallpaperBackend::Noctalia => &[],
    };
    let mut found: Vec<String> = DAEMONS
        .iter()
        .filter(|d| !mine.contains(d))
        .filter(|d| runner.run("pgrep", &["-x", d]).is_ok())
        .map(|d| d.to_string())
        .collect();
    if backend != WallpaperBackend::Noctalia && crate::noctalia::wallpaper_enabled_on_disk() == Some(true) {
        found.push("Noctalia's wallpaper".into());
    }
    found
}

/// [`apply_and_remember`] through the chosen backend, each monitor with its
/// fit ([`Fits`]).
pub fn apply_and_remember_with(
    runner: &dyn Runner,
    store: &Store,
    backend: WallpaperBackend,
    monitors: &[String],
    path: &Path,
) -> Result<ApplyReport> {
    if !crate::library::is_supported_image(path) {
        return Err(Error::Invalid(format!(
            "unsupported image type: {}",
            path.display()
        )));
    }
    if monitors.is_empty() {
        return Err(Error::Invalid("no monitor selected".into()));
    }

    // Read once, not per monitor.
    let backend = Backend::new(backend, store);
    let fits = Fits::load(store);
    let apply_one = |m: &str| backend.apply(runner, m, path, fits.of(m, path));

    let mut report = ApplyReport::default();
    for m in monitors {
        match apply_one(m) {
            Ok(()) => match store.set_current(m, path) {
                Ok(()) => report.applied.push(m.clone()),
                // The wallpaper is on screen; only the bookkeeping failed.
                Err(e) => report
                    .failed
                    .push((m.clone(), format!("applied, but could not save state: {e}"))),
            },
            Err(e) => report.failed.push((m.clone(), e.to_string())),
        }
    }
    Ok(report)
}

/// Put back what each of `monitors` showed before ([`Store::previous`]),
/// with its fit; monitors with nothing earlier are reported as failed.
pub fn apply_previous(runner: &dyn Runner, store: &Store, backend: WallpaperBackend, monitors: &[String]) -> Result<ApplyReport> {
    let fits = Fits::load(store);
    let backend = Backend::new(backend, store);
    let mut report = ApplyReport::default();
    for m in monitors {
        let Some(path) = store.previous(m)? else {
            report.failed.push((m.clone(), "nothing earlier to go back to".into()));
            continue;
        };
        let applied = backend.apply(runner, m, &path, fits.of(m, &path));
        match applied.and_then(|()| store.went_back(m, &path)) {
            Ok(()) => report.applied.push(m.clone()),
            Err(e) => report.failed.push((m.clone(), e.to_string())),
        }
    }
    Ok(report)
}

/// Outcome of [`restore`].
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
pub struct RestoreReport {
    pub applied: Vec<String>,
    /// (monitor, reason)
    pub skipped: Vec<(String, String)>,
}

/// Re-apply the saved wallpapers. Meant to run at login, where hyprpaper may
/// still be starting, so each monitor is retried `attempts` times, waiting
/// `delay` between tries (via `sleep`, so tests don't actually wait).
pub fn restore_with(
    runner: &dyn Runner,
    store: &Store,
    attempts: u32,
    delay: Duration,
    sleep: &dyn Fn(Duration),
) -> Result<RestoreReport> {
    let state = store.load_state()?;
    let fits = Fits::load(store);
    let backend = Backend::from_settings(store);
    let monitors = list_monitors(runner)?;

    let mut report = RestoreReport::default();
    for (name, path) in &state.current {
        if !monitors.iter().any(|m| &m.name == name) {
            report
                .skipped
                .push((name.clone(), "monitor not connected".into()));
            continue;
        }
        if !path.is_file() {
            report
                .skipped
                .push((name.clone(), format!("file is gone: {}", path.display())));
            continue;
        }

        let mut last_err = None;
        let mut done = false;
        for attempt in 0..attempts.max(1) {
            match backend.apply(runner, name, path, fits.of(name, path)) {
                Ok(()) => {
                    done = true;
                    break;
                }
                Err(e) => {
                    last_err = Some(e);
                    if attempt + 1 < attempts.max(1) {
                        sleep(delay);
                    }
                }
            }
        }
        if done {
            report.applied.push(name.clone());
        } else if let Some(e) = last_err {
            report.skipped.push((name.clone(), e.to_string()));
        }
    }
    Ok(report)
}

/// Apply again what's on the monitors that `which(monitor, path)` picks,
/// with their fit as it is now: after a fit changes, so it shows right away.
/// Through the configured backend; callers skip Noctalia, which keeps its own
/// fill mode ([`Backend::fits`]). Monitors that aren't
/// connected, or whose file is gone, are left alone.
pub fn reapply(runner: &dyn Runner, store: &Store, which: &dyn Fn(&str, &Path) -> bool) -> Result<ApplyReport> {
    let state = store.load_state()?;
    let fits = Fits::load(store);
    let backend = Backend::from_settings(store);
    let connected = list_monitors(runner)?;
    let mut report = ApplyReport::default();
    for (m, path) in &state.current {
        if !which(m, path) || !connected.iter().any(|c| &c.name == m) || !path.is_file() {
            continue;
        }
        match backend.apply(runner, m, path, fits.of(m, path)) {
            Ok(()) => report.applied.push(m.clone()),
            Err(e) => report.failed.push((m.clone(), e.to_string())),
        }
    }
    Ok(report)
}

/// [`restore_with`] using real sleeping.
pub fn restore(
    runner: &dyn Runner,
    store: &Store,
    attempts: u32,
    delay: Duration,
) -> Result<RestoreReport> {
    restore_with(runner, store, attempts, delay, &std::thread::sleep)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::Dirs;
    use std::cell::RefCell;
    use std::collections::VecDeque;
    use std::fs;

    /// Replays scripted replies and records every call.
    struct Mock {
        calls: RefCell<Vec<Vec<String>>>,
        replies: RefCell<VecDeque<Result<String>>>,
        /// Reply used once the script runs out.
        default: String,
    }

    impl Mock {
        fn new(replies: Vec<Result<String>>) -> Mock {
            Mock {
                calls: RefCell::new(Vec::new()),
                replies: RefCell::new(replies.into()),
                default: "ok".into(),
            }
        }
        fn calls(&self) -> Vec<String> {
            self.calls.borrow().iter().map(|c| c.join(" ")).collect()
        }
    }

    impl Runner for Mock {
        fn run(&self, program: &str, args: &[&str]) -> Result<String> {
            let mut call = vec![program.to_string()];
            call.extend(args.iter().map(|a| a.to_string()));
            self.calls.borrow_mut().push(call);
            self.replies
                .borrow_mut()
                .pop_front()
                .unwrap_or_else(|| Ok(self.default.clone()))
        }
    }

    fn cmd_err(msg: &str) -> Error {
        Error::Command {
            cmd: "hyprctl".into(),
            msg: msg.into(),
        }
    }

    fn temp_image(dir: &Path, name: &str) -> PathBuf {
        let p = dir.join(name);
        fs::write(&p, b"img").unwrap();
        p
    }

    #[test]
    fn arg_format() {
        let p = Path::new("/pics/a.png");
        assert_eq!(wallpaper_arg("DP-1", p, None).unwrap(), "DP-1,/pics/a.png");
        assert_eq!(
            wallpaper_arg("DP-1", p, Some(FitMode::Cover)).unwrap(),
            "DP-1,/pics/a.png,cover"
        );
    }

    #[test]
    fn arg_rejects_bad_input() {
        let ok = Path::new("/pics/a.png");
        assert!(wallpaper_arg("", ok, None).is_err());
        assert!(wallpaper_arg("DP,1", ok, None).is_err());
        assert!(wallpaper_arg("DP-1\n", ok, None).is_err());
        assert!(wallpaper_arg("DP-1", Path::new("/pics/a,b.png"), None).is_err());
        assert!(wallpaper_arg("DP-1", Path::new("/pics/a\nb.png"), None).is_err());
        assert!(wallpaper_arg("DP-1", Path::new("relative.png"), None).is_err());
    }

    #[test]
    fn direct_form_is_one_call() {
        let tmp = tempfile::tempdir().unwrap();
        let img = temp_image(tmp.path(), "a.png");
        let mock = Mock::new(vec![Ok("ok".into())]);
        apply(&mock, "DP-1", &img, None).unwrap();
        assert_eq!(
            mock.calls(),
            vec![format!(
                "hyprctl hyprpaper wallpaper DP-1,{}",
                img.display()
            )]
        );
    }

    #[test]
    fn empty_reply_counts_as_success() {
        let tmp = tempfile::tempdir().unwrap();
        let img = temp_image(tmp.path(), "a.png");
        let mock = Mock::new(vec![Ok(String::new())]);
        apply(&mock, "DP-1", &img, None).unwrap();
        assert_eq!(mock.calls().len(), 1);
    }

    #[test]
    fn falls_back_to_preload_then_wallpaper() {
        let tmp = tempfile::tempdir().unwrap();
        let img = temp_image(tmp.path(), "a.png");
        // Old hyprpaper: the direct form says the image isn't preloaded.
        let mock = Mock::new(vec![
            Ok("wallpaper failed (not preloaded)".into()),
            Ok("ok".into()), // preload
            Ok("ok".into()), // wallpaper
            Ok("ok".into()), // unload unused
        ]);
        apply(&mock, "DP-1", &img, None).unwrap();
        let calls = mock.calls();
        assert_eq!(calls.len(), 4);
        assert!(calls[1].starts_with("hyprctl hyprpaper preload "));
        assert!(calls[2].starts_with("hyprctl hyprpaper wallpaper DP-1,"));
        assert_eq!(calls[3], "hyprctl hyprpaper unload unused");
    }

    #[test]
    fn error_exit_on_direct_form_also_falls_back() {
        let tmp = tempfile::tempdir().unwrap();
        let img = temp_image(tmp.path(), "a.png");
        let mock = Mock::new(vec![Err(cmd_err("boom"))]);
        apply(&mock, "DP-1", &img, None).unwrap();
        assert_eq!(mock.calls().len(), 4);
    }

    #[test]
    fn reports_hyprpapers_reply_when_everything_fails() {
        let tmp = tempfile::tempdir().unwrap();
        let img = temp_image(tmp.path(), "a.png");
        let mock = Mock::new(vec![
            Ok("nope".into()),
            Ok("preload failed: bad image".into()),
        ]);
        let err = apply(&mock, "DP-1", &img, None).unwrap_err();
        assert!(err.to_string().contains("preload failed: bad image"));
    }

    #[test]
    fn failure_keeps_the_direct_forms_reply() {
        let tmp = tempfile::tempdir().unwrap();
        let img = temp_image(tmp.path(), "a.png");
        let mock = Mock::new(vec![
            Ok("invalid fit mode".into()),
            Err(cmd_err("invalid hyprpaper request")),
        ]);
        let err = apply(&mock, "DP-1", &img, None).unwrap_err().to_string();
        assert!(err.contains("invalid fit mode"));
        assert!(err.contains("invalid hyprpaper request"));
    }

    #[test]
    fn dead_hyprpaper_is_reported_as_not_running() {
        let tmp = tempfile::tempdir().unwrap();
        let sock = tmp.path().join(".hyprpaper.sock");
        let not_running = |sock: &Path| {
            let err = explain_failure(Error::Hyprpaper("invalid request".into()), Some(sock));
            matches!(err, Error::HyprpaperNotRunning)
        };

        // No socket file.
        assert!(not_running(&sock));

        // Socket file left behind by an exited hyprpaper.
        drop(std::os::unix::net::UnixListener::bind(&sock).unwrap());
        assert!(sock.exists());
        assert!(not_running(&sock));

        // Something listening: hyprpaper is up, keep the original error.
        fs::remove_file(&sock).unwrap();
        let _listener = std::os::unix::net::UnixListener::bind(&sock).unwrap();
        assert!(!not_running(&sock));

        // Not in a Hyprland session (no instance dir): don't guess.
        let elsewhere = tmp.path().join("nope").join(".hyprpaper.sock");
        assert!(!not_running(&elsewhere));
        let err = explain_failure(Error::Hyprpaper("x".into()), None);
        assert_eq!(err.to_string(), "hyprpaper: x");
    }

    #[test]
    fn missing_file_never_reaches_hyprctl() {
        let mock = Mock::new(vec![]);
        let err = apply(&mock, "DP-1", Path::new("/no/such/file.png"), None).unwrap_err();
        assert!(err.to_string().contains("not a file"));
        assert!(mock.calls().is_empty());
    }

    #[test]
    fn apply_to_tries_every_monitor_and_reports_the_first_error() {
        let tmp = tempfile::tempdir().unwrap();
        let img = temp_image(tmp.path(), "a.png");
        // DP-1: direct form fails, preload fails -> error.
        // DP-2: direct form succeeds.
        let mock = Mock::new(vec![
            Ok("bad".into()),
            Ok("preload failed".into()),
            Ok("ok".into()),
        ]);
        let monitors = vec!["DP-1".to_string(), "DP-2".to_string()];
        let err = apply_to(&mock, &monitors, &img, None).unwrap_err();
        assert!(err.to_string().contains("preload failed"));
        let calls = mock.calls();
        assert!(calls.last().unwrap().contains("DP-2"));
    }

    #[test]
    fn apply_and_remember_saves_only_successes() {
        let tmp = tempfile::tempdir().unwrap();
        let store = Store::new(Dirs::under(tmp.path()));
        let img = temp_image(tmp.path(), "a.png");

        // DP-1 fails completely, DP-2 works.
        let mock = Mock::new(vec![
            Ok("bad".into()),
            Ok("preload failed".into()),
            Ok("ok".into()),
        ]);
        let monitors = vec!["DP-1".to_string(), "DP-2".to_string()];
        let report = apply_and_remember(&mock, &store, &monitors, &img).unwrap();

        assert_eq!(report.applied, vec!["DP-2".to_string()]);
        assert_eq!(report.failed.len(), 1);
        assert_eq!(report.failed[0].0, "DP-1");

        let state = store.load_state().unwrap();
        assert_eq!(state.current.len(), 1);
        assert_eq!(state.current["DP-2"], img);
    }

    #[test]
    fn apply_and_remember_validates_input_first() {
        let tmp = tempfile::tempdir().unwrap();
        let store = Store::new(Dirs::under(tmp.path()));
        let txt = tmp.path().join("notes.txt");
        fs::write(&txt, b"x").unwrap();
        let img = temp_image(tmp.path(), "a.png");
        let mock = Mock::new(vec![]);

        let m = vec!["DP-1".to_string()];
        assert!(apply_and_remember(&mock, &store, &m, &txt).is_err());
        assert!(apply_and_remember(&mock, &store, &[], &img).is_err());
        assert!(mock.calls().is_empty());
    }

    #[test]
    fn config_has_one_block_per_monitor() {
        let mut state = crate::settings::State::default();
        state.current.insert("DP-1".into(), PathBuf::from("/w/a.png"));
        state.current.insert("DP-2".into(), PathBuf::from("/w/my #1 pick.jpg"));
        state.current.insert("HDMI-A-1".into(), PathBuf::from("/w/$HOME.png"));

        // DP-1 contained, the others with hyprpaper's default.
        let conf = config_for(&state, &|m, _| (m == "DP-1").then_some(FitMode::Contain));
        assert!(conf.contains("wallpaper {\n    monitor = DP-1\n    path = /w/a.png\n    fit_mode = contain\n}"));
        // `#` would start a comment; `##` is a literal one.
        assert!(conf.contains("path = /w/my ##1 pick.jpg"));
        // `$` would expand a variable: left out, with a note.
        assert!(!conf.contains("$HOME.png\n    "));
        assert!(conf.contains("# skipped \"HDMI-A-1\""));

        assert!(conf.contains("path = /w/my ##1 pick.jpg\n}"));
        // No fit mode: hyprpaper's own default.
        assert!(!config_for(&state, &|_, _| None).contains("fit_mode"));
        // Nothing applied yet: just the header, still a valid file.
        let empty = config_for(&crate::settings::State::default(), &|_, _| None);
        assert!(empty.lines().all(|l| l.starts_with('#')));
    }

    #[test]
    fn each_monitor_gets_its_fit_and_reapply_follows_changes() {
        let tmp = tempfile::tempdir().unwrap();
        let store = Store::new(Dirs::under(tmp.path()));
        let img = temp_image(tmp.path(), "a.png");
        let mut settings = Settings::default();
        settings.monitor_fits.insert("DP-2".into(), FitMode::Contain);
        store.save_settings(&settings).unwrap();

        let mock = Mock::new(vec![]);
        let monitors = vec!["DP-1".to_string(), "DP-2".to_string()];
        apply_and_remember(&mock, &store, &monitors, &img).unwrap();
        let p = img.display();
        assert_eq!(
            mock.calls(),
            [format!("hyprctl hyprpaper wallpaper DP-1,{p}"), format!("hyprctl hyprpaper wallpaper DP-2,{p},contain")]
        );
        let conf = std::fs::read_to_string(store.hyprpaper_conf_path()).unwrap();
        assert!(conf.contains("monitor = DP-2\n    path = ") && conf.contains("fit_mode = contain"));

        // The wallpaper's own fit wins on both; the config follows at once.
        store.update_catalog(|c| c.set_fit(&img, Some(FitMode::Tile))).unwrap();
        let conf = std::fs::read_to_string(store.hyprpaper_conf_path()).unwrap();
        assert_eq!(conf.matches("fit_mode = tile").count(), 2);

        // Re-applying only DP-1 (connected, file there) uses the new fit.
        let mock = Mock::new(vec![Ok(MONITORS_JSON.into())]);
        let report = reapply(&mock, &store, &|m, _| m == "DP-1").unwrap();
        assert_eq!(report.applied, ["DP-1"]);
        assert_eq!(mock.calls()[1], format!("hyprctl hyprpaper wallpaper DP-1,{p},tile"));
    }

    #[test]
    fn previous_goes_back_through_the_history() {
        let tmp = tempfile::tempdir().unwrap();
        let store = Store::new(Dirs::under(tmp.path()));
        let (a, b, c) = (temp_image(tmp.path(), "a.png"), temp_image(tmp.path(), "b.png"), temp_image(tmp.path(), "c.png"));
        let m = vec!["DP-1".to_string()];
        for p in [&a, &b, &c] {
            apply_and_remember(&Mock::new(vec![]), &store, &m, p).unwrap();
        }
        assert_eq!(store.load_state().unwrap().history["DP-1"], vec![a.clone(), b.clone()]);

        // Back to b, then to a: going back doesn't record.
        let mock = Mock::new(vec![]);
        let r = apply_previous(&mock, &store, WallpaperBackend::Hyprpaper, &m).unwrap();
        assert_eq!(r.applied, ["DP-1"]);
        assert_eq!(mock.calls(), [format!("hyprctl hyprpaper wallpaper DP-1,{}", b.display())]);
        assert_eq!(store.load_state().unwrap().current["DP-1"], b);
        apply_previous(&Mock::new(vec![]), &store, WallpaperBackend::Hyprpaper, &m).unwrap();
        assert_eq!(store.load_state().unwrap().current["DP-1"], a);
        // Nothing earlier: reported, nothing run.
        let mock = Mock::new(vec![]);
        let r = apply_previous(&mock, &store, WallpaperBackend::Hyprpaper, &m).unwrap();
        assert!(r.applied.is_empty() && r.failed[0].1.contains("nothing earlier"));
        assert!(mock.calls().is_empty());

        // A new apply records again; a file that's gone is skipped; a
        // trashed one leaves the history.
        apply_and_remember(&Mock::new(vec![]), &store, &m, &c).unwrap();
        apply_and_remember(&Mock::new(vec![]), &store, &m, &b).unwrap();
        assert_eq!(store.load_state().unwrap().history["DP-1"], vec![a.clone(), c.clone()]);
        fs::remove_file(&c).unwrap();
        assert_eq!(store.previous("DP-1").unwrap(), Some(a.clone()));
        store.forget_current(&a).unwrap();
        assert_eq!(store.previous("DP-1").unwrap(), None);
    }

    const MONITORS_JSON: &str = r#"[
      {"name":"DP-1","x":0,"y":0,"width":1920,"height":1080},
      {"name":"DP-2","x":1920,"y":0,"width":1920,"height":1080}
    ]"#;

    #[test]
    fn restore_applies_saved_wallpapers_and_skips_stale_entries() {
        let tmp = tempfile::tempdir().unwrap();
        let store = Store::new(Dirs::under(tmp.path()));
        let a = temp_image(tmp.path(), "a.png");
        let b = temp_image(tmp.path(), "b.png");
        store.set_current("DP-1", &a).unwrap();
        store.set_current("DP-2", &b).unwrap();
        store.set_current("HDMI-A-1", &a).unwrap(); // not connected
        fs::remove_file(&b).unwrap(); // file vanished

        let mock = Mock::new(vec![Ok(MONITORS_JSON.into())]);
        let report = restore_with(&mock, &store, 3, Duration::ZERO, &|_| {}).unwrap();

        assert_eq!(report.applied, vec!["DP-1".to_string()]);
        assert_eq!(report.skipped.len(), 2);
        assert!(
            report
                .skipped
                .iter()
                .any(|(m, why)| m == "DP-2" && why.contains("file is gone"))
        );
        assert!(
            report
                .skipped
                .iter()
                .any(|(m, why)| m == "HDMI-A-1" && why.contains("not connected"))
        );
    }

    #[test]
    fn restore_retries_until_hyprpaper_is_up() {
        let tmp = tempfile::tempdir().unwrap();
        let store = Store::new(Dirs::under(tmp.path()));
        let a = temp_image(tmp.path(), "a.png");
        store.set_current("DP-1", &a).unwrap();

        // monitors ok; then hyprpaper unreachable for the first attempt (direct
        // form and the fallback preload both fail), second attempt succeeds.
        let mock = Mock::new(vec![
            Ok(MONITORS_JSON.into()),
            Err(cmd_err("hyprpaper not running")),
            Err(cmd_err("hyprpaper not running")),
        ]);
        let sleeps = RefCell::new(0u32);
        let report = restore_with(&mock, &store, 5, Duration::from_millis(1), &|_| {
            *sleeps.borrow_mut() += 1;
        })
        .unwrap();

        assert_eq!(report.applied, vec!["DP-1".to_string()]);
        assert_eq!(*sleeps.borrow(), 1);
    }

    #[test]
    fn restore_gives_up_after_the_attempt_limit() {
        let tmp = tempfile::tempdir().unwrap();
        let store = Store::new(Dirs::under(tmp.path()));
        let a = temp_image(tmp.path(), "a.png");
        store.set_current("DP-1", &a).unwrap();

        let mut script = vec![Ok(MONITORS_JSON.to_string())];
        for _ in 0..20 {
            script.push(Err(cmd_err("down")));
        }
        let mock = Mock::new(script);
        let report = restore_with(&mock, &store, 2, Duration::ZERO, &|_| {}).unwrap();
        assert!(report.applied.is_empty());
        assert_eq!(report.skipped.len(), 1);
        assert!(report.skipped[0].1.contains("down"));
    }
}
