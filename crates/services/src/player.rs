//! Player service: resolves play contexts against the library, exposes a view for the UI and
//! persists the user's player settings.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use goymusic_audio::engine::{PlayerSnapshot, Resolver};
use goymusic_core::events::PlaybackState;
use goymusic_core::model::Track;
use goymusic_core::queue::RepeatMode;
use goymusic_core::{AlbumId, ArtistId, PlaylistId, TrackId};
use goymusic_library::db::Db;
use goymusic_library::{playlists, queries, state};
use serde::{Deserialize, Serialize};

use crate::error::{ErrorCode, Result, ServiceError};
use crate::watch::WatchStatus;
use crate::Services;

/// Context-track limit of [`PlayerView::up_next`].
pub const UP_NEXT_LIMIT: usize = 100;
pub(crate) const VOLUME_KEY: &str = "player.volume";
pub(crate) const REPLAYGAIN_KEY: &str = "player.replaygain";

/// The list playback comes from (spec §4).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(tag = "kind", content = "id", rename_all = "camelCase")]
pub enum PlayContext {
    Album(AlbumId),
    Artist(ArtistId),
    Playlist(PlaylistId),
    Liked,
    Tracks(Vec<TrackId>),
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct PlayerView {
    pub state: PlaybackState,
    pub track: Option<Track>,
    pub position_ms: u64,
    pub volume: f32,
    pub replaygain: bool,
    pub shuffle: bool,
    pub repeat: RepeatMode,
    /// Indices match `queue_remove` / `queue_move`.
    pub user_queue: Vec<Track>,
    /// Rest of the context in play order, at most [`UP_NEXT_LIMIT`].
    pub up_next: Vec<Track>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub replaygain: bool,
    pub watch: WatchStatus,
}

/// Track id → file, answered from the audio thread's own connection so a long scan or UI
/// query never delays the start of the next track.
pub(crate) fn resolver(db_path: &Path) -> Result<Resolver> {
    let db = Mutex::new(Db::open(db_path)?);
    Ok(Box::new(move |id| {
        let db = db.lock().ok()?;
        queries::track(&db, id).ok().map(|t| PathBuf::from(t.path))
    }))
}

pub(crate) fn saved_volume(db: &Db) -> Result<f32> {
    Ok(state::setting(db, VOLUME_KEY)?
        .and_then(|v| v.parse().ok())
        .unwrap_or(1.0))
}

pub(crate) fn saved_replaygain(db: &Db) -> Result<bool> {
    Ok(state::setting(db, REPLAYGAIN_KEY)?.as_deref() == Some("true"))
}

fn player_gone() -> ServiceError {
    ServiceError::new(ErrorCode::Internal, "the audio thread is not running")
}

impl Services {
    pub fn play(&self, context: PlayContext, start_index: u32) -> Result<()> {
        let tracks = self.resolve(context)?;
        if tracks.is_empty() {
            return Err(ServiceError::invalid("nothing to play"));
        }
        let index = start_index as usize;
        if index >= tracks.len() {
            return Err(ServiceError::invalid(format!(
                "start index {index} out of range ({} tracks)",
                tracks.len()
            )));
        }
        self.with_player(|p| p.play(tracks, index));
        Ok(())
    }

    fn resolve(&self, context: PlayContext) -> Result<Vec<TrackId>> {
        let db = self.db();
        let ids = |tracks: Vec<Track>| tracks.into_iter().map(|t| t.id).collect();
        Ok(match context {
            PlayContext::Album(id) => ids(queries::album_tracks(&db, id)?),
            PlayContext::Artist(id) => ids(queries::artist_tracks(&db, id)?),
            PlayContext::Playlist(id) => playlists::items(&db, id)?.into_iter().map(|i| i.track.id).collect(),
            PlayContext::Liked => ids(playlists::liked_tracks(&db)?),
            PlayContext::Tracks(tracks) => tracks,
        })
    }

    pub fn pause(&self) {
        self.with_player(|p| p.pause());
    }

    pub fn resume(&self) {
        self.with_player(|p| p.resume());
    }

    pub fn next(&self) {
        self.with_player(|p| p.next());
    }

    pub fn previous(&self) {
        self.with_player(|p| p.previous());
    }

    pub fn seek(&self, position_ms: u64) {
        self.with_player(|p| p.seek(position_ms));
    }

    pub fn set_shuffle(&self, on: bool) {
        self.with_player(|p| p.set_shuffle(on));
    }

    pub fn set_repeat(&self, mode: RepeatMode) {
        self.with_player(|p| p.set_repeat(mode));
    }

    /// 0.0–1.0, heard at once and remembered across restarts.
    pub fn set_volume(&self, volume: f32) {
        let volume = volume.clamp(0.0, 1.0);
        self.with_player(|p| p.set_volume(volume));
        if let Err(e) = state::set_setting(&self.db(), VOLUME_KEY, &volume.to_string()) {
            tracing::warn!(error = %e, "could not save the volume");
        }
    }

    pub fn set_replaygain(&self, on: bool) {
        self.with_player(|p| p.set_replaygain(on));
        if let Err(e) = state::set_setting(&self.db(), REPLAYGAIN_KEY, if on { "true" } else { "false" }) {
            tracing::warn!(error = %e, "could not save the ReplayGain setting");
        }
    }

    pub fn queue_add(&self, tracks: Vec<TrackId>) {
        self.with_player(|p| p.queue_add(tracks));
    }

    pub fn queue_remove(&self, index: u32) {
        self.with_player(|p| p.queue_remove(index as usize));
    }

    pub fn queue_move(&self, from: u32, to: u32) {
        self.with_player(|p| p.queue_move(from as usize, to as usize));
    }

    pub(crate) fn snapshot(&self) -> Option<PlayerSnapshot> {
        self.with_player(|p| p.state()).flatten()
    }

    pub fn player_view(&self) -> Result<PlayerView> {
        let snapshot = self.snapshot().ok_or_else(player_gone)?;
        let db = self.db();
        // tracks that left the library since they were queued are skipped
        let tracks = |ids: Vec<TrackId>| -> Vec<Track> {
            ids.into_iter().filter_map(|id| queries::track(&db, id).ok()).collect()
        };
        let queue = &snapshot.queue;
        let user = queue.user_queue();
        let mut upcoming = queue.upcoming(user.len() + UP_NEXT_LIMIT);
        let up_next = upcoming.split_off(user.len().min(upcoming.len()));
        Ok(PlayerView {
            state: snapshot.state,
            track: snapshot.track.and_then(|id| queries::track(&db, id).ok()),
            position_ms: snapshot.position_ms,
            volume: snapshot.volume,
            replaygain: snapshot.replaygain,
            shuffle: queue.shuffle(),
            repeat: queue.repeat(),
            user_queue: tracks(user),
            up_next: tracks(up_next),
        })
    }

    pub fn settings(&self) -> Result<Settings> {
        Ok(Settings {
            replaygain: saved_replaygain(&self.db())?,
            watch: self.watch_status(),
        })
    }
}
