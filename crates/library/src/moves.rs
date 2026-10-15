//! Moved-file detection: a "new" file that matches a "missing" one keeps the old track id,
//! so playlists, likes and history survive reorganising the music folders.

use rusqlite::{params, Connection, OptionalExtension};

use crate::error::Result;
use crate::scanner::{file_stem, DiskFile};
use crate::tags::TrackTags;

const DURATION_TOLERANCE_MS: i64 = 1_000;

#[derive(Clone, Debug)]
pub(crate) struct MissingTrack {
    pub id: i64,
    size: i64,
    duration_ms: i64,
    title: String,
    artist: Option<String>,
    album: Option<String>,
    mb_recording_id: Option<String>,
}

pub(crate) fn load_missing(conn: &Connection, ids: &[i64]) -> Result<Vec<MissingTrack>> {
    let mut stmt = conn.prepare(
        "SELECT t.size, t.duration_ms, t.title,
                (SELECT a.name FROM track_artists ta JOIN artists a ON a.id = ta.artist_id
                  WHERE ta.track_id = t.id ORDER BY ta.position LIMIT 1),
                al.title, t.mb_recording_id
           FROM tracks t LEFT JOIN albums al ON al.id = t.album_id
          WHERE t.id = ?1",
    )?;
    let mut missing = Vec::with_capacity(ids.len());
    for &id in ids {
        let row = stmt
            .query_row(params![id], |r| {
                Ok(MissingTrack {
                    id,
                    size: r.get(0)?,
                    duration_ms: r.get(1)?,
                    title: r.get(2)?,
                    artist: r.get(3)?,
                    album: r.get(4)?,
                    mb_recording_id: r.get(5)?,
                })
            })
            .optional()?;
        missing.extend(row);
    }
    Ok(missing)
}

/// Removes and returns the id of the missing track `file` is a move of, if any.
pub(crate) fn take_match(missing: &mut Vec<MissingTrack>, file: &DiskFile, tags: &TrackTags) -> Option<i64> {
    let title = tags.title.clone().unwrap_or_else(|| file_stem(&file.path));
    let position = missing.iter().position(|m| is_same(m, file, tags, &title))?;
    Some(missing.swap_remove(position).id)
}

fn is_same(m: &MissingTrack, file: &DiskFile, tags: &TrackTags, title: &str) -> bool {
    if let (Some(a), Some(b)) = (&m.mb_recording_id, &tags.mb_recording_id) {
        return a == b;
    }
    m.size == file.size
        && (m.duration_ms - tags.duration_ms as i64).abs() <= DURATION_TOLERANCE_MS
        && m.title.to_lowercase() == title.to_lowercase()
        && same_text(m.artist.as_deref(), tags.artists.first().map(String::as_str))
        && same_text(m.album.as_deref(), tags.album.as_deref())
}

fn same_text(a: Option<&str>, b: Option<&str>) -> bool {
    a.map(str::to_lowercase) == b.map(str::to_lowercase)
}
