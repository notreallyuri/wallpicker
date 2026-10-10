//! Screenshot checks for GPU reveals, directions, interruption, and final pixels.

use super::{Renderer, Transition};
use serde_json::Value;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use wallpicker_core::compose::{self, RgbImage};

struct Desktop {
    renderer: Renderer,
    dir: PathBuf,
    monitor: String,
    pixels: Vec<usize>,
    size: (u32, u32),
}

impl Desktop {
    fn new() -> Self {
        let dir = std::env::temp_dir().join(format!("wallpicker-gpu-test-{}", std::process::id()));
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
        let mut this = Self {
            renderer,
            dir,
            monitor: String::new(),
            pixels: Vec::new(),
            size: (0, 0),
        };
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            let status = this.renderer.status().unwrap();
            let screens = status["screens"].as_array().unwrap();
            if !screens.is_empty() && screens.iter().all(|s| s["pixels"].is_array()) {
                break;
            }
            assert!(Instant::now() < deadline, "outputs did not draw: {status}");
            std::thread::sleep(Duration::from_millis(20));
        }
        std::thread::sleep(Duration::from_secs(1));
        for screen in this.renderer.status().unwrap()["screens"]
            .as_array()
            .unwrap()
        {
            let monitor = screen["monitor"].as_str().unwrap();
            let shot = this.capture(monitor);
            let pixels: Vec<_> = shot
                .pixels()
                .enumerate()
                .filter(|(_, p)| p[0] > 150 && p[1] < 10 && p[2] < 10)
                .map(|(i, _)| i)
                .collect();
            if pixels.len() > this.pixels.len() {
                this.monitor = monitor.to_owned();
                this.pixels = pixels;
                this.size = shot.dimensions();
            }
        }
        assert!(
            this.pixels.len() > (this.size.0 as usize * this.size.1 as usize) / 2,
            "test needs a mostly exposed wallpaper on one monitor"
        );
        this
    }

    fn capture(&self, monitor: &str) -> RgbImage {
        let path = self.dir.join("capture.png");
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
    }

    fn show(&self, file: &str, kind: &str, duration: u32) {
        self.renderer
            .show(
                self.monitor.clone(),
                (self.dir.join(file), None),
                Transition::from_setting(kind, duration),
            )
            .unwrap()
            .recv_timeout(Duration::from_secs(10))
            .unwrap()
            .unwrap();
    }

    fn screen(&self) -> Value {
        self.renderer.status().unwrap()["screens"]
            .as_array()
            .unwrap()
            .iter()
            .find(|s| s["monitor"] == self.monitor)
            .unwrap()
            .clone()
    }

    fn finish(&self) {
        self.wait_for_handoff();
        // Let any compositor layer-exit animation settle too.
        std::thread::sleep(Duration::from_millis(600));
    }

    fn wait_for_handoff(&self) {
        let deadline = Instant::now() + Duration::from_secs(8);
        while self.screen()["fading"] == true {
            assert!(Instant::now() < deadline, "transition did not finish");
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    fn colors(&self, shot: &RgbImage) -> (usize, usize) {
        let mut red = 0;
        let mut green = 0;
        for &i in &self.pixels {
            let p = &shot.as_raw()[i * 3..i * 3 + 3];
            red += usize::from(p[0] > 150 && p[1] < 20 && p[2] < 20);
            green += usize::from(p[0] < 20 && p[1] > 150 && p[2] < 20);
        }
        (red, green)
    }
}

impl Drop for Desktop {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

#[test]
#[ignore = "requires hardware EGL, grim, and an exposed desktop; temporarily paints test wallpapers"]
fn live_shader_effects() {
    let desktop = Desktop::new();
    for kind in [
        "wipe", "left", "right", "top", "bottom", "wave", "grow", "center", "any", "outer",
    ] {
        desktop.show("red.png", "none", 0);
        desktop.show("green.png", kind, 1600);
        std::thread::sleep(Duration::from_millis(450));
        assert_eq!(
            desktop.screen()["shader"],
            true,
            "{kind} fell back instead of rendering on GPU"
        );
        let midway = desktop.capture(&desktop.monitor);
        let (red, green) = desktop.colors(&midway);
        assert!(
            red > desktop.pixels.len() / 100 && green > desktop.pixels.len() / 100,
            "{kind} must show old and new regions: red={red}, green={green}"
        );
        if let Some(axis) = match kind {
            "left" | "wipe" => Some((0, true)),
            "right" => Some((0, false)),
            "top" => Some((1, true)),
            "bottom" => Some((1, false)),
            _ => None,
        } {
            let (component, from_low) = axis;
            let mut correct = 0;
            for &i in &desktop.pixels {
                let xy = [i % desktop.size.0 as usize, i / desktop.size.0 as usize];
                let dimension = [desktop.size.0, desktop.size.1][component] as usize;
                let p = &midway.as_raw()[i * 3..i * 3 + 3];
                let is_green = p[1] > 150 && p[0] < 20;
                let is_red = p[0] > 150 && p[1] < 20;
                // Compare far edges, away from feathered transition boundaries.
                if xy[component] < dimension / 8 {
                    correct += usize::from(if from_low { is_green } else { is_red });
                }
                if xy[component] > dimension * 7 / 8 {
                    correct += usize::from(if from_low { is_red } else { is_green });
                }
            }
            assert!(
                correct > desktop.pixels.len() / 10,
                "{kind} moved in the wrong direction"
            );
        }
        desktop.finish();
        let (_, green) = desktop.colors(&desktop.capture(&desktop.monitor));
        assert!(
            green > desktop.pixels.len() * 9 / 10,
            "{kind} did not reach its final image"
        );
        eprintln!("{kind}: visible GPU reveal and final image passed");
    }
    // Replace one active GPU effect with another, then interrupt with a cut.
    desktop.show("red.png", "wave", 5000);
    desktop.show("green.png", "right", 300);
    desktop.finish();
    desktop.show("red.png", "grow", 5000);
    desktop.show("green.png", "none", 0);
    desktop.finish();
    let (_, green) = desktop.colors(&desktop.capture(&desktop.monitor));
    assert!(green > desktop.pixels.len() * 9 / 10);
    desktop.show("red.png", "wave", 5000);
    // Desktop drops its renderer during an active EGL transition.
}

#[test]
#[ignore = "requires hardware EGL, grim, and an exposed desktop; temporarily paints a test image"]
fn live_shader_texture_orientation() {
    let desktop = Desktop::new();
    // Yellow above blue catches both inverted Y coordinates and BGR/RGB swaps.
    let (width, height) = desktop.size;
    let mut pixels = vec![0; width as usize * height as usize * 3];
    for (i, pixel) in pixels.as_chunks_mut::<3>().0.iter_mut().enumerate() {
        pixel.copy_from_slice(if i / (width as usize) < height as usize / 2 {
            &[255, 255, 0]
        } else {
            &[0, 0, 255]
        });
    }
    let image = RgbImage::from_raw(width, height, pixels).unwrap();
    image.save(desktop.dir.join("pattern.png")).unwrap();
    desktop.show("pattern.png", "wipe", 2000);
    std::thread::sleep(Duration::from_millis(600));
    assert_eq!(desktop.screen()["shader"], true);
    let shot = desktop.capture(&desktop.monitor);
    let mut top = (0, 0);
    let mut bottom = (0, 0);
    for &i in &desktop.pixels {
        let x = i % desktop.size.0 as usize;
        let y = i / desktop.size.0 as usize;
        if x >= desktop.size.0 as usize / 8 {
            continue;
        }
        let p = &shot.as_raw()[i * 3..i * 3 + 3];
        if y < desktop.size.1 as usize / 8 {
            top.0 += 1;
            top.1 += usize::from(p[0] > 130 && p[1] > 130 && p[2] < 40);
        }
        if y > desktop.size.1 as usize * 7 / 8 {
            bottom.0 += 1;
            bottom.1 += usize::from(p[0] < 40 && p[1] < 40 && p[2] > 130);
        }
    }
    assert!(
        top.0 > 100 && bottom.0 > 100,
        "need exposed top and bottom left corners"
    );
    assert!(
        top.1 > top.0 * 9 / 10 && bottom.1 > bottom.0 * 9 / 10,
        "wrong texture orientation or color order: top={top:?}, bottom={bottom:?}"
    );
    desktop.finish();
}

#[test]
#[ignore = "requires hardware EGL, grim, and an exposed desktop; temporarily paints a test image"]
fn live_slide_handoff_has_no_shifted_ghost() {
    let desktop = Desktop::new();
    let (width, height) = desktop.size;
    let mut pixels = vec![0; width as usize * height as usize * 3];
    for (i, pixel) in pixels.as_chunks_mut::<3>().0.iter_mut().enumerate() {
        let x = i % width as usize;
        let y = i / width as usize;
        pixel.copy_from_slice(if (x / 16 + y / 16) % 2 == 0 {
            &[255, 255, 0]
        } else {
            &[0, 0, 255]
        });
    }
    RgbImage::from_raw(width, height, pixels)
        .unwrap()
        .save(desktop.dir.join("checker.png"))
        .unwrap();
    for kind in ["left", "right", "top", "bottom"] {
        desktop.show("red.png", "none", 0);
        desktop.show("checker.png", kind, 1000);
        desktop.wait_for_handoff();
        // Capture during layer fade-out, then compare with the settled static
        // image. A frozen, offset slide buffer produces double checker edges.
        let handoff = desktop.capture(&desktop.monitor);
        std::thread::sleep(Duration::from_millis(600));
        let settled = desktop.capture(&desktop.monitor);
        let matching = desktop
            .pixels
            .iter()
            .filter(|&&i| {
                handoff.as_raw()[i * 3..i * 3 + 3]
                    .iter()
                    .zip(&settled.as_raw()[i * 3..i * 3 + 3])
                    .all(|(a, b)| a.abs_diff(*b) < 25)
            })
            .count();
        assert!(
            matching > desktop.pixels.len() * 98 / 100,
            "{kind}: shifted ghost during handoff: {matching}/{} pixels match",
            desktop.pixels.len()
        );
        eprintln!("{kind}: handoff matches settled wallpaper");
    }
}
