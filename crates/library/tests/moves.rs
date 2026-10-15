mod common;

use common::{write_tagged_wav, Meta};
use goymusic_core::events::EventBus;
use goymusic_library::db::Db;
use goymusic_library::{folders, scanner};

fn track_id_at(db: &Db, path: &std::path::Path) -> Option<i64> {
    let path = std::fs::canonicalize(path).ok()?;
    db.conn()
        .query_row("SELECT id FROM tracks WHERE path = ?1", [path.to_str().unwrap()], |r| {
            r.get(0)
        })
        .ok()
}

#[test]
fn moved_file_keeps_its_id() {
    let dir = tempfile::tempdir().unwrap();
    let old = dir.path().join("a.wav");
    write_tagged_wav(
        &old,
        0.3,
        &Meta {
            album: Some("Album"),
            ..Meta::new("Song", "Artist")
        },
    );
    let mut db = Db::open_in_memory().unwrap();
    folders::add(&db, dir.path()).unwrap();
    let events = EventBus::new();
    scanner::scan(&mut db, &events).unwrap();
    let id = track_id_at(&db, &old).unwrap();

    let new = dir.path().join("sorted/Artist/a.wav");
    std::fs::create_dir_all(new.parent().unwrap()).unwrap();
    std::fs::rename(&old, &new).unwrap();
    let report = scanner::scan(&mut db, &events).unwrap();

    assert_eq!((report.moved, report.added, report.removed), (1, 0, 0));
    assert_eq!(track_id_at(&db, &new), Some(id));
}

#[test]
fn musicbrainz_id_matches_even_when_tags_changed() {
    let dir = tempfile::tempdir().unwrap();
    let old = dir.path().join("a.wav");
    write_tagged_wav(
        &old,
        0.3,
        &Meta {
            mb_recording_id: Some("rec-1"),
            ..Meta::new("Song", "Artist")
        },
    );
    let mut db = Db::open_in_memory().unwrap();
    folders::add(&db, dir.path()).unwrap();
    let events = EventBus::new();
    scanner::scan(&mut db, &events).unwrap();
    let id = track_id_at(&db, &old).unwrap();

    std::fs::remove_file(&old).unwrap();
    let new = dir.path().join("b.wav");
    write_tagged_wav(
        &new,
        0.3,
        &Meta {
            mb_recording_id: Some("rec-1"),
            ..Meta::new("Song (Remastered)", "Artist")
        },
    );
    let report = scanner::scan(&mut db, &events).unwrap();

    assert_eq!(report.moved, 1);
    assert_eq!(track_id_at(&db, &new), Some(id));
}

#[test]
fn different_file_is_not_a_move() {
    let dir = tempfile::tempdir().unwrap();
    let old = dir.path().join("a.wav");
    write_tagged_wav(&old, 0.3, &Meta::new("Song", "Artist"));
    let mut db = Db::open_in_memory().unwrap();
    folders::add(&db, dir.path()).unwrap();
    let events = EventBus::new();
    scanner::scan(&mut db, &events).unwrap();

    std::fs::remove_file(&old).unwrap();
    write_tagged_wav(&dir.path().join("b.wav"), 0.3, &Meta::new("Other song", "Artist"));
    let report = scanner::scan(&mut db, &events).unwrap();

    assert_eq!((report.moved, report.added, report.removed), (0, 1, 1));
}

#[test]
fn move_between_library_folders_is_detected() {
    let root = tempfile::tempdir().unwrap();
    let (a, b) = (root.path().join("a"), root.path().join("b"));
    std::fs::create_dir_all(&a).unwrap();
    std::fs::create_dir_all(&b).unwrap();
    write_tagged_wav(&a.join("x.wav"), 0.3, &Meta::new("Song", "Artist"));
    let mut db = Db::open_in_memory().unwrap();
    folders::add(&db, &a).unwrap();
    folders::add(&db, &b).unwrap();
    let events = EventBus::new();
    scanner::scan(&mut db, &events).unwrap();
    let id = track_id_at(&db, &a.join("x.wav")).unwrap();

    std::fs::rename(a.join("x.wav"), b.join("x.wav")).unwrap();
    let report = scanner::scan(&mut db, &events).unwrap();

    assert_eq!(report.moved, 1);
    assert_eq!(track_id_at(&db, &b.join("x.wav")), Some(id));
}
