use rusqlite::Connection;

use crate::error::Result;

/// Deletes tracks (and their FTS rows; FTS5 tables have no foreign keys).
pub(crate) fn delete_tracks(conn: &Connection, ids: &[i64]) -> Result<()> {
    let mut fts = conn.prepare("DELETE FROM tracks_fts WHERE rowid = ?1")?;
    let mut track = conn.prepare("DELETE FROM tracks WHERE id = ?1")?;
    for id in ids {
        fts.execute([id])?;
        track.execute([id])?;
    }
    Ok(())
}

/// Removes albums without tracks and artists referenced by neither tracks nor albums.
pub(crate) fn cleanup_orphans(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "DELETE FROM albums WHERE id NOT IN (SELECT album_id FROM tracks WHERE album_id IS NOT NULL);
         DELETE FROM artists WHERE id NOT IN (SELECT artist_id FROM track_artists)
                               AND id NOT IN (SELECT artist_id FROM albums WHERE artist_id IS NOT NULL);",
    )?;
    Ok(())
}
