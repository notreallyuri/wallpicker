//! app.settings, app.state, app.palette, app.copy_text, app.usage (Settings
//! → Performance), app.noctalia_colors and app.set_noctalia_colors.

use serde_json::Value;

use super::{json, param};
use crate::service::app;
use crate::{AppState, CmdResult};

pub(super) async fn call(state: &AppState, name: &str, params: &Value) -> Option<CmdResult<Value>> {
    Some(match name {
        "settings" => json(app::settings(state).await),
        "state" => json(app::saved_state(state).await),
        "palette" => json(app::palette(state).await),
        "copy_text" => match param::<String>(params, "text") {
            Ok(text) => json(app::copy_text(text).await),
            Err(e) => Err(e),
        },
        "usage" => json(app::usage().await),
        "noctalia_colors" => json(app::noctalia_colors(state).await),
        "set_noctalia_colors" => match param::<bool>(params, "follow") {
            Ok(follow) => json(app::set_noctalia_colors(state, follow).await),
            Err(e) => Err(e),
        },
        _ => return None,
    })
}
