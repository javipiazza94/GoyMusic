use std::path::{Path, PathBuf};
use std::time::Duration;

/// Where the services keep their data and how they pace background work.
#[derive(Clone, Debug)]
pub struct ServicesConfig {
    pub db_path: PathBuf,
    pub art_dir: PathBuf,
    /// Quiet period after a file change before rescanning (spec §6: 2 s).
    pub debounce: Duration,
    /// Rescan period when folders cannot be watched (spec §6: 10 min).
    pub poll_every: Duration,
}

impl ServicesConfig {
    /// `~/.local/share/goymusic/library.db` and `~/.cache/goymusic/art`.
    pub fn default_paths() -> Self {
        let data = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
        let cache = dirs::cache_dir().unwrap_or_else(|| PathBuf::from("."));
        Self {
            db_path: data.join("goymusic/library.db"),
            art_dir: cache.join("goymusic/art"),
            debounce: Duration::from_secs(2),
            poll_every: Duration::from_secs(600),
        }
    }

    /// Everything under `dir`, with short timings for tests.
    pub fn in_dir(dir: &Path) -> Self {
        Self {
            db_path: dir.join("library.db"),
            art_dir: dir.join("art"),
            debounce: Duration::from_millis(200),
            poll_every: Duration::from_secs(600),
        }
    }
}
