//! The app's own data: settings, saved state, the palette, Noctalia's colors;
//! the clipboard; what the interface uses (Settings → Performance).

use wallpicker_core::settings::{Settings, State as SavedState};

use crate::{AppState, CmdResult, blocking, err, theme};

pub async fn settings(state: &AppState) -> CmdResult<Settings> {
    let store = state.store.clone();
    blocking(move || store.load_settings()).await
}

/// What each monitor shows, its history and adjustments.
pub async fn saved_state(state: &AppState) -> CmdResult<SavedState> {
    let store = state.store.clone();
    blocking(move || store.load_state()).await
}

/// The app's colors from the current wallpaper (None: its own).
pub async fn palette(state: &AppState) -> CmdResult<Option<theme::Palette>> {
    let store = state.store.clone();
    crate::rt::spawn_blocking(move || theme::palette(&store)).await.map_err(err)?
}

/// Put `text` on the Wayland clipboard (`wl-copy`, which stays to serve it:
/// it outlives the interface, which exits on close; WebKitGTK's clipboard
/// API wasn't reliable either).
pub async fn copy_text(text: String) -> CmdResult<()> {
    if text.len() > 64 * 1024 {
        return Err("That's too long to copy.".into());
    }
    crate::rt::spawn_blocking(move || {
        use std::io::Write as _;
        let mut child = std::process::Command::new("wl-copy")
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .map_err(|e| format!("Could not run wl-copy (is wl-clipboard installed?): {e}"))?;
        child
            .stdin
            .take()
            .ok_or("wl-copy has no input")?
            .write_all(text.as_bytes())
            .map_err(err)?;
        // wl-copy forks to serve the clipboard; the first process exits soon.
        match child.wait() {
            Ok(s) if s.success() => Ok(()),
            Ok(s) => Err(format!("wl-copy failed ({s})")),
            Err(e) => Err(err(e)),
        }
    })
    .await
    .map_err(err)?
}

/// Whether Noctalia's colors follow the wallpaper (and what they are now).
pub async fn noctalia_colors(state: &AppState) -> CmdResult<theme::NoctaliaColors> {
    let store = state.store.clone();
    crate::rt::spawn_blocking(move || theme::noctalia_colors(&store)).await.map_err(err)
}

/// Noctalia's colors follow the wallpaper, or go back to its scheme before.
pub async fn set_noctalia_colors(state: &AppState, follow: bool) -> CmdResult<theme::NoctaliaColors> {
    let store = state.store.clone();
    crate::rt::spawn_blocking(move || theme::set_noctalia_follow(&store, follow)).await.map_err(err)?
}

/// What the Quickshell interface uses now (Settings → Performance): memory
/// (PSS: shared pages split fairly) of the interface, the process that
/// started `serve`, and of `serve` itself; CPU of both since the last call.
#[derive(serde::Serialize)]
pub struct Usage {
    pub interface_mb: u32,
    pub backend_mb: u32,
    pub total_mb: u32,
    /// % of one core since the previous call (None the first time).
    pub cpu: Option<f32>,
}

pub async fn usage() -> CmdResult<Usage> {
    crate::rt::spawn_blocking(|| {
        let me = std::process::id();
        let parent = std::os::unix::process::parent_id();
        let (interface_mb, backend_mb) = (pss_mb(parent), pss_mb(me));
        Usage { interface_mb, backend_mb, total_mb: interface_mb + backend_mb, cpu: cpu_since_last(&[parent, me]) }
    })
    .await
    .map_err(err)
}

/// A process's proportional memory, in MB (0 if it can't be read).
fn pss_mb(pid: u32) -> u32 {
    let text = std::fs::read_to_string(format!("/proc/{pid}/smaps_rollup")).unwrap_or_default();
    let kb: u64 = text
        .lines()
        .find_map(|l| l.strip_prefix("Pss:"))
        .and_then(|v| v.trim().trim_end_matches("kB").trim().parse().ok())
        .unwrap_or(0);
    (kb / 1024) as u32
}

/// CPU time (user + system, in clock ticks of 1/100 s) of `pids` together.
fn cpu_ticks(pids: &[u32]) -> u64 {
    pids.iter()
        .filter_map(|pid| {
            let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
            // After the command's closing parenthesis: state is field 3, utime 14, stime 15.
            let rest = &stat[stat.rfind(')')? + 2..];
            let f: Vec<&str> = rest.split_whitespace().collect();
            Some(f.get(11)?.parse::<u64>().ok()? + f.get(12)?.parse::<u64>().ok()?)
        })
        .sum()
}

fn cpu_since_last(pids: &[u32]) -> Option<f32> {
    use std::sync::Mutex;
    use std::time::Instant;
    static LAST: Mutex<Option<(Instant, u64)>> = Mutex::new(None);
    let now = (Instant::now(), cpu_ticks(pids));
    let mut last = LAST.lock().unwrap_or_else(|p| p.into_inner());
    let before = last.replace(now);
    let (at, ticks) = before?;
    let secs = now.0.duration_since(at).as_secs_f32();
    (secs > 0.0).then(|| now.1.saturating_sub(ticks) as f32 / 100.0 / secs * 100.0)
}
