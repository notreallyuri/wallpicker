//! library.scan, library.thumbnail, library.catalog, library.set_favorite.

use serde_json::Value;

use super::{json, param};
use crate::service::library;
use crate::{AppState, CmdResult};

pub(super) async fn call(state: &AppState, name: &str, params: &Value) -> Option<CmdResult<Value>> {
    Some(match name {
        "scan" => json(library::scan(state).await),
        "thumbnail" => match param(params, "path") {
            Ok(path) => json(library::thumbnail(state, path).await),
            Err(e) => Err(e),
        },
        "catalog" => json(library::catalog(state).await),
        "set_favorite" => match (param(params, "path"), param(params, "favorite")) {
            (Ok(path), Ok(favorite)) => json(library::set_favorite(state, path, favorite).await),
            (Err(e), _) | (_, Err(e)) => Err(e),
        },
        _ => return None,
    })
}
