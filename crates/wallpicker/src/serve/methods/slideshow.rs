//! slideshow.*: status (when it changes next) and now (Change now,
//! announcing `state-changed`).

use serde_json::{Value, json as value};

use super::super::Out;
use super::json;
use crate::service::slideshow;
use crate::{AppState, CmdResult};

pub(super) async fn call(state: &AppState, out: &Out, name: &str) -> Option<CmdResult<Value>> {
    Some(match name {
        "status" => json(slideshow::status(state).await),
        "now" => {
            let result = json(slideshow::now(state).await);
            // Some monitors may have changed even when others failed.
            out.event("state-changed", value!(null));
            result
        }
        _ => return None,
    })
}
