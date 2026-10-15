//! Instant search: FTS5 prefix matching, plus a Jaro-Winkler fallback for typos when FTS
//! finds little.

use std::collections::HashSet;

use deunicode::deunicode;
use goymusic_core::model::{Album, Artist, Track};
use goymusic_core::{AlbumId, ArtistId, TrackId};
use rusqlite::params;
use strsim::jaro_winkler;

use crate::db::Db;
use crate::error::Result;
use crate::queries::{self, track_from_row, TRACK_SELECT};

pub const FUZZY_THRESHOLD: f64 = 0.85;
pub const FUZZY_BELOW: usize = 5;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SearchResults {
    pub top: Option<TopResult>,
    pub tracks: Vec<Track>,
    pub albums: Vec<Album>,
    pub artists: Vec<Artist>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum TopResult {
    Artist(Artist),
    Album(Album),
    Track(Track),
}

pub fn search(db: &Db, query: &str, limit: usize) -> Result<SearchResults> {
    let tokens = tokenize(query);
    if tokens.is_empty() {
        return Ok(SearchResults::default());
    }
    let any_column = fts_query(&tokens);
    let mut results = SearchResults {
        tracks: fts_tracks(db, &any_column, limit)?,
        albums: fts_albums(db, &format!("album : ({any_column})"), limit)?,
        artists: fts_artists(db, &format!("artists : ({any_column})"), &tokens, limit)?,
        top: None,
    };
    if results.tracks.len() + results.albums.len() + results.artists.len() < FUZZY_BELOW {
        fuzzy_fill(db, &normalize(query), limit, &mut results)?;
    }
    results.top = pick_top(&tokens.join(" "), &results);
    Ok(results)
}

/// Lowercase, accent-free words. Only alphanumerics survive, so FTS5 syntax
/// (quotes, `*`, `-`, parentheses, operators) can never reach the query engine.
fn tokenize(query: &str) -> Vec<String> {
    normalize(query)
        .split(|c: char| !c.is_alphanumeric())
        .filter(|t| !t.is_empty())
        .map(str::to_owned)
        .collect()
}

fn normalize(text: &str) -> String {
    deunicode(text).to_lowercase()
}

/// Every token as a quoted prefix phrase: `"radio"* "head"*` (implicit AND).
fn fts_query(tokens: &[String]) -> String {
    tokens.iter().map(|t| format!("\"{t}\"*")).collect::<Vec<_>>().join(" ")
}

fn fts_tracks(db: &Db, fts: &str, limit: usize) -> Result<Vec<Track>> {
    let sql = format!(
        "{TRACK_SELECT} JOIN tracks_fts ON tracks_fts.rowid = t.id WHERE tracks_fts MATCH ?1 ORDER BY tracks_fts.rank LIMIT ?2"
    );
    let mut stmt = db.conn().prepare(&sql)?;
    let tracks = stmt
        .query_map(params![fts, limit as i64], track_from_row)?
        .collect::<rusqlite::Result<_>>()?;
    Ok(tracks)
}

fn fts_albums(db: &Db, fts: &str, limit: usize) -> Result<Vec<Album>> {
    let mut stmt = db.conn().prepare(
        "SELECT DISTINCT t.album_id FROM tracks_fts JOIN tracks t ON t.id = tracks_fts.rowid
          WHERE tracks_fts MATCH ?1 AND t.album_id IS NOT NULL LIMIT ?2",
    )?;
    let ids: Vec<i64> = stmt
        .query_map(params![fts, limit as i64], |r| r.get(0))?
        .collect::<rusqlite::Result<_>>()?;
    ids.into_iter().map(|id| queries::album(db, AlbumId(id))).collect()
}

/// Artists of matching tracks whose own name matches (a track also lists its featured artists).
fn fts_artists(db: &Db, fts: &str, tokens: &[String], limit: usize) -> Result<Vec<Artist>> {
    let mut stmt = db.conn().prepare(
        "SELECT DISTINCT a.id, a.name FROM tracks_fts
           JOIN track_artists ta ON ta.track_id = tracks_fts.rowid
           JOIN artists a ON a.id = ta.artist_id
          WHERE tracks_fts MATCH ?1",
    )?;
    let all: Vec<Artist> = stmt
        .query_map([fts], |r| {
            Ok(Artist {
                id: ArtistId(r.get(0)?),
                name: r.get(1)?,
            })
        })?
        .collect::<rusqlite::Result<_>>()?;
    Ok(all
        .into_iter()
        .filter(|a| {
            let words = tokenize(&a.name);
            tokens.iter().all(|t| words.iter().any(|w| w.starts_with(t.as_str())))
        })
        .take(limit)
        .collect())
}

fn fuzzy_fill(db: &Db, query: &str, limit: usize, results: &mut SearchResults) -> Result<()> {
    let artists = fuzzy_names(db, "SELECT id, name FROM artists", query)?;
    let known: HashSet<ArtistId> = results.artists.iter().map(|a| a.id).collect();
    for (id, name) in artists {
        if results.artists.len() >= limit {
            break;
        }
        if !known.contains(&ArtistId(id)) {
            results.artists.push(Artist { id: ArtistId(id), name });
        }
    }

    let albums = fuzzy_names(db, "SELECT id, title FROM albums", query)?;
    let known: HashSet<AlbumId> = results.albums.iter().map(|a| a.id).collect();
    for (id, _) in albums {
        if results.albums.len() >= limit {
            break;
        }
        if !known.contains(&AlbumId(id)) {
            results.albums.push(queries::album(db, AlbumId(id))?);
        }
    }

    let tracks = fuzzy_names(db, "SELECT id, title FROM tracks", query)?;
    let known: HashSet<TrackId> = results.tracks.iter().map(|t| t.id).collect();
    for (id, _) in tracks {
        if results.tracks.len() >= limit {
            break;
        }
        if !known.contains(&TrackId(id)) {
            results.tracks.push(queries::track(db, TrackId(id))?);
        }
    }
    Ok(())
}

/// `(id, name)` rows whose normalized name scores ≥ FUZZY_THRESHOLD, best first.
fn fuzzy_names(db: &Db, sql: &str, query: &str) -> Result<Vec<(i64, String)>> {
    let mut stmt = db.conn().prepare(sql)?;
    let rows: Vec<(i64, String)> = stmt
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect::<rusqlite::Result<_>>()?;
    let mut scored: Vec<(f64, i64, String)> = rows
        .into_iter()
        .filter_map(|(id, name)| {
            let score = jaro_winkler(query, &normalize(&name));
            (score >= FUZZY_THRESHOLD).then_some((score, id, name))
        })
        .collect();
    scored.sort_by(|a, b| b.0.total_cmp(&a.0));
    Ok(scored.into_iter().map(|(_, id, name)| (id, name)).collect())
}

fn pick_top(query: &str, results: &SearchResults) -> Option<TopResult> {
    if let Some(artist) = results.artists.iter().find(|a| normalize(&a.name).starts_with(query)) {
        return Some(TopResult::Artist(artist.clone()));
    }
    results
        .albums
        .first()
        .map(|a| TopResult::Album(a.clone()))
        .or_else(|| results.tracks.first().map(|t| TopResult::Track(t.clone())))
        .or_else(|| results.artists.first().map(|a| TopResult::Artist(a.clone())))
}
