//! The local library: scanning, thumbnails, favorites and tags, colors and
//! duplicates.

use std::path::PathBuf;
use wallpicker_core::catalog::{self, Catalog};
use wallpicker_core::cmd::SystemRunner;
use wallpicker_core::duplicates::DuplicateGroup;
use wallpicker_core::hyprpaper::{self};
use wallpicker_core::library::{self, LocalImage};
use wallpicker_core::store::Store;

use crate::commands::wallpapers::trash_path;
use crate::service::{self, library::update_catalog};
use crate::{AppState, CmdResult, blocking, err, wallpaper_path};

#[tauri::command]
pub(crate) async fn scan_local(state: tauri::State<'_, AppState>) -> CmdResult<Vec<LocalImage>> {
    service::library::scan(&state).await
}

/// Returns the path of a cached thumbnail for `path`.
#[tauri::command]
pub(crate) async fn thumbnail(state: tauri::State<'_, AppState>, path: String) -> CmdResult<String> {
    service::library::thumbnail(&state, path).await
}

#[tauri::command]
pub(crate) async fn get_catalog(state: tauri::State<'_, AppState>) -> CmdResult<Catalog> {
    service::library::catalog(&state).await
}

#[tauri::command]
pub(crate) async fn set_favorite(state: tauri::State<'_, AppState>, path: String, favorite: bool) -> CmdResult<Catalog> {
    service::library::set_favorite(&state, path, favorite).await
}

/// The library's duplicate groups, best first in each.
pub(crate) async fn duplicates(state: &AppState) -> CmdResult<Vec<DuplicateGroup>> {
    let store = state.store.clone();
    blocking(move || {
        let settings = store.load_settings()?;
        let images = library::scan(&settings.folders, settings.recursive);
        let catalog = store.load_catalog().unwrap_or_default();
        Ok(wallpicker_core::duplicates::find(
            &images,
            &catalog,
            &store.dirs().cache,
            settings.thumb_width,
        ))
    })
    .await
}

/// Every library image's colors (computed once, cached), by path.
pub fn color_profiles(
    store: &Store,
) -> wallpicker_core::error::Result<
    std::collections::BTreeMap<PathBuf, wallpicker_core::colors::ColorProfile>,
> {
    let settings = store.load_settings()?;
    let images = library::scan(&settings.folders, settings.recursive);
    Ok(wallpicker_core::colors::profiles(
        &images,
        &store.dirs().cache,
        settings.thumb_width,
    ))
}

#[tauri::command]
pub(crate) async fn image_colors(
    state: tauri::State<'_, AppState>,
) -> CmdResult<std::collections::BTreeMap<PathBuf, wallpicker_core::colors::ColorProfile>> {
    let store = state.store.clone();
    blocking(move || color_profiles(&store)).await
}

#[tauri::command]
pub(crate) async fn find_duplicates(
    state: tauri::State<'_, AppState>,
) -> CmdResult<Vec<DuplicateGroup>> {
    duplicates(&state).await
}

#[derive(Debug, Default, serde::Serialize)]
pub struct ResolveReport {
    pub trashed: Vec<String>,
    /// (path, why)
    pub failed: Vec<(String, String)>,
    /// Monitors that showed a trashed copy and now show the kept one.
    pub reapplied: Vec<String>,
}

/// Keep `keep`, move `others` to the trash. The kept one gets their
/// favorite, tags and fit (when it has none), and goes on any monitor that
/// showed one of them. Each copy is trashed with the usual checks; one that
/// fails doesn't stop the rest.
pub(crate) async fn resolve_duplicates_with(
    state: &AppState,
    keep: &str,
    others: &[String],
) -> CmdResult<ResolveReport> {
    let keep_path = wallpaper_path(keep)?;
    let store = state.store.clone();
    let check = keep_path.clone();
    blocking(move || {
        let s = store.load_settings()?;
        let mut roots = s.folders.clone();
        roots.push(s.download_dir.clone());
        library::library_file(&check, &roots)
    })
    .await?;
    let other_paths: Vec<PathBuf> = others
        .iter()
        .map(|o| wallpaper_path(o))
        .collect::<CmdResult<_>>()?;
    if other_paths.contains(&keep_path) {
        return Err("The kept wallpaper can't also be trashed.".into());
    }

    let (k, o) = (keep_path.clone(), other_paths.clone());
    update_catalog(state, move |c| {
        let favorite = o.iter().any(|p| c.is_favorite(p));
        let tags: Vec<String> = o.iter().flat_map(|p| c.tags(p).to_vec()).collect();
        if favorite {
            c.set_favorite(&k, true);
        }
        for t in &tags {
            c.add_tag(&k, t);
        }
    })
    .await?;

    let shown: Vec<String> = state
        .store
        .load_state()
        .map_err(err)?
        .current
        .iter()
        .filter(|(_, p)| other_paths.contains(p))
        .map(|(m, _)| m.clone())
        .collect();

    let mut report = ResolveReport::default();
    for o in others {
        match trash_path(state, o).await {
            Ok(()) => report.trashed.push(o.clone()),
            Err(e) => report.failed.push((o.clone(), e)),
        }
    }
    if !shown.is_empty() {
        let store = state.store.clone();
        let keep = keep_path.clone();
        let applied = blocking(move || {
            let backend = store.load_settings()?.wallpaper_backend;
            hyprpaper::apply_and_remember_with(&SystemRunner, &store, backend, &shown, &keep)
        })
        .await?;
        report.reapplied = applied.applied;
    }
    Ok(report)
}

#[tauri::command]
pub(crate) async fn resolve_duplicates(
    state: tauri::State<'_, AppState>,
    keep: String,
    others: Vec<String>,
) -> CmdResult<ResolveReport> {
    resolve_duplicates_with(&state, &keep, &others).await
}

#[tauri::command]
pub(crate) async fn add_tag(
    state: tauri::State<'_, AppState>,
    path: String,
    tag: String,
) -> CmdResult<Catalog> {
    let path = wallpaper_path(&path)?;
    if catalog::normalize_tag(&tag).is_none() {
        return Err(format!(
            "Tags must be 1 to {} characters, without control characters.",
            catalog::MAX_TAG_LEN
        ));
    }
    update_catalog(&state, move |c| {
        c.add_tag(&path, &tag);
    })
    .await
}

#[tauri::command]
pub(crate) async fn remove_tag(
    state: tauri::State<'_, AppState>,
    path: String,
    tag: String,
) -> CmdResult<Catalog> {
    let path = wallpaper_path(&path)?;
    update_catalog(&state, move |c| c.remove_tag(&path, &tag)).await
}
