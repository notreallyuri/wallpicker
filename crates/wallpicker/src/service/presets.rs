//! Saved setups use the same storage and application operations as the CLI.
use crate::{AppState, CmdResult, blocking};
use serde_json::{Value, json};
use wallpicker_core::{cmd::SystemRunner, presets};

pub async fn call(state: &AppState, name: String, id: String, title: String) -> CmdResult<Value> {
    let store = state.store.clone();
    blocking(move || {
        Ok(match name.as_str() {
            "list" => {
                let mut rows = Vec::new();
                for preset in store.load_presets()? {
                    let missing: Vec<_> = preset
                        .monitors
                        .values()
                        .filter(|m| !m.wallpaper.is_file())
                        .map(|m| m.wallpaper.clone())
                        .collect();
                    let mut row = json!(preset);
                    row["missing_files"] = json!(missing);
                    rows.push(row);
                }
                json!(rows)
            }
            "save" => json!(store.save_current_preset(&title)?),
            "update" => json!(store.update_current_preset(&id)?),
            "rename" => json!(store.rename_preset(&id, &title)?),
            "delete" => {
                store.delete_preset(&id)?;
                Value::Null
            }
            "apply" => json!(presets::apply(&SystemRunner, &store, &id)?),
            _ => unreachable!(),
        })
    })
    .await
}

/// Shared preview/save operations; saving is guarded against concurrent edits.
pub async fn reassign(
    state: &AppState,
    id: String,
    mapping: presets::MonitorMapping,
    expected: Option<presets::Preset>,
    preview: bool,
) -> CmdResult<Value> {
    let store = state.store.clone();
    blocking(move || {
        let preset = if preview {
            store.preview_preset_reassignment(&id, &mapping)?
        } else {
            store.reassign_preset(&id, &mapping, expected.as_ref())?
        };
        Ok(json!(preset))
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use wallpicker_core::{
        settings::State,
        store::{Dirs, Store},
    };

    #[test]
    fn picker_preset_crud_and_missing_files() {
        let root = tempfile::TempDir::new().unwrap();
        let store = Store::new(Dirs::under(root.path()));
        let mut current = State::default();
        let path = root.path().join("missing.png");
        current.current.insert("DP-1".into(), path.clone());
        store.save_state(&current).unwrap();
        let state = AppState::new(store);
        let request = |method: &str, id: &str, name: &str| {
            crate::rt::block_on(call(&state, method.into(), id.into(), name.into()))
        };
        let saved = request("save", "", "Night").unwrap();
        let id = saved["id"].as_str().unwrap();
        let listed = request("list", "", "").unwrap();
        assert_eq!(listed[0]["missing_files"][0], json!(path));
        std::fs::write(&path, b"file now exists").unwrap();
        assert_eq!(request("list", "", "").unwrap()[0]["missing_files"], json!([]));
        assert!(request("save", "", "night").is_err());
        assert_eq!(request("rename", id, "Day").unwrap()["id"], id);
        current.current.insert("DP-2".into(), path);
        state.store.save_state(&current).unwrap();
        assert_eq!(
            request("update", id, "").unwrap()["monitors"]
                .as_object()
                .unwrap()
                .len(),
            2
        );
        assert!(request("apply", "unknown", "").is_err());
        let before = state.store.load_presets().unwrap().remove(0);
        let mapping: presets::MonitorMapping = [("DP-1".into(), "DP-3".into())].into();
        let preview = crate::rt::block_on(reassign(&state, id.into(), mapping.clone(), None, true)).unwrap();
        assert!(preview["monitors"].get("DP-3").is_some());
        assert_eq!(state.store.load_presets().unwrap()[0], before);
        let saved = crate::rt::block_on(reassign(
            &state,
            id.into(),
            mapping.clone(),
            Some(before.clone()),
            false,
        ))
        .unwrap();
        assert_eq!(saved, preview);
        assert!(crate::rt::block_on(reassign(&state, id.into(), mapping, Some(before), false)).is_err());
        request("delete", id, "").unwrap();
        assert_eq!(request("list", "", "").unwrap(), json!([]));
        assert_eq!(state.store.load_state().unwrap().current, current.current);
    }
}
