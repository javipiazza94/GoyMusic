mod common;

use std::time::Duration;

use common::{album_fixture, collect_for, start, wait_for, LONG};
use goymusic_core::events::Event;
use goymusic_core::TrackId;
use goymusic_library::db::Db;
use goymusic_services::Services;

fn library(dir: &std::path::Path) -> (Services, Vec<TrackId>) {
    let music = dir.join("music");
    album_fixture(&music, "First", "Ana", 3);
    let services = start(dir);
    services.add_folder(&music).unwrap();
    assert!(services.wait_idle(LONG));
    let album = services.albums().unwrap()[0].id;
    let ids = services.album(album).unwrap().tracks.iter().map(|t| t.id).collect();
    (services, ids)
}

#[test]
fn playlist_lifecycle_publishes_playlists_changed() {
    let dir = tempfile::tempdir().unwrap();
    let (services, ids) = library(dir.path());
    let events = services.events().subscribe();
    let p = services.create_playlist("Mix").unwrap();
    services.rename_playlist(p.id, "Mezcla").unwrap();
    services.add_to_playlist(p.id, &ids).unwrap();
    services.move_playlist_item(p.id, 0, 2).unwrap();
    services.remove_from_playlist(p.id, &[0]).unwrap();
    let folder = services.create_playlist_folder("Carpeta", None).unwrap();
    services.move_playlist_to_folder(p.id, Some(folder.id)).unwrap();
    let detail = services.playlist(p.id).unwrap();
    assert_eq!(detail.playlist.name, "Mezcla");
    assert_eq!(detail.playlist.folder_id, Some(folder.id));
    let order: Vec<_> = detail.items.iter().map(|i| i.track.id).collect();
    assert_eq!(order, [ids[2], ids[0]]);
    assert_eq!(services.playlist_folders().unwrap(), vec![folder]);
    services.delete_playlist(p.id).unwrap();
    assert!(services.playlists().unwrap().is_empty());
    let changed = collect_for(&events, Duration::from_millis(200))
        .into_iter()
        .filter(|e| *e == Event::PlaylistsChanged)
        .count();
    assert_eq!(changed, 8);
}

#[test]
fn likes_publish_likes_changed_and_list_ids() {
    let dir = tempfile::tempdir().unwrap();
    let (services, ids) = library(dir.path());
    let events = services.events().subscribe();
    services.like(ids[1]).unwrap();
    wait_for(&events, LONG, |e| *e == Event::LikesChanged);
    assert_eq!(services.liked_ids().unwrap(), vec![ids[1]]);
    assert_eq!(services.liked_tracks().unwrap()[0].id, ids[1]);
    services.unlike(ids[1]).unwrap();
    wait_for(&events, LONG, |e| *e == Event::LikesChanged);
    assert!(services.liked_ids().unwrap().is_empty());
}

#[test]
fn import_and_export_m3u_round_trip() {
    let dir = tempfile::tempdir().unwrap();
    let (services, ids) = library(dir.path());
    let p = services.create_playlist("Original").unwrap();
    services.add_to_playlist(p.id, &[ids[2], ids[0]]).unwrap();
    let file = dir.path().join("Viaje.m3u8");
    services.export_m3u(p.id, &file).unwrap();
    let report = services.import_m3u(&file).unwrap();
    assert_eq!(report.playlist.name, "Viaje");
    assert_eq!(report.matched, 2);
    let order: Vec<_> = services
        .playlist(report.playlist.id)
        .unwrap()
        .items
        .iter()
        .map(|i| i.track.id)
        .collect();
    assert_eq!(order, [ids[2], ids[0]]);
}

#[test]
fn likes_work_while_a_scan_holds_the_write_lock() {
    let dir = tempfile::tempdir().unwrap();
    let (services, ids) = library(dir.path());
    let db_path = common::config(dir.path()).db_path;
    let (locked_tx, locked_rx) = std::sync::mpsc::channel();
    let holder = std::thread::spawn(move || {
        let mut db = Db::open(&db_path).unwrap();
        let tx = db.conn_mut().transaction().unwrap();
        tx.execute("INSERT INTO settings (key, value) VALUES ('busy', '1')", [])
            .unwrap();
        locked_tx.send(()).unwrap();
        std::thread::sleep(Duration::from_secs(1));
        tx.commit().unwrap();
    });
    locked_rx.recv().unwrap();
    services.like(ids[0]).expect("waits for the writer instead of failing");
    holder.join().unwrap();
    assert_eq!(services.liked_ids().unwrap(), vec![ids[0]]);
}

/// Writes that run in a transaction (read, then write) must wait for another writer too,
/// not fail at once with "database is locked" (a DEFERRED transaction cannot wait).
#[test]
fn playlist_edits_and_folder_removal_wait_for_another_writer() {
    let dir = tempfile::tempdir().unwrap();
    let (services, ids) = library(dir.path());
    let p = services.create_playlist("P").unwrap();
    let db_path = common::config(dir.path()).db_path;
    let (locked_tx, locked_rx) = std::sync::mpsc::channel();
    let holder = std::thread::spawn(move || {
        let mut db = Db::open(&db_path).unwrap();
        let tx = db.conn_mut().transaction().unwrap();
        tx.execute("INSERT INTO settings (key, value) VALUES ('busy', '1')", [])
            .unwrap();
        locked_tx.send(()).unwrap();
        std::thread::sleep(Duration::from_millis(500));
        tx.commit().unwrap();
    });
    locked_rx.recv().unwrap();
    services.add_to_playlist(p.id, &ids).expect("add waits for the writer");
    holder.join().unwrap();
    assert_eq!(services.playlist(p.id).unwrap().items.len(), 3);

    let db_path = common::config(dir.path()).db_path;
    let (locked_tx, locked_rx) = std::sync::mpsc::channel();
    let holder = std::thread::spawn(move || {
        let mut db = Db::open(&db_path).unwrap();
        let tx = db.conn_mut().transaction().unwrap();
        tx.execute("INSERT INTO settings (key, value) VALUES ('busy2', '1')", [])
            .unwrap();
        locked_tx.send(()).unwrap();
        std::thread::sleep(Duration::from_millis(500));
        tx.commit().unwrap();
    });
    locked_rx.recv().unwrap();
    let folder = services.folders().unwrap()[0].id;
    services.remove_folder(folder).expect("remove waits for the writer");
    holder.join().unwrap();
}
