//! `wallpicker ui [--toggle]`: the Quickshell interface (quickshell/). It
//! runs as `qs -p <its folder>` with this binary as its backend
//! (WALLPICKER_BIN, for `wallpicker serve`). `--toggle` closes it when it's
//! open (a keybind); otherwise both open it.

use std::path::PathBuf;
use std::process::{Command, Stdio};

use super::fail;

/// Where the interface is: WALLPICKER_UI, a checkout this binary was built
/// in (development), or the installed copy.
fn ui_dir() -> Option<PathBuf> {
    let has_shell = |d: &PathBuf| d.join("shell.qml").is_file();
    if let Some(d) = std::env::var_os("WALLPICKER_UI").map(PathBuf::from) {
        return has_shell(&d).then_some(d);
    }
    if let Ok(exe) = std::env::current_exe() {
        for dir in exe.ancestors().skip(1) {
            let d = dir.join("quickshell");
            if has_shell(&d) {
                return Some(d);
            }
        }
    }
    let data = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")))?;
    Some(data.join("wallpicker/ui")).filter(has_shell)
}

/// Whether it's open: its IPC answers.
/// Settings that cut the open picker's memory (measured on the desktop
/// 2026-10-07: ~260 MB -> ~220 MB, interface and backend together):
/// glibc's allocator hands freed memory back instead of keeping a pool per
/// thread (`serve` inherits it: 25 -> 6 MB), and Qt draws through Vulkan,
/// where there's a driver for it, instead of OpenGL (whose AMD driver keeps
/// LLVM's compiler around). Anything already set in the environment wins.
fn lighter(qs: &mut Command) {
    let unset = |key: &str| std::env::var_os(key).is_none();
    for (key, value) in [("MALLOC_ARENA_MAX", "2"), ("MALLOC_MMAP_THRESHOLD_", "1048576"), ("MALLOC_TRIM_THRESHOLD_", "131072")] {
        if unset(key) {
            qs.env(key, value);
        }
    }
    if unset("QSG_RHI_BACKEND") && has_vulkan_driver() {
        qs.env("QSG_RHI_BACKEND", "vulkan");
    }
}

/// A Vulkan driver is installed (an ICD manifest where the loader looks).
fn has_vulkan_driver() -> bool {
    ["/usr/share/vulkan/icd.d", "/etc/vulkan/icd.d"].iter().any(|d| {
        std::fs::read_dir(d).is_ok_and(|mut e| e.any(|f| f.is_ok_and(|f| f.path().extension().is_some_and(|x| x == "json"))))
    })
}

fn running(dir: &PathBuf) -> bool {
    Command::new("qs")
        .arg("-p")
        .arg(dir)
        .args(["ipc", "show"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

pub fn ui(args: &[String]) -> i32 {
    let toggle = args.iter().any(|a| a == "--toggle");
    let Some(dir) = ui_dir() else {
        return fail(&"the interface isn't installed (scripts/install-local.sh puts it in ~/.local/share/wallpicker/ui)");
    };
    if running(&dir) {
        if toggle {
            let _ = Command::new("qs").arg("-p").arg(&dir).args(["ipc", "call", "wallpicker", "toggle"]).status();
        }
        return 0;
    }
    let exe = std::env::current_exe().map(|p| p.into_os_string()).unwrap_or_else(|_| "wallpicker".into());
    let mut qs = Command::new("qs");
    qs.arg("-p").arg(&dir).args(["--no-duplicate", "--daemonize"]).env("WALLPICKER_BIN", exe);
    lighter(&mut qs);
    match qs
        .stdin(Stdio::null())
        // Quickshell keeps its own log (qs log); a launch says nothing.
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
    {
        Ok(s) if s.success() => 0,
        Ok(s) => fail(&format!("quickshell exited with {s}")),
        Err(e) => fail(&format!("could not start quickshell (qs): {e}")),
    }
}
