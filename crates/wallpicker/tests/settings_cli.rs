//! Removed backend and frame-rate selectors fail atomically through the real CLI.
use serde_json::{Value, json};
use std::{fs, process::Command};

#[test]
fn settings_offer_native_choices_and_reject_retired_keys_without_saving() {
    let root = tempfile::tempdir().unwrap();
    let config = root.path().join("config/wallpicker/config.json");
    fs::create_dir_all(config.parent().unwrap()).unwrap();
    let old = json!({"wallpaper_backend":"swww","transition":{"kind":"right","duration_ms":800,"fps":144}}).to_string();
    fs::write(&config, &old).unwrap();
    let run = |args: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_wallpicker"))
            .args(args)
            .env("XDG_CONFIG_HOME", root.path().join("config"))
            .env("XDG_STATE_HOME", root.path().join("state"))
            .env("XDG_DATA_HOME", root.path().join("data"))
            .env("XDG_CACHE_HOME", root.path().join("cache"))
            .output()
            .unwrap()
    };
    let output = run(&["settings", "--json"]);
    assert!(output.status.success());
    let settings: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(settings.get("backend").is_none());
    assert!(settings["options"].get("backends").is_none());
    assert_eq!(settings["transition"]["kind"], "right");
    assert_eq!(settings["options"]["transitions"].as_array().unwrap().len(), 13);
    assert!(
        !settings["options"]["transitions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|c| c["value"] == "simple")
    );
    for key in ["backend=noctalia", "backend=awww", "transition.fps=60"] {
        let output = run(&["settings", "set", "thumb_width=480", key]);
        assert_eq!(output.status.code(), Some(2));
        assert_eq!(fs::read_to_string(&config).unwrap(), old);
    }
}
