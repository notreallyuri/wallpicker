//! Downloading online wallpapers into the download folder (with their
//! metadata and any tags picked), and which ones are there already.

use std::path::PathBuf;

use wallpicker_core::online::OnlineWallpaper;
use wallpicker_core::store::Store;

use super::{details, http_error};
use crate::service::library::update_catalog;
use crate::{AppState, CmdResult, blocking, err};

/// Largest download we accept. Wallpapers are a few MB; this only guards
/// against a misbehaving server.
const MAX_DOWNLOAD_BYTES: usize = 64 * 1024 * 1024;

/// Keys ("provider:id") of the wallpapers already in the download folder: the
/// files [`download_and_tag`] would reuse instead of downloading again.
pub async fn downloaded(state: &AppState) -> CmdResult<Vec<String>> {
    let store = state.store.clone();
    blocking(move || downloaded_keys(&store)).await
}

pub fn downloaded_keys(store: &Store) -> wallpicker_core::Result<Vec<String>> {
    let dir = store.load_settings()?.download_dir;
    let entries = match std::fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e.into()),
    };
    Ok(entries
        .flatten()
        .filter(|e| e.metadata().is_ok_and(|m| m.is_file() && m.len() > 0))
        .filter_map(|e| wallpicker_core::online::key_from_file_name(e.file_name().to_str()?))
        .collect())
}

/// Download a wallpaper into the configured download folder and return where
/// it was saved. Already-downloaded files are reused. New downloads (and old
/// ones without it) get their metadata: where they came from, the source
/// link, uploader and artists (an old one's artist tags then go: they're
/// its artists now). `chosen` (some of its tags, picked in the
/// details) are added as tags either way. `progress(done, total)` follows
/// the bytes (total when the server says).
pub async fn download_and_tag(
    state: &AppState,
    wallpaper: &OnlineWallpaper,
    chosen: &[String],
    progress: impl Fn(usize, Option<usize>),
) -> CmdResult<String> {
    let (dest, fresh) = download(state, wallpaper, progress).await?;

    let store = state.store.clone();
    let check = dest.clone();
    let without = blocking(move || Ok(store.load_catalog()?.metadata(&check).is_none_or(|m| m.is_empty())))
        .await
        .unwrap_or(false);
    // Best effort: the file is saved either way.
    let mut meta = None;
    if fresh || without {
        // Wallhaven's need a request; Moebooru results carry theirs.
        match details(state, wallpaper).await {
            Ok(d) => meta = Some(wallpaper.metadata(&d)),
            Err(e) => eprintln!("wallpicker: no metadata for {}: {e}", dest.display()),
        }
    }
    if meta.is_some() || !chosen.is_empty() {
        let (path, tags) = (dest.clone(), chosen.to_vec());
        let saved = update_catalog(state, move |c| {
            if let Some(meta) = &meta {
                c.fill_metadata(&path, meta);
                c.untag_artists(&path);
            }
            for name in &tags {
                c.add_tag(&path, name);
            }
        })
        .await;
        if let Err(e) = saved {
            eprintln!("wallpicker: could not tag {}: {e}", dest.display());
        }
    }
    Ok(dest.to_string_lossy().into_owned())
}

/// Returns where the file is, and whether it was downloaded just now.
async fn download(
    state: &AppState,
    wallpaper: &OnlineWallpaper,
    progress: impl Fn(usize, Option<usize>),
) -> CmdResult<(PathBuf, bool)> {
    // The wallpaper comes back from the UI, so re-check what ends up in the
    // file name and which host we contact.
    let file_name = wallpaper.file_name().map_err(err)?;
    if !wallpaper.is_allowed_download_url() {
        return Err("Refusing to download from an unexpected address.".into());
    }
    let provider = wallpaper.provider;

    let store = state.store.clone();
    let dir = blocking(move || Ok(store.load_settings()?.download_dir)).await?;
    let dest = dir.join(&file_name);

    if dest.metadata().map(|m| m.len() > 0).unwrap_or(false) {
        return Ok((dest, false));
    }

    let mut resp = state.http.get(&wallpaper.full).send().await.map_err(err)?;
    if !resp.status().is_success() {
        return Err(http_error(provider, resp.status()));
    }
    let is_image = resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.starts_with("image/"));
    if !is_image {
        return Err(format!("{} did not return an image.", provider.label()));
    }
    let too_big = || "The image is larger than the 64 MiB download limit.".to_string();
    let total = resp.content_length().map(|n| n as usize);
    if total.is_some_and(|n| n > MAX_DOWNLOAD_BYTES) {
        return Err(too_big());
    }
    let mut bytes = Vec::with_capacity(total.unwrap_or(0));
    progress(0, total);
    while let Some(chunk) = resp.chunk().await.map_err(err)? {
        bytes.extend_from_slice(&chunk);
        if bytes.len() > MAX_DOWNLOAD_BYTES {
            return Err(too_big());
        }
        progress(bytes.len(), total);
    }

    blocking(move || {
        std::fs::create_dir_all(&dir)?;
        // Write beside the destination, then rename, so a half-finished
        // download is never mistaken for a complete file.
        let tmp = dest.with_extension("part");
        std::fs::write(&tmp, &bytes)?;
        std::fs::rename(&tmp, &dest)?;
        Ok((dest, true))
    })
    .await
}
