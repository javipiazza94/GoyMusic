mod common;

use common::{write_tagged_wav, Meta};
use goymusic_core::events::EventBus;
use goymusic_library::db::Db;
use goymusic_library::{folders, history, queries, scanner};

#[test]
fn recent_albums_lists_most_recently_played_first_without_duplicates() {
    let dir = tempfile::tempdir().unwrap();
    let song = |t, album| Meta {
        album: Some(album),
        ..Meta::new(t, "Ana")
    };
    write_tagged_wav(&dir.path().join("a1.wav"), 0.2, &song("a1", "A"));
    write_tagged_wav(&dir.path().join("a2.wav"), 0.2, &song("a2", "A"));
    write_tagged_wav(&dir.path().join("b1.wav"), 0.2, &song("b1", "B"));
    let mut db = Db::open_in_memory().unwrap();
    folders::add(&db, dir.path()).unwrap();
    scanner::scan(&mut db, &EventBus::new()).unwrap();
    let id = |title: &str| {
        queries::tracks(&db, queries::TrackSort::Title, 0, 10)
            .unwrap()
            .into_iter()
            .find(|t| t.title == title)
            .unwrap()
            .id
    };
    history::record_play(&db, id("a1"), 10).unwrap();
    history::record_play(&db, id("b1"), 20).unwrap();
    history::record_play(&db, id("a2"), 30).unwrap();

    let titles = |limit| -> Vec<String> {
        history::recent_albums(&db, limit)
            .unwrap()
            .into_iter()
            .map(|a| a.title)
            .collect()
    };
    assert_eq!(titles(10), ["A", "B"]);
    assert_eq!(titles(1), ["A"]);
}
