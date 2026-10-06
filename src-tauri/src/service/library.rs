//! The local library: the scan, thumbnails, favorites.

use std::path::PathBuf;

use wallpicker_core::catalog::Catalog;
use wallpicker_core::library::{self, LocalImage};
use wallpicker_core::thumbs;

use crate::{AppState, CmdResult, blocking, err, wallpaper_path};

/// Every image in the wallpaper folders, each with its cached thumbnail when
/// there is one.
pub(crate) async fn scan(state: &AppState) -> CmdResult<Vec<LocalImage>> {
    let store = state.store.clone();
    blocking(move || {
        let s = store.load_settings()?;
        Ok(library::with_cached_thumbs(library::scan(&s.folders, s.recursive), &store.dirs().cache, s.thumb_width))
    })
    .await
}

/// The path of `path`'s thumbnail, made now if there's none (a few at once:
/// decoding a big picture is memory-hungry).
pub(crate) async fn thumbnail(state: &AppState, path: String) -> CmdResult<String> {
    let _permit = state.thumb_gate.acquire().await.map_err(err)?;
    let store = state.store.clone();
    blocking(move || {
        let width = store.load_settings()?.thumb_width;
        let src = PathBuf::from(path);
        if !library::is_supported_image(&src) {
            return Err(wallpicker_core::Error::Invalid("unsupported image type".into()));
        }
        let out = thumbs::ensure_thumb(&store.dirs().cache, &src, width)?;
        Ok(out.to_string_lossy().into_owned())
    })
    .await
}

/// Change the catalog under the lock and return the saved version (two
/// changes at once mustn't lose one).
pub(crate) async fn update_catalog(state: &AppState, change: impl FnOnce(&mut Catalog) + Send + 'static) -> CmdResult<Catalog> {
    let store = state.store.clone();
    let lock = state.catalog_lock.clone();
    blocking(move || {
        // A panic elsewhere while holding the lock doesn't make the file bad.
        let _guard = lock.lock().unwrap_or_else(|p| p.into_inner());
        store.update_catalog(change)
    })
    .await
}

/// Favorites and tags.
pub(crate) async fn catalog(state: &AppState) -> CmdResult<Catalog> {
    let store = state.store.clone();
    blocking(move || store.load_catalog()).await
}

pub(crate) async fn set_favorite(state: &AppState, path: String, favorite: bool) -> CmdResult<Catalog> {
    let path = wallpaper_path(&path)?;
    update_catalog(state, move |c| c.set_favorite(&path, favorite)).await
}
