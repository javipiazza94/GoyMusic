//! Needs a real output device: `cargo test -p goymusic-audio --test cpal_device -- --ignored`

use std::time::{Duration, Instant};

use goymusic_audio::cpal_sink::CpalSink;
use goymusic_audio::sink::AudioSink;

#[test]
#[ignore = "needs an audio device"]
fn default_device_consumes_written_audio() {
    let mut sink = CpalSink::open_default().expect("default output device");
    let format = sink.format();
    sink.set_volume(0.0); // silent check
    let second = vec![0.1; format.sample_rate as usize * format.channels as usize / 2];
    assert!(sink.write(&second) > 0);
    let deadline = Instant::now() + Duration::from_secs(3);
    while sink.played_frames() == 0 && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(sink.played_frames() > 0);
    sink.clear();
    assert_eq!(sink.queued_frames(), 0);
}
