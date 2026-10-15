mod common;

use common::{write_tagged_wav, Meta};
use goymusic_core::events::EventBus;
use goymusic_core::{PlaylistId, TrackId};
use goymusic_library::db::Db;
use goymusic_library::error::LibraryError;
use goymusic_library::queries::{self, TrackSort};
use goymusic_library::{folders, playlists, scanner};

/// Three scanned tracks; returns their ids sorted by title: A, B, C.
fn library() -> (tempfile::TempDir, Db, Vec<TrackId>) {
    let dir = tempfile::tempdir().unwrap();
    for title in ["A", "B", "C"] {
        write_tagged_wav(&dir.path().join(format!("{title}.wav")), 0.2, &Meta::new(title, "X"));
    }
    let mut db = Db::open_in_memory().unwrap();
    folders::add(&db, dir.path()).unwrap();
    scanner::scan(&mut db, &EventBus::new()).unwrap();
    let ids = queries::tracks(&db, TrackSort::Title, 0, 10)
        .unwrap()
        .into_iter()
        .map(|t| t.id)
        .collect();
    (dir, db, ids)
}

fn titles(db: &Db, id: PlaylistId) -> Vec<String> {
    playlists::items(db, id)
        .unwrap()
        .into_iter()
        .map(|i| i.track.title)
        .collect()
}

#[test]
fn create_add_and_list() {
    let (_dir, mut db, t) = library();
    let p = playlists::create(&db, "Road trip").unwrap();
    playlists::add_tracks(&mut db, p.id, &[t[2], t[0]]).unwrap();
    assert_eq!(titles(&db, p.id), vec!["C", "A"]);
    let listed = playlists::list(&db).unwrap();
    assert_eq!((listed[0].name.as_str(), listed[0].track_count), ("Road trip", 2));
}

#[test]
fn duplicates_are_allowed() {
    let (_dir, mut db, t) = library();
    let p = playlists::create(&db, "Loop").unwrap();
    playlists::add_tracks(&mut db, p.id, &[t[0], t[0]]).unwrap();
    assert_eq!(titles(&db, p.id), vec!["A", "A"]);
}

#[test]
fn move_and_remove_items_keep_positions_dense() {
    let (_dir, mut db, t) = library();
    let p = playlists::create(&db, "Mix").unwrap();
    playlists::add_tracks(&mut db, p.id, &t).unwrap();
    playlists::move_item(&mut db, p.id, 0, 2).unwrap();
    assert_eq!(titles(&db, p.id), vec!["B", "C", "A"]);
    playlists::remove_items(&mut db, p.id, &[0, 2]).unwrap();
    assert_eq!(titles(&db, p.id), vec!["C"]);
    let positions: Vec<u32> = playlists::items(&db, p.id)
        .unwrap()
        .into_iter()
        .map(|i| i.position)
        .collect();
    assert_eq!(positions, vec![0]);
}

#[test]
fn out_of_range_positions_are_rejected() {
    let (_dir, mut db, t) = library();
    let p = playlists::create(&db, "Mix").unwrap();
    playlists::add_tracks(&mut db, p.id, &t[..1]).unwrap();
    assert!(matches!(
        playlists::move_item(&mut db, p.id, 0, 5),
        Err(LibraryError::InvalidPosition { position: 5, len: 1 })
    ));
    assert!(matches!(
        playlists::remove_items(&mut db, p.id, &[3]),
        Err(LibraryError::InvalidPosition { position: 3, len: 1 })
    ));
    assert_eq!(titles(&db, p.id), vec!["A"]);
}

#[test]
fn unknown_playlist_or_track_fails_cleanly() {
    let (_dir, mut db, _t) = library();
    assert!(matches!(
        playlists::get(&db, PlaylistId(77)),
        Err(LibraryError::NotFound {
            kind: "playlist",
            id: 77
        })
    ));
    assert!(matches!(
        playlists::add_tracks(&mut db, PlaylistId(77), &[TrackId(1)]),
        Err(LibraryError::NotFound { .. })
    ));
    let p = playlists::create(&db, "P").unwrap();
    assert!(matches!(
        playlists::add_tracks(&mut db, p.id, &[TrackId(9999)]),
        Err(LibraryError::Db(_))
    ));
}

#[test]
fn rename_delete_and_folders() {
    let (_dir, db, _t) = library();
    let p = playlists::create(&db, "Old").unwrap();
    playlists::rename(&db, p.id, "New").unwrap();
    let folder = playlists::create_folder(&db, "Moods", None).unwrap();
    playlists::move_to_folder(&db, p.id, Some(folder.id)).unwrap();
    let got = playlists::get(&db, p.id).unwrap();
    assert_eq!((got.name.as_str(), got.folder_id), ("New", Some(folder.id)));
    assert_eq!(playlists::folders(&db).unwrap(), vec![folder]);
    playlists::delete(&db, p.id).unwrap();
    assert!(playlists::list(&db).unwrap().is_empty());
}

#[test]
fn likes_are_idempotent_and_newest_first() {
    let (_dir, db, t) = library();
    playlists::like(&db, t[0]).unwrap();
    playlists::like(&db, t[2]).unwrap();
    playlists::like(&db, t[0]).unwrap();
    let liked: Vec<_> = playlists::liked_tracks(&db)
        .unwrap()
        .into_iter()
        .map(|t| t.title)
        .collect();
    assert_eq!(liked, vec!["C", "A"]);
    playlists::unlike(&db, t[2]).unwrap();
    assert!(!playlists::is_liked(&db, t[2]).unwrap());
    assert!(playlists::is_liked(&db, t[0]).unwrap());
}
