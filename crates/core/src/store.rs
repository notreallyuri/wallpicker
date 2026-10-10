use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::catalog::Catalog;
use crate::error::Result;
use crate::online::OnlineFilters;
use crate::settings::{Settings, State};
use crate::wallhaven::SearchQuery;

/// Where wallpicker keeps its files (XDG base directories).
#[derive(Debug, Clone)]
pub struct Dirs {
    /// `config.json`
    pub config: PathBuf,
    /// `state.json`
    pub state: PathBuf,
    /// `library.json` (favorites and tags): user data, unlike the state.
    pub data: PathBuf,
    /// Thumbnails
    pub cache: PathBuf,
}

impl Dirs {
    pub fn from_env() -> Dirs {
        Dirs::from_lookup(|k| std::env::var_os(k).map(PathBuf::from))
    }

    /// `lookup` returns an environment variable. Per the XDG spec, relative
    /// values are ignored.
    pub fn from_lookup(lookup: impl Fn(&str) -> Option<PathBuf>) -> Dirs {
        let home = lookup("HOME").unwrap_or_else(|| PathBuf::from("."));
        let pick = |var: &str, fallback: &[&str]| -> PathBuf {
            let base = lookup(var)
                .filter(|p| p.is_absolute())
                .unwrap_or_else(|| fallback.iter().fold(home.clone(), |p, s| p.join(s)));
            base.join("wallpicker")
        };
        Dirs {
            config: pick("XDG_CONFIG_HOME", &[".config"]),
            state: pick("XDG_STATE_HOME", &[".local", "state"]),
            data: pick("XDG_DATA_HOME", &[".local", "share"]),
            cache: pick("XDG_CACHE_HOME", &[".cache"]),
        }
    }

    /// Everything under one root. Used by tests.
    pub fn under(root: &Path) -> Dirs {
        Dirs {
            config: root.join("config"),
            state: root.join("state"),
            data: root.join("data"),
            cache: root.join("cache"),
        }
    }
}

/// Loads and saves `Settings`, `State` and the `Catalog`.
#[derive(Debug, Clone)]
pub struct Store {
    dirs: Dirs,
}

impl Store {
    pub fn new(dirs: Dirs) -> Store {
        Store { dirs }
    }

    pub fn dirs(&self) -> &Dirs {
        &self.dirs
    }

    fn settings_path(&self) -> PathBuf {
        self.dirs.config.join("config.json")
    }

    fn state_path(&self) -> PathBuf {
        self.dirs.state.join("state.json")
    }

    fn online_filters_path(&self) -> PathBuf {
        self.dirs.state.join("online.json")
    }

    /// Where the Wallhaven filters were kept before there were providers.
    fn old_wallhaven_filters_path(&self) -> PathBuf {
        self.dirs.state.join("wallhaven.json")
    }

    fn catalog_path(&self) -> PathBuf {
        self.dirs.data.join("library.json")
    }

    /// A missing file means defaults. A file that exists but can't be parsed is
    /// an error, so a typo never silently resets the user's config.
    pub fn load_settings(&self) -> Result<Settings> {
        match fs::read_to_string(self.settings_path()) {
            Ok(text) => Ok(serde_json::from_str::<Settings>(&text)?.sanitized()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Settings::default()),
            Err(e) => Err(e.into()),
        }
    }

    pub fn save_settings(&self, settings: &Settings) -> Result<()> {
        let text = serde_json::to_string_pretty(&settings.clone().sanitized())?;
        // The file may hold an API key, so keep it private to the user.
        atomic_write(&self.settings_path(), text.as_bytes(), true)?;
        Ok(())
    }

    pub fn load_state(&self) -> Result<State> {
        match fs::read_to_string(self.state_path()) {
            Ok(text) => Ok(serde_json::from_str(&text)?),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(State::default()),
            Err(e) => Err(e.into()),
        }
    }

    pub fn save_state(&self, state: &State) -> Result<()> {
        let text = serde_json::to_string_pretty(state)?;
        atomic_write(&self.state_path(), text.as_bytes(), false)?;
        Ok(())
    }

    /// Forget `path` as any monitor's current wallpaper (it was deleted), so
    /// `--restore` doesn't go looking for it.
    pub fn forget_current(&self, path: &Path) -> Result<()> {
        let mut state = self.load_state()?;
        let before = state.clone();
        state.current.retain(|_, p| p != path);
        let shown = state.current.clone();
        state.adjust.retain(|m, _| shown.contains_key(m));
        // Nothing to go back to either.
        for h in state.history.values_mut() {
            h.retain(|p| p != path);
        }
        if state != before {
            self.save_state(&state)?;
        }
        Ok(())
    }

    /// Remember that `monitor` now shows `path`; what it showed before goes
    /// into its history.
    pub fn set_current(&self, monitor: &str, path: &Path) -> Result<()> {
        let mut state = self.load_state()?;
        remember_current(&mut state, monitor, path);
        self.save_state(&state)?;
        crate::transform::prune(&self.dirs.cache, &state);
        Ok(())
    }

    /// `monitor` shows `path` because its workspace changed: what it shows
    /// now, but not something chosen, so it doesn't go into the history
    /// (Previous stays "the wallpaper I picked before").
    pub fn show_for_workspace(&self, monitor: &str, path: &Path) -> Result<()> {
        let mut state = self.load_state()?;
        if state.current.get(monitor).is_some_and(|p| p != path) {
            state.adjust.remove(monitor);
            end_span_on(&mut state, monitor);
        }
        state.current.insert(monitor.to_string(), path.to_path_buf());
        self.save_state(&state)?;
        crate::transform::prune(&self.dirs.cache, &state);
        Ok(())
    }

    /// Adjust what `monitor` shows (None-like: back to as it is). Applying it
    /// is the caller's (`wallpapers::reapply`).
    pub fn set_adjust(&self, monitor: &str, mut adjust: crate::settings::Adjust) -> Result<State> {
        adjust.frame = adjust.frame.and_then(crate::settings::Frame::sanitized);
        let mut state = self.load_state()?;
        if !state.current.contains_key(monitor) {
            return Err(crate::error::Error::Invalid(format!(
                "{monitor} shows nothing wallpicker set"
            )));
        }
        // Out of the span: adjusted on its own.
        if adjust.span.is_none() {
            end_span_on(&mut state, monitor);
        }
        if adjust.is_none() {
            state.adjust.remove(monitor);
        } else {
            state.adjust.insert(monitor.to_string(), adjust);
        }
        self.save_state(&state)?;
        Ok(state)
    }

    /// `path` spanned with `look`: each monitor in `cuts` its part, mirrored
    /// like the span; `places`, where they are on the canvas. They must show
    /// `path` already ([`Store::set_current`]).
    pub fn set_span(
        &self,
        path: &Path,
        look: crate::settings::SpanLook,
        cuts: &[(String, crate::settings::Cut)],
        places: std::collections::BTreeMap<String, crate::settings::SpanRect>,
    ) -> Result<State> {
        let mut state = self.load_state()?;
        for (m, cut) in cuts {
            if state.current.get(m).is_some_and(|p| p == path) {
                state.adjust.insert(
                    m.clone(),
                    crate::settings::Adjust {
                        span: Some(*cut),
                        flip: look.flip,
                        ..Default::default()
                    },
                );
            }
        }
        let monitors = cuts.iter().map(|(m, _)| m.clone()).collect();
        state.span = Some(crate::settings::Spanned {
            path: path.to_path_buf(),
            look,
            monitors,
            places,
        });
        self.save_state(&state)?;
        crate::transform::prune(&self.dirs.cache, &state);
        Ok(state)
    }

    /// The monitors act on their own again, each keeping its part.
    pub fn separate(&self) -> Result<State> {
        let mut state = self.load_state()?;
        state.span = None;
        self.save_state(&state)?;
        Ok(state)
    }

    /// What `monitor` showed before, skipping files that are gone.
    pub fn previous(&self, monitor: &str) -> Result<Option<PathBuf>> {
        Ok(self
            .load_state()?
            .history
            .get(monitor)
            .and_then(|h| h.iter().rev().find(|p| p.is_file()).cloned()))
    }

    /// `monitor` went back to `path` (from [`Store::previous`]): it leaves the
    /// history (with any newer, gone files), and nothing new is recorded, so
    /// going back again goes further back.
    pub fn went_back(&self, monitor: &str, path: &Path) -> Result<()> {
        let mut state = self.load_state()?;
        if let Some(h) = state.history.get_mut(monitor)
            && let Some(i) = h.iter().rposition(|p| p == path)
        {
            h.truncate(i);
        }
        if state.current.get(monitor).is_some_and(|p| p != path) {
            state.adjust.remove(monitor);
            end_span_on(&mut state, monitor);
        }
        state.current.insert(monitor.to_string(), path.to_path_buf());
        self.save_state(&state)?;
        crate::transform::prune(&self.dirs.cache, &state);
        Ok(())
    }
}

impl Store {
    /// The online filters from last time, or the defaults. Without
    /// `online.json`, the older `wallhaven.json` (Wallhaven filters only) is
    /// carried over.
    pub fn load_online_filters(&self) -> Result<OnlineFilters> {
        match fs::read_to_string(self.online_filters_path()) {
            Ok(text) => return Ok(serde_json::from_str::<OnlineFilters>(&text)?.filters_only()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.into()),
        }
        match fs::read_to_string(self.old_wallhaven_filters_path()) {
            Ok(text) => Ok(OnlineFilters {
                wallhaven: serde_json::from_str::<SearchQuery>(&text)?,
                ..OnlineFilters::default()
            }
            .filters_only()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(OnlineFilters::default()),
            Err(e) => Err(e.into()),
        }
    }

    pub fn save_online_filters(&self, filters: &OnlineFilters) -> Result<()> {
        let text = serde_json::to_string_pretty(&filters.clone().filters_only())?;
        atomic_write(&self.online_filters_path(), text.as_bytes(), false)
    }

    /// Like the settings, a file that can't be parsed is an error rather than
    /// an empty catalog, so a bad edit never wipes the user's tags.
    pub fn load_catalog(&self) -> Result<Catalog> {
        match fs::read_to_string(self.catalog_path()) {
            Ok(text) => Ok(serde_json::from_str(&text)?),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Catalog::default()),
            Err(e) => Err(e.into()),
        }
    }

    /// Save a `library.json` from an older version in today's format (a
    /// snapshot of it taken first). Where a migration of its contents would
    /// go; version 2 only adds fields. Callers that can run concurrently
    /// must serialize calls themselves, as for [`Store::update_catalog`].
    pub fn upgrade_catalog(&self) -> Result<()> {
        if !self.catalog_path().is_file() || self.load_catalog()?.version >= crate::catalog::VERSION {
            return Ok(());
        }
        self.update_catalog(|_| {}).map(|_| ())
    }

    /// Load, change and save the catalog, returning the saved version. Callers
    /// that can run concurrently must serialize calls themselves.
    pub fn update_catalog(&self, change: impl FnOnce(&mut Catalog)) -> Result<Catalog> {
        let mut catalog = self.load_catalog()?;
        // Best effort: an edit isn't refused over its safety copy.
        let _ = self.snapshot_catalog();
        change(&mut catalog);
        catalog.version = crate::catalog::VERSION;
        let text = serde_json::to_string_pretty(&catalog)?;
        atomic_write(&self.catalog_path(), text.as_bytes(), false)?;
        Ok(catalog)
    }
}

/// One of the library's automatic snapshots.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Snapshot {
    /// "library-2026-10-06" (its file name without .json): what to restore.
    pub id: String,
    /// "2026-10-06", or "2026-10-06 14:03:22" for one made before a restore.
    pub label: String,
    /// How many wallpapers it has favorites, tags or fits for.
    pub items: usize,
    pub presets: Option<usize>,
}

/// How many daily snapshots are kept.
pub const SNAPSHOTS_KEPT: usize = 14;

impl Store {
    /// Where the snapshots go when no folder is set.
    pub fn default_snapshots_dir(&self) -> PathBuf {
        self.dirs.data.join("backups")
    }

    /// The library's automatic snapshots: the folder in the settings, else
    /// [`Store::default_snapshots_dir`].
    pub fn snapshots_dir(&self) -> PathBuf {
        self.load_settings()
            .ok()
            .and_then(|s| s.snapshot_dir)
            .unwrap_or_else(|| self.default_snapshots_dir())
    }

    /// Keep the snapshots in `dir` from now on (None: the default folder),
    /// copying the ones there are, so none are lost. Returns the folder.
    pub fn set_snapshots_dir(&self, dir: Option<PathBuf>) -> Result<PathBuf> {
        let old = self.snapshots_dir();
        let mut settings = self.load_settings()?;
        settings.snapshot_dir = dir.filter(|d| *d != self.default_snapshots_dir());
        let new = settings
            .snapshot_dir
            .clone()
            .unwrap_or_else(|| self.default_snapshots_dir());
        if new != old {
            fs::create_dir_all(&new)?;
            for id in self.snapshot_ids()? {
                let name = format!("{id}.json");
                let target = new.join(&name);
                if !target.exists() {
                    fs::copy(old.join(&name), target)?;
                }
            }
        }
        self.save_settings(&settings)?;
        Ok(new)
    }

    /// Before the first change of a day: a copy of the library as it was
    /// (`backups/library-YYYY-MM-DD.json`), keeping the newest
    /// [`SNAPSHOTS_KEPT`].
    pub(crate) fn snapshot_catalog(&self) -> Result<()> {
        let src = self.catalog_path();
        if !src.is_file() && !self.presets_path().is_file() {
            return Ok(());
        }
        let (date, _) = crate::backup::date_and_time(now_secs());
        let dir = self.snapshots_dir();
        let target = dir.join(format!("library-{date}.json"));
        if target.exists() {
            return Ok(());
        }
        fs::create_dir_all(&dir)?;
        let backup = self.export_backup(crate::backup::date_and_time(now_secs()).1)?;
        atomic_write(&target, &serde_json::to_vec_pretty(&backup)?, false)?;
        self.prune_snapshots()
    }

    fn prune_snapshots(&self) -> Result<()> {
        let mut names = self.snapshot_ids()?;
        // Newest first (the names sort by date).
        names.sort_unstable_by(|a, b| b.cmp(a));
        for old in names.iter().skip(SNAPSHOTS_KEPT) {
            let _ = fs::remove_file(self.snapshots_dir().join(format!("{old}.json")));
        }
        Ok(())
    }

    fn snapshot_ids(&self) -> Result<Vec<String>> {
        let dir = match fs::read_dir(self.snapshots_dir()) {
            Ok(d) => d,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => return Err(e.into()),
        };
        Ok(dir
            .filter_map(|e| e.ok())
            .filter_map(|e| e.file_name().to_str().map(str::to_string))
            .filter_map(|n| n.strip_suffix(".json").map(str::to_string))
            .filter(|n| is_snapshot_id(n))
            .collect())
    }

    /// The snapshots, newest first.
    pub fn snapshots(&self) -> Result<Vec<Snapshot>> {
        let mut ids = self.snapshot_ids()?;
        ids.sort_unstable_by(|a, b| b.cmp(a));
        Ok(ids
            .into_iter()
            .map(|id| {
                let contents = fs::read_to_string(self.snapshots_dir().join(format!("{id}.json")))
                    .ok()
                    .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok());
                let (items, presets) = contents.map_or((0, None), |v| {
                    if v.get("wallpicker_backup").is_some() {
                        serde_json::from_value::<crate::backup::Backup>(v).map_or((0,None), |b| (b.items.len(), b.presets.map(|p| p.items.len())))
                    } else {
                        serde_json::from_value::<Catalog>(v).map_or((0,None), |c| (c.items.len(), None))
                    }
                });
                let rest = &id["library-".len()..];
                let label = match rest.split_at_checked(10) {
                    Some((date, time)) if time.len() == 7 => {
                        format!("{date} {}:{}:{}", &time[1..3], &time[3..5], &time[5..7])
                    }
                    _ => rest.to_string(),
                };
                Snapshot { id, label, items, presets }
            })
            .collect())
    }

    /// Put snapshot `id` back as the library. What's there now is kept first,
    /// as a snapshot of its own (so a restore can be undone too).
    pub fn restore_snapshot(&self, id: &str) -> Result<Catalog> {
        if !is_snapshot_id(id) {
            return Err(crate::error::Error::Invalid(format!("no such snapshot: {id}")));
        }
        let text = fs::read_to_string(self.snapshots_dir().join(format!("{id}.json")))?;
        let value: serde_json::Value = serde_json::from_str(&text)?;
        let (mut snapshot, mut presets) = if value.get("wallpicker_backup").is_some() {
            let backup: crate::backup::Backup = serde_json::from_value(value)?;
            backup.validate()?;
            let mut catalog = Catalog::default();
            for item in backup.items { catalog.items.insert(item.path, item.entry); }
            (catalog, backup.presets)
        } else { (serde_json::from_value::<Catalog>(value)?, None) };
        let _lock = self.backup_lock()?;
        if let Some(saved) = &mut presets { saved.next_id = saved.next_id.max(self.preset_collection()?.next_id); }

        snapshot.version = crate::catalog::VERSION;
        let src = self.catalog_path();
        if src.is_file() || self.presets_path().is_file() {
            let secs = now_secs();
            let (date, _) = crate::backup::date_and_time(secs);
            let stamp = format!("{:02}{:02}{:02}", (secs / 3600) % 24, (secs / 60) % 60, secs % 60);
            fs::create_dir_all(self.snapshots_dir())?;
            let backup = self.export_backup(crate::backup::date_and_time(secs).1)?;
            atomic_write(&self.snapshots_dir().join(format!("library-{date}-{stamp}.json")), &serde_json::to_vec_pretty(&backup)?, false)?;
        }
        self.write_backup_data(&snapshot, presets.as_ref())?;
        Ok(snapshot)
    }
}

/// "library-2026-10-06" or "library-2026-10-06-142233", nothing else (so an
/// id from outside can't name another file).
fn is_snapshot_id(id: &str) -> bool {
    let Some(rest) = id.strip_prefix("library-") else {
        return false;
    };
    let b = rest.as_bytes();
    let date_ok = b.len() >= 10
        && b[..10].iter().enumerate().all(|(i, c)| {
            if i == 4 || i == 7 {
                *c == b'-'
            } else {
                c.is_ascii_digit()
            }
        });
    date_ok && (b.len() == 10 || (b.len() == 17 && b[10] == b'-' && b[11..].iter().all(u8::is_ascii_digit)))
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// Shared success-only history update for ordinary applies and presets.
pub(crate) fn remember_current(state: &mut State, monitor: &str, path: &Path) {
    if let Some(old) = state.current.get(monitor).cloned()
        && old != path
    {
        let history = state.history.entry(monitor.into()).or_default();
        history.retain(|p| *p != old);
        history.push(old);
        if history.len() > crate::settings::HISTORY_LEN {
            history.remove(0);
        }
    }
    if state.current.get(monitor).is_some_and(|p| p != path) {
        state.adjust.remove(monitor);
        end_span_on(state, monitor);
    }
    state.current.insert(monitor.into(), path.to_path_buf());
}

/// Write to a temporary file in the same directory, then rename over the
/// target, so a crash can't leave a half-written file.
pub(crate) fn atomic_write(path: &Path, bytes: &[u8], private: bool) -> Result<()> {
    let dir = path.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(dir)?;

    let tmp = path.with_extension("json.tmp");
    let mut opts = fs::OpenOptions::new();
    opts.write(true).create(true).truncate(true);
    #[cfg(unix)]
    if private {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o600);
    }
    #[cfg(not(unix))]
    let _ = private;

    let mut file = opts.open(&tmp)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    drop(file);
    fs::rename(&tmp, path)?;
    Ok(())
}

/// `monitor` shows something else: the span it was part of ends (the
/// others keep their parts).
fn end_span_on(state: &mut State, monitor: &str) {
    if state.span.as_ref().is_some_and(|s| s.covers(monitor)) {
        state.span = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn xdg_defaults_and_overrides() {
        let env: HashMap<&str, &str> = HashMap::from([("HOME", "/home/u")]);
        let d = Dirs::from_lookup(|k| env.get(k).map(PathBuf::from));
        assert_eq!(d.config, PathBuf::from("/home/u/.config/wallpicker"));
        assert_eq!(d.state, PathBuf::from("/home/u/.local/state/wallpicker"));
        assert_eq!(d.cache, PathBuf::from("/home/u/.cache/wallpicker"));
        assert_eq!(d.data, PathBuf::from("/home/u/.local/share/wallpicker"));

        let env: HashMap<&str, &str> = HashMap::from([
            ("HOME", "/home/u"),
            ("XDG_CONFIG_HOME", "/cfg"),
            ("XDG_CACHE_HOME", "relative/ignored"),
        ]);
        let d = Dirs::from_lookup(|k| env.get(k).map(PathBuf::from));
        assert_eq!(d.config, PathBuf::from("/cfg/wallpicker"));
        assert_eq!(d.cache, PathBuf::from("/home/u/.cache/wallpicker"));
    }

    #[test]
    fn a_span_lasts_until_a_monitor_shows_something_else() {
        use crate::settings::{Cut, SpanLook};
        let tmp = tempfile::tempdir().unwrap();
        let store = Store::new(Dirs::under(tmp.path()));
        let (wide, other) = (Path::new("/w/wide.png"), Path::new("/w/other.png"));
        let cut = |x| Cut {
            x,
            y: 0,
            width: 1920,
            height: 1080,
            out_width: 1920,
            out_height: 1080,
        };
        let cuts = [("DP-1".to_string(), cut(0)), ("DP-2".to_string(), cut(1920))];
        for (m, _) in &cuts {
            store.set_current(m, wide).unwrap();
        }
        let look = SpanLook {
            zoom: 150,
            ..SpanLook::default()
        };
        let state = store.set_span(wide, look, &cuts, Default::default()).unwrap();
        assert_eq!(state.span.as_ref().unwrap().monitors, ["DP-1", "DP-2"]);
        // A monitor outside it changes freely.
        store.set_current("DP-3", other).unwrap();
        assert!(store.load_state().unwrap().span.is_some());
        assert_eq!(state.span.as_ref().unwrap().look, look);
        assert_eq!(state.adjust["DP-2"].span, Some(cut(1920)));

        // The same picture again keeps it; another one ends it.
        store.set_current("DP-1", wide).unwrap();
        assert!(store.load_state().unwrap().span.is_some());
        store.set_current("DP-1", other).unwrap();
        let state = store.load_state().unwrap();
        assert!(state.span.is_none());
        // DP-2 keeps its part.
        assert_eq!(state.adjust["DP-2"].span, Some(cut(1920)));

        // Separating keeps every part.
        store.set_current("DP-1", wide).unwrap();
        store.set_span(wide, look, &cuts, Default::default()).unwrap();
        let state = store.separate().unwrap();
        assert!(state.span.is_none() && state.adjust.len() == 2);
    }

    #[test]
    fn adjustments_last_until_the_wallpaper_changes() {
        use crate::settings::{Adjust, Flip};
        let tmp = tempfile::tempdir().unwrap();
        let store = Store::new(Dirs::under(tmp.path()));
        let (a, b) = (Path::new("/w/a.jpg"), Path::new("/w/b.jpg"));
        let mirrored = Adjust {
            flip: Flip { h: true, v: false },
            ..Default::default()
        };
        assert!(store.set_adjust("DP-1", mirrored).is_err(), "nothing shown there yet");
        store.set_current("DP-1", a).unwrap();
        store.set_current("DP-2", a).unwrap();
        store.set_adjust("DP-1", mirrored).unwrap();
        // The same wallpaper again keeps it; the other monitor never had it.
        store.set_current("DP-1", a).unwrap();
        let s = store.load_state().unwrap();
        assert_eq!(s.adjust_for("DP-1", a), mirrored);
        assert_eq!(s.adjust_for("DP-2", a), Adjust::default());
        // Another wallpaper clears it, and so does going back.
        store.set_current("DP-1", b).unwrap();
        assert!(store.load_state().unwrap().adjust.is_empty());
        store.set_adjust("DP-1", mirrored).unwrap();
        store.went_back("DP-1", a).unwrap();
        assert!(store.load_state().unwrap().adjust.is_empty());
    }

    #[test]
    fn snapshots_daily_and_restore() {
        let tmp = tempfile::tempdir().unwrap();
        let store = Store::new(Dirs::under(tmp.path()));
        // The first edit has nothing to copy yet; the second copies the first.
        store
            .update_catalog(|c| c.set_favorite(Path::new("/w/a.jpg"), true))
            .unwrap();
        assert!(store.snapshots().unwrap().is_empty());
        store
            .update_catalog(|c| c.set_favorite(Path::new("/w/b.jpg"), true))
            .unwrap();
        store
            .update_catalog(|c| c.set_favorite(Path::new("/w/c.jpg"), true))
            .unwrap();
        let snaps = store.snapshots().unwrap();
        // One a day: as the library was before today's first copied change.
        assert_eq!(snaps.len(), 1);
        assert_eq!(snaps[0].items, 1);

        let restored = store.restore_snapshot(&snaps[0].id).unwrap();
        assert_eq!(restored.items.len(), 1);
        assert_eq!(store.load_catalog().unwrap().items.len(), 1);
        // What was there (3) is kept as a timestamped snapshot.
        let snaps = store.snapshots().unwrap();
        assert_eq!(snaps.len(), 2);
        assert!(
            snaps
                .iter()
                .any(|s| s.items == 3 && s.label.len() == "2026-10-06 14:03:22".len())
        );

        assert!(store.restore_snapshot("../config").is_err());
        assert!(store.restore_snapshot("library-2026-1x-06").is_err());
    }

    #[test]
    fn an_old_library_is_saved_as_the_current_version() {
        let tmp = tempfile::tempdir().unwrap();
        let store = Store::new(Dirs::under(tmp.path()));
        // Nothing there: nothing written.
        store.upgrade_catalog().unwrap();
        assert!(!store.catalog_path().exists());

        fs::create_dir_all(store.catalog_path().parent().unwrap()).unwrap();
        fs::write(
            store.catalog_path(),
            r#"{"items": {"/w/a.jpg": {"favorite": true, "tags": ["x"]}}}"#,
        )
        .unwrap();
        assert_eq!(store.load_catalog().unwrap().version, 1);
        store.upgrade_catalog().unwrap();
        let c = store.load_catalog().unwrap();
        assert_eq!(c.version, crate::catalog::VERSION);
        assert_eq!(c.tags(Path::new("/w/a.jpg")), ["x"]);
        // Written first in the file, where a script reading it looks.
        assert!(
            fs::read_to_string(store.catalog_path())
                .unwrap()
                .starts_with("{\n  \"version\": 2,")
        );
        // A version-1 snapshot restored comes back as the current one.
        fs::create_dir_all(store.snapshots_dir()).unwrap();
        fs::write(
            store.snapshots_dir().join("library-2026-01-01.json"),
            r#"{"items": {}}"#,
        )
        .unwrap();
        assert_eq!(
            store.restore_snapshot("library-2026-01-01").unwrap().version,
            crate::catalog::VERSION
        );
        assert_eq!(store.load_catalog().unwrap().version, crate::catalog::VERSION);
    }

    #[test]
    fn snapshots_move_to_another_folder() {
        let tmp = tempfile::tempdir().unwrap();
        let store = Store::new(Dirs::under(tmp.path()));
        let old = store.default_snapshots_dir();
        fs::create_dir_all(&old).unwrap();
        fs::write(old.join("library-2026-10-01.json"), r#"{"items":{}}"#).unwrap();
        let synced = tmp.path().join("Sync/wallpicker");
        assert_eq!(store.set_snapshots_dir(Some(synced.clone())).unwrap(), synced);
        assert!(synced.join("library-2026-10-01.json").is_file());
        assert_eq!(store.snapshots_dir(), synced);
        assert_eq!(store.snapshots().unwrap().len(), 1);
        // Back to the default: stored as no folder.
        store.set_snapshots_dir(Some(old.clone())).unwrap();
        assert_eq!(store.load_settings().unwrap().snapshot_dir, None);
    }

    #[test]
    fn snapshots_keep_the_newest() {
        let tmp = tempfile::tempdir().unwrap();
        let store = Store::new(Dirs::under(tmp.path()));
        let dir = store.snapshots_dir();
        fs::create_dir_all(&dir).unwrap();
        for day in 1..=20 {
            fs::write(dir.join(format!("library-2026-09-{day:02}.json")), "{}").unwrap();
        }
        fs::write(dir.join("notes.txt"), "kept").unwrap();
        store.prune_snapshots().unwrap();
        let snaps = store.snapshots().unwrap();
        assert_eq!(snaps.len(), SNAPSHOTS_KEPT);
        assert_eq!(snaps[0].label, "2026-09-20");
        assert!(dir.join("notes.txt").exists());
    }

    #[test]
    fn settings_round_trip_and_defaults() {
        let tmp = tempfile::tempdir().unwrap();
        let store = Store::new(Dirs::under(tmp.path()));

        assert_eq!(store.load_settings().unwrap(), Settings::default());

        let s = Settings {
            recursive: false,
            wallhaven_api_key: Some("secret".into()),
            ..Settings::default()
        };
        store.save_settings(&s).unwrap();
        assert_eq!(store.load_settings().unwrap(), s);
    }

    #[cfg(unix)]
    #[test]
    fn settings_file_is_private() {
        use std::os::unix::fs::PermissionsExt;
        let tmp = tempfile::tempdir().unwrap();
        let store = Store::new(Dirs::under(tmp.path()));
        store.save_settings(&Settings::default()).unwrap();
        let mode = fs::metadata(store.settings_path()).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600);
    }

    #[test]
    fn corrupt_settings_are_an_error_not_a_reset() {
        let tmp = tempfile::tempdir().unwrap();
        let store = Store::new(Dirs::under(tmp.path()));
        fs::create_dir_all(&store.dirs().config).unwrap();
        fs::write(store.settings_path(), "{ nope").unwrap();
        assert!(store.load_settings().is_err());
    }

    #[test]
    fn state_remembers_each_monitor() {
        let tmp = tempfile::tempdir().unwrap();
        let store = Store::new(Dirs::under(tmp.path()));
        assert!(store.load_state().unwrap().current.is_empty());

        store.set_current("DP-1", Path::new("/a.png")).unwrap();
        store.set_current("DP-2", Path::new("/b.png")).unwrap();
        store.set_current("DP-1", Path::new("/c.png")).unwrap();

        let state = store.load_state().unwrap();
        assert_eq!(state.current.len(), 2);
        assert_eq!(state.current["DP-1"], PathBuf::from("/c.png"));
        assert_eq!(state.current["DP-2"], PathBuf::from("/b.png"));

        store.forget_current(Path::new("/b.png")).unwrap();
        let state = store.load_state().unwrap();
        assert_eq!(state.current.len(), 1);
        assert!(state.current.contains_key("DP-1"));
    }

    #[test]
    fn online_filters_round_trip_without_text_or_paging() {
        use crate::moebooru::{MoebooruQuery, MoebooruSort};
        use crate::online::Provider;
        use crate::wallhaven::Sorting;
        let tmp = tempfile::tempdir().unwrap();
        let store = Store::new(Dirs::under(tmp.path()));
        assert_eq!(store.load_online_filters().unwrap(), OnlineFilters::default());

        let f = OnlineFilters {
            provider: Provider::Konachan,
            wallhaven: SearchQuery {
                q: "forest".into(),
                page: 4,
                sorting: Sorting::Favorites,
                ..SearchQuery::default()
            },
            konachan: MoebooruQuery {
                tags: "night".into(),
                sorting: MoebooruSort::Score,
                ..MoebooruQuery::default()
            },
            yandere: MoebooruQuery {
                sorting: MoebooruSort::Random,
                ..MoebooruQuery::default()
            },
        };
        store.save_online_filters(&f).unwrap();
        let loaded = store.load_online_filters().unwrap();
        assert_eq!(loaded.provider, Provider::Konachan);
        assert_eq!(loaded.wallhaven.sorting, Sorting::Favorites);
        assert_eq!((loaded.wallhaven.q.as_str(), loaded.wallhaven.page), ("", 1));
        assert_eq!(loaded.konachan.sorting, MoebooruSort::Score);
        assert_eq!(loaded.yandere.sorting, MoebooruSort::Random);
        assert_eq!(loaded.konachan.tags, "");
    }

    #[test]
    fn old_wallhaven_filters_are_carried_over() {
        use crate::wallhaven::Sorting;
        let tmp = tempfile::tempdir().unwrap();
        let store = Store::new(Dirs::under(tmp.path()));
        fs::create_dir_all(&store.dirs().state).unwrap();
        let old = SearchQuery {
            sorting: Sorting::Views,
            ..SearchQuery::default()
        };
        fs::write(store.old_wallhaven_filters_path(), serde_json::to_string(&old).unwrap()).unwrap();
        assert_eq!(store.load_online_filters().unwrap().wallhaven.sorting, Sorting::Views);
    }

    #[test]
    fn catalog_round_trips_and_corruption_is_an_error() {
        let tmp = tempfile::tempdir().unwrap();
        let store = Store::new(Dirs::under(tmp.path()));
        assert_eq!(store.load_catalog().unwrap(), Catalog::default());

        let saved = store
            .update_catalog(|c| {
                c.set_favorite(Path::new("/a.png"), true);
                c.add_tag(Path::new("/a.png"), "dark");
            })
            .unwrap();
        assert_eq!(store.load_catalog().unwrap(), saved);
        assert!(saved.is_favorite(Path::new("/a.png")));

        fs::write(store.catalog_path(), "{ nope").unwrap();
        assert!(store.load_catalog().is_err());
        // A failed load must not be "fixed" by saving over it.
        assert!(store.update_catalog(|_| {}).is_err());
        assert_eq!(fs::read_to_string(store.catalog_path()).unwrap(), "{ nope");
    }
}
