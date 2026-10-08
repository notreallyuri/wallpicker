//! The local library: the scan, thumbnails, favorites, tags and artists,
//! colors, duplicates.

use std::path::PathBuf;

use wallpicker_core::catalog::{self, Catalog};
use wallpicker_core::cmd::SystemRunner;
use wallpicker_core::duplicates::DuplicateGroup;
use wallpicker_core::hyprpaper;
use wallpicker_core::store::Store;
use wallpicker_core::library::{self, LocalImage};
use wallpicker_core::thumbs;

use crate::service::wallpapers::trash_path;
use crate::{AppState, CmdResult, blocking, err, wallpaper_path};

/// Every image in the wallpaper folders, each with its cached thumbnail when
/// there is one.
pub async fn scan(state: &AppState) -> CmdResult<Vec<LocalImage>> {
    let store = state.store.clone();
    blocking(move || {
        let s = store.load_settings()?;
        Ok(library::with_cached_thumbs(library::scan(&s.folders, s.recursive), &store.dirs().cache, s.thumb_width))
    })
    .await
}

/// A scanned image and where it sits: which wallpaper folder (`root`, an
/// index into `Library::roots`) and the subfolder in it ("" at its top).
#[derive(serde::Serialize)]
pub struct Placed {
    #[serde(flatten)]
    pub image: LocalImage,
    pub root: usize,
    pub folder: String,
}

/// A wallpaper folder from the settings.
#[derive(serde::Serialize)]
pub struct Root {
    pub path: PathBuf,
    pub name: String,
}

/// The library with each image's place, for browsing by folder.
#[derive(serde::Serialize)]
pub struct Library {
    pub images: Vec<Placed>,
    pub roots: Vec<Root>,
}

pub async fn library(state: &AppState) -> CmdResult<Library> {
    let store = state.store.clone();
    let images = scan(state).await?;
    blocking(move || {
        let folders = store.load_settings()?.folders;
        let images = images
            .into_iter()
            .map(|image| {
                let (root, folder) = library::folder_in(&image.path, &folders).unwrap_or((0, String::new()));
                Placed { image, root, folder }
            })
            .collect();
        let roots = folders
            .iter()
            .map(|p| Root { name: p.file_name().map_or_else(|| p.display().to_string(), |n| n.to_string_lossy().into_owned()), path: p.clone() })
            .collect();
        Ok(Library { images, roots })
    })
    .await
}

/// The path of `path`'s thumbnail, made now if there's none (a few at once:
/// decoding a big picture is memory-hungry).
pub async fn thumbnail(state: &AppState, path: String) -> CmdResult<String> {
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
pub async fn update_catalog(state: &AppState, change: impl FnOnce(&mut Catalog) + Send + 'static) -> CmdResult<Catalog> {
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
pub async fn catalog(state: &AppState) -> CmdResult<Catalog> {
    let store = state.store.clone();
    blocking(move || store.load_catalog()).await
}

pub async fn set_favorite(state: &AppState, path: String, favorite: bool) -> CmdResult<Catalog> {
    let path = wallpaper_path(&path)?;
    update_catalog(state, move |c| c.set_favorite(&path, favorite)).await
}

pub async fn add_tag(state: &AppState, path: String, tag: String) -> CmdResult<Catalog> {
    let path = wallpaper_path(&path)?;
    if catalog::normalize_tag(&tag).is_none() {
        return Err(format!("Tags must be 1 to {} characters, without control characters.", catalog::MAX_TAG_LEN));
    }
    update_catalog(state, move |c| {
        c.add_tag(&path, &tag);
    })
    .await
}

pub async fn remove_tag(state: &AppState, path: String, tag: String) -> CmdResult<Catalog> {
    let path = wallpaper_path(&path)?;
    update_catalog(state, move |c| c.remove_tag(&path, &tag)).await
}

/// The page `path` was downloaded from, on its provider's site (None when
/// it wasn't, or that isn't known).
pub async fn origin_page(state: &AppState, path: String) -> CmdResult<Option<String>> {
    let path = wallpaper_path(&path)?;
    let store = state.store.clone();
    blocking(move || {
        let site = store.load_settings()?.konachan_site;
        let catalog = store.load_catalog()?;
        Ok(catalog.metadata(&path).and_then(|m| m.origin.as_ref()).and_then(|o| wallpicker_core::online::origin_page(o, site)))
    })
    .await
}

/// Make `path`'s artists `artists` (cleaned like tags, each once).
pub async fn set_artists(state: &AppState, path: String, artists: Vec<catalog::Person>) -> CmdResult<Catalog> {
    let path = wallpaper_path(&path)?;
    if let Some(bad) = artists.iter().find(|a| catalog::normalize_tag(&a.name).is_none()) {
        return Err(format!(
            "{:?}: names must be 1 to {} characters, without control characters.",
            bad.name,
            catalog::MAX_TAG_LEN
        ));
    }
    update_catalog(state, move |c| c.set_artists(&path, &artists)).await
}

/// The wallpapers with `color` (a hue, or dark / light / warm / muted), the
/// most of it first: (path, score).
pub async fn with_color(state: &AppState, color: String) -> CmdResult<Vec<(PathBuf, f32)>> {
    let Some(color) = wallpicker_core::settings::known_color(Some(color.clone())) else {
        return Err(format!("{color}? one of {}", wallpicker_core::colors::HUES.iter().chain(wallpicker_core::colors::TONES.iter()).map(|(k, _)| *k).collect::<Vec<_>>().join(", ")));
    };
    let store = state.store.clone();
    blocking(move || {
        let mut found: Vec<(PathBuf, f32)> = color_profiles(&store)?
            .into_iter()
            .filter(|(_, p)| wallpicker_core::colors::matches(p, &color))
            .map(|(path, p)| {
                let score = wallpicker_core::colors::score(&p, &color);
                (path, score)
            })
            .collect();
        found.sort_by(|a, b| b.1.total_cmp(&a.1));
        Ok(found)
    })
    .await
}

#[derive(Debug, Default, serde::Serialize)]
pub struct ResolveReport {
    pub trashed: Vec<String>,
    /// (path, why)
    pub failed: Vec<(String, String)>,
    /// Monitors that showed a trashed copy and now show the kept one.
    pub reapplied: Vec<String>,
}

/// The library's duplicate groups, best first in each.
pub async fn duplicates(state: &AppState) -> CmdResult<Vec<DuplicateGroup>> {
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

/// Keep `keep`, move `others` to the trash. The kept one gets their
/// favorite, tags, metadata and fit (when it has none), and goes on any monitor that
/// showed one of them. Each copy is trashed with the usual checks; one that
/// fails doesn't stop the rest.
pub async fn resolve_duplicates_with(
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
        let metas: Vec<_> = o.iter().filter_map(|p| c.metadata(p).cloned()).collect();
        if favorite {
            c.set_favorite(&k, true);
        }
        for t in &tags {
            c.add_tag(&k, t);
        }
        for m in &metas {
            c.fill_metadata(&k, m);
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
