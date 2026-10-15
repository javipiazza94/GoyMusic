use goymusic_core::queue::{Previous, Queue, RepeatMode};
use goymusic_core::TrackId;
use rand::rngs::StdRng;
use rand::SeedableRng;

fn ids(n: i64) -> Vec<TrackId> {
    (1..=n).map(TrackId).collect()
}

fn rng() -> StdRng {
    StdRng::seed_from_u64(7)
}

#[test]
fn play_context_starts_at_index() {
    let mut q = Queue::new();
    assert_eq!(q.play_context(ids(3), 1, &mut rng()), Some(TrackId(2)));
    assert_eq!(q.current(), Some(TrackId(2)));
}

#[test]
fn next_walks_context_then_stops() {
    let mut q = Queue::new();
    q.play_context(ids(3), 0, &mut rng());
    assert_eq!(q.next(), Some(TrackId(2)));
    assert_eq!(q.next(), Some(TrackId(3)));
    assert_eq!(q.next(), None);
    assert_eq!(q.next(), None);
}

#[test]
fn repeat_all_wraps_to_start() {
    let mut q = Queue::new();
    q.set_repeat(RepeatMode::All);
    q.play_context(ids(3), 2, &mut rng());
    assert_eq!(q.next(), Some(TrackId(1)));
}

#[test]
fn repeat_one_replays_when_track_ends_but_manual_next_advances() {
    let mut q = Queue::new();
    q.set_repeat(RepeatMode::One);
    q.play_context(ids(3), 0, &mut rng());
    assert_eq!(q.on_track_ended(), Some(TrackId(1)));
    assert_eq!(q.next(), Some(TrackId(2)));
}

#[test]
fn previous_after_threshold_restarts_track() {
    let mut q = Queue::new();
    q.play_context(ids(3), 0, &mut rng());
    q.next();
    assert_eq!(q.previous(3_001), Previous::Restart);
    assert_eq!(q.current(), Some(TrackId(2)));
}

#[test]
fn previous_within_threshold_goes_back_and_next_returns() {
    let mut q = Queue::new();
    q.play_context(ids(3), 0, &mut rng());
    q.next();
    q.next();
    assert_eq!(q.previous(1_000), Previous::Track(TrackId(2)));
    assert_eq!(q.current(), Some(TrackId(2)));
    assert_eq!(q.next(), Some(TrackId(3)));
}

#[test]
fn previous_with_no_history_restarts() {
    let mut q = Queue::new();
    q.play_context(ids(3), 0, &mut rng());
    assert_eq!(q.previous(0), Previous::Restart);
}

#[test]
fn empty_or_out_of_range_context_plays_nothing() {
    let mut q = Queue::new();
    assert_eq!(q.play_context(vec![], 0, &mut rng()), None);
    assert_eq!(q.play_context(ids(2), 5, &mut rng()), None);
    assert_eq!(q.current(), None);
    assert_eq!(q.next(), None);
}
