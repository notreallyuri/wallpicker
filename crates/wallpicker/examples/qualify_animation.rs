//! Opt-in local media qualification; no daemon, compositor or saved-state writes.
//! cargo run --release -p wallpicker --example qualify_animation -- --loops 3 FILE...
use serde_json::json;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::{Arc, mpsc};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};
use wallpicker::decoding::{Budget, DecodedFrame, Options, Usage, Worker};
use wallpicker_core::media::{Kind, Limits, Source};

const MIB: u64 = 1024 * 1024;

fn main() {
    if std::env::args_os().nth(1).as_deref() == Some(std::ffi::OsStr::new("__decode")) {
        std::process::exit(wallpicker::decoding::child_main());
    }
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let mut args = std::env::args_os().skip(1);
    let (mut loops, mut child_mib) = (3u64, 128u64);
    let mut paths = Vec::new();
    while let Some(arg) = args.next() {
        if arg == "--" {
            paths.extend(args.map(PathBuf::from));
            break;
        } else if arg == "--loops" || arg == "--child-mib" {
            let value: u64 = args
                .next()
                .and_then(|s| s.to_str().and_then(|s| s.parse().ok()))
                .ok_or("expected a positive integer after --loops/--child-mib")?;
            if value == 0 || value > 4096 {
                return Err("--loops/--child-mib must be between 1 and 4096".into());
            }
            if arg == "--loops" {
                loops = value;
            } else {
                child_mib = value;
            }
        } else {
            paths.push(PathBuf::from(arg));
        }
    }
    if paths.is_empty() {
        return Err("usage: qualify_animation [--loops 3] [--child-mib 128] [--] FILE...".into());
    }
    let mut failed = false;
    for path in paths {
        match qualify(&path, loops, child_mib * MIB) {
            Ok(report) => println!("{report}"),
            Err(error) => {
                failed = true;
                println!("{}", json!({"path": path, "error": error}));
            }
        }
    }
    if failed {
        Err("one or more qualification inputs failed".into())
    } else {
        Ok(())
    }
}

fn receive(worker: &Worker, sequence: u64) -> Result<DecodedFrame, String> {
    let frame = worker
        .recv_timeout(Duration::from_secs(10))
        .map_err(|e| e.to_string())??;
    if frame.generation != 1 || frame.sequence != sequence {
        return Err("worker returned an unexpected generation/sequence".into());
    }
    Ok(frame)
}

fn fingerprint(frame: &DecodedFrame) -> (u64, Duration) {
    let mut hash = DefaultHasher::new();
    frame.frame().pixels.as_raw().hash(&mut hash);
    (hash.finish(), frame.frame().delay.expect("animated frame"))
}

fn qualify(path: &Path, loops: u64, cap: u64) -> Result<serde_json::Value, String> {
    let path = path.canonicalize().map_err(|e| e.to_string())?;
    let source = Source::open(&path, Limits::default()).map_err(|e| e.to_string())?;
    let info = source.info().clone();
    if info.kind != Kind::Animated {
        return Err("input is not animated".into());
    }
    let frame_bytes = u64::from(info.width) * u64::from(info.height) * 4;
    let reserved = cap + frame_bytes * 3 + MIB;
    let budget = Budget::new(reserved, 1);
    let options = Options {
        address_space_bytes: cap,
        frame_allowance: 0,
        frame_timeout: Duration::from_secs(5),
    };
    let executable = std::env::current_exe().map_err(|e| e.to_string())?;
    let started = Instant::now();
    let (mut worker, first) = Worker::prepare(&executable, source.clone(), 1, &budget, options, Arc::new(|| {}))
        .map_err(|e| e.to_string())?;
    let first_ms = started.elapsed().as_secs_f64() * 1000.0;
    let pid = worker.process_id().ok_or("worker stopped after first frame")?;
    let sampler = Sampler::new(pid);
    let mut expected = vec![fingerprint(&first)];
    drop(first);
    worker.pause(false);
    // One complete warm-up loop; retain only fingerprints/delays, never pixels.
    let count = u64::from(info.frame_count);
    for sequence in 1..count {
        expected.push(fingerprint(&receive(&worker, sequence)?));
    }
    let encoded_seconds: f64 = expected.iter().map(|(_, d)| d.as_secs_f64()).sum();
    let playback_seconds: f64 = expected
        .iter()
        .map(|(_, d)| d.max(&Duration::from_millis(10)).as_secs_f64())
        .sum();
    let baseline = memory(pid);
    let cpu_before = cpu_seconds(pid);
    let started = Instant::now();
    for sequence in count..count * (loops + 1) {
        let frame = receive(&worker, sequence)?;
        if fingerprint(&frame) != expected[(sequence % count) as usize] {
            return Err(format!("loop pixels or timing changed at sequence {sequence}"));
        }
    }
    let elapsed = started.elapsed().as_secs_f64();
    let child_cpu = cpu_seconds(pid) - cpu_before;
    let end = memory(pid);
    let peak = sampler.finish();
    // Hold all three leases to force bounded backpressure before cancellation.
    let mut held = Vec::new();
    for sequence in count * (loops + 1)..count * (loops + 1) + 3 {
        held.push(receive(&worker, sequence)?);
    }
    worker.pause(true);
    let backpressured = memory(pid);
    let started = Instant::now();
    worker.cancel();
    let cancel_ms = started.elapsed().as_secs_f64() * 1000.0;
    if Path::new(&format!("/proc/{pid}")).exists() || budget.usage().workers != 1 {
        return Err("child reaping or retained-frame reservation failed".into());
    }
    drop(held);
    drop(worker);
    if budget.usage() != Usage::default() {
        return Err("admission was not fully released".into());
    }
    // Small replacement sweep with first-frame validation and complete teardown.
    let mut replacement_peak = 0;
    for _ in 0..3 {
        let (worker, first) = Worker::prepare(&executable, source.clone(), 1, &budget, options, Arc::new(|| {}))
            .map_err(|e| e.to_string())?;
        if fingerprint(&first) != expected[0] {
            return Err("replacement changed the representative frame".into());
        }
        replacement_peak = replacement_peak.max(memory(worker.process_id().ok_or("replacement stopped")?).parent_rss);
        drop((worker, first));
        if budget.usage() != Usage::default() {
            return Err("replacement leaked admission".into());
        }
    }
    Ok(json!({
        "path": path, "profile": if cfg!(debug_assertions) { "debug" } else { "release" },
        "format": format!("{:?}", info.format), "dimensions": [info.width, info.height],
        "encoded_bytes": std::fs::metadata(&path).map_err(|e| e.to_string())?.len(),
        "frames_per_loop": count, "encoded_loop_seconds": encoded_seconds,
        "playback_loop_seconds": playback_seconds, "minimum_delay_ms": expected.iter().map(|(_, d)| d.as_millis()).min(),
        "maximum_delay_ms": expected.iter().map(|(_, d)| d.as_millis()).max(),
        "measured_loops": loops, "verified_frames": count * (loops + 1),
        "first_frame_ms": first_ms, "measured_seconds": elapsed,
        "frames_per_second_with_hashing": (count * loops) as f64 / elapsed,
        "source_time_per_wall_time": playback_seconds * loops as f64 / elapsed,
        "child_cpu_seconds": child_cpu,
        "child_address_space_cap_bytes": cap, "reserved_bytes": reserved,
        "child_rss_warm_bytes": baseline.child_rss, "child_rss_end_bytes": end.child_rss,
        "child_rss_sampled_peak_bytes": peak.child_rss, "child_rss_high_water_bytes": end.child_high_water,
        "parent_rss_warm_bytes": baseline.parent_rss, "parent_rss_end_bytes": end.parent_rss,
        "parent_rss_sampled_peak_bytes": peak.parent_rss,
        "parent_rss_backpressured_bytes": backpressured.parent_rss,
        "child_rss_backpressured_bytes": backpressured.child_rss,
        "parent_rss_replacement_peak_bytes": replacement_peak,
        "backpressured_cancel_ms": cancel_ms, "replacement_runs": 3, "reservation_after_cleanup": budget.usage().bytes,
    }))
}

#[derive(Default)]
struct Memory {
    child_rss: u64,
    child_high_water: u64,
    parent_rss: u64,
}
fn status_bytes(status: &str, field: &str) -> u64 {
    status
        .lines()
        .find_map(|s| s.strip_prefix(field))
        .and_then(|s| s.split_whitespace().next())
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0)
        * 1024
}
fn memory(pid: u32) -> Memory {
    let child = std::fs::read_to_string(format!("/proc/{pid}/status")).unwrap_or_default();
    let parent = std::fs::read_to_string("/proc/self/status").unwrap_or_default();
    Memory {
        child_rss: status_bytes(&child, "VmRSS:"),
        child_high_water: status_bytes(&child, "VmHWM:"),
        parent_rss: status_bytes(&parent, "VmRSS:"),
    }
}
fn cpu_seconds(pid: u32) -> f64 {
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).unwrap_or_default();
    let fields: Vec<_> = stat
        .rsplit_once(')')
        .map_or("", |(_, rest)| rest)
        .split_whitespace()
        .collect();
    let ticks: u64 = [11, 12]
        .into_iter()
        .filter_map(|i| fields.get(i)?.parse::<u64>().ok())
        .sum();
    // SAFETY: sysconf reads the process clock-tick constant, without pointers.
    ticks as f64 / unsafe { libc::sysconf(libc::_SC_CLK_TCK) }.max(1) as f64
}

struct Sampler {
    stop: mpsc::Sender<()>,
    thread: Option<JoinHandle<Memory>>,
}
impl Sampler {
    fn new(pid: u32) -> Self {
        let (stop, receiver) = mpsc::channel();
        let thread = std::thread::spawn(move || {
            let mut peak = memory(pid);
            while receiver.recv_timeout(Duration::from_millis(20)) == Err(mpsc::RecvTimeoutError::Timeout) {
                let sample = memory(pid);
                peak.child_rss = peak.child_rss.max(sample.child_rss);
                peak.parent_rss = peak.parent_rss.max(sample.parent_rss);
            }
            peak
        });
        Self {
            stop,
            thread: Some(thread),
        }
    }
    fn finish(mut self) -> Memory {
        let _ = self.stop.send(());
        self.thread.take().unwrap().join().unwrap()
    }
}
impl Drop for Sampler {
    fn drop(&mut self) {
        let _ = self.stop.send(());
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
