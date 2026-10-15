mod common;

use common::{write_tagged_wav, Meta};
use goymusic_core::events::EventBus;
use goymusic_library::db::Db;
use goymusic_library::{folders, m3u, playlists, scanner};

fn library() -> (tempfile::TempDir, Db) {
    let dir = tempfile::tempdir().unwrap();
    write_tagged_wav(&dir.path().join("music/a.wav"), 1.0, &Meta::new("Alpha", "Artist"));
    write_tagged_wav(&dir.path().join("music/b.wav"), 1.0, &Meta::new("Beta", "Artist"));
    let mut db = Db::open_in_memory().unwrap();
    folders::add(&db, &dir.path().join("music")).unwrap();
    scanner::scan(&mut db, &EventBus::new()).unwrap();
    (dir, db)
}

#[test]
fn import_resolves_relative_paths_and_reports_missing() {
    let (dir, mut db) = library();
    let list = dir.path().join("lists/mine.m3u8");
    std::fs::create_dir_all(list.parent().unwrap()).unwrap();
    std::fs::write(
        &list,
        "\u{feff}#EXTM3U\n#EXTINF:1,Artist - Beta\n../music/b.wav\r\n\n../music/a.wav\nnowhere/c.mp3\n",
    )
    .unwrap();

    let report = m3u::import(&mut db, "Imported", &list).unwrap();

    assert_eq!(report.matched, 2);
    assert_eq!(report.missing, vec!["nowhere/c.mp3".to_owned()]);
    let titles: Vec<_> = playlists::items(&db, report.playlist.id)
        .unwrap()
        .into_iter()
        .map(|i| i.track.title)
        .collect();
    assert_eq!(titles, vec!["Beta", "Alpha"]);
    assert_eq!(report.playlist.track_count, 2);
}

#[test]
fn export_writes_extended_m3u_that_imports_back() {
    let (dir, mut db) = library();
    let list = dir.path().join("first.m3u8");
    std::fs::write(&list, "music/a.wav\nmusic/b.wav\n").unwrap();
    let first = m3u::import(&mut db, "First", &list).unwrap();

    let out = dir.path().join("out/exported.m3u8");
    std::fs::create_dir_all(out.parent().unwrap()).unwrap();
    m3u::export(&db, first.playlist.id, &out).unwrap();
    let text = std::fs::read_to_string(&out).unwrap();
    assert!(text.starts_with("#EXTM3U\n"));
    assert!(text.contains("#EXTINF:1,Artist - Alpha\n"));

    let again = m3u::import(&mut db, "Again", &out).unwrap();
    assert_eq!((again.matched, again.missing.len()), (2, 0));
}
