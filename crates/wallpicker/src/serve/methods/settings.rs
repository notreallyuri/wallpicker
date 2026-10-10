//! settings.*: options (what the form offers: from core), save (announcing
//! `settings-changed`, and `state-changed`: a changed fit re-applies
//! the wallpapers), conflicts and stop_program (other programs drawing
//! wallpapers).

use serde_json::{Value, json as value};
use wallpicker_core::colors::{HUES, TONES};
use wallpicker_core::settings::{self};

use super::super::Out;
use super::{json, param};
use crate::service::settings as service;
use crate::{AppState, CmdResult};

pub(super) async fn call(state: &AppState, out: &Out, name: &str, params: &Value) -> Option<CmdResult<Value>> {
    Some(match name {
        "options" => Ok(options()),
        "save" => match param(params, "settings") {
            Ok(s) => {
                let result = json(service::save(state, s).await);
                if result.is_ok() {
                    out.event("settings-changed", value!(null));
                    out.event("state-changed", value!(null));
                }
                result
            }
            Err(e) => Err(e),
        },
        "conflicts" => json(service::conflicts().await),
        "stop_program" => match param::<String>(params, "name") {
            Ok(name) => json(service::stop_program(name).await),
            Err(e) => Err(e),
        },
        _ => return None,
    })
}

/// The form's choices: native transitions, fits, provider sites and slideshow colors.
fn options() -> Value {
    let pairs = |items: &[(&str, &str)]| -> Vec<Value> {
        items.iter().map(|(v, l)| value!({ "value": v, "label": l })).collect()
    };
    value!({
        "transitions": pairs(&settings::TRANSITIONS),
        "fits": pairs(&settings::fit_modes()),
        "konachan_sites": pairs(&settings::KONACHAN_SITES),
        "hues": pairs(&HUES),
        "tones": pairs(&TONES),
    })
}
