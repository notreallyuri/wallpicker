//! Exercise the real CLI in an isolated XDG profile; no compositor changes.

use std::process::{Command, Output};
use tempfile::TempDir;
use wallpicker_core::{
    settings::State,
    store::{Dirs, Store},
};

fn run(root: &TempDir, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_wallpicker"))
        .env("XDG_CONFIG_HOME", root.path().join("config"))
        .env("XDG_STATE_HOME", root.path().join("state"))
        .env("XDG_DATA_HOME", root.path().join("data"))
        .env("XDG_CACHE_HOME", root.path().join("cache"))
        .args(args)
        .output()
        .unwrap()
}

fn json(root: &TempDir, args: &[&str]) -> serde_json::Value {
    let output = run(root, args);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

#[test]
fn cli_preset_crud_roundtrip_and_errors() {
    let root = TempDir::new().unwrap();
    let store = Store::new(Dirs {
        config: root.path().join("config/wallpicker"),
        state: root.path().join("state/wallpicker"),
        data: root.path().join("data/wallpicker"),
        cache: root.path().join("cache/wallpicker"),
    });
    assert_eq!(
        json(&root, &["presets", "list", "--json"]),
        serde_json::json!([])
    );
    assert!(!run(&root, &["presets", "save", "Empty"]).status.success());
    let mut state = State::default();
    state
        .current
        .insert("DP-1".into(), root.path().join("source.png"));
    store.save_state(&state).unwrap();
    let saved = json(&root, &["presets", "save", "Matching pair", "--json"]);
    let id = saved["id"].as_str().unwrap();
    assert_eq!(saved["name"], "Matching pair");
    assert_eq!(json(&root, &["presets", "list", "--json"])[0], saved);
    let renamed = json(&root, &["presets", "rename", id, "Night", "--json"]);
    assert_eq!(renamed["id"], id);
    assert_eq!(renamed["name"], "Night");
    state
        .current
        .insert("DP-1".into(), root.path().join("changed.png"));
    store.save_state(&state).unwrap();
    let updated = json(&root, &["presets", "update", id, "--json"]);
    assert_eq!(
        updated["monitors"]["DP-1"]["wallpaper"],
        root.path().join("changed.png").to_str().unwrap()
    );
    assert!(!run(&root, &["presets", "save", "night"]).status.success());
    assert!(
        !run(&root, &["presets", "apply", "nonexistent"])
            .status
            .success()
    );
    assert!(!run(&root, &["presets", "rename", id]).status.success());
    assert!(
        !run(&root, &["presets", "save", "Name", "--invalid"])
            .status
            .success()
    );
    assert!(run(&root, &["presets", "--help"]).status.success());
    assert!(run(&root, &["presets", "delete", id]).status.success());
    assert_eq!(
        json(&root, &["presets", "list", "--json"]),
        serde_json::json!([])
    );
}
