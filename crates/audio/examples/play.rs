//! Manual check on a real device: plays the given files in order, gapless.
//! `cargo run -p goymusic-audio --example play -- a.flac b.flac`

use std::path::PathBuf;
use std::sync::Arc;

use goymusic_audio::cpal_sink::CpalSink;
use goymusic_audio::player::Player;
use goymusic_audio::sink::AudioSink;
use goymusic_core::events::{Event, EventBus, PlaybackState};
use goymusic_core::TrackId;

fn main() {
    let files: Vec<PathBuf> = std::env::args().skip(1).map(PathBuf::from).collect();
    if files.is_empty() {
        eprintln!("usage: play <file>...");
        std::process::exit(2);
    }
    let bus = Arc::new(EventBus::new());
    let events = bus.subscribe();
    let paths = files.clone();
    let player = Player::spawn(
        Box::new(|| CpalSink::open_default().map(|s| Box::new(s) as Box<dyn AudioSink>)),
        Box::new(move |id: TrackId| paths.get(id.0 as usize).cloned()),
        bus,
    );
    player.play((0..files.len() as i64).map(TrackId).collect(), 0);
    for event in events {
        match event {
            Event::PlayerStateChanged {
                state: PlaybackState::Ended,
                ..
            } => break,
            Event::PlayerStateChanged { state, track: Some(t) } => {
                println!("{state:?}: {}", files[t.0 as usize].display())
            }
            Event::PlaybackError { track, reason } => {
                eprintln!("error in {}: {reason}", files[track.0 as usize].display())
            }
            Event::AudioDeviceLost { reason } => eprintln!("device lost: {reason}"),
            _ => {}
        }
    }
}
