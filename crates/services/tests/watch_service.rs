mod common;

use std::time::Duration;

use common::{
    album_fixture, collect_for, config, scans_started, start, start_with, wait_for, write_tagged_wav, Meta, LONG,
};
use goymusic_core::events::Event;
use goymusic_library::queries::TrackSort;
use goymusic_services::WatchStatus;

#[test]
fn a_new_file_is_indexed_without_asking() {
    let dir = tempfile::tempdir().unwrap();
    let music = dir.path().join("music");
    album_fixture(&music, "First", "Ana", 1);
    let services = start(dir.path());
    services.add_folder(&music).unwrap();
    assert!(services.wait_idle(LONG));
    let events = services.events().subscribe();
    write_tagged_wav(&music.join("new.wav"), 0.5, &Meta::new("Nueva", "Bea"));
    wait_for(&events, LONG, |e| *e == Event::LibraryUpdated);
    assert!(services
        .tracks(TrackSort::Title, 0, 10)
        .unwrap()
        .iter()
        .any(|t| t.title == "Nueva"));
}

#[test]
fn a_burst_of_changes_triggers_one_scan() {
    let dir = tempfile::tempdir().unwrap();
    let music = dir.path().join("music");
    album_fixture(&music, "First", "Ana", 1);
    let services = start(dir.path());
    services.add_folder(&music).unwrap();
    assert!(services.wait_idle(LONG));
    let events = services.events().subscribe();
    album_fixture(&music, "Copied", "Bea", 20);
    let seen = collect_for(&events, Duration::from_secs(2));
    assert_eq!(scans_started(&seen), 1, "{seen:?}");
    assert_eq!(services.tracks(TrackSort::Title, 0, 100).unwrap().len(), 21);
}

#[test]
fn moved_album_folder_stays_in_its_playlist() {
    let dir = tempfile::tempdir().unwrap();
    let music = dir.path().join("music");
    album_fixture(&music, "AlbumA", "Ana", 3);
    let services = start(dir.path());
    services.add_folder(&music).unwrap();
    assert!(services.wait_idle(LONG));
    let album = services.albums().unwrap()[0].id;
    let ids: Vec<_> = services.album(album).unwrap().tracks.iter().map(|t| t.id).collect();
    let playlist = services.create_playlist("Favoritas").unwrap();
    services.add_to_playlist(playlist.id, &ids).unwrap();
    let events = services.events().subscribe();

    std::fs::create_dir_all(music.join("Moved")).unwrap();
    std::fs::rename(music.join("AlbumA"), music.join("Moved/AlbumA")).unwrap();
    wait_for(&events, LONG, |e| *e == Event::LibraryUpdated);
    assert!(services.wait_idle(LONG));

    let items = services.playlist(playlist.id).unwrap().items;
    assert_eq!(items.iter().map(|i| i.track.id).collect::<Vec<_>>(), ids);
    assert!(
        items.iter().all(|i| i.track.path.contains("/Moved/AlbumA/")),
        "{items:?}"
    );
}

#[test]
fn an_unwatchable_folder_falls_back_to_polling() {
    let dir = tempfile::tempdir().unwrap();
    let music = dir.path().join("music");
    album_fixture(&music, "First", "Ana", 1);
    let services = start(dir.path());
    services.add_folder(&music).unwrap();
    assert!(services.wait_idle(LONG));
    drop(services);
    std::fs::rename(&music, dir.path().join("elsewhere")).unwrap();

    let mut cfg = config(dir.path());
    cfg.poll_every = Duration::from_millis(300);
    let services = start_with(cfg);
    let events = services.events().subscribe();
    assert!(matches!(services.watch_status(), WatchStatus::Polling { .. }));
    let scans = scans_started(&collect_for(&events, Duration::from_secs(2)));
    assert!(scans >= 2, "{scans} scans while polling every 300 ms");
}

#[test]
fn removing_a_folder_stops_watching_it() {
    let dir = tempfile::tempdir().unwrap();
    let music = dir.path().join("music");
    album_fixture(&music, "First", "Ana", 1);
    let services = start(dir.path());
    let folder = services.add_folder(&music).unwrap();
    assert!(services.wait_idle(LONG));
    services.remove_folder(folder.id).unwrap();
    let events = services.events().subscribe();
    write_tagged_wav(&music.join("late.wav"), 0.5, &Meta::new("Tarde", "Bea"));
    assert_eq!(scans_started(&collect_for(&events, Duration::from_secs(1))), 0);
}

#[test]
fn settings_report_the_watch_status() {
    let dir = tempfile::tempdir().unwrap();
    let music = dir.path().join("music");
    album_fixture(&music, "First", "Ana", 1);
    let services = start(dir.path());
    services.add_folder(&music).unwrap();
    assert_eq!(services.settings().unwrap().watch, WatchStatus::Live);
}
