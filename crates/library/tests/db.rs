use goymusic_library::db::Db;

fn table_exists(db: &Db, name: &str) -> bool {
    db.conn()
        .query_row("SELECT count(*) FROM sqlite_master WHERE name = ?1", [name], |r| {
            r.get::<_, i64>(0)
        })
        .unwrap()
        == 1
}

#[test]
fn in_memory_db_is_migrated_with_foreign_keys_on() {
    let db = Db::open_in_memory().unwrap();
    for table in [
        "folders",
        "tracks",
        "albums",
        "artists",
        "playlists",
        "playlist_items",
        "likes",
        "queue_state",
        "settings",
        "tracks_fts",
    ] {
        assert!(table_exists(&db, table), "missing table {table}");
    }
    let fk: i64 = db.conn().query_row("PRAGMA foreign_keys", [], |r| r.get(0)).unwrap();
    assert_eq!(fk, 1);
    let version: i64 = db.conn().query_row("PRAGMA user_version", [], |r| r.get(0)).unwrap();
    assert_eq!(version, 1);
}

#[test]
fn file_db_is_created_with_parent_dirs_and_reopens() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("nested/library.db");
    drop(Db::open(&path).unwrap());
    let db = Db::open(&path).unwrap();
    let mode: String = db.conn().query_row("PRAGMA journal_mode", [], |r| r.get(0)).unwrap();
    assert_eq!(mode, "wal");
}
