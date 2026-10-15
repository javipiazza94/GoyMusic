mod common;

use common::{write_tagged_wav, write_wav, Meta};
use goymusic_core::events::{Event, EventBus};
use goymusic_library::db::Db;
use goymusic_library::{folders, scanner};

fn count(db: &Db, sql: &str) -> i64 {
    db.conn().query_row(sql, [], |r| r.get(0)).unwrap()
}

fn library(dir: &std::path::Path) -> (Db, EventBus) {
    let db = Db::open_in_memory().unwrap();
    folders::add(&db, dir).unwrap();
    (db, EventBus::new())
}

fn album_track(title: &str) -> Meta<'_> {
    Meta {
        album: Some("OK Computer"),
        album_artist: Some("Radiohead"),
        ..Meta::new(title, "Radiohead")
    }
}

#[test]
fn scan_adds_tracks_albums_and_artists() {
    let dir = tempfile::tempdir().unwrap();
    write_tagged_wav(&dir.path().join("cd/01.wav"), 0.3, &album_track("Airbag"));
    write_tagged_wav(&dir.path().join("cd/02.wav"), 0.3, &album_track("Paranoid Android"));
    let (mut db, events) = library(dir.path());

    let report = scanner::scan(&mut db, &events).unwrap();

    assert_eq!((report.added, report.errors.len()), (2, 0));
    assert_eq!(count(&db, "SELECT count(*) FROM tracks"), 2);
    assert_eq!(count(&db, "SELECT count(*) FROM albums"), 1);
    assert_eq!(count(&db, "SELECT count(*) FROM artists"), 1);
    assert_eq!(
        count(&db, "SELECT count(*) FROM tracks_fts WHERE tracks_fts MATCH 'airbag'"),
        1
    );
}

#[test]
fn rescan_without_changes_reads_nothing() {
    let dir = tempfile::tempdir().unwrap();
    write_tagged_wav(&dir.path().join("a.wav"), 0.3, &Meta::new("A", "X"));
    let (mut db, events) = library(dir.path());
    scanner::scan(&mut db, &events).unwrap();

    let report = scanner::scan(&mut db, &events).unwrap();
    assert_eq!(
        (report.unchanged, report.added, report.updated, report.removed),
        (1, 0, 0, 0)
    );
}

#[test]
fn changed_file_is_updated_in_place() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("a.wav");
    write_tagged_wav(&path, 0.3, &Meta::new("Old", "X"));
    let (mut db, events) = library(dir.path());
    scanner::scan(&mut db, &events).unwrap();
    let id = count(&db, "SELECT id FROM tracks");

    write_tagged_wav(&path, 0.3, &Meta::new("A much longer new title", "X"));
    let report = scanner::scan(&mut db, &events).unwrap();

    assert_eq!(report.updated, 1);
    let title: String = db
        .conn()
        .query_row("SELECT title FROM tracks WHERE id = ?1", [id], |r| r.get(0))
        .unwrap();
    assert_eq!(title, "A much longer new title");
}

#[test]
fn deleted_file_is_removed_with_orphans() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("a.wav");
    write_tagged_wav(&path, 0.3, &album_track("Airbag"));
    let (mut db, events) = library(dir.path());
    scanner::scan(&mut db, &events).unwrap();

    std::fs::remove_file(&path).unwrap();
    let report = scanner::scan(&mut db, &events).unwrap();

    assert_eq!(report.removed, 1);
    assert_eq!(count(&db, "SELECT count(*) FROM tracks"), 0);
    assert_eq!(count(&db, "SELECT count(*) FROM albums"), 0);
    assert_eq!(count(&db, "SELECT count(*) FROM artists"), 0);
    assert_eq!(count(&db, "SELECT count(*) FROM tracks_fts"), 0);
}

#[test]
fn untagged_file_uses_file_stem_as_title() {
    let dir = tempfile::tempdir().unwrap();
    write_wav(&dir.path().join("Mystery Song.wav"), 0.3);
    let (mut db, events) = library(dir.path());
    scanner::scan(&mut db, &events).unwrap();
    let title: String = db
        .conn()
        .query_row("SELECT title FROM tracks", [], |r| r.get(0))
        .unwrap();
    assert_eq!(title, "Mystery Song");
}

#[test]
fn corrupt_file_is_reported_and_scan_continues() {
    let dir = tempfile::tempdir().unwrap();
    write_tagged_wav(&dir.path().join("good.wav"), 0.3, &Meta::new("Good", "X"));
    std::fs::write(dir.path().join("bad.mp3"), b"not audio at all").unwrap();
    let (mut db, events) = library(dir.path());

    let report = scanner::scan(&mut db, &events).unwrap();

    assert_eq!(report.added, 1);
    assert_eq!(report.errors.len(), 1);
    assert!(report.errors[0].path.ends_with("bad.mp3"));
}

#[cfg(unix)]
#[test]
fn non_utf8_file_name_is_reported_not_fatal() {
    use std::os::unix::ffi::OsStrExt;
    let dir = tempfile::tempdir().unwrap();
    write_wav(&dir.path().join(std::ffi::OsStr::from_bytes(b"bad\xff.wav")), 0.3);
    write_tagged_wav(&dir.path().join("ok.wav"), 0.3, &Meta::new("Ok", "X"));
    let (mut db, events) = library(dir.path());

    let report = scanner::scan(&mut db, &events).unwrap();

    assert_eq!(report.added, 1);
    assert_eq!(report.errors.len(), 1);
}

#[test]
fn ignores_non_audio_files() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("cover.jpg"), b"jpeg").unwrap();
    std::fs::write(dir.path().join("notes.txt"), b"text").unwrap();
    let (mut db, events) = library(dir.path());
    let report = scanner::scan(&mut db, &events).unwrap();
    assert_eq!(report, scanner::ScanReport::default());
}

#[test]
fn publishes_progress_and_library_updated_only_on_change() {
    let dir = tempfile::tempdir().unwrap();
    write_tagged_wav(&dir.path().join("a.wav"), 0.3, &Meta::new("A", "X"));
    write_tagged_wav(&dir.path().join("b.wav"), 0.3, &Meta::new("B", "X"));
    let (mut db, events) = library(dir.path());
    let rx = events.subscribe();

    scanner::scan(&mut db, &events).unwrap();
    let first: Vec<Event> = rx.try_iter().collect();
    assert!(first.contains(&Event::ScanProgress { done: 2, total: 2 }));
    assert!(first.contains(&Event::LibraryUpdated));

    scanner::scan(&mut db, &events).unwrap();
    let second: Vec<Event> = rx.try_iter().collect();
    assert!(!second.contains(&Event::LibraryUpdated));
}

#[test]
fn removing_a_folder_removes_its_tracks() {
    let dir = tempfile::tempdir().unwrap();
    write_tagged_wav(&dir.path().join("a.wav"), 0.3, &Meta::new("A", "X"));
    let (mut db, events) = library(dir.path());
    scanner::scan(&mut db, &events).unwrap();

    let folder = folders::list(&db).unwrap().remove(0);
    folders::remove(&mut db, folder.id).unwrap();

    assert_eq!(count(&db, "SELECT count(*) FROM tracks"), 0);
    assert_eq!(count(&db, "SELECT count(*) FROM tracks_fts"), 0);
}

// A scan that could not enumerate a folder (external disk asleep, NFS timeout, permissions)
// must NOT delete the tracks it failed to see, or their playlist membership and likes vanish
// via ON DELETE CASCADE. Removals only happen on a clean scan.
#[cfg(unix)]
#[test]
fn enumeration_error_does_not_delete_known_tracks() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let album = dir.path().join("album");
    write_tagged_wav(&album.join("01.wav"), 0.3, &Meta::new("Keeper", "X"));
    let (mut db, events) = library(dir.path());
    scanner::scan(&mut db, &events).unwrap();
    let track_id = count(&db, "SELECT id FROM tracks");
    db.conn()
        .execute("INSERT INTO likes (track_id, liked_at) VALUES (?1, 1)", [track_id])
        .unwrap();

    // make the subfolder unreadable so WalkDir yields an error for it
    std::fs::set_permissions(&album, std::fs::Permissions::from_mode(0o000)).unwrap();
    let report = scanner::scan(&mut db, &events).unwrap();
    std::fs::set_permissions(&album, std::fs::Permissions::from_mode(0o755)).unwrap(); // restore so tempdir cleans up

    assert!(!report.errors.is_empty(), "enumeration error should be reported");
    assert_eq!(report.removed, 0, "an unreadable folder must not trigger deletions");
    assert_eq!(count(&db, "SELECT count(*) FROM tracks"), 1);
    assert_eq!(count(&db, "SELECT count(*) FROM likes"), 1);
}

// A file moved A->B whose destination can't be read in this scan (half-copied, locked) must
// not delete the original: the removal waits for a scan with no errors.
#[test]
fn move_with_unreadable_destination_keeps_original() {
    let dir = tempfile::tempdir().unwrap();
    let old = dir.path().join("a.wav");
    write_tagged_wav(&old, 0.3, &Meta::new("Song", "Artist"));
    let (mut db, events) = library(dir.path());
    scanner::scan(&mut db, &events).unwrap();
    let track_id = count(&db, "SELECT id FROM tracks");
    db.conn()
        .execute("INSERT INTO likes (track_id, liked_at) VALUES (?1, 1)", [track_id])
        .unwrap();

    std::fs::remove_file(&old).unwrap();
    std::fs::write(dir.path().join("b.wav"), b"half-copied, not yet valid audio").unwrap();
    let report = scanner::scan(&mut db, &events).unwrap();

    assert_eq!(report.errors.len(), 1);
    assert_eq!(report.removed, 0, "a read error must not delete the move source");
    assert_eq!(count(&db, "SELECT count(*) FROM tracks"), 1);
    assert_eq!(count(&db, "SELECT count(*) FROM likes"), 1);
}
