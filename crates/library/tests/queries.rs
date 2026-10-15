mod common;

use common::{write_tagged_wav, Meta};
use goymusic_core::events::EventBus;
use goymusic_core::{AlbumId, TrackId};
use goymusic_library::db::Db;
use goymusic_library::error::LibraryError;
use goymusic_library::queries::{self, TrackSort};
use goymusic_library::{folders, scanner};

fn sample_library() -> (tempfile::TempDir, Db) {
    let dir = tempfile::tempdir().unwrap();
    let ok = |t, n| Meta {
        album: Some("OK Computer"),
        album_artist: Some("Radiohead"),
        track: Some(n),
        ..Meta::new(t, "Radiohead")
    };
    write_tagged_wav(&dir.path().join("ok/02.wav"), 0.3, &ok("Paranoid Android", 2));
    write_tagged_wav(&dir.path().join("ok/01.wav"), 0.3, &ok("Airbag", 1));
    write_tagged_wav(
        &dir.path().join("b/01.wav"),
        0.3,
        &Meta {
            album: Some("Blackstar"),
            ..Meta::new("Lazarus", "David Bowie")
        },
    );
    let mut db = Db::open_in_memory().unwrap();
    folders::add(&db, dir.path()).unwrap();
    scanner::scan(&mut db, &EventBus::new()).unwrap();
    (dir, db)
}

#[test]
fn albums_are_listed_with_artist_and_track_count() {
    let (_dir, db) = sample_library();
    let albums = queries::albums(&db).unwrap();
    let summary: Vec<_> = albums
        .iter()
        .map(|a| (a.title.as_str(), a.artist.as_deref(), a.track_count))
        .collect();
    assert_eq!(
        summary,
        vec![
            ("Blackstar", Some("David Bowie"), 1),
            ("OK Computer", Some("Radiohead"), 2)
        ]
    );
}

#[test]
fn album_tracks_follow_track_numbers() {
    let (_dir, db) = sample_library();
    let ok = queries::albums(&db)
        .unwrap()
        .into_iter()
        .find(|a| a.title == "OK Computer")
        .unwrap();
    let titles: Vec<_> = queries::album_tracks(&db, ok.id)
        .unwrap()
        .into_iter()
        .map(|t| t.title)
        .collect();
    assert_eq!(titles, vec!["Airbag", "Paranoid Android"]);
}

#[test]
fn artists_and_their_albums() {
    let (_dir, db) = sample_library();
    let artists = queries::artists(&db).unwrap();
    assert_eq!(
        artists.iter().map(|a| a.name.as_str()).collect::<Vec<_>>(),
        vec!["David Bowie", "Radiohead"]
    );
    let albums = queries::artist_albums(&db, artists[1].id).unwrap();
    assert_eq!(
        albums.iter().map(|a| a.title.as_str()).collect::<Vec<_>>(),
        vec!["OK Computer"]
    );
}

#[test]
fn tracks_are_sorted_and_paged() {
    let (_dir, db) = sample_library();
    let by_title: Vec<_> = queries::tracks(&db, TrackSort::Title, 0, 10)
        .unwrap()
        .into_iter()
        .map(|t| t.title)
        .collect();
    assert_eq!(by_title, vec!["Airbag", "Lazarus", "Paranoid Android"]);
    let page: Vec<_> = queries::tracks(&db, TrackSort::Title, 1, 1)
        .unwrap()
        .into_iter()
        .map(|t| t.title)
        .collect();
    assert_eq!(page, vec!["Lazarus"]);
    assert_eq!(queries::track_count(&db).unwrap(), 3);
}

#[test]
fn track_has_artists_and_album() {
    let (_dir, db) = sample_library();
    let first = queries::tracks(&db, TrackSort::Title, 0, 1).unwrap().remove(0);
    let track = queries::track(&db, first.id).unwrap();
    assert_eq!(track.artists, vec!["Radiohead".to_owned()]);
    assert_eq!(track.album.as_deref(), Some("OK Computer"));
    assert_eq!(track.track_no, Some(1));
}

#[test]
fn unknown_ids_are_not_found() {
    let (_dir, db) = sample_library();
    assert!(matches!(
        queries::track(&db, TrackId(999)),
        Err(LibraryError::NotFound { kind: "track", id: 999 })
    ));
    assert!(matches!(
        queries::album(&db, AlbumId(999)),
        Err(LibraryError::NotFound { kind: "album", id: 999 })
    ));
}
