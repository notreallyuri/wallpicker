//! files.*: the system's file dialogs (the XDG portal): save, open, folder.
//! Params: title, and optionally folder (where it starts), name (save: the
//! suggested file name), filter ({name, glob}: "*.json"). The answer is the
//! picked path, or null when cancelled.

use std::path::PathBuf;

use serde::Deserialize;
use serde_json::Value;

use super::{json, param};
use crate::CmdResult;
use crate::service::files::{self, Filter};

#[derive(Deserialize)]
struct FilterParam {
    name: String,
    glob: String,
}

pub(super) async fn call(name: &str, params: &Value) -> Option<CmdResult<Value>> {
    let title = param::<String>(params, "title").unwrap_or_default();
    let folder = param::<Option<PathBuf>>(params, "folder").ok().flatten();
    let file = param::<Option<String>>(params, "name").ok().flatten();
    let filter = param::<Option<FilterParam>>(params, "filter").ok().flatten();
    let filter = filter.as_ref().map(|f| Filter { name: &f.name, glob: &f.glob });
    Some(match name {
        "save" => json(files::save_file(&title, folder.as_deref(), file.as_deref(), filter).await),
        "open" => json(files::open_file(&title, folder.as_deref(), filter).await),
        "folder" => json(files::folder(&title, folder.as_deref()).await),
        _ => return None,
    })
}
