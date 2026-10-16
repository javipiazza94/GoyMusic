use std::path::{Path, PathBuf};

use goymusic_core::events::Event;
use goymusic_core::model::{Album, Artist, Folder, Track};
use goymusic_core::{AlbumId, ArtistId, FolderId, TrackId};
use goymusic_library::art::{self, ArtSize};
use goymusic_library::queries::{self, TrackSort};
use goymusic_library::search::{self, SearchResults};
use goymusic_library::{folders, history};
use serde::Serialize;

use crate::error::Result;
use crate::Services;

#[derive(Clone, Debug, PartialEq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct AlbumDetail {
    pub album: Album,
    pub tracks: Vec<Track>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct ArtistDetail {
    pub artist: Artist,
    pub albums: Vec<Album>,
    pub tracks: Vec<Track>,
}

impl Services {
    pub fn folders(&self) -> Result<Vec<Folder>> {
        Ok(folders::list(&self.db())?)
    }

    /// Adds a library folder and scans it in the background.
    pub fn add_folder(&self, path: &Path) -> Result<Folder> {
        let folder = folders::add(&self.db(), path)?;
        self.watcher.watch(Path::new(&folder.path));
        self.scans.request();
        Ok(folder)
    }

    pub fn remove_folder(&self, id: FolderId) -> Result<()> {
        let path = folders::list(&self.db())?
            .into_iter()
            .find(|f| f.id == id)
            .map(|f| f.path);
        folders::remove(&mut self.db(), id)?;
        if let Some(path) = path {
            self.watcher.unwatch(Path::new(&path));
        }
        self.events.publish(Event::LibraryUpdated);
        Ok(())
    }

    pub fn rescan(&self) {
        self.scans.request();
    }

    pub fn albums(&self) -> Result<Vec<Album>> {
        Ok(queries::albums(&self.db())?)
    }

    pub fn album(&self, id: AlbumId) -> Result<AlbumDetail> {
        let db = self.db();
        Ok(AlbumDetail {
            album: queries::album(&db, id)?,
            tracks: queries::album_tracks(&db, id)?,
        })
    }

    pub fn artists(&self) -> Result<Vec<Artist>> {
        Ok(queries::artists(&self.db())?)
    }

    pub fn artist(&self, id: ArtistId) -> Result<ArtistDetail> {
        let db = self.db();
        Ok(ArtistDetail {
            artist: queries::artist(&db, id)?,
            albums: queries::artist_albums(&db, id)?,
            tracks: queries::artist_tracks(&db, id)?,
        })
    }

    pub fn tracks(&self, sort: TrackSort, offset: u32, limit: u32) -> Result<Vec<Track>> {
        Ok(queries::tracks(&self.db(), sort, offset, limit)?)
    }

    pub fn track(&self, id: TrackId) -> Result<Track> {
        Ok(queries::track(&self.db(), id)?)
    }

    pub fn recent_albums(&self, limit: u32) -> Result<Vec<Album>> {
        Ok(history::recent_albums(&self.db(), limit)?)
    }

    pub fn search(&self, query: &str, limit: u32) -> Result<SearchResults> {
        Ok(search::search(&self.db(), query, limit as usize)?)
    }

    /// Cached thumbnail of the album cover; generated on its own connection so a slow
    /// resize never holds up UI commands.
    pub fn album_art(&self, id: AlbumId, size: ArtSize) -> Result<Option<PathBuf>> {
        let db = self.art_db.lock().expect("art database poisoned");
        Ok(art::album_art(&db, id, size, &self.config.art_dir)?)
    }
}
