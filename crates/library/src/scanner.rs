//! Incremental scan: walk the library folders, read tags only for new or changed files and
//! keep the database in sync.

use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::UNIX_EPOCH;

use goymusic_core::events::{Event, EventBus};
use rayon::prelude::*;
use rusqlite::{params, Connection, TransactionBehavior};
use walkdir::WalkDir;

use crate::db::Db;
use crate::error::{LibraryError, Result};
use crate::maintenance;
use crate::moves;
use crate::tags::{read_tags, TrackTags};

pub const AUDIO_EXTENSIONS: &[&str] = &["mp3", "flac", "ogg", "oga", "wav", "m4a", "mp4", "aac"];
const PROGRESS_EVERY: usize = 100;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ScanReport {
    pub added: usize,
    pub updated: usize,
    pub moved: usize,
    pub removed: usize,
    pub unchanged: usize,
    pub errors: Vec<ScanError>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScanError {
    pub path: String,
    pub message: String,
}

#[derive(Clone, Debug)]
pub(crate) struct DiskFile {
    pub folder_id: i64,
    pub path: String,
    pub size: i64,
    pub mtime: i64,
}

struct Known {
    id: i64,
    size: i64,
    mtime: i64,
}

pub fn scan(db: &mut Db, events: &EventBus) -> Result<ScanReport> {
    scan_cancellable(db, events, &AtomicBool::new(false))
}

/// [`scan`] that gives up as soon as `cancel` is set (closing the app). A cancelled scan
/// writes nothing: its transaction is rolled back, and the next scan starts over.
pub fn scan_cancellable(db: &mut Db, events: &EventBus, cancel: &AtomicBool) -> Result<ScanReport> {
    let cancelled = || cancel.load(Ordering::Relaxed);
    let mut report = ScanReport::default();
    let disk = walk_folders(db.conn(), &mut report)?;
    let known = known_tracks(db.conn())?;
    let on_disk: HashSet<&str> = disk.iter().map(|f| f.path.as_str()).collect();

    let mut to_read: Vec<(&DiskFile, Option<i64>)> = Vec::new();
    for file in &disk {
        match known.get(&file.path) {
            Some(k) if k.size == file.size && k.mtime == file.mtime => report.unchanged += 1,
            Some(k) => to_read.push((file, Some(k.id))),
            None => to_read.push((file, None)),
        }
    }
    let missing: Vec<i64> = known
        .iter()
        .filter(|(path, _)| !on_disk.contains(path.as_str()))
        .map(|(_, k)| k.id)
        .collect();

    let total = to_read.len() as u64;
    events.publish(Event::ScanProgress { done: 0, total });
    let read: Vec<_> = to_read
        .par_iter()
        .map(|&(file, id)| {
            let tags = if cancelled() {
                Err(LibraryError::Cancelled)
            } else {
                read_tags(Path::new(&file.path))
            };
            (file, id, tags)
        })
        .collect();
    if cancelled() {
        return Err(LibraryError::Cancelled);
    }

    let tx = db
        .conn_mut()
        .transaction_with_behavior(TransactionBehavior::Immediate)?;
    let mut missing = moves::load_missing(&tx, &missing)?;
    for (done, (file, existing, tags)) in read.into_iter().enumerate() {
        if cancelled() {
            return Err(LibraryError::Cancelled);
        }
        match tags {
            // an unreadable changed file keeps its previous row
            Err(e) => report.errors.push(ScanError {
                path: file.path.clone(),
                message: e.to_string(),
            }),
            Ok(tags) => match existing {
                Some(id) => {
                    write_track(&tx, file, &tags, Some(id))?;
                    report.updated += 1;
                }
                None => match moves::take_match(&mut missing, file, &tags) {
                    Some(id) => {
                        write_track(&tx, file, &tags, Some(id))?;
                        report.moved += 1;
                    }
                    None => {
                        write_track(&tx, file, &tags, None)?;
                        report.added += 1;
                    }
                },
            },
        }
        if (done + 1) % PROGRESS_EVERY == 0 {
            events.publish(Event::ScanProgress {
                done: done as u64 + 1,
                total,
            });
        }
    }
    // Only delete tracks whose absence we are sure of. An error this scan (a folder we could
    // not enumerate, or a move destination we could not read) can hide a file that is really
    // still there, and `tracks` cascades to `playlist_items` and `likes` — so a transient
    // failure would permanently destroy user curation. When any error occurred, defer removals
    // to a later clean scan.
    if report.errors.is_empty() {
        let removed: Vec<i64> = missing.iter().map(|m| m.id).collect();
        maintenance::delete_tracks(&tx, &removed)?;
        report.removed = removed.len();
    }
    maintenance::cleanup_orphans(&tx)?;
    tx.commit()?;

    events.publish(Event::ScanProgress { done: total, total });
    if report.added + report.updated + report.moved + report.removed > 0 {
        events.publish(Event::LibraryUpdated);
    }
    Ok(report)
}

fn walk_folders(conn: &Connection, report: &mut ScanReport) -> Result<Vec<DiskFile>> {
    let folders: Vec<(i64, String)> = {
        let mut stmt = conn.prepare("SELECT id, path FROM folders")?;
        let rows = stmt
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
            .collect::<rusqlite::Result<_>>()?;
        rows
    };
    let mut files = Vec::new();
    for (folder_id, root) in folders {
        for entry in WalkDir::new(&root).follow_links(true) {
            let entry = match entry {
                Ok(entry) => entry,
                Err(e) => {
                    let path = e.path().map_or_else(|| root.clone(), |p| p.display().to_string());
                    report.errors.push(ScanError {
                        path,
                        message: e.to_string(),
                    });
                    continue;
                }
            };
            if !entry.file_type().is_file() || !is_audio(entry.path()) {
                continue;
            }
            let Some(path) = entry.path().to_str() else {
                report.errors.push(ScanError {
                    path: entry.path().display().to_string(),
                    message: "path is not valid UTF-8".into(),
                });
                continue;
            };
            let meta = match entry.metadata() {
                Ok(meta) => meta,
                Err(e) => {
                    report.errors.push(ScanError {
                        path: path.to_owned(),
                        message: e.to_string(),
                    });
                    continue;
                }
            };
            let mtime = meta
                .modified()
                .ok()
                .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                .map_or(0, |d| d.as_secs() as i64);
            files.push(DiskFile {
                folder_id,
                path: path.to_owned(),
                size: meta.len() as i64,
                mtime,
            });
        }
    }
    Ok(files)
}

fn is_audio(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|ext| AUDIO_EXTENSIONS.iter().any(|a| a.eq_ignore_ascii_case(ext)))
}

fn known_tracks(conn: &Connection) -> Result<HashMap<String, Known>> {
    let mut stmt = conn.prepare("SELECT path, id, size, mtime FROM tracks")?;
    let known = stmt
        .query_map([], |r| {
            Ok((
                r.get(0)?,
                Known {
                    id: r.get(1)?,
                    size: r.get(2)?,
                    mtime: r.get(3)?,
                },
            ))
        })?
        .collect::<rusqlite::Result<_>>()?;
    Ok(known)
}

/// Inserts a track, or updates the row `existing` (same file changed, or a moved file),
/// and refreshes its artists and FTS row. Returns the track id.
pub(crate) fn write_track(conn: &Connection, file: &DiskFile, tags: &TrackTags, existing: Option<i64>) -> Result<i64> {
    let title = tags.title.clone().unwrap_or_else(|| file_stem(&file.path));
    let album_id = match &tags.album {
        Some(album) => Some(upsert_album(conn, album, tags)?),
        None => None,
    };
    let duration_ms = tags.duration_ms as i64;
    let id = match existing {
        Some(id) => {
            conn.execute(
                "UPDATE tracks SET path = ?1, folder_id = ?2, size = ?3, mtime = ?4, title = ?5, album_id = ?6,
                     album_artist = ?7, track_no = ?8, disc_no = ?9, year = ?10, genre = ?11, duration_ms = ?12,
                     mb_recording_id = ?13
                 WHERE id = ?14",
                params![
                    file.path,
                    file.folder_id,
                    file.size,
                    file.mtime,
                    title,
                    album_id,
                    tags.album_artist,
                    tags.track_no,
                    tags.disc_no,
                    tags.year,
                    tags.genre,
                    duration_ms,
                    tags.mb_recording_id,
                    id
                ],
            )?;
            id
        }
        None => {
            conn.execute(
                "INSERT INTO tracks (path, folder_id, size, mtime, title, album_id, album_artist, track_no, disc_no,
                                     year, genre, duration_ms, mb_recording_id)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
                params![
                    file.path,
                    file.folder_id,
                    file.size,
                    file.mtime,
                    title,
                    album_id,
                    tags.album_artist,
                    tags.track_no,
                    tags.disc_no,
                    tags.year,
                    tags.genre,
                    duration_ms,
                    tags.mb_recording_id
                ],
            )?;
            conn.last_insert_rowid()
        }
    };
    conn.execute("DELETE FROM track_artists WHERE track_id = ?1", [id])?;
    for (position, name) in tags.artists.iter().enumerate() {
        let artist_id = upsert_artist(conn, name)?;
        conn.execute(
            "INSERT OR IGNORE INTO track_artists (track_id, artist_id, position) VALUES (?1, ?2, ?3)",
            params![id, artist_id, position as i64],
        )?;
    }
    conn.execute("DELETE FROM tracks_fts WHERE rowid = ?1", [id])?;
    conn.execute(
        "INSERT INTO tracks_fts (rowid, title, artists, album) VALUES (?1, ?2, ?3, ?4)",
        params![id, title, tags.artists.join(" "), tags.album.as_deref().unwrap_or("")],
    )?;
    Ok(id)
}

fn upsert_artist(conn: &Connection, name: &str) -> Result<i64> {
    conn.execute(
        "INSERT INTO artists (name) VALUES (?1) ON CONFLICT(name) DO NOTHING",
        [name],
    )?;
    Ok(conn.query_row("SELECT id FROM artists WHERE name = ?1", [name], |r| r.get(0))?)
}

fn upsert_album(conn: &Connection, title: &str, tags: &TrackTags) -> Result<i64> {
    let artist = tags
        .album_artist
        .as_deref()
        .or_else(|| tags.artists.first().map(String::as_str));
    let artist_id = artist.map(|name| upsert_artist(conn, name)).transpose()?;
    let key = album_key(title, artist, tags.mb_release_id.as_deref());
    conn.execute(
        "INSERT INTO albums (title, artist_id, mb_release_id, year, group_key) VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT(group_key) DO UPDATE SET year = COALESCE(albums.year, excluded.year)",
        params![title, artist_id, tags.mb_release_id, tags.year, key],
    )?;
    Ok(conn.query_row("SELECT id FROM albums WHERE group_key = ?1", [&key], |r| r.get(0))?)
}

/// Album identity: the MusicBrainz release when tagged, else album artist + title
/// (case-insensitive), so compilations stay one album.
fn album_key(title: &str, artist: Option<&str>, mb_release_id: Option<&str>) -> String {
    match mb_release_id {
        Some(id) => format!("mb:{id}"),
        None => format!(
            "t:{}\u{1f}{}",
            artist.unwrap_or("").trim().to_lowercase(),
            title.trim().to_lowercase()
        ),
    }
}

pub(crate) fn file_stem(path: &str) -> String {
    Path::new(path)
        .file_stem()
        .map_or_else(|| path.to_owned(), |s| s.to_string_lossy().into_owned())
}
