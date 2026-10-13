//! The wallpaper renderer as the picker shows it: what plays on each monitor
//! (the daemon's status), and what the daemon and its decoders use (GPU,
//! CPU, memory) as rates since the previous poll.

mod gpu;

use std::collections::BTreeMap;
use std::sync::Mutex;
use std::time::Instant;

use serde::Serialize;
use serde_json::{Value, json};

use crate::{CmdResult, err};

/// A monitor's animation, while it shows one.
#[derive(Debug, Serialize, PartialEq)]
pub struct Monitor {
    /// playing, paused, still (playback off when applied), unavailable (no
    /// hardware EGL), frozen (a decoding error), preparing or replacing.
    pub state: String,
    /// Presentations per second since the previous poll.
    pub fps: Option<f32>,
    /// Why it's a still, or what stopped it.
    pub reason: Option<String>,
}

#[derive(Debug, Default, Serialize)]
pub struct Playback {
    /// The daemon is running and draws.
    pub running: bool,
    pub paused: bool,
    pub monitors: BTreeMap<String, Monitor>,
}

/// What each monitor plays.
pub async fn playback() -> CmdResult<Playback> {
    crate::rt::spawn_blocking(|| match daemon_status() {
        Some(status) => playback_of(&status, Instant::now()),
        None => Playback::default(),
    })
    .await
    .map_err(err)
}

fn playback_of(status: &Value, now: Instant) -> Playback {
    // monitor -> (its presentation's output generation, presentations, when).
    static LAST: Mutex<BTreeMap<String, (u64, u64, Instant)>> = Mutex::new(BTreeMap::new());
    let mut last = LAST.lock().unwrap_or_else(|p| p.into_inner());
    let mut monitors = BTreeMap::new();
    for screen in status["screens"].as_array().into_iter().flatten() {
        let (Some(name), Some(state)) = (screen["monitor"].as_str(), screen["playback"]["state"].as_str()) else {
            continue;
        };
        let playback = &screen["playback"];
        let fps = match (playback["output_generation"].as_u64(), playback["presentations"].as_u64()) {
            (Some(generation), Some(presentations)) => {
                let before = last.insert(name.to_string(), (generation, presentations, now));
                before
                    .filter(|(g, p, _)| *g == generation && *p <= presentations)
                    .and_then(|(_, p, at)| {
                        let secs = now.duration_since(at).as_secs_f32();
                        (secs > 0.0).then(|| (presentations - p) as f32 / secs)
                    })
            }
            _ => None,
        };
        let reason = playback["reason"].as_str().or(playback["error"].as_str()).map(String::from);
        monitors.insert(
            name.to_string(),
            Monitor {
                state: state.to_string(),
                fps,
                reason,
            },
        );
    }
    Playback {
        running: status["screens"].is_array(),
        paused: status["playback_paused"].as_bool().unwrap_or(false),
        monitors,
    }
}

#[derive(Debug, Default, Serialize)]
pub struct Usage {
    pub running: bool,
    /// The daemon's share of the GPU's time (all its engines), in percent.
    pub gpu: Option<f32>,
    /// False: this GPU driver doesn't report what a program uses.
    pub gpu_reported: bool,
    /// Memory region ("vram", "gtt"...) -> MB the daemon holds on the GPU.
    pub gpu_memory_mb: BTreeMap<String, f32>,
    /// The whole GPU's busy percentage, every program's together (amdgpu).
    pub gpu_busy: Option<u32>,
    /// Percent of one core.
    pub cpu: Option<f32>,
    /// Its own memory, and the libraries mapped from disk (the GPU driver's
    /// mostly, shared with other programs).
    pub memory_mb: f32,
    pub libraries_mb: f32,
    /// Animation decoders: separate processes.
    pub decoders: usize,
    pub decoders_cpu: Option<f32>,
    pub decoders_memory_mb: f32,
}

/// The daemon's and its decoders' use, since the previous poll.
pub async fn usage() -> CmdResult<Usage> {
    crate::rt::spawn_blocking(|| {
        let Some(pid) = daemon_status().and_then(|s| s["pid"].as_u64()).map(|p| p as u32) else {
            return Usage::default();
        };
        usage_of(pid)
    })
    .await
    .map_err(err)
}

struct Sample {
    at: Instant,
    daemon: u32,
    gpu_ns: u64,
    /// pid -> CPU clock ticks.
    ticks: BTreeMap<u32, u64>,
}

fn usage_of(pid: u32) -> Usage {
    static LAST: Mutex<Option<Sample>> = Mutex::new(None);
    let clients = gpu::clients(pid);
    let decoders = decoders(pid);
    let ticks: BTreeMap<u32, u64> = std::iter::once(pid)
        .chain(decoders.iter().copied())
        .map(|p| (p, super::app::cpu_ticks(&[p])))
        .collect();
    let now = Sample {
        at: Instant::now(),
        daemon: pid,
        gpu_ns: clients.iter().map(|c| c.engine_ns).sum(),
        ticks,
    };
    let mut last = LAST.lock().unwrap_or_else(|p| p.into_inner());
    let before = last.take().filter(|b| b.daemon == pid);
    let secs = before.as_ref().map(|b| now.at.duration_since(b.at).as_secs_f32()).filter(|s| *s > 0.0);
    // A decoder started since counts from its start: it's that recent.
    let cpu = |pids: &[u32]| -> Option<f32> {
        let (before, secs) = (before.as_ref()?, secs?);
        let ticks: u64 = pids
            .iter()
            .map(|p| now.ticks[p].saturating_sub(before.ticks.get(p).copied().unwrap_or(0)))
            .sum();
        Some(ticks as f32 / 100.0 / secs * 100.0)
    };
    let mut memory = BTreeMap::new();
    for client in &clients {
        for (region, bytes) in &client.memory {
            *memory.entry(region.clone()).or_insert(0.0) += *bytes as f32 / 1024.0 / 1024.0;
        }
    }
    let (memory_mb, libraries_mb) = rss_mb(pid);
    let usage = Usage {
        running: true,
        gpu: before
            .as_ref()
            .zip(secs)
            .filter(|_| !clients.is_empty())
            .map(|(b, secs)| now.gpu_ns.saturating_sub(b.gpu_ns) as f32 / 1e9 / secs * 100.0),
        gpu_reported: !clients.is_empty(),
        gpu_memory_mb: memory,
        gpu_busy: clients.iter().find_map(|c| c.pdev.as_deref()).and_then(gpu::device_busy),
        cpu: cpu(&[pid]),
        memory_mb,
        libraries_mb,
        decoders: decoders.len(),
        decoders_cpu: cpu(&decoders),
        decoders_memory_mb: decoders.iter().map(|d| rss_mb(*d).0).sum(),
    };
    *last = Some(now);
    usage
}

fn daemon_status() -> Option<Value> {
    wallpicker_core::daemon::ask(&json!({ "cmd": "status" })).ok()
}

/// The daemon's animation decoders (`wallpicker __decode` children).
fn decoders(daemon: u32) -> Vec<u32> {
    let Ok(entries) = std::fs::read_dir("/proc") else {
        return Vec::new();
    };
    entries
        .flatten()
        .filter_map(|e| e.file_name().to_str()?.parse::<u32>().ok())
        .filter(|pid| {
            let parent = std::fs::read_to_string(format!("/proc/{pid}/stat"))
                .ok()
                .and_then(|s| s[s.rfind(')')? + 2..].split_whitespace().nth(1)?.parse::<u32>().ok());
            parent == Some(daemon)
                && std::fs::read(format!("/proc/{pid}/cmdline")).is_ok_and(|c| c.split(|b| *b == 0).any(|a| a == b"__decode"))
        })
        .collect()
}

/// A process's own (anonymous) memory and its file-backed memory, in MB.
fn rss_mb(pid: u32) -> (f32, f32) {
    let status = std::fs::read_to_string(format!("/proc/{pid}/status")).unwrap_or_default();
    let field = |name: &str| {
        status
            .lines()
            .find_map(|l| l.strip_prefix(name))
            .and_then(|v| v.trim().trim_end_matches("kB").trim().parse::<f32>().ok())
            .map_or(0.0, |kb| kb / 1024.0)
    };
    (field("RssAnon:"), field("RssFile:"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn playing_monitors_and_their_presentation_rate() {
        let status = |presentations: u64, generation: u64| {
            json!({ "ok": true, "playback_paused": false, "screens": [
                { "monitor": "DP-1", "playback": { "state": "playing", "output_generation": generation,
                    "presentations": presentations } },
                { "monitor": "DP-2", "playback": null },
                { "monitor": "DP-3", "playback": { "state": "unavailable", "reason": "no hardware EGL" } },
            ]})
        };
        let start = Instant::now();
        let first = playback_of(&status(10, 1), start);
        assert!(first.running && !first.paused);
        assert_eq!(first.monitors.len(), 2);
        assert_eq!(first.monitors["DP-1"].fps, None);
        assert_eq!(first.monitors["DP-3"].reason.as_deref(), Some("no hardware EGL"));
        let second = playback_of(&status(40, 1), start + Duration::from_secs(2));
        assert_eq!(second.monitors["DP-1"].fps, Some(15.0));
        // A new presentation (replaced or adjusted) starts counting again.
        let third = playback_of(&status(3, 2), start + Duration::from_secs(4));
        assert_eq!(third.monitors["DP-1"].fps, None);
        assert!(!playback_of(&json!({ "ok": true, "screens": null }), start).running);
    }

    #[test]
    fn this_process_usage_reads_without_a_gpu() {
        // The test process holds no GPU client: GPU use isn't reported.
        let me = std::process::id();
        let first = usage_of(me);
        assert!(first.running && !first.gpu_reported && first.gpu.is_none());
        assert!(first.memory_mb > 0.0);
        assert!(usage_of(me).cpu.is_some());
        assert!(decoders(me).is_empty());
    }
}
