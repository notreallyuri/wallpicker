//! Tauri backend. Deliberately thin: every decision that can be tested without
//! a desktop session lives in `wallpicker-core`. The window's commands are in
//! `commands/` by area; this file has what they share (the app state,
//! `blocking`, `wallpaper_path`) and starts the app.

pub mod backup;
pub mod cli;
mod commands;
pub mod open;
pub mod theme;
mod tray;
mod usage;

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tauri::Manager;
use tokio::sync::Semaphore;
use wallpicker_core::cmd::SystemRunner;
use wallpicker_core::hyprpaper;
use wallpicker_core::settings::WallpaperBackend;
use wallpicker_core::store::{Dirs, Store};

struct AppState {
    store: Store,
    http: reqwest::Client,
    /// Decoding a big PNG is memory-hungry; the grid asks for many thumbnails
    /// at once, so only a few are generated concurrently.
    thumb_gate: Arc<Semaphore>,
    /// `library.json` is loaded, changed and saved per command; this keeps two
    /// commands from interleaving and losing one's change.
    catalog_lock: Arc<Mutex<()>>,
}

impl AppState {
    /// Shared by the app and the command line.
    fn new(store: Store) -> AppState {
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
        }
    }
}

type CmdResult<T> = Result<T, String>;

fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

/// Run blocking core code off the UI thread.
async fn blocking<T, F>(f: F) -> CmdResult<T>
where
    T: Send + 'static,
    F: FnOnce() -> wallpicker_core::Result<T> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(f)
        .await
        .map_err(err)?
        .map_err(err)
}

// ------------------------------------------------------------------ local ----

// ---------------------------------------------------------------- catalog ----

fn wallpaper_path(path: &str) -> CmdResult<PathBuf> {
    let p = PathBuf::from(path);
    if p.is_absolute() {
        Ok(p)
    } else {
        Err(format!("Not an absolute path: {path:?}"))
    }
}

// ----------------------------------------------------------------- online ----

// -------------------------------------------------------------------- app ----

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

/// A development build (plain `cargo build`) loads the UI from the dev
/// server instead of embedding it; without `cargo tauri dev` running, its
/// window would just say "connection refused". Say what's going on instead.
#[cfg(dev)]
fn require_dev_server(context: &tauri::Context) {
    use std::net::{TcpStream, ToSocketAddrs};
    let Some(url) = context.config().build.dev_url.as_ref() else {
        return;
    };
    let host = url.host_str().unwrap_or("localhost");
    let port = url.port_or_known_default().unwrap_or(80);
    let reachable = (host, port).to_socket_addrs().is_ok_and(|mut addrs| {
        addrs.any(|a| TcpStream::connect_timeout(&a, std::time::Duration::from_millis(300)).is_ok())
    });
    if !reachable {
        eprintln!(
            "wallpicker: this is a development build; its window loads the UI from {url},\n\
             which isn't running. Use `cargo tauri dev` while developing, or build the app with\n\
             `cargo tauri build --no-bundle` (that embeds the UI). `wallpicker list` and\n\
             `wallpicker apply` work in either build."
        );
        std::process::exit(1);
    }
}

/// glibc's malloc keeps freed memory for reuse: big blocks once its dynamic
/// mmap threshold has grown (to 32 MB), and one pool per thread (the app has
/// dozens). After a few full-size images that's hundreds of MB held while the
/// app sits in the tray. A fixed 1 MB threshold sends big blocks straight
/// back to the system when freed, and two pools are plenty for a UI.
fn tune_malloc() {
    #[cfg(all(target_os = "linux", target_env = "gnu"))]
    // SAFETY: mallopt only adjusts allocator parameters; called once at startup.
    unsafe {
        libc::mallopt(libc::M_MMAP_THRESHOLD, 1024 * 1024);
        libc::mallopt(libc::M_ARENA_MAX, 2);
    }
}

/// The app. `start_hidden` (`wallpicker --tray`): only the tray icon at
/// first, for an autostart.
pub fn run(start_hidden: bool) {
    tune_malloc();
    let context = tauri::generate_context!();
    #[cfg(dev)]
    require_dev_server(&context);
    let dirs = Dirs::from_env();
    let cache = dirs.cache.clone();

    let store = Store::new(dirs);
    // hyprpaper.conf sources this file; make sure it exists (and matches the
    // saved state) before anything else, even if nothing gets applied.
    if let Err(e) = store.write_hyprpaper_conf() {
        eprintln!("wallpicker: could not write the hyprpaper config: {e}");
    }

    tauri::Builder::default()
        // First: a second `wallpicker` hands over to this one and exits.
        .plugin(tauri_plugin_single_instance::init(|app, argv, cwd| {
            tray::second_launch(app, &argv, std::path::Path::new(&cwd))
        }))
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState::new(store.clone()))
        .setup(move |app| {
            // Without a tray (nothing to show it in), the app just runs as a
            // window; closing it quits.
            if let Err(e) = tray::setup(app.handle()) {
                eprintln!("wallpicker: no tray icon: {e}");
            }
            // The window isn't made at startup (tauri.conf: `create: false`);
            // `--tray` leaves it unmade until it's wanted.
            if !start_hidden {
                tray::show(app.handle());
            }
            // Thumbnails are shown through Tauri's asset protocol, which only
            // serves allow-listed folders. The cache location depends on
            // XDG_CACHE_HOME, so it is allowed here instead of in tauri.conf.
            std::fs::create_dir_all(&cache)?;
            app.asset_protocol_scope().allow_directory(&cache, true)?;
            // A wallpaper per workspace: follows Hyprland while the app runs
            // (waits while it's off, or while `wallpicker workspaces run` has it).
            let handle = app.handle().clone();
            let listener_store = store.clone();
            std::thread::spawn(move || {
                use tauri::Emitter;
                wallpicker_core::workspaces::listen(
                    &SystemRunner,
                    &listener_store,
                    &|| true,
                    &|| {
                        let _ = handle.emit("state-changed", ());
                    },
                );
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::wallpapers::list_monitors,
            commands::app::get_settings,
            commands::app::open_timing,
            commands::app::save_settings,
            commands::app::get_state,
            commands::library::scan_local,
            commands::library::thumbnail,
            commands::wallpapers::apply_wallpaper,
            commands::online::online_search,
            commands::online::get_online_filters,
            commands::online::save_online_filters,
            commands::online::online_details,
            commands::online::online_download,
            commands::online::online_downloaded,
            commands::online::open_url,
            commands::library::get_catalog,
            commands::library::set_favorite,
            commands::library::add_tag,
            commands::library::remove_tag,
            commands::wallpapers::trash_wallpaper,
            commands::wallpapers::show_in_folder,
            commands::wallpapers::copy_text,
            commands::app::slideshow_status,
            commands::app::slideshow_now,
            commands::library::find_duplicates,
            commands::library::image_colors,
            commands::wallpapers::previous_wallpaper,
            commands::app::wallpaper_palette,
            commands::app::take_open_request,
            commands::wallpapers::set_monitor_adjust,
            commands::wallpapers::workspace_rows,
            commands::wallpapers::set_workspace_wallpaper,
            commands::wallpapers::framing_image,
            commands::app::app_usage,
            commands::online::online_preview,
            commands::app::window_background,
            commands::backup::backup_export_pick,
            commands::backup::backup_import_pick,
            commands::backup::snapshot_dir,
            commands::backup::snapshot_dir_pick,
            commands::backup::backup_snapshots,
            commands::backup::backup_restore,
            commands::wallpapers::wallpaper_conflicts,
            commands::wallpapers::stop_wallpaper_program,
            commands::app::noctalia_colors,
            commands::app::set_noctalia_colors,
            commands::online::wallhaven_collections,
            commands::online::tag_suggestions,
            commands::online::wallhaven_collection,
            commands::library::resolve_duplicates,
        ])
        .build(context)
        .expect("error while building wallpicker")
        .run(|app, event| {
            // Closing the last window would end the app; with the tray it
            // goes on (the window is made again when wanted). Quitting from
            // the tray menu asks with an exit code, and isn't stopped.
            if let tauri::RunEvent::ExitRequested {
                code: None, api, ..
            } = event
                && tray::hides_on_close(app)
            {
                api.prevent_exit();
            }
        });
}
