//! Noctalia's palette remains independent of retired wallpaper backends.
use serde_json::{Value, json};
use std::{fs, os::unix::fs::PermissionsExt, process::Command};

#[test]
fn colors_follow_native_wallpapers_and_restore_the_previous_scheme() {
    let root = tempfile::tempdir().unwrap();
    let dir = root.path();
    let bin = dir.join("bin");
    fs::create_dir(&bin).unwrap();
    let mock = bin.join("noctalia");
    fs::write(
        &mock,
        r#"#!/usr/bin/python3
import json, os, sys
from pathlib import Path
root = Path(os.environ['COLOR_TEST_ROOT'])
a = sys.argv[1:]
with (root/'calls').open('a') as f: f.write(json.dumps(a)+'\n')
if a == ['msg', 'color-scheme-get']:
    print((root/'scheme').read_text())
elif a[:2] == ['msg', 'color-scheme-set']:
    (root/'scheme').write_text(' '.join(a[2:]))
elif a[0] == 'theme':
    roles = ['primary','on_primary','secondary','on_secondary','tertiary','on_tertiary',
        'error','on_error','surface','on_surface','surface_variant','on_surface_variant',
        'outline','shadow','hover','on_hover','terminal_foreground','terminal_background',
        'terminal_cursor','terminal_cursor_text','terminal_selection_fg','terminal_selection_bg']
    roles += ['terminal_'+p+'_'+c for p in ['normal','bright']
        for c in ['black','red','green','yellow','blue','magenta','cyan','white']]
    values = {r:'#123456' for r in roles}
    print(json.dumps({'dark':values,'light':values}))
else:
    sys.exit(99)
"#,
    )
    .unwrap();
    fs::set_permissions(&mock, fs::Permissions::from_mode(0o755)).unwrap();
    fs::write(dir.join("scheme"), "community Garnet").unwrap();
    for folder in [
        "config/wallpicker",
        "state/wallpicker",
        "data/wallpicker",
        "cache/wallpicker",
    ] {
        fs::create_dir_all(dir.join(folder)).unwrap();
    }
    let config = dir.join("config/wallpicker/config.json");
    fs::write(
        &config,
        json!({"wallpaper_backend":"noctalia","transition":{"kind":"simple","fps":120}}).to_string(),
    )
    .unwrap();
    let first = dir.join("first.png");
    let second = dir.join("second.png");
    fs::write(&first, "first mock image").unwrap();
    fs::write(&second, "second mock image").unwrap();
    let state = dir.join("state/wallpicker/state.json");
    let set_image = |image: &std::path::Path| fs::write(&state, json!({"current":{"DP-1":image}}).to_string()).unwrap();
    set_image(&first);
    let run = |args: &[&str]| {
        let out = Command::new(env!("CARGO_BIN_EXE_wallpicker"))
            .args(args)
            .env("PATH", &bin)
            .env("COLOR_TEST_ROOT", dir)
            .env("XDG_CONFIG_HOME", dir.join("config"))
            .env("XDG_STATE_HOME", dir.join("state"))
            .env("XDG_DATA_HOME", dir.join("data"))
            .env("XDG_CACHE_HOME", dir.join("cache"))
            .output()
            .unwrap();
        assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
        String::from_utf8(out.stdout).unwrap()
    };
    assert!(run(&["noctalia-colors", "on"]).starts_with("follows the wallpaper"));
    assert_eq!(fs::read_to_string(dir.join("scheme")).unwrap(), "custom wallpicker");
    let palette: Value =
        serde_json::from_slice(&fs::read(dir.join("config/noctalia/palettes/wallpicker.json")).unwrap()).unwrap();
    assert_eq!(palette["dark"]["mPrimary"], "#123456");
    set_image(&second);
    assert!(run(&["noctalia-colors", "on"]).starts_with("follows the wallpaper"));
    assert!(run(&["noctalia-colors", "off"]).starts_with("its own scheme"));
    assert_eq!(fs::read_to_string(dir.join("scheme")).unwrap(), "community Garnet");
    let calls: Vec<Value> = fs::read_to_string(dir.join("calls"))
        .unwrap()
        .lines()
        .map(|s| serde_json::from_str(s).unwrap())
        .collect();
    let themes: Vec<_> = calls.iter().filter(|a| a[0] == "theme").collect();
    assert_eq!(themes.len(), 2);
    assert_eq!(themes[0][1], first.to_str().unwrap());
    assert_eq!(themes[1][1], second.to_str().unwrap());
    assert!(!calls.iter().any(|a| a[1] == "wallpaper-set"));
    let saved: Value = serde_json::from_slice(&fs::read(config).unwrap()).unwrap();
    assert!(saved.get("wallpaper_backend").is_none());
    assert_eq!(saved["transition"]["kind"], "fade");
}
