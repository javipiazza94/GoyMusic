use serde::{Deserialize, Serialize};

use crate::{AlbumId, ArtistId, FolderId, PlaylistFolderId, PlaylistId, TrackId};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Track {
    pub id: TrackId,
    pub path: String,
    pub title: String,
    pub artists: Vec<String>,
    pub album: Option<String>,
    pub album_id: Option<AlbumId>,
    pub album_artist: Option<String>,
    pub track_no: Option<u32>,
    pub disc_no: Option<u32>,
    pub year: Option<i32>,
    pub genre: Option<String>,
    pub duration_ms: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Album {
    pub id: AlbumId,
    pub title: String,
    pub artist: Option<String>,
    pub year: Option<i32>,
    pub track_count: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Artist {
    pub id: ArtistId,
    pub name: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Folder {
    pub id: FolderId,
    pub path: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Playlist {
    pub id: PlaylistId,
    pub name: String,
    pub folder_id: Option<PlaylistFolderId>,
    pub track_count: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlaylistFolder {
    pub id: PlaylistFolderId,
    pub name: String,
    pub parent_id: Option<PlaylistFolderId>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlaylistItem {
    pub position: u32,
    pub track: Track,
}
