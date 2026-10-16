//! Needs ffmpeg with libmp3lame (MP3 encoders are not pure Rust):
//! `cargo test -p goymusic-audio --test mp3_gapless -- --ignored`
mod common;

use std::path::Path;
use std::process::Command;

use common::{heard, longest_quiet_run, sine, write_wav, Rig};
use goymusic_audio::sink::SinkFormat;
use goymusic_core::TrackId;

fn to_mp3(wav: &Path, mp3: &Path) {
    let status = Command::new("ffmpeg")
        .args(["-loglevel", "error", "-y", "-i"])
        .arg(wav)
        .args(["-c:a", "libmp3lame", "-b:a", "192k"])
        .arg(mp3)
        .status()
        .expect("ffmpeg not installed");
    assert!(status.success());
}

#[test]
#[ignore = "needs ffmpeg with libmp3lame"]
fn mp3_album_plays_without_gaps() {
    let dir = tempfile::tempdir().unwrap();
    let mut files = Vec::new();
    for (i, (start, frames)) in [(0u64, 50_000u64), (50_000, 40_000)].iter().enumerate() {
        let wav = dir.path().join(format!("{i}.wav"));
        let mp3 = dir.path().join(format!("{i}.mp3"));
        write_wav(&wav, 44_100, 2, &sine(44_100, 2, 440.0, *start, *frames));
        to_mp3(&wav, &mp3);
        files.push((i as i64 + 1, mp3));
    }
    let mut rig = Rig::new(
        SinkFormat {
            sample_rate: 44_100,
            channels: 2,
        },
        &files,
    );

    rig.engine.play_context(vec![TrackId(1), TrackId(2)], 0);
    rig.run(10_000);

    let played = rig.sink(0).played();
    assert_eq!(played.len() / 2, 90_000, "encoder delay and padding trimmed exactly");
    assert!(
        longest_quiet_run(&played[2_000..], 2, 0.01) <= 3,
        "silence at the boundary"
    );
    assert_eq!(heard(&rig.drain_events()), vec![1, 2]);
}
