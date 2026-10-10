//! presets.*: stable IDs, explicit snapshot replacement, and application reports.
use super::{super::Out, param};
use crate::{AppState, CmdResult, service};
use serde_json::{Value, json};

pub(super) async fn call(state: &AppState, out: &Out, name: &str, params: &Value) -> Option<CmdResult<Value>> {
    if !matches!(name, "list" | "save" | "update" | "rename" | "delete" | "apply") {
        return None;
    }
    let args = (|| {
        let id = if matches!(name, "update" | "rename" | "delete" | "apply") {
            param(params, "id")?
        } else {
            String::new()
        };
        let title = if matches!(name, "save" | "rename") {
            param(params, "name")?
        } else {
            String::new()
        };
        Ok::<_, String>((id, title))
    })();
    let result = match args {
        Ok((id, title)) => service::presets::call(state, name.into(), id, title).await,
        Err(e) => Err(e),
    };
    if result.is_ok() && name != "list" {
        out.event(
            if name == "apply" {
                "state-changed"
            } else {
                "presets-changed"
            },
            json!(null),
        );
        if name == "apply" {
            out.event("settings-changed", json!(null));
        }
    }
    Some(result)
}
