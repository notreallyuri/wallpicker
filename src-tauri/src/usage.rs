//! What wallpicker is using right now, for Settings → Performance: memory
//! (PSS: shared pages split fairly) of the app and of the WebKit processes it
//! started, and CPU since the last time it was asked.

use std::sync::Mutex;
use std::time::Instant;

use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct Usage {
    /// The app itself: tray, backend, window.
    pub app_mb: u32,
    /// WebKit's web process: the interface.
    pub interface_mb: u32,
    /// WebKit's other helpers (network).
    pub helpers_mb: u32,
    pub total_mb: u32,
    /// % of one core since the previous call (None the first time).
    pub cpu: Option<f32>,
}

fn pss_kb(pid: u32) -> u64 {
    std::fs::read_to_string(format!("/proc/{pid}/smaps_rollup"))
        .ok()
        .and_then(|t| t.lines().find(|l| l.starts_with("Pss:")).and_then(|l| l.split_whitespace().nth(1)?.parse().ok()))
        .unwrap_or(0)
}

/// (parent pid, utime + stime in clock ticks) from /proc/<pid>/stat.
fn stat(pid: u32) -> Option<(u32, u64)> {
    let text = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    // After the command name, which may contain spaces: state, ppid, ...
    let rest: Vec<&str> = text.rsplit_once(')')?.1.split_whitespace().collect();
    Some((rest.get(1)?.parse().ok()?, rest.get(11)?.parse::<u64>().ok()? + rest.get(12)?.parse::<u64>().ok()?))
}

fn name(pid: u32) -> String {
    std::fs::read_to_string(format!("/proc/{pid}/comm")).map(|s| s.trim().to_string()).unwrap_or_default()
}

/// The processes this one started (WebKit's web and network processes).
fn children(me: u32) -> Vec<u32> {
    let Ok(dir) = std::fs::read_dir("/proc") else { return Vec::new() };
    dir.filter_map(|e| e.ok()?.file_name().to_str()?.parse::<u32>().ok())
        .filter(|&p| stat(p).is_some_and(|(ppid, _)| ppid == me))
        .collect()
}

static LAST: Mutex<Option<(Instant, u64)>> = Mutex::new(None);

pub fn usage() -> Usage {
    let me = std::process::id();
    let kids = children(me);
    let (mut interface, mut helpers) = (0, 0);
    for &k in &kids {
        if name(k).starts_with("WebKitWebProces") {
            interface += pss_kb(k);
        } else {
            helpers += pss_kb(k);
        }
    }
    let app = pss_kb(me);
    let ticks: u64 = std::iter::once(me).chain(kids).filter_map(|p| stat(p).map(|s| s.1)).sum();
    let hz = 100.0; // USER_HZ on Linux
    let now = Instant::now();
    let cpu = LAST.lock().ok().and_then(|mut last| {
        let previous = last.replace((now, ticks));
        previous.map(|(t, before)| {
            let secs = now.duration_since(t).as_secs_f64().max(0.001);
            // A child that ended makes the sum drop: no negative usage.
            (ticks.saturating_sub(before) as f64 / hz / secs * 100.0) as f32
        })
    });
    let mb = |kb: u64| (kb / 1024) as u32;
    Usage { app_mb: mb(app), interface_mb: mb(interface), helpers_mb: mb(helpers), total_mb: mb(app + interface + helpers), cpu }
}
