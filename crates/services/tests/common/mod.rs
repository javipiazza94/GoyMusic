#![allow(dead_code)] // each test binary uses a different subset

#[path = "../../../library/tests/common/mod.rs"]
mod lib_common;

use std::path::{Path, PathBuf};
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use goymusic_audio::sink::{AudioSink, CaptureHandle, CaptureSink, SinkFactory, SinkFormat};
use goymusic_core::events::Event;
use goymusic_services::{Services, ServicesConfig};
pub use lib_common::*;

/// Capture sinks in hold mode: audio queues up but is never "heard", so a track stays
/// current until the test moves on. The handles let a test release audio if it needs to.
pub fn capture_sink() -> (SinkFactory, Arc<Mutex<Vec<CaptureHandle>>>) {
    let handles = Arc::new(Mutex::new(Vec::new()));
    let made = handles.clone();
    let factory: SinkFactory = Box::new(move || {
        let (sink, handle) = CaptureSink::new(SinkFormat {
            sample_rate: 8_000,
            channels: 1,
        });
        handle.hold(true);
        made.lock().unwrap().push(handle);
        Ok(Box::new(sink) as Box<dyn AudioSink>)
    });
    (factory, handles)
}

pub fn config(dir: &Path) -> ServicesConfig {
    ServicesConfig::in_dir(&dir.join("state"))
}

pub fn start(dir: &Path) -> Services {
    start_with(config(dir))
}

pub fn start_with(config: ServicesConfig) -> Services {
    Services::start(config, capture_sink().0).expect("services start")
}

/// Waits for the first event matching `pred`; panics listing what was seen instead.
pub fn wait_for(rx: &Receiver<Event>, timeout: Duration, pred: impl Fn(&Event) -> bool) -> Event {
    let deadline = Instant::now() + timeout;
    let mut seen = Vec::new();
    loop {
        match rx.recv_timeout(deadline.saturating_duration_since(Instant::now())) {
            Ok(e) if pred(&e) => return e,
            Ok(e) => seen.push(e),
            Err(_) => panic!("event not seen within {timeout:?}; got {seen:?}"),
        }
    }
}

/// Every event received during `period`.
pub fn collect_for(rx: &Receiver<Event>, period: Duration) -> Vec<Event> {
    let deadline = Instant::now() + period;
    let mut events = Vec::new();
    while let Ok(e) = rx.recv_timeout(deadline.saturating_duration_since(Instant::now())) {
        events.push(e);
    }
    events
}

/// Scans started among `events`. A scan opens with `ScanProgress { done: 0 }`; one with
/// nothing to read also closes with `{ done: 0, total: 0 }` right after, counted once.
pub fn scans_started(events: &[Event]) -> usize {
    let mut count = 0;
    let mut previous_empty = false;
    for e in events {
        match e {
            Event::ScanProgress { done: 0, total: 0 } if previous_empty => previous_empty = false,
            Event::ScanProgress { done: 0, total } => {
                count += 1;
                previous_empty = *total == 0;
            }
            _ => previous_empty = false,
        }
    }
    count
}

/// `n` one-second tracks of `album` by `artist` in `root/album`, numbered from 1.
pub fn album_fixture(root: &Path, album: &str, artist: &str, n: u32) -> Vec<PathBuf> {
    (1..=n)
        .map(|i| {
            let path = root.join(album).join(format!("{i:02}.wav"));
            let title = format!("{album} {i}");
            write_tagged_wav(
                &path,
                1.0,
                &Meta {
                    album: Some(album),
                    album_artist: Some(artist),
                    track: Some(i),
                    ..Meta::new(&title, artist)
                },
            );
            path
        })
        .collect()
}

pub const LONG: Duration = Duration::from_secs(10);
