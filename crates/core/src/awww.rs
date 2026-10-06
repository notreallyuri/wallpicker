//! awww (named swww until 2025) as the wallpaper backend: `awww img <image>
//! -o <monitor> --resize <how>` with a transition. Its daemon,
//! `awww-daemon`, draws the wallpapers and remembers them across restarts;
//! when it isn't running, wallpicker starts it and tries again.
//!
//! wallpicker's fit modes map onto awww's resize modes: cover → crop
//! (awww's default), contain → fit, fill → stretch, tile → no (awww doesn't
//! tile: the image is centered at its own size).

use std::path::Path;
use std::time::Duration;

use crate::cmd::Runner;
use crate::error::{Error, Result};
use crate::settings::{FitMode, Transition};

/// The client and daemon commands: awww's, or swww's on an older install.
pub fn programs() -> (&'static str, &'static str) {
    if !on_path("awww") && on_path("swww") {
        ("swww", "swww-daemon")
    } else {
        ("awww", "awww-daemon")
    }
}

fn on_path(name: &str) -> bool {
    std::env::var_os("PATH")
        .is_some_and(|paths| std::env::split_paths(&paths).any(|dir| dir.join(name).is_file()))
}

/// awww's `--resize` for a fit (None: awww's default, crop).
pub fn resize(fit: Option<FitMode>) -> &'static str {
    match fit {
        None | Some(FitMode::Cover) => "crop",
        Some(FitMode::Contain) => "fit",
        Some(FitMode::Fill) => "stretch",
        Some(FitMode::Tile) => "no",
    }
}

/// The arguments to `awww img`.
pub fn img_args(monitor: &str, path: &Path, fit: Option<FitMode>, t: &Transition) -> Vec<String> {
    let seconds = f64::from(t.duration_ms) / 1000.0;
    vec![
        "img".into(),
        "--outputs".into(),
        monitor.into(),
        "--resize".into(),
        resize(fit).into(),
        "--transition-type".into(),
        t.kind.clone(),
        "--transition-duration".into(),
        format!("{seconds}"),
        "--transition-fps".into(),
        t.fps.to_string(),
        path.to_string_lossy().into_owned(),
    ]
}

/// awww's answer when its daemon isn't there: no socket file ("Make sure
/// awww-daemon is running"), or a socket nobody answers (a stale one).
fn daemon_missing(e: &Error) -> bool {
    let msg = e.to_string();
    msg.contains("-daemon is running") || msg.contains("failed to connect to socket")
}

/// Show `path` on `monitor`, starting the daemon first if it isn't running.
pub fn apply(runner: &dyn Runner, monitor: &str, path: &Path, fit: Option<FitMode>, t: &Transition) -> Result<()> {
    apply_with(runner, monitor, path, fit, t, &std::thread::sleep)
}

/// [`apply`] with the waiting for the daemon injectable (tests).
pub fn apply_with(
    runner: &dyn Runner,
    monitor: &str,
    path: &Path,
    fit: Option<FitMode>,
    t: &Transition,
    sleep: &dyn Fn(Duration),
) -> Result<()> {
    let (client, daemon) = programs();
    let args = img_args(monitor, path, fit, t);
    let argv: Vec<&str> = args.iter().map(String::as_str).collect();
    match runner.run(client, &argv) {
        Ok(_) => Ok(()),
        Err(e) if daemon_missing(&e) => {
            runner.spawn(daemon, &[])?;
            // Up to 3 s for it to come up (`query` answers once it has).
            let mut up = false;
            for _ in 0..30 {
                if runner.run(client, &["query"]).is_ok() {
                    up = true;
                    break;
                }
                sleep(Duration::from_millis(100));
            }
            if !up {
                return Err(Error::Command {
                    cmd: daemon.to_string(),
                    msg: format!("started {daemon}, but it didn't come up; start it from Hyprland's autostart"),
                });
            }
            runner.run(client, &argv).map(drop)
        }
        Err(e) => Err(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::collections::VecDeque;

    struct Mock {
        calls: RefCell<Vec<String>>,
        replies: RefCell<VecDeque<Result<String>>>,
    }

    impl Mock {
        fn new(replies: Vec<Result<String>>) -> Mock {
            Mock { calls: RefCell::new(Vec::new()), replies: RefCell::new(replies.into()) }
        }
    }

    impl Runner for Mock {
        fn run(&self, program: &str, args: &[&str]) -> Result<String> {
            self.calls.borrow_mut().push(format!("{program} {}", args.join(" ")));
            self.replies.borrow_mut().pop_front().unwrap_or_else(|| Ok(String::new()))
        }
        fn spawn(&self, program: &str, _args: &[&str]) -> Result<()> {
            self.calls.borrow_mut().push(format!("spawn {program}"));
            Ok(())
        }
    }

    fn missing() -> Error {
        Error::Command {
            cmd: "awww img".into(),
            msg: "Socket file '/run/user/1000/wayland-1-awww-daemon.socket' not found. Make sure awww-daemon is running, and that the --namespace argument matches for the client and the daemon".into(),
        }
    }

    #[test]
    fn maps_fits_and_transition() {
        let t = Transition { kind: "wipe".into(), duration_ms: 1500, fps: 144 };
        let args = img_args("DP-1", Path::new("/w/a.png"), Some(FitMode::Contain), &t).join(" ");
        assert_eq!(
            args,
            "img --outputs DP-1 --resize fit --transition-type wipe --transition-duration 1.5 --transition-fps 144 /w/a.png"
        );
        assert_eq!(resize(None), "crop");
        assert_eq!(resize(Some(FitMode::Fill)), "stretch");
        assert_eq!(resize(Some(FitMode::Tile)), "no");
    }

    #[test]
    fn starts_the_daemon_when_missing() {
        // img fails (no daemon), query fails once, then works, img again.
        let mock = Mock::new(vec![Err(missing()), Err(missing()), Ok(String::new()), Ok(String::new())]);
        let slept = RefCell::new(0);
        apply_with(&mock, "DP-1", Path::new("/w/a.png"), None, &Transition::default(), &|_| *slept.borrow_mut() += 1)
            .unwrap();
        let calls = mock.calls.borrow();
        let (client, daemon) = programs();
        assert!(calls[0].starts_with(&format!("{client} img")));
        assert_eq!(calls[1], format!("spawn {daemon}"));
        assert_eq!(calls[2], format!("{client} query"));
        assert_eq!(calls[3], format!("{client} query"));
        assert!(calls[4].starts_with(&format!("{client} img")));
        assert_eq!(*slept.borrow(), 1);
    }

    #[test]
    fn other_errors_are_reported_as_they_are() {
        let bad = Error::Command { cmd: "awww img".into(), msg: "none of the requested outputs are valid".into() };
        let mock = Mock::new(vec![Err(bad)]);
        let e = apply_with(&mock, "HDMI-9", Path::new("/w/a.png"), None, &Transition::default(), &|_| {}).unwrap_err();
        assert!(e.to_string().contains("none of the requested outputs are valid"));
        assert_eq!(mock.calls.borrow().len(), 1);
    }
}
