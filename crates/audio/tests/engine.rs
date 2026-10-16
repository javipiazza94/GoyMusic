mod common;

use std::path::PathBuf;

use common::{heard, longest_quiet_run, ramp, sine, to_i16, write_flac, write_wav, Rig};
use goymusic_audio::sink::{SinkError, SinkFormat};
use goymusic_core::events::{Event, PlaybackState};
use goymusic_core::queue::RepeatMode;
use goymusic_core::TrackId;

const CD: SinkFormat = SinkFormat {
    sample_rate: 44_100,
    channels: 2,
};
const LOW: SinkFormat = SinkFormat {
    sample_rate: 8_000,
    channels: 1,
};

/// Writes `frames` frames of one continuous 8 kHz mono ramp-free sine per track.
fn short_tracks(dir: &std::path::Path, seconds: &[u64]) -> Vec<(i64, PathBuf)> {
    let mut start = 0;
    seconds
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let path = dir.join(format!("{i}.wav"));
            let frames = s * 8_000;
            write_wav(&path, 8_000, 1, &sine(8_000, 1, 440.0, start, frames));
            start += frames;
            (i as i64 + 1, path)
        })
        .collect()
}

#[test]
fn gapless_flac_and_wav_join_sample_exact() {
    let dir = tempfile::tempdir().unwrap();
    let parts = [
        (0u64, 30_000u64, "a.flac"),
        (30_000, 20_000, "b.wav"),
        (50_000, 11_111, "c.flac"),
    ];
    let mut files = Vec::new();
    for (i, (start, frames, name)) in parts.iter().enumerate() {
        let path = dir.path().join(name);
        let samples = sine(44_100, 2, 440.0, *start, *frames);
        if name.ends_with(".flac") {
            write_flac(&path, 44_100, 2, &samples);
        } else {
            write_wav(&path, 44_100, 2, &samples);
        }
        files.push((i as i64 + 1, path));
    }
    let mut rig = Rig::new(CD, &files);

    rig.engine.play_context(vec![TrackId(1), TrackId(2), TrackId(3)], 0);
    rig.run(10_000);

    assert_eq!(
        to_i16(&rig.sink(0).played()),
        sine(44_100, 2, 440.0, 0, 61_111),
        "not one continuous wave"
    );
    let events = rig.drain_events();
    assert_eq!(heard(&events), vec![1, 2, 3]);
    assert_eq!(rig.engine.state(), PlaybackState::Ended);
    assert!(events.contains(&Event::PlayerStateChanged {
        state: PlaybackState::Ended,
        track: Some(TrackId(3))
    }));
}

#[test]
fn gapless_through_the_resampler_has_no_silence() {
    let dir = tempfile::tempdir().unwrap();
    let mut files = Vec::new();
    for (i, (start, frames)) in [(0u64, 30_000u64), (30_000, 20_000)].iter().enumerate() {
        let path = dir.path().join(format!("{i}.flac"));
        write_flac(&path, 44_100, 1, &sine(44_100, 1, 440.0, *start, *frames));
        files.push((i as i64 + 1, path));
    }
    let format = SinkFormat {
        sample_rate: 48_000,
        channels: 2,
    };
    let mut rig = Rig::new(format, &files);

    rig.engine.play_context(vec![TrackId(1), TrackId(2)], 0);
    rig.run(10_000);

    let played = rig.sink(0).played();
    let expected_frames = 30_000 * 48_000 / 44_100 + 20_000 * 48_000 / 44_100;
    assert!(
        (played.len() as i64 / 2 - expected_frames as i64).abs() <= 2,
        "frames lost or added"
    );
    // a 440 Hz sine at 0.25 is under 0.01 for at most 2 samples per zero crossing
    assert!(
        longest_quiet_run(&played[2_000..], 2, 0.01) <= 3,
        "silence at the track boundary"
    );
}

#[test]
fn next_track_is_preloaded_ten_seconds_before_the_end() {
    let dir = tempfile::tempdir().unwrap();
    let files = short_tracks(dir.path(), &[12, 1]);
    let mut rig = Rig::new(LOW, &files);
    let sink = rig.sink(0);
    sink.hold(true);

    rig.engine.play_context(vec![TrackId(1), TrackId(2)], 0);
    let mut preloaded_at = None;
    for _ in 0..1_000 {
        rig.engine.pump();
        if rig.engine.preloaded().is_some() {
            preloaded_at = Some(rig.engine.position_ms());
            break;
        }
        sink.play(400, 1); // 50 ms of "device time" per pump
    }
    // the decoder runs ahead of the device by the 500 ms buffer plus one packet
    let at = preloaded_at.expect("never preloaded");
    assert!((1_300..=2_000).contains(&at), "preloaded at {at} ms");
    assert_eq!(rig.engine.preloaded(), Some(TrackId(2)));
}

#[test]
fn editing_the_queue_replaces_the_preloaded_track() {
    let dir = tempfile::tempdir().unwrap();
    let files = short_tracks(dir.path(), &[3, 1, 1]);
    let mut rig = Rig::new(LOW, &files);
    let sink = rig.sink(0);
    sink.hold(true);
    rig.engine.play_context(vec![TrackId(1), TrackId(2)], 0);
    rig.engine.pump();
    assert_eq!(rig.engine.preloaded(), Some(TrackId(2)), "3 s track: preloaded at once");

    rig.engine.queue_add(&[TrackId(3)]);
    assert_eq!(rig.engine.preloaded(), None);
    rig.engine.pump();
    assert_eq!(rig.engine.preloaded(), Some(TrackId(3)));

    sink.hold(false);
    sink.play(usize::MAX / 2, 1);
    rig.run(10_000);
    assert_eq!(heard(&rig.drain_events()), vec![1, 3, 2]);
}

#[test]
fn queue_edit_while_the_successor_is_buffered_restarts_cleanly() {
    let dir = tempfile::tempdir().unwrap();
    let files = short_tracks(dir.path(), &[1, 1, 1]);
    let mut rig = Rig::new(LOW, &files);
    let sink = rig.sink(0);
    sink.hold(true);
    rig.engine.play_context(vec![TrackId(1), TrackId(2)], 0);
    for _ in 0..100 {
        rig.engine.pump();
        if rig.engine.buffered_tracks().len() == 2 {
            break;
        }
        sink.play(400, 1);
    }
    assert_eq!(rig.engine.buffered_tracks(), vec![TrackId(1), TrackId(2)]);
    let clears = sink.clears();

    rig.engine.queue_add(&[TrackId(3)]);

    assert_eq!(rig.engine.buffered_tracks(), vec![TrackId(1)]);
    assert_eq!(sink.clears(), clears + 1);
    sink.hold(false);
    sink.play(usize::MAX / 2, 1);
    rig.run(10_000);
    assert_eq!(heard(&rig.drain_events()), vec![1, 3, 2]);
    let total = sink.played().len() as i64;
    assert!((total - 24_000).abs() <= 8, "1 s lost or repeated: {total} frames");
}

#[test]
fn unreadable_and_unknown_tracks_are_reported_and_skipped() {
    let dir = tempfile::tempdir().unwrap();
    let mut files = short_tracks(dir.path(), &[1, 1]);
    let bad = dir.path().join("bad.flac");
    std::fs::write(&bad, b"not audio").unwrap();
    files.push((3, bad));
    let mut rig = Rig::new(LOW, &files);

    // 4 is not in the library at all
    rig.engine
        .play_context(vec![TrackId(4), TrackId(1), TrackId(3), TrackId(2)], 0);
    rig.run(10_000);

    let events = rig.drain_events();
    assert_eq!(heard(&events), vec![1, 2]);
    let errors: Vec<i64> = events
        .iter()
        .filter_map(|e| match e {
            Event::PlaybackError { track, .. } => Some(track.0),
            _ => None,
        })
        .collect();
    assert_eq!(errors, vec![4, 3]);
    assert_eq!(rig.engine.state(), PlaybackState::Ended);
}

#[test]
fn a_queue_of_broken_files_on_repeat_ends_instead_of_looping() {
    let dir = tempfile::tempdir().unwrap();
    let files: Vec<(i64, PathBuf)> = (1..=2)
        .map(|i| {
            let p = dir.path().join(format!("{i}.mp3"));
            std::fs::write(&p, b"garbage").unwrap();
            (i, p)
        })
        .collect();
    let mut rig = Rig::new(LOW, &files);
    rig.engine.set_repeat(RepeatMode::All);

    rig.engine.play_context(vec![TrackId(1), TrackId(2)], 0);

    assert_eq!(rig.engine.state(), PlaybackState::Ended);
    let errors = rig
        .drain_events()
        .iter()
        .filter(|e| matches!(e, Event::PlaybackError { .. }))
        .count();
    assert_eq!(errors, 3, "each queued track tried once, plus one");
}

#[test]
fn repeat_one_loops_the_track_gaplessly() {
    let dir = tempfile::tempdir().unwrap();
    let files = short_tracks(dir.path(), &[1]);
    let mut rig = Rig::new(LOW, &files);
    rig.engine.set_repeat(RepeatMode::One);

    rig.engine.play_context(vec![TrackId(1)], 0);
    for _ in 0..40 {
        rig.engine.pump();
    }

    let once = sine(8_000, 1, 440.0, 0, 8_000);
    let played = to_i16(&rig.sink(0).played());
    assert!(played.len() >= 16_000);
    assert_eq!(&played[..8_000], &once[..]);
    assert_eq!(&played[8_000..16_000], &once[..]);
}

#[test]
fn seek_lands_on_the_exact_sample() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("ramp.flac");
    write_flac(&path, 8_000, 1, &ramp(30_000));
    let mut rig = Rig::new(LOW, &[(1, path)]);
    let sink = rig.sink(0);
    sink.hold(true);
    rig.engine.play_context(vec![TrackId(1)], 0);
    rig.engine.pump();

    rig.engine.seek(1_500);
    assert_eq!(rig.engine.position_ms(), 1_500);
    rig.engine.pump();
    sink.play(1, 1);

    let played = sink.played();
    assert_eq!(to_i16(&played[played.len() - 1..]), vec![12_000]);
}

#[test]
fn pause_resume_and_position() {
    let dir = tempfile::tempdir().unwrap();
    let files = short_tracks(dir.path(), &[5]);
    let mut rig = Rig::new(LOW, &files);
    let sink = rig.sink(0);
    sink.hold(true);
    rig.engine.play_context(vec![TrackId(1)], 0);
    rig.advance(0, 8_000);
    assert_eq!(rig.engine.position_ms(), 1_000);

    rig.engine.pause();
    sink.play(8_000, 1); // a paused device plays nothing
    assert_eq!(
        (rig.engine.state(), rig.engine.position_ms()),
        (PlaybackState::Paused, 1_000)
    );
    rig.engine.resume();
    assert_eq!(rig.engine.state(), PlaybackState::Playing);
    rig.advance(0, 4_000);
    assert_eq!(rig.engine.position_ms(), 1_500);
}

#[test]
fn next_previous_and_restart() {
    let dir = tempfile::tempdir().unwrap();
    let files = short_tracks(dir.path(), &[5, 5, 5]);
    let mut rig = Rig::new(LOW, &files);
    let sink = rig.sink(0);
    sink.hold(true);
    rig.engine.play_context(vec![TrackId(1), TrackId(2), TrackId(3)], 0);

    rig.engine.next();
    assert_eq!(rig.engine.current(), Some(TrackId(2)));
    rig.engine.previous(); // at 0 ms: back to the previous track
    assert_eq!(rig.engine.current(), Some(TrackId(1)));

    rig.advance(0, 32_000);
    assert!(rig.engine.position_ms() > 3_000);
    rig.engine.previous(); // after 3 s: restart
    assert_eq!((rig.engine.current(), rig.engine.position_ms()), (Some(TrackId(1)), 0));
}

#[test]
fn volume_is_immediate_and_replaygain_applies_to_new_tracks() {
    use lofty::config::WriteOptions;
    use lofty::prelude::*;
    use lofty::tag::{ItemKey, Tag, TagType};
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("loud.wav");
    write_wav(&path, 8_000, 1, &vec![16_384; 800]); // constant 0.5
    let mut tag = Tag::new(TagType::Id3v2);
    tag.insert_text(ItemKey::ReplayGainTrackGain, "-6.02 dB".into());
    tag.save_to_path(&path, WriteOptions::default()).unwrap();
    let mut rig = Rig::new(LOW, &[(1, path)]);

    rig.engine.set_volume(0.5);
    rig.engine.play_context(vec![TrackId(1)], 0);
    rig.run(1_000);
    assert!((rig.sink(0).played()[0] - 0.5 * 0.125).abs() < 1e-4);

    rig.engine.set_volume(1.0);
    rig.engine.set_replaygain(true);
    rig.engine.play_context(vec![TrackId(1)], 0);
    rig.run(1_000);
    assert!((rig.sink(0).played().last().unwrap() - 0.25).abs() < 1e-3);
}

#[test]
fn lost_device_pauses_and_resumes_on_the_default_device() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("ramp.wav");
    write_wav(&path, 8_000, 1, &ramp(30_000));
    let mut rig = Rig::new(LOW, &[(1, path)]);
    let first = rig.sink(0);
    first.hold(true);
    rig.engine.play_context(vec![TrackId(1)], 0);
    rig.advance(0, 8_000);

    first.fail(SinkError::Device("unplugged".into()));
    rig.factory_fails.store(true, std::sync::atomic::Ordering::SeqCst);
    rig.engine.pump();

    assert_eq!(rig.engine.state(), PlaybackState::Paused);
    assert!(rig.engine.needs_sink());
    assert_eq!(rig.engine.position_ms(), 1_000);
    assert!(rig.drain_events().contains(&Event::AudioDeviceLost {
        reason: "audio device error: unplugged".into()
    }));
    assert!(!rig.engine.try_reopen_sink(), "still no device");

    rig.factory_fails.store(false, std::sync::atomic::Ordering::SeqCst);
    assert!(rig.engine.try_reopen_sink());
    assert_eq!(
        (rig.engine.state(), rig.engine.position_ms()),
        (PlaybackState::Paused, 1_000)
    );
    rig.engine.resume();
    rig.engine.pump();
    assert_eq!(
        to_i16(&rig.sink(1).played()[..1]),
        vec![8_000],
        "resumes at the same sample"
    );
}

#[test]
fn restore_starts_paused_at_the_saved_position() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("ramp.wav");
    write_wav(&path, 8_000, 1, &ramp(30_000));
    let mut rig = Rig::new(LOW, &[(1, path)]);
    let mut queue = goymusic_core::queue::Queue::new();
    queue.play_context(vec![TrackId(1)], 0, &mut rand::thread_rng());

    rig.engine.restore(queue, 1_500);

    let snapshot = rig.engine.snapshot();
    assert_eq!(
        (snapshot.state, snapshot.track, snapshot.position_ms),
        (PlaybackState::Paused, Some(TrackId(1)), 1_500)
    );
    rig.engine.pump();
    assert!(rig.sink(0).played().is_empty(), "paused: nothing plays");
    rig.engine.resume();
    rig.engine.pump();
    assert_eq!(to_i16(&rig.sink(0).played()[..1]), vec![12_000]);
}

#[test]
fn position_events_only_while_playing() {
    let dir = tempfile::tempdir().unwrap();
    let files = short_tracks(dir.path(), &[5]);
    let mut rig = Rig::new(LOW, &files);
    rig.sink(0).hold(true);
    rig.engine.play_context(vec![TrackId(1)], 0);
    rig.engine.pump();
    rig.sink(0).play(4_000, 1);
    rig.drain_events();

    rig.engine.publish_position();
    rig.engine.pause();
    rig.engine.publish_position();

    let positions: Vec<Event> = rig
        .drain_events()
        .into_iter()
        .filter(|e| matches!(e, Event::PositionChanged { .. }))
        .collect();
    assert_eq!(
        positions,
        vec![Event::PositionChanged {
            track: TrackId(1),
            position_ms: 500
        }]
    );
}

// ---- final review fixes ----

#[test]
fn empty_or_undecodable_tracks_on_repeat_end_instead_of_spinning() {
    let dir = tempfile::tempdir().unwrap();
    let empty = dir.path().join("empty.wav");
    write_wav(&empty, 8_000, 1, &[]);
    let corrupt = dir.path().join("corrupt.flac");
    write_flac(&corrupt, 8_000, 1, &sine(8_000, 1, 440.0, 0, 8_000));
    let mut bytes = std::fs::read(&corrupt).unwrap();
    for b in bytes.iter_mut().skip(42) {
        *b = 0xA5; // header intact, every frame garbage
    }
    std::fs::write(&corrupt, bytes).unwrap();
    let mut rig = Rig::new(LOW, &[(1, empty), (2, corrupt)]);
    rig.engine.set_repeat(RepeatMode::All);

    rig.engine.play_context(vec![TrackId(1), TrackId(2)], 0);
    rig.run(1_000);

    assert_eq!(rig.engine.state(), PlaybackState::Ended);
    assert!(rig.drain_events().len() < 100, "event storm");
}

#[test]
fn resampled_gapless_join_matches_the_unsplit_track() {
    let dir = tempfile::tempdir().unwrap();
    let whole = dir.path().join("whole.flac");
    write_flac(&whole, 44_100, 1, &sine(44_100, 1, 440.0, 0, 50_000));
    let mut split = Vec::new();
    for (i, (start, frames)) in [(0u64, 30_000u64), (30_000, 20_000)].iter().enumerate() {
        let path = dir.path().join(format!("{i}.flac"));
        write_flac(&path, 44_100, 1, &sine(44_100, 1, 440.0, *start, *frames));
        split.push((i as i64 + 1, path));
    }
    let format = SinkFormat {
        sample_rate: 48_000,
        channels: 1,
    };
    let mut reference = Rig::new(format, &[(1, whole)]);
    reference.engine.play_context(vec![TrackId(1)], 0);
    reference.run(10_000);
    let mut rig = Rig::new(format, &split);
    rig.engine.play_context(vec![TrackId(1), TrackId(2)], 0);
    rig.run(10_000);

    let (a, b) = (reference.sink(0).played(), rig.sink(0).played());
    assert_eq!(a.len(), b.len());
    let worst = a.iter().zip(&b).map(|(x, y)| (x - y).abs()).fold(0f32, f32::max);
    assert!(worst < 1e-3, "click at the join: {worst}");
}

#[test]
fn previous_and_play_work_after_the_queue_ends() {
    let dir = tempfile::tempdir().unwrap();
    let files = short_tracks(dir.path(), &[1, 1]);
    let mut rig = Rig::new(LOW, &files);
    rig.engine.play_context(vec![TrackId(1), TrackId(2)], 0);
    rig.run(10_000);
    assert_eq!(rig.engine.state(), PlaybackState::Ended);

    rig.engine.previous();
    assert_eq!(
        (rig.engine.state(), rig.engine.current()),
        (PlaybackState::Playing, Some(TrackId(2)))
    );
    rig.run(10_000);

    rig.engine.resume();
    assert_eq!(
        (rig.engine.state(), rig.engine.current()),
        (PlaybackState::Playing, Some(TrackId(1)))
    );
}

#[test]
fn queue_edit_beyond_the_first_buffered_successor_resyncs() {
    let dir = tempfile::tempdir().unwrap();
    let files = short_tracks(dir.path(), &[1, 1, 1, 1]);
    let short = dir.path().join("short.wav");
    write_wav(&short, 8_000, 1, &sine(8_000, 1, 440.0, 0, 1_600)); // 0.2 s
    let mut files: Vec<(i64, PathBuf)> = files.into_iter().filter(|(id, _)| *id != 2).collect();
    files.push((2, short));
    let mut rig = Rig::new(LOW, &files);
    let sink = rig.sink(0);
    sink.hold(true);
    rig.engine.play_context(vec![TrackId(1), TrackId(4)], 0);
    rig.engine.queue_add(&[TrackId(2), TrackId(3)]);
    for _ in 0..100 {
        rig.engine.pump();
        if rig.engine.buffered_tracks().len() == 3 {
            break;
        }
        sink.play(400, 1);
    }
    assert_eq!(rig.engine.buffered_tracks(), vec![TrackId(1), TrackId(2), TrackId(3)]);

    rig.engine.queue_remove(1); // drop 3, already buffered behind 2

    sink.hold(false);
    sink.play(usize::MAX / 2, 1);
    rig.run(10_000);
    assert_eq!(heard(&rig.drain_events()), vec![1, 2, 4]);
}

#[test]
fn transient_device_error_reopens_without_pausing() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("ramp.wav");
    write_wav(&path, 8_000, 1, &ramp(30_000));
    let mut rig = Rig::new(LOW, &[(1, path)]);
    rig.sink(0).hold(true);
    rig.engine.play_context(vec![TrackId(1)], 0);
    rig.advance(0, 8_000);
    rig.drain_events();

    rig.sink(0).fail(SinkError::Device("xrun".into()));
    rig.engine.pump();

    assert_eq!(rig.engine.state(), PlaybackState::Playing);
    assert!(!rig.engine.needs_sink());
    assert!(!rig
        .drain_events()
        .iter()
        .any(|e| matches!(e, Event::AudioDeviceLost { .. })));
    rig.engine.pump();
    assert_eq!(to_i16(&rig.sink(1).played()[..1]), vec![8_000]);
}

#[test]
fn seek_and_restore_while_paused_publish_the_new_position() {
    let dir = tempfile::tempdir().unwrap();
    let files = short_tracks(dir.path(), &[5]);
    let mut rig = Rig::new(LOW, &files);
    rig.sink(0).hold(true);
    rig.engine.play_context(vec![TrackId(1)], 0);
    rig.engine.pause();
    rig.drain_events();

    rig.engine.seek(1_500);
    let queue = rig.engine.queue().clone();
    rig.engine.restore(queue, 2_500);

    let positions: Vec<Event> = rig
        .drain_events()
        .into_iter()
        .filter(|e| matches!(e, Event::PositionChanged { .. }))
        .collect();
    assert_eq!(
        positions,
        vec![
            Event::PositionChanged {
                track: TrackId(1),
                position_ms: 1_500
            },
            Event::PositionChanged {
                track: TrackId(1),
                position_ms: 2_500
            },
        ]
    );
}
