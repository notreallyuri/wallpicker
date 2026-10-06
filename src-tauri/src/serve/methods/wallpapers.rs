//! wallpapers.*: monitors, apply, previous (announcing `state-changed`:
//! what the monitors show), and the files: trash (announcing
//! `library-changed` too), show_in_folder.

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
        "show_in_folder" => {
            return Some(match param::<String>(params, "path") {
                Ok(path) => json(wallpapers::show_in_folder(state, path).await),
                Err(e) => Err(e),
            });
        }
        "trash" => match param::<String>(params, "path") {
            Ok(path) => {
                let result = json(wallpapers::trash_path(state, &path).await);
                if result.is_ok() {
                    out.event("library-changed", value!(null));
                }
                result
            }
            Err(e) => Err(e),
        },
        _ => return None,
    };
    if result.is_ok() {
        out.event("state-changed", value!(null));
    }
    Some(result)
}
