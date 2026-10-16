mod common;

use std::sync::Arc;
use std::time::{Duration, Instant};

use common::{capture_factory, resolver, sine, write_wav};
use goymusic_audio::player::Player;
use goymusic_audio::sink::SinkFormat;
use goymusic_core::events::{Event, EventBus, PlaybackState};
use goymusic_core::TrackId;

#[test]
fn plays_a_context_on_its_own_thread_and_stops_on_drop() {
    let dir = tempfile::tempdir().unwrap();
    let files: Vec<_> = (1..=2)
        .map(|i| {
            let p = dir.path().join(format!("{i}.wav"));
            write_wav(&p, 8_000, 1, &sine(8_000, 1, 440.0, 0, 4_000));
            (i, p)
        })
        .collect();
    let bus = Arc::new(EventBus::new());
    let events = bus.subscribe();
    let (factory, sinks, _) = capture_factory(SinkFormat {
        sample_rate: 8_000,
        channels: 1,
    });
    let player = Player::spawn(factory, resolver(&files), bus.clone());

    player.set_volume(1.0);
    player.play(vec![TrackId(1), TrackId(2)], 0);
    let deadline = Instant::now() + Duration::from_secs(5);
    let ended = Event::PlayerStateChanged {
        state: PlaybackState::Ended,
        track: Some(TrackId(2)),
    };
    loop {
        let e = events
            .recv_timeout(deadline.saturating_duration_since(Instant::now()))
            .expect("no Ended event");
        if e == ended {
            break;
        }
    }

    let state = player.state().expect("thread answers");
    assert_eq!(state.state, PlaybackState::Ended);
    assert_eq!(sinks.lock().unwrap()[0].played().len(), 8_000);
    drop(player);
    assert_eq!(bus.subscriber_count(), 1, "thread joined, bus intact");
}
