//! Retired backend settings never reset wallpaper state or unrelated preferences.
use crate::{
    settings::{Settings, State, TRANSITIONS},
    store::{Dirs, Store},
};
use serde_json::json;

#[test]
fn legacy_backend_choices_and_fps_are_ignored_without_losing_preferences() {
    for backend in ["hyprpaper", "awww", "swww", "noctalia", "wallpicker"] {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::new(Dirs::under(dir.path()));
        std::fs::create_dir_all(&store.dirs().config).unwrap();
        let old = json!({
            "wallpaper_backend": backend,
            "fit_mode": "contain",
            "monitor_fits": {"DP-2": "tile"},
            "transition": {"kind": "left", "duration_ms": 1700, "fps": 144},
            "noctalia_previous_scheme": "community Garnet"
        });
        std::fs::write(store.dirs().config.join("config.json"), old.to_string()).unwrap();
        let mut state = State::default();
        state.current.insert("DP-1".into(), dir.path().join("original.png"));
        store.save_state(&state).unwrap();
        let preset = store.save_current_preset("Keep").unwrap();
        let state_bytes = std::fs::read(store.dirs().state.join("state.json")).unwrap();
        let preset_bytes = std::fs::read(store.dirs().data.join("presets.json")).unwrap();
        let settings = store.load_settings().unwrap();
        assert_eq!(settings.transition.kind, "left");
        assert_eq!(settings.transition.duration_ms, 1700);
        assert_eq!(settings.fit_mode.unwrap().as_str(), "contain");
        assert_eq!(settings.monitor_fits["DP-2"].as_str(), "tile");
        assert_eq!(settings.noctalia_previous_scheme.as_deref(), Some("community Garnet"));
        store.save_settings(&settings).unwrap();
        let saved: serde_json::Value =
            serde_json::from_slice(&std::fs::read(store.dirs().config.join("config.json")).unwrap()).unwrap();
        assert!(saved.get("wallpaper_backend").is_none());
        assert!(saved["transition"].get("fps").is_none());
        assert_eq!(
            std::fs::read(store.dirs().state.join("state.json")).unwrap(),
            state_bytes
        );
        assert_eq!(
            std::fs::read(store.dirs().data.join("presets.json")).unwrap(),
            preset_bytes
        );
        assert_eq!(store.load_presets().unwrap(), [preset]);
        assert!(!store.dirs().state.join("hyprpaper.conf").exists());
    }
}
#[test]
fn supported_transitions_survive_and_simple_or_unknown_kinds_fall_back_to_fade() {
    for kind in TRANSITIONS.iter().map(|(kind, _)| *kind).chain(["simple", "unknown"]) {
        let settings: Settings =
            serde_json::from_value(json!({"transition": {"kind": kind, "duration_ms": 777}})).unwrap();
        let sanitized = settings.sanitized();
        assert_eq!(
            sanitized.transition.kind,
            if kind == "simple" || kind == "unknown" {
                "fade"
            } else {
                kind
            }
        );
        assert_eq!(sanitized.transition.duration_ms, 777);
    }
}
