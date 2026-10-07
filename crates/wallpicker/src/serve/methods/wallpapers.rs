//! wallpapers.*: monitors, apply, previous, adjust (announcing
//! `state-changed`: what the monitors show), the Adjust dialog's
//! framing_image and fit_modes, and the files: trash (announcing
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
        "adjust" => match (param(params, "monitor"), param(params, "adjust")) {
            (Ok(monitor), Ok(adjust)) => json(wallpapers::set_adjust(state, monitor, adjust).await),
            (Err(e), _) | (_, Err(e)) => Err(e),
        },
        "framing_image" => {
            return Some(match param::<String>(params, "path") {
                Ok(path) => json(wallpapers::framing_image(state, path).await),
                Err(e) => Err(e),
            });
        }
        "fit_modes" => return Some(json(wallpapers::fit_modes(state).await)),
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
