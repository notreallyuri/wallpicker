// Prevents an extra console window on Windows release builds; harmless on Linux.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

//! `wallpicker-app`: the old Tauri window. `wallpicker` hands it the
//! window's arguments: none (open it), `--tray`, `--toggle`, `--open`.

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    // Checked here, before a running window gets the arguments: a mistake
    // is reported to whoever typed it.
    let cwd = std::env::current_dir().unwrap_or_default();
    match wallpicker_app::open::parse(&args, &cwd) {
        Ok(Some(place)) => wallpicker_app::open::set_pending(place),
        Ok(None) => {}
        Err(e) => {
            eprintln!("wallpicker: {e}");
            std::process::exit(2);
        }
    }

    // `--toggle` (or `--open`) with nothing running yet just opens it.
    wallpicker_app::run(args.iter().any(|a| a == "--tray"));
}
