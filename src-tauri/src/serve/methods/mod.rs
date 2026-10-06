//! The methods `serve` answers, by area: each area's `call` takes the
//! method's name after the dot ("library.scan" -> library, "scan").

mod app;
mod library;
mod wallpapers;

use serde::de::DeserializeOwned;
use serde_json::Value;

use super::Out;
use crate::{AppState, CmdResult};

pub(super) async fn call(state: &AppState, out: &Out, method: &str, params: Value) -> CmdResult<Value> {
    let (area, name) = method.split_once('.').unwrap_or((method, ""));
    let result = match area {
        "app" => app::call(state, name, &params).await,
        "library" => library::call(state, out, name, &params).await,
        "wallpapers" => wallpapers::call(state, out, name, &params).await,
        _ => None,
    };
    result.unwrap_or_else(|| Err(format!("no method {method}")))
}

/// `params[key]` as a `T`.
fn param<T: DeserializeOwned>(params: &Value, key: &str) -> CmdResult<T> {
    let value = params.get(key).cloned().unwrap_or(Value::Null);
    serde_json::from_value(value).map_err(|e| format!("{key}: {e}"))
}

/// A result as JSON.
fn json<T: serde::Serialize>(result: CmdResult<T>) -> CmdResult<Value> {
    result.and_then(|v| serde_json::to_value(v).map_err(|e| e.to_string()))
}
