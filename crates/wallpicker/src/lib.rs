//! wallpicker without a window: the command line (`cli/`), `wallpicker
//! serve` for the Quickshell interface (`serve/`), and what both do
//! (`service/`: plain functions over the shared `AppState`, one file per
//! area). The old Tauri window (`wallpicker-app`, src-tauri) uses this crate
//! too. Every decision that can be tested without a desktop session lives
//! in `wallpicker-core`.

pub mod backup;
pub mod cli;
pub mod rt;
mod serve;
pub mod service;
pub mod theme;

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::sync::Semaphore;
use wallpicker_core::cmd::SystemRunner;
use wallpicker_core::hyprpaper;
use wallpicker_core::settings::WallpaperBackend;
use wallpicker_core::store::{Dirs, Store};

/// `wallpicker serve` (the Quickshell interface's backend).
pub fn serve() -> i32 {
    serve::run()
}

pub struct AppState {
    pub store: Store,
    pub http: reqwest::Client,
    /// Decoding a big PNG is memory-hungry; the grid asks for many thumbnails
    /// at once, so only a few are generated concurrently.
    pub thumb_gate: Arc<Semaphore>,
    /// `library.json` is loaded, changed and saved per command; this keeps two
    /// commands from interleaving and losing one's change.
    pub catalog_lock: Arc<Mutex<()>>,
    /// Adjusting saves the state, makes the copy and deletes the copies the
    /// state no longer uses: two at once would each delete the other's.
    pub adjust_lock: Arc<Mutex<()>>,
}

impl AppState {
    /// Shared by the command line, `serve` and the Tauri window.
    pub fn new(store: Store) -> AppState {
        let http = reqwest::Client::builder()
            .user_agent(concat!("wallpicker/", env!("CARGO_PKG_VERSION")))
            .timeout(Duration::from_secs(30))
            .build()
            .expect("failed to build HTTP client");
        AppState {
            store,
            http,
            thumb_gate: Arc::new(Semaphore::new(3)),
            catalog_lock: Arc::new(Mutex::new(())),
            adjust_lock: Arc::new(Mutex::new(())),
        }
    }
}

pub type CmdResult<T> = Result<T, String>;

pub fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

/// Run blocking core code off the async threads.
pub async fn blocking<T, F>(f: F) -> CmdResult<T>
where
    T: Send + 'static,
    F: FnOnce() -> wallpicker_core::Result<T> + Send + 'static,
{
    rt::spawn_blocking(f).await.map_err(err)?.map_err(err)
}

pub fn wallpaper_path(path: &str) -> CmdResult<PathBuf> {
    let p = PathBuf::from(path);
    if p.is_absolute() {
        Ok(p)
    } else {
        Err(format!("Not an absolute path: {path:?}"))
    }
}

/// `wallpicker --restore`: re-apply the saved wallpapers and exit.
/// Returns the process exit code.
pub fn restore_cli() -> i32 {
    let store = Store::new(Dirs::from_env());
    let backend = store
        .load_settings()
        .map(|s| s.wallpaper_backend)
        .unwrap_or_default();
    if backend == WallpaperBackend::Noctalia {
        println!("wallpicker uses Noctalia, which restores its wallpapers itself; nothing to do");
        return 0;
    }
    // Retry for ~20s: at login hyprpaper may still be starting.
    match hyprpaper::restore(&SystemRunner, &store, 40, Duration::from_millis(500)) {
        Ok(report) => {
            for m in &report.applied {
                println!("restored {m}");
            }
            for (m, why) in &report.skipped {
                eprintln!("skipped {m}: {why}");
            }
            0
        }
        Err(e) => {
            eprintln!("wallpicker: {e}");
            1
        }
    }
}
