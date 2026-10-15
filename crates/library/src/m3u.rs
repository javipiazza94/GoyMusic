use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use goymusic_core::model::Playlist;
use goymusic_core::{PlaylistId, TrackId};
use rusqlite::OptionalExtension;

use crate::db::Db;
use crate::error::Result;
use crate::playlists;

#[derive(Debug)]
pub struct ImportReport {
    pub playlist: Playlist,
    pub matched: usize,
    /// Entries (as written in the file) not found in the library.
    pub missing: Vec<String>,
}

/// Creates playlist `name` from an M3U/M3U8 file. Relative entries resolve from the file's
/// directory; entries not in the library are reported, not fatal.
pub fn import(db: &mut Db, name: &str, m3u: &Path) -> Result<ImportReport> {
    let text = std::fs::read_to_string(m3u)?;
    let base = m3u.parent().unwrap_or_else(|| Path::new("."));
    let mut tracks = Vec::new();
    let mut missing = Vec::new();
    for entry in text.lines().map(|l| l.trim_start_matches('\u{feff}').trim()) {
        if entry.is_empty() || entry.starts_with('#') {
            continue;
        }
        let candidate = if Path::new(entry).is_absolute() {
            PathBuf::from(entry)
        } else {
            base.join(entry)
        };
        let resolved = std::fs::canonicalize(&candidate)
            .ok()
            .and_then(|p| p.to_str().map(str::to_owned));
        let id = match resolved {
            Some(path) => db
                .conn()
                .query_row("SELECT id FROM tracks WHERE path = ?1", [&path], |r| r.get::<_, i64>(0))
                .optional()?,
            None => None,
        };
        match id {
            Some(id) => tracks.push(TrackId(id)),
            None => missing.push(entry.to_owned()),
        }
    }
    let created = playlists::create(db, name)?;
    playlists::add_tracks(db, created.id, &tracks)?;
    Ok(ImportReport {
        playlist: playlists::get(db, created.id)?,
        matched: tracks.len(),
        missing,
    })
}

/// Writes the playlist as extended M3U8 with absolute paths.
pub fn export(db: &Db, id: PlaylistId, path: &Path) -> Result<()> {
    let mut out = String::from("#EXTM3U\n");
    for item in playlists::items(db, id)? {
        let track = &item.track;
        let _ = writeln!(
            out,
            "#EXTINF:{},{} - {}",
            track.duration_ms / 1000,
            track.artists.join(", "),
            track.title
        );
        let _ = writeln!(out, "{}", track.path);
    }
    std::fs::write(path, out)?;
    Ok(())
}
