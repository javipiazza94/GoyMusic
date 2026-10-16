//! Play history: one row per track that started playing.

use goymusic_core::model::Album;
use goymusic_core::TrackId;
use rusqlite::params;

use crate::db::Db;
use crate::error::Result;
use crate::queries::{album_from_row, ALBUM_SELECT};

/// `played_at` is a Unix timestamp in seconds.
pub fn record_play(db: &Db, track: TrackId, played_at: i64) -> Result<()> {
    db.conn().execute(
        "INSERT INTO play_history (track_id, played_at) VALUES (?1, ?2)",
        params![track.0, played_at],
    )?;
    Ok(())
}

/// Albums of the tracks played most recently, each once, latest first.
pub fn recent_albums(db: &Db, limit: u32) -> Result<Vec<Album>> {
    let sql = format!(
        "{ALBUM_SELECT}
         JOIN (SELECT t.album_id AS album_id, max(h.played_at) AS last, max(h.id) AS last_id
                 FROM play_history h JOIN tracks t ON t.id = h.track_id
                WHERE t.album_id IS NOT NULL GROUP BY t.album_id) recent ON recent.album_id = al.id
         ORDER BY recent.last DESC, recent.last_id DESC LIMIT ?1"
    );
    let mut stmt = db.conn().prepare(&sql)?;
    let albums = stmt
        .query_map([limit], album_from_row)?
        .collect::<rusqlite::Result<_>>()?;
    Ok(albums)
}
