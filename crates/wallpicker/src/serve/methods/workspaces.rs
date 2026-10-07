//! workspaces.*: rows (Hyprland's workspaces and their wallpapers) and set
//! (a workspace's wallpaper; `path` null clears it).

use serde_json::Value;

use super::{json, param};
use crate::service::workspaces;
use crate::{AppState, CmdResult};

pub(super) async fn call(state: &AppState, name: &str, params: &Value) -> Option<CmdResult<Value>> {
    Some(match name {
        "rows" => json(workspaces::rows(state).await),
        "set" => match (param::<String>(params, "workspace"), param::<Option<String>>(params, "path")) {
            (Ok(ws), Ok(path)) => json(workspaces::set(state, ws, path).await),
            (Err(e), _) | (_, Err(e)) => Err(e),
        },
        _ => return None,
    })
}
