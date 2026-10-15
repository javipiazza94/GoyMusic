use goymusic_core::queue::Queue;
use goymusic_core::TrackId;
use goymusic_library::db::Db;
use goymusic_library::state;
use rand::rngs::StdRng;
use rand::SeedableRng;

#[test]
fn nothing_saved_yet() {
    let db = Db::open_in_memory().unwrap();
    assert!(state::load_queue(&db).unwrap().is_none());
    assert_eq!(state::setting(&db, "theme").unwrap(), None);
}

#[test]
fn queue_and_position_survive_a_restart() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("library.db");
    let mut queue = Queue::new();
    queue.play_context((1..=5).map(TrackId).collect(), 2, &mut StdRng::seed_from_u64(1));
    queue.queue_add(&[TrackId(42)]);
    {
        let db = Db::open(&path).unwrap();
        state::save_queue(&db, &queue, 61_500).unwrap();
        state::save_queue(&db, &queue, 62_000).unwrap(); // overwrites, single row
    }
    let db = Db::open(&path).unwrap();
    assert_eq!(state::load_queue(&db).unwrap(), Some((queue, 62_000)));
}

#[test]
fn settings_overwrite() {
    let db = Db::open_in_memory().unwrap();
    state::set_setting(&db, "theme", "dark").unwrap();
    state::set_setting(&db, "theme", "light").unwrap();
    assert_eq!(state::setting(&db, "theme").unwrap().as_deref(), Some("light"));
}
