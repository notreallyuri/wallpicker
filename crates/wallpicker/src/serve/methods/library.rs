//! library.*: the library (scan with places), thumbnails, the catalog
//! (favorites, tags, metadata; set_artists, origin_page), colors, duplicates.

use serde_json::{Value, json as value};

use super::super::Out;
use super::{json, param};
use crate::service::library;
use crate::{AppState, CmdResult};

pub(super) async fn call(state: &AppState, out: &Out, name: &str, params: &Value) -> Option<CmdResult<Value>> {
    let p = |key: &str| param::<String>(params, key);
    Some(match name {
        "scan" => json(library::library(state).await),
        "thumbnail" => match p("path") {
            Ok(path) => json(library::thumbnail(state, path).await),
            Err(e) => Err(e),
        },
        "catalog" => json(library::catalog(state).await),
        "set_favorite" => match (p("path"), param(params, "favorite")) {
            (Ok(path), Ok(favorite)) => json(library::set_favorite(state, path, favorite).await),
            (Err(e), _) | (_, Err(e)) => Err(e),
        },
        "add_tag" | "remove_tag" => match (p("path"), p("tag")) {
            (Ok(path), Ok(tag)) if name == "add_tag" => json(library::add_tag(state, path, tag).await),
            (Ok(path), Ok(tag)) => json(library::remove_tag(state, path, tag).await),
            (Err(e), _) | (_, Err(e)) => Err(e),
        },
        // Its page on the provider it came from, or null.
        "origin_page" => match p("path") {
            Ok(path) => json(library::origin_page(state, path).await),
            Err(e) => Err(e),
        },
        // `artists`: [{name, query?}], all of them, in order.
        "set_artists" => match (p("path"), param(params, "artists")) {
            (Ok(path), Ok(artists)) => json(library::set_artists(state, path, artists).await),
            (Err(e), _) | (_, Err(e)) => Err(e),
        },
        // [[path, score], ...], the most of the color first.
        "with_color" => match p("color") {
            Ok(color) => json(library::with_color(state, color).await),
            Err(e) => Err(e),
        },
        "duplicates" => json(library::duplicates(state).await),
        // Keep one, trash the others; the library changed.
        "resolve_duplicates" => match (p("keep"), param::<Vec<String>>(params, "others")) {
            (Ok(keep), Ok(others)) => {
                let result = json(library::resolve_duplicates_with(state, &keep, &others).await);
                if result.is_ok() {
                    out.event("library-changed", value!(null));
                    out.event("state-changed", value!(null));
                }
                result
            }
            (Err(e), _) | (_, Err(e)) => Err(e),
        },
        _ => return None,
    })
}
