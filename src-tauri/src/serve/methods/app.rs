//! app.settings, app.state, app.palette.

use serde_json::Value;

use super::json;
use crate::service::app;
use crate::{AppState, CmdResult};

pub(super) async fn call(state: &AppState, name: &str, _params: &Value) -> Option<CmdResult<Value>> {
    Some(match name {
        "settings" => json(app::settings(state).await),
        "state" => json(app::saved_state(state).await),
        "palette" => json(app::palette(state).await),
        _ => return None,
    })
}
