//! backup.*: default_path (where an export is suggested), export, import
//! (merged or replacing), snapshots, snapshot_dir, set_snapshot_dir (`dir`
//! null: the default folder), restore. Import and restore change the
//! library's favorites and tags: they announce `library-changed`.

use serde_json::{Value, json as value};

use super::super::Out;
use super::{json, param};
use crate::service::backup;
use crate::{AppState, CmdResult};

pub(super) async fn call(state: &AppState, out: &Out, name: &str, params: &Value) -> Option<CmdResult<Value>> {
    let p = |key: &str| param::<String>(params, key);
    let changed = |result: CmdResult<Value>| {
        if result.is_ok() {
            out.event("library-changed", value!(null));
            out.event("presets-changed", value!(null));
        }
        result
    };
    Some(match name {
        "default_path" => json(Ok(backup::default_export_path())),
        "export" => match p("path") {
            Ok(path) => json(backup::export(state, path).await),
            Err(e) => Err(e),
        },
        "import" => match (p("path"), param::<bool>(params, "replace")) {
            (Ok(path), Ok(replace)) => changed(json(backup::import(state, path, replace).await)),
            (Err(e), _) | (_, Err(e)) => Err(e),
        },
        "snapshots" => json(backup::snapshots(state).await),
        "snapshot_dir" => json(backup::snapshot_dir(state).await),
        "set_snapshot_dir" => match param(params, "dir") {
            Ok(dir) => json(backup::set_snapshot_dir(state, dir).await),
            Err(e) => Err(e),
        },
        "restore" => match p("id") {
            Ok(id) => changed(json(backup::restore(state, id).await)),
            Err(e) => Err(e),
        },
        _ => return None,
    })
}
