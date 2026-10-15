use std::path::Path;

use lofty::prelude::*;
use lofty::tag::{ItemKey, Tag};

use crate::error::{LibraryError, Result};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct TrackTags {
    pub title: Option<String>,
    pub artists: Vec<String>,
    pub album_artist: Option<String>,
    pub album: Option<String>,
    pub track_no: Option<u32>,
    pub disc_no: Option<u32>,
    pub year: Option<i32>,
    pub genre: Option<String>,
    pub duration_ms: u64,
    pub mb_recording_id: Option<String>,
    pub mb_release_id: Option<String>,
}

pub fn read_tags(path: &Path) -> Result<TrackTags> {
    let tagged = lofty::read_from_path(path).map_err(|e| LibraryError::Tags {
        path: path.display().to_string(),
        message: e.to_string(),
    })?;
    let duration_ms = tagged.properties().duration().as_millis() as u64;
    let Some(tag) = tagged.primary_tag().or_else(|| tagged.first_tag()) else {
        return Ok(TrackTags {
            duration_ms,
            ..TrackTags::default()
        });
    };
    let mut artists: Vec<String> = Vec::new();
    for artist in tag
        .get_strings(&ItemKey::TrackArtist)
        .map(str::trim)
        .filter(|a| !a.is_empty())
    {
        if !artists.iter().any(|known| known.eq_ignore_ascii_case(artist)) {
            artists.push(artist.to_owned());
        }
    }
    Ok(TrackTags {
        title: text(tag, &ItemKey::TrackTitle),
        artists,
        album_artist: text(tag, &ItemKey::AlbumArtist),
        album: text(tag, &ItemKey::AlbumTitle),
        track_no: number(tag, &ItemKey::TrackNumber),
        disc_no: number(tag, &ItemKey::DiscNumber),
        year: text(tag, &ItemKey::Year)
            .or_else(|| text(tag, &ItemKey::RecordingDate))
            .and_then(|d| leading_year(&d)),
        genre: text(tag, &ItemKey::Genre),
        duration_ms,
        mb_recording_id: text(tag, &ItemKey::MusicBrainzRecordingId),
        mb_release_id: text(tag, &ItemKey::MusicBrainzReleaseId),
    })
}

fn text(tag: &Tag, key: &ItemKey) -> Option<String> {
    tag.get_string(key)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
}

/// "3" or "3/12" → 3.
fn number(tag: &Tag, key: &ItemKey) -> Option<u32> {
    text(tag, key)?.split('/').next()?.trim().parse().ok()
}

/// "1997", "1997-05-21" → 1997.
fn leading_year(date: &str) -> Option<i32> {
    date.get(..4)?.parse().ok()
}
