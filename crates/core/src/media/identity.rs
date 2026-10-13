use std::collections::VecDeque;
use std::fs::Metadata;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use super::{Limits, Result, Source, invalid};

/// High-resolution identity. ctime also catches same-size rewrites whose mtime
/// was restored; inode/device detect atomic replacement. This is not a content
/// hash or a guarantee against concurrent writes during a decode.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct FileIdentity {
    pub device: u64,
    pub inode: u64,
    pub len: u64,
    pub modified: SystemTime,
    pub changed_seconds: i64,
    pub changed_nanos: i64,
}

impl FileIdentity {
    pub fn read(path: &Path) -> Result<Self> {
        Self::from_metadata(&std::fs::metadata(path)?)
    }

    pub(super) fn from_metadata(meta: &Metadata) -> Result<Self> {
        if !meta.is_file() {
            return Err(invalid("media must be a regular file"));
        }
        Ok(Self {
            device: meta.dev(),
            inode: meta.ino(),
            len: meta.len(),
            modified: meta.modified()?,
            changed_seconds: meta.ctime(),
            changed_nanos: meta.ctime_nsec(),
        })
    }
}

/// Bounded LRU of inspected metadata only. No open files or pixel buffers.
/// A cache has fixed limits so a hit cannot bypass a stricter caller's policy.
pub struct MetadataCache {
    limits: Limits,
    capacity: usize,
    entries: VecDeque<(PathBuf, Source)>,
}

impl MetadataCache {
    pub fn new(limits: Limits, capacity: usize) -> Self {
        Self {
            limits,
            capacity,
            entries: VecDeque::new(),
        }
    }

    pub fn get(&mut self, path: &Path) -> Result<Source> {
        let path = path.canonicalize()?;
        let identity = FileIdentity::read(&path)?;
        if let Some(i) = self.entries.iter().position(|(p, _)| *p == path) {
            let (_, source) = self.entries.remove(i).expect("existing cache entry");
            if source.identity == identity {
                self.entries.push_back((path, source.clone()));
                return Ok(source);
            }
        }
        let source = Source::open(&path, self.limits)?;
        if self.capacity > 0 {
            while self.entries.len() >= self.capacity {
                self.entries.pop_front();
            }
            self.entries.push_back((path, source.clone()));
        }
        Ok(source)
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }
    pub fn len(&self) -> usize {
        self.entries.len()
    }
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}
