use goymusic_core::queue::{Previous, Queue};
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
fn user_queue_has_priority_then_context_resumes() {
    let mut q = Queue::new();
    q.play_context(ids(3), 0, &mut rng());
    q.queue_add(&[TrackId(100)]);
    assert_eq!(q.next(), Some(TrackId(100)));
    assert_eq!(q.next(), Some(TrackId(2)));
}

#[test]
fn queue_remove_and_move() {
    let mut q = Queue::new();
    q.play_context(ids(1), 0, &mut rng());
    q.queue_add(&[TrackId(10), TrackId(11), TrackId(12)]);
    assert_eq!(q.queue_remove(1), Some(TrackId(11)));
    assert!(q.queue_move(1, 0));
    assert_eq!(q.user_queue(), vec![TrackId(12), TrackId(10)]);
    assert!(!q.queue_move(5, 0));
    assert_eq!(q.queue_remove(9), None);
}

#[test]
fn upcoming_lists_user_queue_then_context() {
    let mut q = Queue::new();
    q.play_context(ids(4), 1, &mut rng());
    q.queue_add(&[TrackId(50)]);
    assert_eq!(q.upcoming(10), vec![TrackId(50), TrackId(3), TrackId(4)]);
    assert_eq!(q.upcoming(2), vec![TrackId(50), TrackId(3)]);
}

#[test]
fn shuffle_keeps_current_and_is_a_permutation() {
    let mut r = rng();
    let mut q = Queue::new();
    q.play_context(ids(10), 4, &mut r);
    q.set_shuffle(true, &mut r);
    assert_eq!(q.current(), Some(TrackId(5)));
    let mut upcoming = q.upcoming(20);
    upcoming.sort();
    let expected: Vec<TrackId> = ids(10).into_iter().filter(|&t| t != TrackId(5)).collect();
    assert_eq!(upcoming, expected);
}

#[test]
fn play_context_with_shuffle_on_starts_with_chosen_track() {
    let mut r = rng();
    let mut q = Queue::new();
    q.set_shuffle(true, &mut r);
    assert_eq!(q.play_context(ids(10), 7, &mut r), Some(TrackId(8)));
    let upcoming = q.upcoming(20);
    assert_eq!(upcoming.len(), 9);
    assert!(!upcoming.contains(&TrackId(8)));
}

#[test]
fn shuffle_off_continues_in_natural_order_after_current() {
    let mut r = rng();
    let mut q = Queue::new();
    q.play_context(ids(10), 0, &mut r);
    q.set_shuffle(true, &mut r);
    let x = q.next().unwrap();
    q.set_shuffle(false, &mut r);
    let expected = if x.0 < 10 { Some(TrackId(x.0 + 1)) } else { None };
    assert_eq!(q.next(), expected);
}

#[test]
fn previous_survives_shuffle_toggle() {
    let mut r = rng();
    let mut q = Queue::new();
    q.play_context(ids(5), 0, &mut r);
    q.next();
    q.set_shuffle(true, &mut r);
    assert_eq!(q.previous(0), Previous::Track(TrackId(1)));
}

#[test]
fn queue_state_roundtrips_through_json() {
    let mut r = rng();
    let mut q = Queue::new();
    q.play_context(ids(6), 2, &mut r);
    q.set_shuffle(true, &mut r);
    q.queue_add(&[TrackId(99)]);
    q.next();
    let json = serde_json::to_string(&q).unwrap();
    assert_eq!(serde_json::from_str::<Queue>(&json).unwrap(), q);
}
