use std::time::{SystemTime, UNIX_EPOCH};

use goymusic_core::model::{Playlist, PlaylistFolder, PlaylistItem, Track};
use goymusic_core::{PlaylistFolderId, PlaylistId, TrackId};
use rusqlite::{params, Connection, OptionalExtension, Row};

use crate::db::Db;
use crate::error::{LibraryError, Result};
use crate::queries::{track_from_row, TRACK_SELECT};

fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as i64)
}

const PLAYLIST_SELECT: &str = "
    SELECT p.id, p.name, p.folder_id, (SELECT count(*) FROM playlist_items i WHERE i.playlist_id = p.id)
      FROM playlists p";

fn playlist_from_row(r: &Row) -> rusqlite::Result<Playlist> {
    Ok(Playlist {
        id: PlaylistId(r.get(0)?),
        name: r.get(1)?,
        folder_id: r.get::<_, Option<i64>>(2)?.map(PlaylistFolderId),
        track_count: r.get(3)?,
    })
}

fn not_found(id: PlaylistId) -> LibraryError {
    LibraryError::NotFound {
        kind: "playlist",
        id: id.0,
    }
}

fn ensure_exists(conn: &Connection, id: PlaylistId) -> Result<()> {
    conn.query_row("SELECT 1 FROM playlists WHERE id = ?1", [id.0], |_| Ok(()))
        .optional()?
        .ok_or_else(|| not_found(id))
}

pub fn create(db: &Db, name: &str) -> Result<Playlist> {
    db.conn().execute(
        "INSERT INTO playlists (name, created_at) VALUES (?1, ?2)",
        params![name, now_secs()],
    )?;
    get(db, PlaylistId(db.conn().last_insert_rowid()))
}

pub fn rename(db: &Db, id: PlaylistId, name: &str) -> Result<()> {
    match db
        .conn()
        .execute("UPDATE playlists SET name = ?1 WHERE id = ?2", params![name, id.0])?
    {
        0 => Err(not_found(id)),
        _ => Ok(()),
    }
}

pub fn delete(db: &Db, id: PlaylistId) -> Result<()> {
    match db.conn().execute("DELETE FROM playlists WHERE id = ?1", [id.0])? {
        0 => Err(not_found(id)),
        _ => Ok(()),
    }
}

pub fn list(db: &Db) -> Result<Vec<Playlist>> {
    let sql = format!("{PLAYLIST_SELECT} ORDER BY p.name COLLATE NOCASE, p.id");
    let mut stmt = db.conn().prepare(&sql)?;
    let playlists = stmt
        .query_map([], playlist_from_row)?
        .collect::<rusqlite::Result<_>>()?;
    Ok(playlists)
}

pub fn get(db: &Db, id: PlaylistId) -> Result<Playlist> {
    let sql = format!("{PLAYLIST_SELECT} WHERE p.id = ?1");
    db.conn()
        .query_row(&sql, [id.0], playlist_from_row)
        .optional()?
        .ok_or_else(|| not_found(id))
}

pub fn items(db: &Db, id: PlaylistId) -> Result<Vec<PlaylistItem>> {
    ensure_exists(db.conn(), id)?;
    let sql =
        format!("{TRACK_SELECT} JOIN playlist_items i ON i.track_id = t.id WHERE i.playlist_id = ?1 ORDER BY i.pos");
    let mut stmt = db.conn().prepare(&sql)?;
    let tracks: Vec<Track> = stmt
        .query_map([id.0], track_from_row)?
        .collect::<rusqlite::Result<_>>()?;
    Ok(tracks
        .into_iter()
        .enumerate()
        .map(|(position, track)| PlaylistItem {
            position: position as u32,
            track,
        })
        .collect())
}

fn item_track_ids(conn: &Connection, id: PlaylistId) -> Result<Vec<i64>> {
    let mut stmt = conn.prepare("SELECT track_id FROM playlist_items WHERE playlist_id = ?1 ORDER BY pos")?;
    let ids = stmt.query_map([id.0], |r| r.get(0))?.collect::<rusqlite::Result<_>>()?;
    Ok(ids)
}

/// Rewrites the playlist with positions 0..n (simpler and safer than shifting positions).
fn rewrite_items(conn: &Connection, id: PlaylistId, track_ids: &[i64]) -> Result<()> {
    conn.execute("DELETE FROM playlist_items WHERE playlist_id = ?1", [id.0])?;
    let mut insert = conn.prepare("INSERT INTO playlist_items (playlist_id, pos, track_id) VALUES (?1, ?2, ?3)")?;
    for (pos, track_id) in track_ids.iter().enumerate() {
        insert.execute(params![id.0, pos as i64, track_id])?;
    }
    Ok(())
}

pub fn add_tracks(db: &mut Db, id: PlaylistId, tracks: &[TrackId]) -> Result<()> {
    let tx = db.conn_mut().transaction()?;
    ensure_exists(&tx, id)?;
    let mut ids = item_track_ids(&tx, id)?;
    ids.extend(tracks.iter().map(|t| t.0));
    rewrite_items(&tx, id, &ids)?;
    tx.commit()?;
    Ok(())
}

pub fn remove_items(db: &mut Db, id: PlaylistId, positions: &[u32]) -> Result<()> {
    let tx = db.conn_mut().transaction()?;
    ensure_exists(&tx, id)?;
    let ids = item_track_ids(&tx, id)?;
    if let Some(&bad) = positions.iter().find(|&&p| p as usize >= ids.len()) {
        return Err(LibraryError::InvalidPosition {
            position: bad as usize,
            len: ids.len(),
        });
    }
    let kept: Vec<i64> = ids
        .into_iter()
        .enumerate()
        .filter(|(i, _)| !positions.contains(&(*i as u32)))
        .map(|(_, t)| t)
        .collect();
    rewrite_items(&tx, id, &kept)?;
    tx.commit()?;
    Ok(())
}

pub fn move_item(db: &mut Db, id: PlaylistId, from: u32, to: u32) -> Result<()> {
    let tx = db.conn_mut().transaction()?;
    ensure_exists(&tx, id)?;
    let mut ids = item_track_ids(&tx, id)?;
    for position in [from, to] {
        if position as usize >= ids.len() {
            return Err(LibraryError::InvalidPosition {
                position: position as usize,
                len: ids.len(),
            });
        }
    }
    let track = ids.remove(from as usize);
    ids.insert(to as usize, track);
    rewrite_items(&tx, id, &ids)?;
    tx.commit()?;
    Ok(())
}

pub fn create_folder(db: &Db, name: &str, parent: Option<PlaylistFolderId>) -> Result<PlaylistFolder> {
    db.conn().execute(
        "INSERT INTO playlist_folders (name, parent_id) VALUES (?1, ?2)",
        params![name, parent.map(|p| p.0)],
    )?;
    Ok(PlaylistFolder {
        id: PlaylistFolderId(db.conn().last_insert_rowid()),
        name: name.to_owned(),
        parent_id: parent,
    })
}

pub fn folders(db: &Db) -> Result<Vec<PlaylistFolder>> {
    let mut stmt = db
        .conn()
        .prepare("SELECT id, name, parent_id FROM playlist_folders ORDER BY name COLLATE NOCASE, id")?;
    let folders = stmt
        .query_map([], |r| {
            Ok(PlaylistFolder {
                id: PlaylistFolderId(r.get(0)?),
                name: r.get(1)?,
                parent_id: r.get::<_, Option<i64>>(2)?.map(PlaylistFolderId),
            })
        })?
        .collect::<rusqlite::Result<_>>()?;
    Ok(folders)
}

pub fn move_to_folder(db: &Db, id: PlaylistId, folder: Option<PlaylistFolderId>) -> Result<()> {
    match db.conn().execute(
        "UPDATE playlists SET folder_id = ?1 WHERE id = ?2",
        params![folder.map(|f| f.0), id.0],
    )? {
        0 => Err(not_found(id)),
        _ => Ok(()),
    }
}

pub fn like(db: &Db, track: TrackId) -> Result<()> {
    db.conn().execute(
        "INSERT OR IGNORE INTO likes (track_id, liked_at) VALUES (?1, ?2)",
        params![track.0, now_secs()],
    )?;
    Ok(())
}

pub fn unlike(db: &Db, track: TrackId) -> Result<()> {
    db.conn().execute("DELETE FROM likes WHERE track_id = ?1", [track.0])?;
    Ok(())
}

pub fn is_liked(db: &Db, track: TrackId) -> Result<bool> {
    Ok(db
        .conn()
        .query_row("SELECT 1 FROM likes WHERE track_id = ?1", [track.0], |_| Ok(()))
        .optional()?
        .is_some())
}

/// Liked tracks, most recently liked first (`likes.id` grows with every like).
pub fn liked_tracks(db: &Db) -> Result<Vec<Track>> {
    let sql = format!("{TRACK_SELECT} JOIN likes l ON l.track_id = t.id ORDER BY l.id DESC");
    let mut stmt = db.conn().prepare(&sql)?;
    let tracks = stmt.query_map([], track_from_row)?.collect::<rusqlite::Result<_>>()?;
    Ok(tracks)
}
