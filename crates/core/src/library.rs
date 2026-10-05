use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// File extensions hyprpaper can display and the thumbnailer can decode.
pub const IMAGE_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "webp"];

pub fn is_supported_image(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| IMAGE_EXTENSIONS.iter().any(|x| x.eq_ignore_ascii_case(e)))
        .unwrap_or(false)
}

/// A wallpaper file found in one of the configured folders.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalImage {
    pub path: PathBuf,
    pub name: String,
    pub size: u64,
    /// Modification time as seconds since the Unix epoch (0 if unknown).
    pub modified: u64,
    /// Pixel size, read from the file's header (0 if it can't be read).
    #[serde(default)]
    pub width: u32,
    #[serde(default)]
    pub height: u32,
}

impl LocalImage {
    /// Smaller than `width`×`height` on either side (unknown sizes aren't).
    pub fn smaller_than(&self, width: u32, height: u32) -> bool {
        self.width > 0 && self.height > 0 && (self.width < width || self.height < height)
    }
}

#[cfg(feature = "native")]
const MAX_DEPTH: usize = 8;

/// Scan `folders` for images, newest first.
///
/// Hidden entries are skipped and symlinks are not followed, so a link back to
/// a parent folder can't cause a loop. Missing folders are ignored.
#[cfg(feature = "native")]
pub fn scan(folders: &[PathBuf], recursive: bool) -> Vec<LocalImage> {
    let mut out = Vec::new();
    for folder in folders {
        walk(folder, recursive, 0, &mut out);
    }
    out.sort_by(|a, b| {
        b.modified
            .cmp(&a.modified)
            .then_with(|| a.name.cmp(&b.name))
    });
    out.dedup_by(|a, b| a.path == b.path);
    out
}

#[cfg(feature = "native")]
fn walk(dir: &Path, recursive: bool, depth: usize, out: &mut Vec<LocalImage>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') {
            continue;
        }
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        let path = entry.path();
        if file_type.is_dir() {
            if recursive && depth < MAX_DEPTH {
                walk(&path, recursive, depth + 1, out);
            }
        } else if file_type.is_file() && is_supported_image(&path) {
            let Ok(meta) = entry.metadata() else { continue };
            let modified = meta
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0);
            // Only the header is read, not the whole image.
            let (width, height) = image::image_dimensions(&path).unwrap_or((0, 0));
            out.push(LocalImage {
                path,
                name,
                size: meta.len(),
                modified,
                width,
                height,
            });
        }
    }
}

/// The file `path` refers to, if it may be moved to the trash: an existing
/// supported image (not a symlink) inside one of `roots` (the wallpaper
/// folders and the download folder). Paths are resolved first, so `..` or a
/// symlinked folder can't point outside them. Anything else is refused, so a
/// confused or tampered UI can't remove other files.
#[cfg(feature = "native")]
pub fn trash_target(path: &Path, roots: &[PathBuf]) -> crate::Result<PathBuf> {
    use crate::Error;
    let refuse = |why: &str| Err(Error::Invalid(format!("{}: {why}", path.display())));
    if !path.is_absolute() {
        return refuse("not an absolute path");
    }
    if !is_supported_image(path) {
        return refuse("not a wallpaper image");
    }
    // The link itself, not what it points to: trashing a symlink would only
    // remove the link, which isn't what "delete this wallpaper" means.
    let meta = std::fs::symlink_metadata(path)?;
    if !meta.is_file() {
        return refuse("not a regular file");
    }
    let real = path.canonicalize()?;
    let inside = roots
        .iter()
        .filter_map(|r| r.canonicalize().ok())
        .any(|root| real.starts_with(&root) && real != root);
    if !inside {
        return refuse("not inside your wallpaper or download folders");
    }
    Ok(real)
}

/// The image `path` refers to, resolved, if it's one of the library's: an
/// existing supported image inside one of `roots`. For actions that only
/// point at it (show in folder); removing it has its own, stricter check
/// ([`trash_target`]).
#[cfg(feature = "native")]
pub fn library_file(path: &Path, roots: &[PathBuf]) -> crate::Result<PathBuf> {
    use crate::Error;
    let refuse = |why: &str| Err(Error::Invalid(format!("{}: {why}", path.display())));
    if !path.is_absolute() {
        return refuse("not an absolute path");
    }
    if !is_supported_image(path) {
        return refuse("not a wallpaper image");
    }
    let real = path.canonicalize()?;
    let inside = roots
        .iter()
        .filter_map(|r| r.canonicalize().ok())
        .any(|root| real.starts_with(&root) && real != root);
    if !inside || !real.is_file() {
        return refuse("not inside your wallpaper or download folders");
    }
    Ok(real)
}

/// `file://` URI for an absolute path: everything but unreserved characters
/// and `/` is percent-encoded, so spaces and `#` survive.
pub fn file_uri(path: &Path) -> String {
    let mut out = String::from("file://");
    for &b in path.to_string_lossy().as_bytes() {
        if b.is_ascii_alphanumeric() || b"-._~/".contains(&b) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// Which of `roots` holds `path`, and the folder inside it: "" at the top,
/// "a/b" further down, always with `/`. The longest matching root wins, so a
/// root inside another one (say, a download folder inside the wallpaper
/// folder) counts as its own. No canonicalizing: the paths come from a scan
/// of these same roots.
pub fn folder_in(path: &Path, roots: &[PathBuf]) -> Option<(usize, String)> {
    let parent = path.parent()?;
    let (index, root) = roots
        .iter()
        .enumerate()
        .filter(|(_, r)| parent.starts_with(r))
        .max_by_key(|(_, r)| r.components().count())?;
    let inside = parent.strip_prefix(root).ok()?;
    let folder = inside
        .components()
        .map(|c| c.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/");
    Some((index, folder))
}

/// A folder of the library: one of the wallpaper folders (`root`, an index
/// into them) and a folder inside it ("" for its top, "a/b" further down).
/// `None` in the functions below stands for the list of wallpaper folders,
/// which is only a place when there are several.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Place {
    pub root: usize,
    pub folder: String,
}

impl Place {
    /// Is `other` this place, or somewhere below it?
    pub fn contains(&self, other: &Place) -> bool {
        self.root == other.root
            && (self.folder.is_empty()
                || other.folder == self.folder
                || other.folder.starts_with(&format!("{}/", self.folder)))
    }
}

/// Where browsing starts: the only wallpaper folder's top, or the list of
/// them when there are several.
pub fn top_place(roots: usize) -> Option<Place> {
    (roots <= 1).then(|| Place { root: 0, folder: String::new() })
}

/// Is an image at `place` shown when browsing `at` (at it or below)?
pub fn place_inside(place: &Place, at: Option<&Place>) -> bool {
    at.is_none_or(|at| at.contains(place))
}

/// A folder shown while browsing: its name, where it leads, how many images
/// are in it or below, and the first of them (by index into `places`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Subfolder {
    pub name: String,
    pub place: Place,
    pub count: usize,
    pub first: usize,
}

/// What browsing `at` shows, given where each image is (`places`, in display
/// order): its subfolders (by name; the wallpaper folders, in order, for the
/// list) and the indexes of the images right in it.
pub fn browse(places: &[Place], at: Option<&Place>, root_names: &[String]) -> (Vec<Subfolder>, Vec<usize>) {
    let mut folders: Vec<Subfolder> = Vec::new();
    let mut direct = Vec::new();
    for (i, p) in places.iter().enumerate() {
        let (name, place) = match at {
            None => (
                root_names.get(p.root).cloned().unwrap_or_else(|| "?".into()),
                Place { root: p.root, folder: String::new() },
            ),
            Some(at) if !at.contains(p) => continue,
            Some(at) if p.folder == at.folder => {
                direct.push(i);
                continue;
            }
            Some(at) => {
                let rest = if at.folder.is_empty() { &p.folder[..] } else { &p.folder[at.folder.len() + 1..] };
                let name = rest.split('/').next().unwrap_or(rest).to_string();
                let folder = if at.folder.is_empty() { name.clone() } else { format!("{}/{name}", at.folder) };
                (name, Place { root: p.root, folder })
            }
        };
        match folders.iter_mut().find(|f| f.place == place) {
            Some(f) => f.count += 1,
            None => folders.push(Subfolder { name, place, count: 1, first: i }),
        }
    }
    if at.is_none() {
        folders.sort_by_key(|f| f.place.root);
    } else {
        folders.sort_by_key(|f| f.name.to_lowercase());
    }
    (folders, direct)
}

/// One level up from `at`: `Some(None)` is the list of wallpaper folders,
/// `None` means there's nothing above.
pub fn parent_place(at: Option<&Place>, roots: usize) -> Option<Option<Place>> {
    let at = at?;
    if at.folder.is_empty() {
        return (roots > 1).then_some(None);
    }
    let up = at.folder.rsplit_once('/').map_or("", |(up, _)| up);
    Some(Some(Place { root: at.root, folder: up.to_string() }))
}

/// The way to `at`, for a breadcrumb: (label, place) from the top.
pub fn trail(at: Option<&Place>, root_names: &[String], all_label: &str) -> Vec<(String, Option<Place>)> {
    let mut steps = Vec::new();
    if root_names.len() > 1 {
        steps.push((all_label.to_string(), None));
    }
    let Some(at) = at else { return steps };
    let root = root_names.get(at.root).cloned().unwrap_or_else(|| "?".into());
    steps.push((root, Some(Place { root: at.root, folder: String::new() })));
    let mut path = String::new();
    for part in at.folder.split('/').filter(|p| !p.is_empty()) {
        path = if path.is_empty() { part.to_string() } else { format!("{path}/{part}") };
        steps.push((part.to_string(), Some(Place { root: at.root, folder: path.clone() })));
    }
    steps
}

#[cfg(test)]
mod place_tests {
    use super::*;

    fn at(root: usize, folder: &str) -> Place {
        Place { root, folder: folder.into() }
    }

    #[test]
    fn browsing_one_root() {
        let places = [at(0, ""), at(0, "anime"), at(0, "anime/mitsu"), at(0, "zz"), at(0, "anime")];
        let names = vec!["w".to_string()];
        let top = top_place(1);
        let (folders, direct) = browse(&places, top.as_ref(), &names);
        let summary: Vec<_> = folders.iter().map(|f| (f.name.as_str(), f.count, f.first)).collect();
        assert_eq!(summary, [("anime", 3, 1), ("zz", 1, 3)]);
        assert_eq!(direct, [0]);
        let (folders, direct) = browse(&places, Some(&at(0, "anime")), &names);
        assert_eq!(folders[0].place, at(0, "anime/mitsu"));
        assert_eq!(direct, [1, 4]);
        // "ani" isn't a parent of "anime".
        assert!(!at(0, "ani").contains(&at(0, "anime")));
        assert!(place_inside(&at(0, "anime/mitsu"), Some(&at(0, "anime"))));
    }

    #[test]
    fn browsing_several_roots() {
        let places = [at(1, ""), at(0, "x"), at(0, "")];
        let names = vec!["w".to_string(), "dl".to_string()];
        assert_eq!(top_place(2), None);
        let (folders, direct) = browse(&places, None, &names);
        let summary: Vec<_> = folders.iter().map(|f| (f.name.as_str(), f.count)).collect();
        assert_eq!(summary, [("w", 2), ("dl", 1)]);
        assert!(direct.is_empty());
    }

    #[test]
    fn going_up_and_the_trail() {
        assert_eq!(parent_place(Some(&at(0, "a/b")), 1), Some(Some(at(0, "a"))));
        assert_eq!(parent_place(Some(&at(0, "a")), 1), Some(Some(at(0, ""))));
        assert_eq!(parent_place(Some(&at(0, "")), 1), None);
        assert_eq!(parent_place(Some(&at(0, "")), 2), Some(None));
        assert_eq!(parent_place(None, 2), None);
        let names = vec!["w".to_string(), "dl".to_string()];
        let t = trail(Some(&at(0, "a/b")), &names, "All");
        let labels: Vec<_> = t.iter().map(|(l, _)| l.as_str()).collect();
        assert_eq!(labels, ["All", "w", "a", "b"]);
        assert_eq!(t[2].1, Some(at(0, "a")));
    }
}

#[cfg(test)]
mod folder_tests {
    use super::*;

    #[test]
    fn smaller_than_needs_a_known_size() {
        let img = |width, height| LocalImage { path: "/a.jpg".into(), name: "a.jpg".into(), size: 1, modified: 0, width, height };
        assert!(img(1920, 1080).smaller_than(2560, 1440));
        assert!(!img(2560, 1440).smaller_than(2560, 1440));
        // Wide but short: smaller on one side counts.
        assert!(img(3440, 1080).smaller_than(2560, 1440));
        assert!(!img(0, 0).smaller_than(1920, 1080));
    }

    #[test]
    fn file_uris_are_percent_encoded() {
        assert_eq!(file_uri(Path::new("/w/a b/#1 ç.jpg")), "file:///w/a%20b/%231%20%C3%A7.jpg");
        assert_eq!(file_uri(Path::new("/w/plain-name_1.jpg")), "file:///w/plain-name_1.jpg");
    }

    #[test]
    fn folder_is_relative_to_the_deepest_root() {
        let roots = vec![PathBuf::from("/w"), PathBuf::from("/w/dl"), PathBuf::from("/other")];
        assert_eq!(folder_in(Path::new("/w/a.jpg"), &roots), Some((0, String::new())));
        assert_eq!(folder_in(Path::new("/w/anime/mitsu/a.jpg"), &roots), Some((0, "anime/mitsu".into())));
        // Inside the nested root, it's that root's top.
        assert_eq!(folder_in(Path::new("/w/dl/a.jpg"), &roots), Some((1, String::new())));
        assert_eq!(folder_in(Path::new("/other/x/a.png"), &roots), Some((2, "x".into())));
        // "/wx" isn't inside "/w": whole components only.
        assert_eq!(folder_in(Path::new("/wx/a.jpg"), &roots), None);
        assert_eq!(folder_in(Path::new("/elsewhere/a.jpg"), &roots), None);
    }
}

#[cfg(all(test, feature = "native"))]
mod tests {
    use super::*;

    #[test]
    fn library_file_must_be_an_image_inside_the_folders() {
        let base = std::env::temp_dir().join(format!("wallpicker-libfile-{}", std::process::id()));
        let (root, outside) = (base.join("walls"), base.join("other"));
        std::fs::create_dir_all(root.join("sub")).unwrap();
        std::fs::create_dir_all(&outside).unwrap();
        for f in [root.join("sub/a.jpg"), outside.join("b.jpg"), root.join("notes.txt")] {
            std::fs::write(f, b"x").unwrap();
        }
        let roots = vec![root.clone()];
        assert!(library_file(&root.join("sub/a.jpg"), &roots).is_ok());
        assert!(library_file(&outside.join("b.jpg"), &roots).is_err());
        assert!(library_file(&root.join("sub/../../other/b.jpg"), &roots).is_err());
        assert!(library_file(&root.join("notes.txt"), &roots).is_err());
        assert!(library_file(&root.join("missing.jpg"), &roots).is_err());
        assert!(library_file(Path::new("sub/a.jpg"), &roots).is_err());
        std::fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn only_images_inside_the_folders_can_be_trashed() {
        let tmp = tempfile::tempdir().unwrap();
        let walls = tmp.path().join("walls");
        let other = tmp.path().join("other");
        std::fs::create_dir_all(walls.join("sub")).unwrap();
        std::fs::create_dir_all(&other).unwrap();
        let roots = vec![walls.clone()];
        let ok = walls.join("sub/a.png");
        std::fs::write(&ok, b"x").unwrap();
        assert_eq!(trash_target(&ok, &roots).unwrap(), ok.canonicalize().unwrap());

        let outside = other.join("b.png");
        std::fs::write(&outside, b"x").unwrap();
        assert!(trash_target(&outside, &roots).is_err());
        // `..` that leaves the folder.
        assert!(trash_target(&walls.join("../other/b.png"), &roots).is_err());
        // Not an image.
        let txt = walls.join("notes.txt");
        std::fs::write(&txt, b"x").unwrap();
        assert!(trash_target(&txt, &roots).is_err());
        // Relative, missing.
        assert!(trash_target(Path::new("a.png"), &roots).is_err());
        assert!(trash_target(&walls.join("gone.png"), &roots).is_err());
        // A symlink in the folder pointing outside it.
        #[cfg(unix)]
        {
            let link = walls.join("link.png");
            std::os::unix::fs::symlink(&outside, &link).unwrap();
            assert!(trash_target(&link, &roots).is_err());
            assert!(outside.exists());
        }
    }
    use std::fs;

    #[test]
    fn extension_check_is_case_insensitive() {
        assert!(is_supported_image(Path::new("a.PNG")));
        assert!(is_supported_image(Path::new("/x/y.JpEg")));
        assert!(!is_supported_image(Path::new("a.gif")));
        assert!(!is_supported_image(Path::new("noext")));
    }

    #[test]
    fn scans_recursively_skipping_hidden_and_non_images() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        fs::write(root.join("a.png"), b"x").unwrap();
        fs::write(root.join("notes.txt"), b"x").unwrap();
        fs::write(root.join(".hidden.png"), b"x").unwrap();
        fs::create_dir(root.join("sub")).unwrap();
        fs::write(root.join("sub").join("b.jpg"), b"x").unwrap();
        fs::create_dir(root.join(".cache")).unwrap();
        fs::write(root.join(".cache").join("c.png"), b"x").unwrap();

        let recursive = scan(&[root.to_path_buf()], true);
        let mut names: Vec<_> = recursive.iter().map(|i| i.name.as_str()).collect();
        names.sort();
        assert_eq!(names, ["a.png", "b.jpg"]);

        let flat = scan(&[root.to_path_buf()], false);
        assert_eq!(flat.len(), 1);
        assert_eq!(flat[0].name, "a.png");
    }

    #[test]
    fn missing_folder_and_duplicates_are_fine() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(tmp.path().join("a.png"), b"x").unwrap();
        let found = scan(
            &[
                tmp.path().to_path_buf(),
                tmp.path().to_path_buf(),
                PathBuf::from("/definitely/not/here"),
            ],
            true,
        );
        assert_eq!(found.len(), 1);
    }

    #[cfg(unix)]
    #[test]
    fn symlinked_directories_are_not_followed() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(tmp.path().join("a.png"), b"x").unwrap();
        std::os::unix::fs::symlink(tmp.path(), tmp.path().join("loop")).unwrap();
        let found = scan(&[tmp.path().to_path_buf()], true);
        assert_eq!(found.len(), 1);
    }
}
