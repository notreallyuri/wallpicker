//! Talking to `wallpicker daemon` over its socket (abstract, named for the
//! session): one JSON line asked, one answered, `{"error": "..."}` when it
//! couldn't. Its renderer draws the wallpapers for the wallpicker backend.

use std::io::{BufRead, BufReader, Write};
use std::os::linux::net::SocketAddrExt;
use std::os::unix::net::{SocketAddr, UnixStream};
use std::path::Path;
use std::time::Duration;

use serde_json::{Value, json};

use crate::cmd::Runner;
use crate::error::{Error, Result};
use crate::settings::FitMode;

/// This session's daemon socket: one per Wayland session and Hyprland
/// instance.
pub fn address() -> std::io::Result<SocketAddr> {
    let runtime = std::env::var("XDG_RUNTIME_DIR").unwrap_or_default();
    let sig = std::env::var("HYPRLAND_INSTANCE_SIGNATURE").unwrap_or_default();
    SocketAddr::from_abstract_name(format!("wallpicker-daemon:{runtime}:{sig}"))
}

/// Ask the daemon and wait for its answer: [`Error::DaemonNotRunning`]
/// when nobody's listening.
pub fn ask(request: &Value) -> Result<Value> {
    let mut stream = UnixStream::connect_addr(&address()?).map_err(|e| match e.kind() {
        std::io::ErrorKind::ConnectionRefused | std::io::ErrorKind::NotFound => Error::DaemonNotRunning,
        _ => e.into(),
    })?;
    // Decoding and fitting a big picture takes a moment; a stuck daemon
    // shouldn't hang the picker.
    stream.set_read_timeout(Some(Duration::from_secs(30)))?;
    writeln!(stream, "{request}")?;
    let mut line = String::new();
    BufReader::new(stream).read_line(&mut line)?;
    if line.is_empty() {
        return Err(Error::Invalid("wallpicker daemon closed the connection without answering".into()));
    }
    let answer: Value = serde_json::from_str(&line)?;
    match answer.get("error").and_then(Value::as_str) {
        Some(e) => Err(Error::Invalid(format!("wallpicker daemon: {e}"))),
        None => Ok(answer),
    }
}

/// Show `path` on `monitor` through the daemon's renderer, starting the
/// daemon first if it isn't running.
pub fn show(runner: &dyn Runner, monitor: &str, path: &Path, fit: Option<FitMode>) -> Result<()> {
    show_with(runner, monitor, path, fit, &ask, &std::thread::sleep)
}

/// [`show`] with the socket and the waiting injectable (tests).
pub fn show_with(
    runner: &dyn Runner,
    monitor: &str,
    path: &Path,
    fit: Option<FitMode>,
    ask: &dyn Fn(&Value) -> Result<Value>,
    sleep: &dyn Fn(Duration),
) -> Result<()> {
    let request = json!({ "cmd": "show", "monitor": monitor, "path": path, "fit": fit });
    match ask(&request) {
        Err(Error::DaemonNotRunning) => {}
        other => return other.map(drop),
    }
    // This program is wallpicker (the command line, the picker's backend).
    let exe = std::env::current_exe().map_or_else(|_| "wallpicker".into(), |p| p.to_string_lossy().into_owned());
    runner.spawn(&exe, &["daemon"])?;
    // Up to 3 s for it to come up.
    for _ in 0..30 {
        match ask(&json!({ "cmd": "status" })) {
            Err(Error::DaemonNotRunning) => sleep(Duration::from_millis(100)),
            _ => return ask(&request).map(drop),
        }
    }
    Err(Error::Invalid("started wallpicker daemon, but it didn't come up; start it from Hyprland's autostart".into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::collections::VecDeque;

    struct Spawns(RefCell<Vec<String>>);

    impl Runner for Spawns {
        fn run(&self, program: &str, _args: &[&str]) -> Result<String> {
            panic!("ran {program}")
        }
        fn spawn(&self, _program: &str, args: &[&str]) -> Result<()> {
            self.0.borrow_mut().push(args.join(" "));
            Ok(())
        }
    }

    /// Answers in turn, recording what was asked.
    fn script(answers: Vec<Result<Value>>) -> (RefCell<Vec<String>>, RefCell<VecDeque<Result<Value>>>) {
        (RefCell::new(Vec::new()), RefCell::new(answers.into()))
    }

    #[test]
    fn starts_the_daemon_when_missing() {
        let (asked, answers) = script(vec![
            Err(Error::DaemonNotRunning),
            Err(Error::DaemonNotRunning),
            Ok(json!({ "ok": true })),
            Ok(json!({ "ok": true })),
        ]);
        let ask = |r: &Value| {
            asked.borrow_mut().push(r["cmd"].as_str().unwrap().to_string());
            answers.borrow_mut().pop_front().unwrap()
        };
        let runner = Spawns(RefCell::new(Vec::new()));
        let slept = RefCell::new(0);
        show_with(&runner, "DP-1", Path::new("/w/a.png"), Some(FitMode::Contain), &ask, &|_| *slept.borrow_mut() += 1)
            .unwrap();
        assert_eq!(*asked.borrow(), ["show", "status", "status", "show"]);
        assert_eq!(*runner.0.borrow(), ["daemon"]);
        assert_eq!(*slept.borrow(), 1);
    }

    #[test]
    fn its_errors_are_reported_as_they_are() {
        let (_, answers) = script(vec![Err(Error::Invalid("wallpicker daemon: no monitor named HDMI-9".into()))]);
        let ask = |_: &Value| answers.borrow_mut().pop_front().unwrap();
        let runner = Spawns(RefCell::new(Vec::new()));
        let e = show_with(&runner, "HDMI-9", Path::new("/w/a.png"), None, &ask, &|_| {}).unwrap_err();
        assert!(e.to_string().contains("no monitor named HDMI-9"));
        assert!(runner.0.borrow().is_empty());
    }
}
