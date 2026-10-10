//! Adjustment updates cut immediately; wallpaper/preset selections still animate.
use crate::{
    Result,
    cmd::Runner,
    settings::{Settings, SpanLook},
    store::{Dirs, Store},
    wallpapers::{self, SpanAsk},
};
use std::cell::RefCell;
use std::path::Path;

#[derive(Default)]
struct Mock(RefCell<Vec<Vec<String>>>);
impl Runner for Mock {
    fn show_wallpaper(
        &self,
        _monitor: &str,
        _path: &Path,
        _fit: Option<crate::settings::FitMode>,
        transition: &crate::settings::Transition,
    ) -> Result<()> {
        self.run("wallpicker", &["--transition-type", &transition.kind])
            .map(drop)
    }
    fn run(&self, program: &str, args: &[&str]) -> Result<String> {
        self.0.borrow_mut().push(
            std::iter::once(program.to_owned())
                .chain(args.iter().map(|s| s.to_string()))
                .collect(),
        );
        if args == ["monitors", "-j"] {
            Ok(r#"[{"name":"DP-1","width":1920,"height":1080,"x":0,"y":0},{"name":"DP-2","width":1920,"height":1080,"x":1920,"y":0}]"#.into())
        } else {
            Ok(String::new())
        }
    }
}
impl Mock {
    fn transitions(&self) -> Vec<String> {
        self.0
            .borrow()
            .iter()
            .filter_map(|args| {
                args.iter()
                    .position(|a| a == "--transition-type")
                    .map(|i| args[i + 1].clone())
            })
            .collect()
    }
}
fn image(root: &Path, name: &str) -> std::path::PathBuf {
    let path = root.join(name);
    image::RgbImage::from_pixel(16, 8, image::Rgb([200, 40, 80]))
        .save(&path)
        .unwrap();
    path
}
fn setup(root: &Path) -> Store {
    let store = Store::new(Dirs::under(root));
    let mut settings = Settings::default();
    settings.transition.kind = "left".into();
    settings.transition.duration_ms = 1000;
    store.save_settings(&settings).unwrap();
    store
}

#[test]
fn reapplying_adjustments_cuts_but_selection_keeps_transition() {
    let root = tempfile::tempdir().unwrap();
    let store = setup(root.path());
    let path = image(root.path(), "a.png");
    let runner = Mock::default();
    wallpapers::apply_and_remember(&runner, &store, &["DP-1".into()], &path).unwrap();
    wallpapers::reapply(&runner, &store, &|_, _| true).unwrap();
    wallpapers::apply_and_remember(&runner, &store, &["DP-1".into()], &path).unwrap();
    assert_eq!(runner.transitions(), ["left", "none", "left"]);
    assert_eq!(store.load_settings().unwrap().transition.duration_ms, 1000);
    assert_eq!(store.load_settings().unwrap().transition.kind, "left");
}

#[test]
fn changing_span_look_cuts_but_new_span_wallpaper_animates() {
    let root = tempfile::tempdir().unwrap();
    let store = setup(root.path());
    let a = image(root.path(), "a.png");
    let b = image(root.path(), "b.png");
    let runner = Mock::default();
    let ask = || SpanAsk {
        monitors: Some(vec!["DP-1".into(), "DP-2".into()]),
        layout: None,
    };
    wallpapers::span_and_remember(&runner, &store, &a, SpanLook::default(), ask()).unwrap();
    let look = SpanLook {
        zoom: 150,
        x: 650,
        ..Default::default()
    };
    wallpapers::span_and_remember(&runner, &store, &a, look, ask()).unwrap();
    wallpapers::span_and_remember(&runner, &store, &b, look, ask()).unwrap();
    assert_eq!(runner.transitions(), ["left", "left", "none", "none", "left", "left"]);
    assert_eq!(store.load_settings().unwrap().transition.kind, "left");
}
