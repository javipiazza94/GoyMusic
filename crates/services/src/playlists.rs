use std::path::Path;

use goymusic_core::events::Event;
use goymusic_core::model::{Playlist, PlaylistFolder, PlaylistItem, Track};
use goymusic_core::{PlaylistFolderId, PlaylistId, TrackId};
use goymusic_library::m3u::{self, ImportReport};
use goymusic_library::playlists;
use serde::Serialize;

use crate::error::{Result, ServiceError};
use crate::Services;

#[derive(Clone, Debug, PartialEq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct PlaylistDetail {
    pub playlist: Playlist,
    pub items: Vec<PlaylistItem>,
}

impl Services {
    fn playlists_changed<T>(&self, result: T) -> Result<T> {
        self.events.publish(Event::PlaylistsChanged);
        Ok(result)
    }

    fn likes_changed(&self) -> Result<()> {
        self.events.publish(Event::LikesChanged);
        Ok(())
    }

    pub fn playlists(&self) -> Result<Vec<Playlist>> {
        Ok(playlists::list(&self.db())?)
    }

    pub fn playlist_folders(&self) -> Result<Vec<PlaylistFolder>> {
        Ok(playlists::folders(&self.db())?)
    }

    pub fn playlist(&self, id: PlaylistId) -> Result<PlaylistDetail> {
        let db = self.db();
        Ok(PlaylistDetail {
            playlist: playlists::get(&db, id)?,
            items: playlists::items(&db, id)?,
        })
    }

    pub fn create_playlist(&self, name: &str) -> Result<Playlist> {
        let created = playlists::create(&self.db(), name)?;
        self.playlists_changed(created)
    }

    pub fn rename_playlist(&self, id: PlaylistId, name: &str) -> Result<()> {
        playlists::rename(&self.db(), id, name)?;
        self.playlists_changed(())
    }

    pub fn delete_playlist(&self, id: PlaylistId) -> Result<()> {
        playlists::delete(&self.db(), id)?;
        self.playlists_changed(())
    }

    pub fn add_to_playlist(&self, id: PlaylistId, tracks: &[TrackId]) -> Result<()> {
        playlists::add_tracks(&mut self.db(), id, tracks)?;
        self.playlists_changed(())
    }

    pub fn remove_from_playlist(&self, id: PlaylistId, positions: &[u32]) -> Result<()> {
        playlists::remove_items(&mut self.db(), id, positions)?;
        self.playlists_changed(())
    }

    pub fn move_playlist_item(&self, id: PlaylistId, from: u32, to: u32) -> Result<()> {
        playlists::move_item(&mut self.db(), id, from, to)?;
        self.playlists_changed(())
    }

    pub fn create_playlist_folder(&self, name: &str, parent: Option<PlaylistFolderId>) -> Result<PlaylistFolder> {
        let folder = playlists::create_folder(&self.db(), name, parent)?;
        self.playlists_changed(folder)
    }

    pub fn move_playlist_to_folder(&self, id: PlaylistId, folder: Option<PlaylistFolderId>) -> Result<()> {
        playlists::move_to_folder(&self.db(), id, folder)?;
        self.playlists_changed(())
    }

    pub fn like(&self, track: TrackId) -> Result<()> {
        playlists::like(&self.db(), track)?;
        self.likes_changed()
    }

    pub fn unlike(&self, track: TrackId) -> Result<()> {
        playlists::unlike(&self.db(), track)?;
        self.likes_changed()
    }

    pub fn liked_tracks(&self) -> Result<Vec<Track>> {
        Ok(playlists::liked_tracks(&self.db())?)
    }

    pub fn liked_ids(&self) -> Result<Vec<TrackId>> {
        Ok(self.liked_tracks()?.into_iter().map(|t| t.id).collect())
    }

    /// Imports an M3U/M3U8 file as a new playlist named after the file.
    pub fn import_m3u(&self, path: &Path) -> Result<ImportReport> {
        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .filter(|s| !s.is_empty())
            .ok_or_else(|| ServiceError::invalid(format!("not a playlist file: {}", path.display())))?;
        let report = m3u::import(&mut self.db(), name, path)?;
        self.playlists_changed(report)
    }

    pub fn export_m3u(&self, id: PlaylistId, path: &Path) -> Result<()> {
        Ok(m3u::export(&self.db(), id, path)?)
    }
}
