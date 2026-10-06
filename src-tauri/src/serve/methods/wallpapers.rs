//! wallpapers.monitors, wallpapers.apply, wallpapers.previous. Applying and
//! going back announce `state-changed` (what the monitors show).

use serde_json::{Value, json as value};

use super::super::Out;
use super::{json, param};
use crate::service::wallpapers;
use crate::{AppState, CmdResult};

pub(super) async fn call(state: &AppState, out: &Out, name: &str, params: &Value) -> Option<CmdResult<Value>> {
    let result = match name {
        "monitors" => return Some(json(wallpapers::monitors().await)),
        "apply" => match (param(params, "monitors"), param(params, "path")) {
            (Ok(monitors), Ok(path)) => json(wallpapers::apply(state, monitors, path).await),
            (Err(e), _) | (_, Err(e)) => Err(e),
        },
        "previous" => match param(params, "monitors") {
            Ok(monitors) => json(wallpapers::previous(state, monitors).await),
            Err(e) => Err(e),
        },
        _ => return None,
    };
    if result.is_ok() {
        out.event("state-changed", value!(null));
    }
    Some(result)
}
