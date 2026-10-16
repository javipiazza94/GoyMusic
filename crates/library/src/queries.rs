use goymusic_core::model::{Album, Artist, Track};
use goymusic_core::{AlbumId, ArtistId, TrackId};
use rusqlite::{params, OptionalExtension, Row};

use crate::db::Db;
use crate::error::{LibraryError, Result};

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub enum TrackSort {
    Title,
    Artist,
    Album,
    RecentlyAdded,
}

/// Columns read by [`track_from_row`]. Artists come ordered and joined with U+001F.
pub(crate) const TRACK_SELECT: &str = "
    SELECT t.id, t.path, t.title, al.title, t.album_id, t.album_artist, t.track_no, t.disc_no, t.year, t.genre,
           t.duration_ms,
           (SELECT group_concat(name, char(31)) FROM
               (SELECT a.name AS name FROM track_artists ta JOIN artists a ON a.id = ta.artist_id
                 WHERE ta.track_id = t.id ORDER BY ta.position)) AS artists_joined
      FROM tracks t LEFT JOIN albums al ON al.id = t.album_id";

pub(crate) fn track_from_row(r: &Row) -> rusqlite::Result<Track> {
    let artists: Option<String> = r.get(11)?;
    Ok(Track {
        id: TrackId(r.get(0)?),
        path: r.get(1)?,
        title: r.get(2)?,
        album: r.get(3)?,
        album_id: r.get::<_, Option<i64>>(4)?.map(AlbumId),
        album_artist: r.get(5)?,
        track_no: r.get(6)?,
        disc_no: r.get(7)?,
        year: r.get(8)?,
        genre: r.get(9)?,
        duration_ms: r.get::<_, i64>(10)? as u64,
        artists: artists
            .map(|s| s.split('\u{1f}').map(str::to_owned).collect())
            .unwrap_or_default(),
    })
}

pub(crate) const ALBUM_SELECT: &str = "
    SELECT al.id, al.title, ar.name, al.year, (SELECT count(*) FROM tracks t WHERE t.album_id = al.id)
      FROM albums al LEFT JOIN artists ar ON ar.id = al.artist_id";

pub(crate) fn album_from_row(r: &Row) -> rusqlite::Result<Album> {
    Ok(Album {
        id: AlbumId(r.get(0)?),
        title: r.get(1)?,
        artist: r.get(2)?,
        year: r.get(3)?,
        track_count: r.get(4)?,
    })
}

pub fn albums(db: &Db) -> Result<Vec<Album>> {
    let sql = format!("{ALBUM_SELECT} ORDER BY ar.name COLLATE NOCASE, al.title COLLATE NOCASE");
    let mut stmt = db.conn().prepare(&sql)?;
    let albums = stmt.query_map([], album_from_row)?.collect::<rusqlite::Result<_>>()?;
    Ok(albums)
}

pub fn album(db: &Db, id: AlbumId) -> Result<Album> {
    let sql = format!("{ALBUM_SELECT} WHERE al.id = ?1");
    db.conn()
        .query_row(&sql, [id.0], album_from_row)
        .optional()?
        .ok_or(LibraryError::NotFound {
            kind: "album",
            id: id.0,
        })
}

pub fn album_tracks(db: &Db, id: AlbumId) -> Result<Vec<Track>> {
    let sql = format!(
        "{TRACK_SELECT} WHERE t.album_id = ?1 ORDER BY coalesce(t.disc_no, 1), t.track_no, t.title COLLATE NOCASE"
    );
    let mut stmt = db.conn().prepare(&sql)?;
    let tracks = stmt
        .query_map([id.0], track_from_row)?
        .collect::<rusqlite::Result<_>>()?;
    Ok(tracks)
}

pub fn artists(db: &Db) -> Result<Vec<Artist>> {
    let mut stmt = db
        .conn()
        .prepare("SELECT id, name FROM artists ORDER BY name COLLATE NOCASE")?;
    let artists = stmt
        .query_map([], |r| {
            Ok(Artist {
                id: ArtistId(r.get(0)?),
                name: r.get(1)?,
            })
        })?
        .collect::<rusqlite::Result<_>>()?;
    Ok(artists)
}

pub fn artist(db: &Db, id: ArtistId) -> Result<Artist> {
    db.conn()
        .query_row("SELECT id, name FROM artists WHERE id = ?1", [id.0], |r| {
            Ok(Artist {
                id: ArtistId(r.get(0)?),
                name: r.get(1)?,
            })
        })
        .optional()?
        .ok_or(LibraryError::NotFound {
            kind: "artist",
            id: id.0,
        })
}

/// Tracks the artist appears on, album by album (oldest first), in disc and track order.
pub fn artist_tracks(db: &Db, id: ArtistId) -> Result<Vec<Track>> {
    let sql = format!(
        "{TRACK_SELECT} WHERE t.id IN (SELECT track_id FROM track_artists WHERE artist_id = ?1)
         ORDER BY al.year, al.title COLLATE NOCASE, coalesce(t.disc_no, 1), t.track_no, t.id"
    );
    let mut stmt = db.conn().prepare(&sql)?;
    let tracks = stmt
        .query_map([id.0], track_from_row)?
        .collect::<rusqlite::Result<_>>()?;
    Ok(tracks)
}

pub fn artist_albums(db: &Db, id: ArtistId) -> Result<Vec<Album>> {
    let sql = format!("{ALBUM_SELECT} WHERE al.artist_id = ?1 ORDER BY al.year, al.title COLLATE NOCASE");
    let mut stmt = db.conn().prepare(&sql)?;
    let albums = stmt
        .query_map([id.0], album_from_row)?
        .collect::<rusqlite::Result<_>>()?;
    Ok(albums)
}

pub fn tracks(db: &Db, sort: TrackSort, offset: u32, limit: u32) -> Result<Vec<Track>> {
    let order = match sort {
        TrackSort::Title => "t.title COLLATE NOCASE, t.id",
        TrackSort::Artist => "artists_joined COLLATE NOCASE, al.title COLLATE NOCASE, t.track_no, t.id",
        TrackSort::Album => "al.title COLLATE NOCASE, coalesce(t.disc_no, 1), t.track_no, t.id",
        TrackSort::RecentlyAdded => "t.id DESC",
    };
    let sql = format!("{TRACK_SELECT} ORDER BY {order} LIMIT ?1 OFFSET ?2");
    let mut stmt = db.conn().prepare(&sql)?;
    let tracks = stmt
        .query_map(params![limit, offset], track_from_row)?
        .collect::<rusqlite::Result<_>>()?;
    Ok(tracks)
}

pub fn track(db: &Db, id: TrackId) -> Result<Track> {
    let sql = format!("{TRACK_SELECT} WHERE t.id = ?1");
    db.conn()
        .query_row(&sql, [id.0], track_from_row)
        .optional()?
        .ok_or(LibraryError::NotFound {
            kind: "track",
            id: id.0,
        })
}

pub fn track_count(db: &Db) -> Result<u64> {
    Ok(db
        .conn()
        .query_row("SELECT count(*) FROM tracks", [], |r| r.get::<_, i64>(0))? as u64)
}
