//! The system's file dialogs, through the XDG desktop portal's FileChooser
//! (KDE's, GNOME's... whichever the desktop has): save a file, open one,
//! pick a folder. Each answers the path, or None when cancelled.

use std::path::{Path, PathBuf};

use ashpd::desktop::ResponseError;
use ashpd::desktop::file_chooser::{FileFilter, SelectedFiles};

use crate::{CmdResult, err};

/// Files matching `glob` ("*.json"), under `name`.
pub struct Filter<'a> {
    pub name: &'a str,
    pub glob: &'a str,
}

pub async fn save_file(title: &str, folder: Option<&Path>, name: Option<&str>, filter: Option<Filter<'_>>) -> CmdResult<Option<PathBuf>> {
    let mut request = SelectedFiles::save_file().title(title).modal(true);
    if let Some(name) = name {
        request = request.current_name(name);
    }
    if let Some(dir) = folder {
        request = request.current_folder(dir).map_err(err)?;
    }
    if let Some(f) = filter {
        request = request.filter(FileFilter::new(f.name).glob(f.glob));
    }
    first(request.send().await.map_err(err)?.response())
}

pub async fn open_file(title: &str, folder: Option<&Path>, filter: Option<Filter<'_>>) -> CmdResult<Option<PathBuf>> {
    let mut request = SelectedFiles::open_file().title(title).modal(true).multiple(false);
    if let Some(dir) = folder {
        request = request.current_folder(dir).map_err(err)?;
    }
    if let Some(f) = filter {
        request = request.filter(FileFilter::new(f.name).glob(f.glob));
    }
    first(request.send().await.map_err(err)?.response())
}

pub async fn folder(title: &str, folder: Option<&Path>) -> CmdResult<Option<PathBuf>> {
    let mut request = SelectedFiles::open_file().title(title).modal(true).directory(true);
    if let Some(dir) = folder {
        request = request.current_folder(dir).map_err(err)?;
    }
    first(request.send().await.map_err(err)?.response())
}

/// The picked path; None when the dialog was cancelled.
fn first(response: ashpd::Result<SelectedFiles>) -> CmdResult<Option<PathBuf>> {
    match response {
        Ok(files) => Ok(files.uris().first().and_then(|u| file_path(u.as_str()))),
        Err(ashpd::Error::Response(ResponseError::Cancelled)) => Ok(None),
        Err(e) => Err(format!("The file dialog failed: {e}")),
    }
}

/// `file:///home/me/a%20b.json` -> `/home/me/a b.json` (anything else: None).
fn file_path(uri: &str) -> Option<PathBuf> {
    use std::os::unix::ffi::OsStringExt;
    let rest = uri.strip_prefix("file://")?;
    // An authority before the path (file://localhost/...) is dropped.
    let path = &rest[rest.find('/')?..];
    let bytes = path.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let hex = |b: u8| (b as char).to_digit(16);
        match (bytes[i], bytes.get(i + 1).and_then(|b| hex(*b)), bytes.get(i + 2).and_then(|b| hex(*b))) {
            (b'%', Some(h), Some(l)) => {
                out.push((h * 16 + l) as u8);
                i += 3;
            }
            (b, _, _) => {
                out.push(b);
                i += 1;
            }
        }
    }
    Some(PathBuf::from(std::ffi::OsString::from_vec(out)))
}

#[cfg(test)]
mod tests {
    use super::file_path;
    use std::path::PathBuf;

    #[test]
    fn file_uris_become_paths() {
        assert_eq!(file_path("file:///home/me/a%20b%C3%A9.json"), Some(PathBuf::from("/home/me/a bé.json")));
        assert_eq!(file_path("file://localhost/tmp/x"), Some(PathBuf::from("/tmp/x")));
        assert_eq!(file_path("https://example.com/x"), None);
    }
}
