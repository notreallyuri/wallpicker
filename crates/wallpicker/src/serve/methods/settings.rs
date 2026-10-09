//! settings.*: options (what the form offers: from core), save (announcing
//! `settings-changed`, and `state-changed`: a new backend or fit re-applies
//! the wallpapers), conflicts and stop_program (other programs drawing
//! wallpapers).

use serde_json::{Value, json as value};
use wallpicker_core::colors::{HUES, TONES};
use wallpicker_core::settings::{self, WallpaperBackend};

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
        "conflicts" => match param(params, "backend") {
            Ok(backend) => json(service::conflicts(backend).await),
            Err(e) => Err(e),
        },
        "stop_program" => match param::<String>(params, "name") {
            Ok(name) => json(service::stop_program(name).await),
            Err(e) => Err(e),
        },
        _ => return None,
    })
}

/// The form's choices, each [{value, label}]: the backends, awww's
/// transitions, the fits each backend does ("" is its default), Konachan's
/// sites, the colors a slideshow set can ask for.
fn options() -> Value {
    let pairs = |items: &[(&str, &str)]| -> Vec<Value> {
        items.iter().map(|(v, l)| value!({ "value": v, "label": l })).collect()
    };
    value!({
        "backends": pairs(&settings::BACKENDS),
        "transitions": pairs(&settings::TRANSITIONS),
        "fits": {
            "hyprpaper": pairs(&settings::fit_modes(WallpaperBackend::Hyprpaper)),
            "awww": pairs(&settings::fit_modes(WallpaperBackend::Awww)),
            "wallpicker": pairs(&settings::fit_modes(WallpaperBackend::Wallpicker)),
        },
        "konachan_sites": pairs(&settings::KONACHAN_SITES),
        "hues": pairs(&HUES),
        "tones": pairs(&TONES),
    })
}
