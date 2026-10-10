//! Live Wayland lifecycle and screenshot-based fade regression tests.

use super::{Renderer, Transition};
use serde_json::Value;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use wallpicker_core::compose::{self, RgbImage};

/// Uses temporary surfaces and the saved wallpapers; no daemon or saved
/// state is changed. Run on a visible Wayland desktop with alpha modifier.
#[test]
#[ignore = "requires a live Wayland desktop and saved wallpaper"]
fn live_fade_lifecycle() {
    use wallpicker_core::store::{Dirs, Store};
    let store = Store::new(Dirs::from_env());
    let renderer = Renderer::start(Arc::new(move |name| {
        wallpicker_core::hyprpaper::saved_for(&store, name)
    }))
    .unwrap();
    let wait = |predicate: &dyn Fn(&Value) -> bool| {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            let status = renderer.status().unwrap();
            if predicate(&status) {
                return status;
            }
            assert!(Instant::now() < deadline, "renderer timed out: {status}");
            std::thread::sleep(Duration::from_millis(20));
        }
    };
    let status = wait(&|s| {
        s["screens"]
            .as_array()
            .unwrap()
            .iter()
            .any(|s| s["file"].is_string() && s["pixels"].is_array())
    });
    let screen = status["screens"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["file"].is_string() && s["pixels"].is_array())
        .unwrap();
    let monitor = screen["monitor"].as_str().unwrap().to_owned();
    let picture = (PathBuf::from(screen["file"].as_str().unwrap()), None);
    let show = |transition| {
        renderer
            .show(monitor.clone(), picture.clone(), transition)
            .unwrap()
            .recv_timeout(Duration::from_secs(10))
            .unwrap()
            .unwrap();
    };
    let fading = |s: &Value| {
        s["screens"]
            .as_array()
            .unwrap()
            .iter()
            .find(|s| s["monitor"].as_str() == Some(&monitor))
            .unwrap()["fading"]
            .as_bool()
            .unwrap()
    };
    let fade = Transition::from_setting("fade", 500);
    show(fade);
    assert!(
        fading(&renderer.status().unwrap()),
        "alpha modifier transition unavailable"
    );
    wait(&|s| !fading(s));
    // Replace an active fade, then interrupt another with a cut.
    show(Transition::from_setting("fade", 5000));
    show(fade);
    wait(&|s| !fading(s));
    show(Transition::from_setting("fade", 5000));
    show(None);
    assert!(!fading(&renderer.status().unwrap()));
    // Shutdown while an overlay is still alive exercises overlay cleanup.
    show(Transition::from_setting("fade", 5000));
    assert!(renderer.running());
    drop(renderer);
}

/// Verifies actual compositor output, not just opacity requests. Temporarily
/// paints red/green test wallpapers; dropping the renderer reveals the
/// user's existing surfaces again. Requires grim and exposed wallpaper.
#[test]
#[ignore = "requires a visible Wayland desktop and grim; briefly paints test wallpapers"]
fn live_fade_blends_pixels() {
    let dir = std::env::temp_dir().join(format!("wallpicker-fade-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let red = dir.join("red.png");
    let green = dir.join("green.png");
    RgbImage::from_raw(1, 1, vec![255, 0, 0])
        .unwrap()
        .save(&red)
        .unwrap();
    RgbImage::from_raw(1, 1, vec![0, 255, 0])
        .unwrap()
        .save(&green)
        .unwrap();
    let renderer = Renderer::start(Arc::new(move |_| Some((red.clone(), None)))).unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let status = renderer.status().unwrap();
        let screens = status["screens"].as_array().unwrap();
        if !screens.is_empty() && screens.iter().all(|s| s["pixels"].is_array()) {
            break;
        }
        assert!(Instant::now() < deadline, "outputs did not draw: {status}");
        std::thread::sleep(Duration::from_millis(20));
    }
    // Let the compositor's own layer entry animation settle.
    std::thread::sleep(Duration::from_secs(1));
    let capture = |monitor: &str| {
        let path = dir.join("capture.png");
        let result = std::process::Command::new("grim")
            .args(["-o", monitor])
            .arg(&path)
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "grim: {}",
            String::from_utf8_lossy(&result.stderr)
        );
        compose::open(&path).unwrap()
    };
    let mut exposed = None;
    for screen in renderer.status().unwrap()["screens"].as_array().unwrap() {
        let monitor = screen["monitor"].as_str().unwrap();
        let shot = capture(monitor);
        let pixels: Vec<usize> = shot
            .pixels()
            .enumerate()
            .filter(|(_, p)| p.0[0] > 150 && p.0[1] < 10 && p.0[2] < 10)
            .map(|(i, _)| i)
            .collect();
        if pixels.len() > 1000 {
            exposed = Some((monitor.to_owned(), pixels));
            break;
        }
    }
    let (monitor, pixels) = exposed.expect("need exposed wallpaper on at least one monitor");
    renderer
        .show(
            monitor.clone(),
            (green, None),
            Transition::from_setting("fade", 2000),
        )
        .unwrap()
        .recv_timeout(Duration::from_secs(10))
        .unwrap()
        .unwrap();
    std::thread::sleep(Duration::from_millis(800));
    let midway = capture(&monitor);
    let mixed = pixels
        .iter()
        .filter(|&&i| {
            let p = &midway.as_raw()[i * 3..i * 3 + 3];
            p[0] > 30 && p[1] > 30 && p[2] < 20
        })
        .count();
    assert!(
        mixed > pixels.len() / 2,
        "no visible blend: {mixed}/{} exposed pixels contain both red and green",
        pixels.len()
    );
    eprintln!("visible blend: {mixed}/{} exposed pixels", pixels.len());
    let deadline = Instant::now() + Duration::from_secs(5);
    while renderer.status().unwrap()["screens"]
        .as_array()
        .unwrap()
        .iter()
        .any(|s| s["monitor"] == monitor && s["fading"] == true)
    {
        assert!(Instant::now() < deadline, "fade did not finish");
        std::thread::sleep(Duration::from_millis(20));
    }
    std::thread::sleep(Duration::from_secs(1));
    let finished = capture(&monitor);
    let green = pixels
        .iter()
        .filter(|&&i| {
            let p = &finished.as_raw()[i * 3..i * 3 + 3];
            p[0] < 10 && p[1] > 150 && p[2] < 10
        })
        .count();
    assert!(
        green > pixels.len() / 2,
        "final wallpaper did not become green"
    );
    drop(renderer);
    std::fs::remove_dir_all(dir).unwrap();
}
