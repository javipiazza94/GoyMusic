//! One typed Tauri command per service call. Thin on purpose: all behaviour lives in
//! `goymusic-services`, so a future server adapter exposes the same thing.

use std::path::Path;
use std::sync::Arc;

use goymusic_core::model::{Album, Artist, Folder, Playlist, PlaylistFolder, Track};
use goymusic_core::queue::RepeatMode;
use goymusic_core::{AlbumId, ArtistId, FolderId, PlaylistFolderId, PlaylistId, TrackId};
use goymusic_library::m3u::ImportReport;
use goymusic_library::queries::TrackSort;
use goymusic_library::search::SearchResults;
use goymusic_services::{
    AlbumDetail, ArtistDetail, PlayContext, PlayerView, PlaylistDetail, ServiceError, Services, Settings,
};
use tauri::State;

type Svc<'a> = State<'a, Arc<Services>>;
type Res<T> = Result<T, ServiceError>;

// ---- library ----

#[tauri::command]
#[specta::specta]
pub async fn folders(s: Svc<'_>) -> Res<Vec<Folder>> {
    s.folders()
}

#[tauri::command]
#[specta::specta]
pub async fn add_folder(s: Svc<'_>, path: String) -> Res<Folder> {
    s.add_folder(Path::new(&path))
}

#[tauri::command]
#[specta::specta]
pub async fn remove_folder(s: Svc<'_>, id: FolderId) -> Res<()> {
    s.remove_folder(id)
}

#[tauri::command]
#[specta::specta]
pub async fn rescan(s: Svc<'_>) -> Res<()> {
    s.rescan();
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn albums(s: Svc<'_>) -> Res<Vec<Album>> {
    s.albums()
}

#[tauri::command]
#[specta::specta]
pub async fn album(s: Svc<'_>, id: AlbumId) -> Res<AlbumDetail> {
    s.album(id)
}

#[tauri::command]
#[specta::specta]
pub async fn artists(s: Svc<'_>) -> Res<Vec<Artist>> {
    s.artists()
}

#[tauri::command]
#[specta::specta]
pub async fn artist(s: Svc<'_>, id: ArtistId) -> Res<ArtistDetail> {
    s.artist(id)
}

#[tauri::command]
#[specta::specta]
pub async fn tracks(s: Svc<'_>, sort: TrackSort, offset: u32, limit: u32) -> Res<Vec<Track>> {
    s.tracks(sort, offset, limit)
}

#[tauri::command]
#[specta::specta]
pub async fn track(s: Svc<'_>, id: TrackId) -> Res<Track> {
    s.track(id)
}

#[tauri::command]
#[specta::specta]
pub async fn recent_albums(s: Svc<'_>, limit: u32) -> Res<Vec<Album>> {
    s.recent_albums(limit)
}

#[tauri::command]
#[specta::specta]
pub async fn search(s: Svc<'_>, query: String, limit: u32) -> Res<SearchResults> {
    s.search(&query, limit)
}

// ---- playlists ----

#[tauri::command]
#[specta::specta]
pub async fn playlists(s: Svc<'_>) -> Res<Vec<Playlist>> {
    s.playlists()
}

#[tauri::command]
#[specta::specta]
pub async fn playlist_folders(s: Svc<'_>) -> Res<Vec<PlaylistFolder>> {
    s.playlist_folders()
}

#[tauri::command]
#[specta::specta]
pub async fn playlist(s: Svc<'_>, id: PlaylistId) -> Res<PlaylistDetail> {
    s.playlist(id)
}

#[tauri::command]
#[specta::specta]
pub async fn create_playlist(s: Svc<'_>, name: String) -> Res<Playlist> {
    s.create_playlist(&name)
}

#[tauri::command]
#[specta::specta]
pub async fn rename_playlist(s: Svc<'_>, id: PlaylistId, name: String) -> Res<()> {
    s.rename_playlist(id, &name)
}

#[tauri::command]
#[specta::specta]
pub async fn delete_playlist(s: Svc<'_>, id: PlaylistId) -> Res<()> {
    s.delete_playlist(id)
}

#[tauri::command]
#[specta::specta]
pub async fn add_to_playlist(s: Svc<'_>, id: PlaylistId, tracks: Vec<TrackId>) -> Res<()> {
    s.add_to_playlist(id, &tracks)
}

#[tauri::command]
#[specta::specta]
pub async fn remove_from_playlist(s: Svc<'_>, id: PlaylistId, positions: Vec<u32>) -> Res<()> {
    s.remove_from_playlist(id, &positions)
}

#[tauri::command]
#[specta::specta]
pub async fn move_playlist_item(s: Svc<'_>, id: PlaylistId, from: u32, to: u32) -> Res<()> {
    s.move_playlist_item(id, from, to)
}

#[tauri::command]
#[specta::specta]
pub async fn create_playlist_folder(s: Svc<'_>, name: String, parent: Option<PlaylistFolderId>) -> Res<PlaylistFolder> {
    s.create_playlist_folder(&name, parent)
}

#[tauri::command]
#[specta::specta]
pub async fn move_playlist_to_folder(s: Svc<'_>, id: PlaylistId, folder: Option<PlaylistFolderId>) -> Res<()> {
    s.move_playlist_to_folder(id, folder)
}

#[tauri::command]
#[specta::specta]
pub async fn like(s: Svc<'_>, track: TrackId) -> Res<()> {
    s.like(track)
}

#[tauri::command]
#[specta::specta]
pub async fn unlike(s: Svc<'_>, track: TrackId) -> Res<()> {
    s.unlike(track)
}

#[tauri::command]
#[specta::specta]
pub async fn liked_tracks(s: Svc<'_>) -> Res<Vec<Track>> {
    s.liked_tracks()
}

#[tauri::command]
#[specta::specta]
pub async fn liked_ids(s: Svc<'_>) -> Res<Vec<TrackId>> {
    s.liked_ids()
}

#[tauri::command]
#[specta::specta]
pub async fn import_m3u(s: Svc<'_>, path: String) -> Res<ImportReport> {
    s.import_m3u(Path::new(&path))
}

#[tauri::command]
#[specta::specta]
pub async fn export_m3u(s: Svc<'_>, id: PlaylistId, path: String) -> Res<()> {
    s.export_m3u(id, Path::new(&path))
}

// ---- player ----

#[tauri::command]
#[specta::specta]
pub async fn play(s: Svc<'_>, context: PlayContext, start_index: u32) -> Res<()> {
    s.play(context, start_index)
}

#[tauri::command]
#[specta::specta]
pub async fn pause(s: Svc<'_>) -> Res<()> {
    s.pause();
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn resume(s: Svc<'_>) -> Res<()> {
    s.resume();
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn next(s: Svc<'_>) -> Res<()> {
    s.next();
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn previous(s: Svc<'_>) -> Res<()> {
    s.previous();
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn seek(s: Svc<'_>, position_ms: u64) -> Res<()> {
    s.seek(position_ms);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn set_shuffle(s: Svc<'_>, on: bool) -> Res<()> {
    s.set_shuffle(on);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn set_repeat(s: Svc<'_>, mode: RepeatMode) -> Res<()> {
    s.set_repeat(mode);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn set_volume(s: Svc<'_>, volume: f32) -> Res<()> {
    s.set_volume(volume);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn set_replaygain(s: Svc<'_>, on: bool) -> Res<()> {
    s.set_replaygain(on);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn queue_add(s: Svc<'_>, tracks: Vec<TrackId>) -> Res<()> {
    s.queue_add(tracks);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn queue_remove(s: Svc<'_>, index: u32) -> Res<()> {
    s.queue_remove(index);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn queue_move(s: Svc<'_>, from: u32, to: u32) -> Res<()> {
    s.queue_move(from, to);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn player_view(s: Svc<'_>) -> Res<PlayerView> {
    s.player_view()
}

#[tauri::command]
#[specta::specta]
pub async fn settings(s: Svc<'_>) -> Res<Settings> {
    s.settings()
}

// ---- windows ----

#[tauri::command]
#[specta::specta]
pub async fn show_mini_player(app: tauri::AppHandle) {
    crate::window::show_mini(&app);
}

#[tauri::command]
#[specta::specta]
pub async fn hide_mini_player(app: tauri::AppHandle) {
    crate::window::hide_mini(&app);
}

#[tauri::command]
#[specta::specta]
pub async fn set_mini_on_top(app: tauri::AppHandle, on: bool) {
    crate::window::set_mini_on_top(&app, on);
}

#[tauri::command]
#[specta::specta]
pub async fn show_main_window(app: tauri::AppHandle) {
    crate::window::show_main(&app);
}
