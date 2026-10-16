mod common;

use std::time::Instant;

use common::{write_tagged_wav, Meta};
use goymusic_core::events::EventBus;
use goymusic_library::db::Db;
use goymusic_library::search::{search, TopResult};
use goymusic_library::{folders, scanner};
use rusqlite::params;

fn library() -> (tempfile::TempDir, Db) {
    let dir = tempfile::tempdir().unwrap();
    let ok = |t| Meta {
        album: Some("OK Computer"),
        album_artist: Some("Radiohead"),
        ..Meta::new(t, "Radiohead")
    };
    write_tagged_wav(&dir.path().join("1.wav"), 0.2, &ok("Airbag"));
    write_tagged_wav(&dir.path().join("2.wav"), 0.2, &ok("Paranoid Android"));
    write_tagged_wav(
        &dir.path().join("3.wav"),
        0.2,
        &Meta {
            album: Some("Lemonade"),
            ..Meta::new("Formation", "Beyoncé")
        },
    );
    let mut db = Db::open_in_memory().unwrap();
    folders::add(&db, dir.path()).unwrap();
    scanner::scan(&mut db, &EventBus::new()).unwrap();
    (dir, db)
}

#[test]
fn prefix_matches_while_typing() {
    let (_dir, db) = library();
    let r = search(&db, "radi", 10).unwrap();
    assert_eq!(
        r.artists.iter().map(|a| a.name.as_str()).collect::<Vec<_>>(),
        vec!["Radiohead"]
    );
    assert_eq!(r.tracks.len(), 2);
    assert!(matches!(r.top, Some(TopResult::Artist(ref a)) if a.name == "Radiohead"));
}

#[test]
fn several_words_must_all_match() {
    let (_dir, db) = library();
    let r = search(&db, "paranoid andr", 10).unwrap();
    assert_eq!(
        r.tracks.iter().map(|t| t.title.as_str()).collect::<Vec<_>>(),
        vec!["Paranoid Android"]
    );
}

#[test]
fn album_search() {
    let (_dir, db) = library();
    let r = search(&db, "ok comp", 10).unwrap();
    assert_eq!(
        r.albums.iter().map(|a| a.title.as_str()).collect::<Vec<_>>(),
        vec!["OK Computer"]
    );
}

#[test]
fn typo_finds_artist_through_fuzzy_fallback() {
    let (_dir, db) = library();
    let r = search(&db, "radioheaf", 10).unwrap();
    assert!(r.artists.iter().any(|a| a.name == "Radiohead"), "{r:?}");
}

#[test]
fn accents_are_ignored() {
    let (_dir, db) = library();
    let r = search(&db, "beyonce", 10).unwrap();
    assert!(r.artists.iter().any(|a| a.name == "Beyoncé"), "{r:?}");
    assert_eq!(r.tracks.len(), 1);
}

#[test]
fn fts_syntax_is_treated_as_text() {
    let (_dir, db) = library();
    for query in [
        "\"",
        "*",
        "AND",
        "air -bag",
        "(paranoid",
        "NEAR(a b)",
        "\"unclosed",
        "''",
    ] {
        assert!(search(&db, query, 10).is_ok(), "query {query:?} failed");
    }
}

#[test]
fn empty_query_returns_nothing() {
    let (_dir, db) = library();
    assert_eq!(search(&db, "   ", 10).unwrap(), Default::default());
}

/// Spec target: < 50 ms with 50 000 tracks. Run with `cargo test --release -- --ignored`.
#[test]
#[ignore]
fn search_is_fast_on_a_large_library() {
    let mut db = Db::open_in_memory().unwrap();
    {
        let tx = db.conn_mut().transaction().unwrap();
        tx.execute("INSERT INTO folders (id, path) VALUES (1, '/synthetic')", [])
            .unwrap();
        for i in 0..50_000i64 {
            let artist = format!("Artist {}", i % 2_000);
            let title = format!("Song number {i} love");
            tx.execute("INSERT OR IGNORE INTO artists (name) VALUES (?1)", [&artist])
                .unwrap();
            let artist_id: i64 = tx
                .query_row("SELECT id FROM artists WHERE name = ?1", [&artist], |r| r.get(0))
                .unwrap();
            tx.execute(
                "INSERT INTO tracks (id, path, folder_id, size, mtime, title, duration_ms) VALUES (?1, ?2, 1, 1, 1, ?3, 1000)",
                params![i + 1, format!("/synthetic/{i}.mp3"), title],
            )
            .unwrap();
            tx.execute(
                "INSERT INTO track_artists (track_id, artist_id, position) VALUES (?1, ?2, 0)",
                params![i + 1, artist_id],
            )
            .unwrap();
            tx.execute(
                "INSERT INTO tracks_fts (rowid, title, artists, album) VALUES (?1, ?2, ?3, '')",
                params![i + 1, title, artist],
            )
            .unwrap();
        }
        tx.commit().unwrap();
    }
    for query in ["love", "artist 19", "artsit 1999"] {
        let start = Instant::now();
        search(&db, query, 20).unwrap();
        let elapsed = start.elapsed();
        assert!(elapsed.as_millis() < 50, "{query:?} took {elapsed:?}");
    }
}

#[test]
fn search_results_serialize_for_the_ui() {
    let (_dir, db) = library();
    let v = serde_json::to_value(search(&db, "Radiohead", 10).unwrap()).unwrap();
    assert_eq!(v["top"]["kind"], "artist");
    assert_eq!(v["top"]["item"]["name"], "Radiohead");
    assert!(v["tracks"].is_array());
}
