mod common;

use common::{album_fixture, collect_for, scans_started, start, wait_for, write_png, write_tagged_wav, Meta, LONG};
use goymusic_core::events::Event;
use goymusic_core::AlbumId;
use goymusic_library::art::ArtSize;
use goymusic_library::queries::TrackSort;
use goymusic_library::search::TopResult;
use goymusic_services::ErrorCode;
use std::time::Duration;

#[test]
fn adding_a_folder_scans_it_in_the_background() {
    let dir = tempfile::tempdir().unwrap();
    let music = dir.path().join("music");
    album_fixture(&music, "First", "Ana", 2);
    let services = start(dir.path());
    let events = services.events().subscribe();
    let folder = services.add_folder(&music).unwrap();
    assert_eq!(services.folders().unwrap(), vec![folder]);
    assert!(services.wait_idle(LONG));
    wait_for(&events, LONG, |e| *e == Event::LibraryUpdated);
    assert_eq!(services.albums().unwrap().len(), 1);
}

#[test]
fn overlapping_folder_is_rejected_with_its_code() {
    let dir = tempfile::tempdir().unwrap();
    let music = dir.path().join("music");
    album_fixture(&music, "First", "Ana", 1);
    let services = start(dir.path());
    services.add_folder(&music.join("First")).unwrap();
    assert_eq!(services.add_folder(&music).unwrap_err().code, ErrorCode::FolderOverlap);
}

#[test]
fn album_and_artist_details() {
    let dir = tempfile::tempdir().unwrap();
    let music = dir.path().join("music");
    album_fixture(&music, "First", "Ana", 3);
    let services = start(dir.path());
    services.add_folder(&music).unwrap();
    assert!(services.wait_idle(LONG));
    let album = services.albums().unwrap()[0].clone();
    let detail = services.album(album.id).unwrap();
    assert_eq!(detail.album, album);
    let titles: Vec<_> = detail.tracks.iter().map(|t| t.title.as_str()).collect();
    assert_eq!(titles, ["First 1", "First 2", "First 3"]);
    let ana = services
        .artists()
        .unwrap()
        .into_iter()
        .find(|a| a.name == "Ana")
        .unwrap();
    let artist = services.artist(ana.id).unwrap();
    assert_eq!(artist.artist, ana);
    assert_eq!(artist.albums, vec![album]);
    assert_eq!(artist.tracks.len(), 3);
    assert_eq!(services.tracks(TrackSort::Title, 0, 2).unwrap().len(), 2);
    assert_eq!(services.track(detail.tracks[0].id).unwrap(), detail.tracks[0]);
}

#[test]
fn unknown_ids_are_not_found() {
    let dir = tempfile::tempdir().unwrap();
    let services = start(dir.path());
    assert_eq!(services.album(AlbumId(999)).unwrap_err().code, ErrorCode::NotFound);
}

#[test]
fn rescan_requests_during_a_scan_are_coalesced() {
    let dir = tempfile::tempdir().unwrap();
    let music = dir.path().join("music");
    album_fixture(&music, "First", "Ana", 3);
    let services = start(dir.path());
    services.add_folder(&music).unwrap();
    assert!(services.wait_idle(LONG));
    let events = services.events().subscribe();
    for _ in 0..10 {
        services.rescan();
    }
    assert!(services.wait_idle(LONG));
    let scans = scans_started(&collect_for(&events, Duration::from_millis(200)));
    assert!((1..=2).contains(&scans), "{scans} scans for 10 requests");
}

#[test]
fn removing_a_folder_drops_its_tracks() {
    let dir = tempfile::tempdir().unwrap();
    let music = dir.path().join("music");
    album_fixture(&music, "First", "Ana", 2);
    let services = start(dir.path());
    let folder = services.add_folder(&music).unwrap();
    assert!(services.wait_idle(LONG));
    let events = services.events().subscribe();
    services.remove_folder(folder.id).unwrap();
    wait_for(&events, LONG, |e| *e == Event::LibraryUpdated);
    assert!(services.tracks(TrackSort::Title, 0, 10).unwrap().is_empty());
}

#[test]
fn search_finds_by_typo() {
    let dir = tempfile::tempdir().unwrap();
    let music = dir.path().join("music");
    write_tagged_wav(&music.join("a.wav"), 0.3, &Meta::new("Creep", "Radiohead"));
    let services = start(dir.path());
    services.add_folder(&music).unwrap();
    assert!(services.wait_idle(LONG));
    let results = services.search("radioheaf", 10).unwrap();
    assert!(
        matches!(results.top, Some(TopResult::Artist(ref a)) if a.name == "Radiohead"),
        "{results:?}"
    );
}

#[test]
fn album_art_is_served_from_the_cache_dir() {
    let dir = tempfile::tempdir().unwrap();
    let music = dir.path().join("music");
    album_fixture(&music, "First", "Ana", 1);
    write_png(&music.join("First/cover.png"), 300, 300);
    let services = start(dir.path());
    services.add_folder(&music).unwrap();
    assert!(services.wait_idle(LONG));
    let album = services.albums().unwrap()[0].id;
    let art = services.album_art(album, ArtSize::Small).unwrap().unwrap();
    assert!(art.starts_with(common::config(dir.path()).art_dir), "{art:?}");
}
