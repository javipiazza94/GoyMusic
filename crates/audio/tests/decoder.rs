mod common;

use common::{ramp, sine, to_i16, write_flac, write_wav};
use goymusic_audio::decoder::Decoder;
use goymusic_audio::AudioError;

fn decode_all(d: &mut Decoder) -> Vec<f32> {
    let mut out = Vec::new();
    while let Some(chunk) = d.next_chunk().unwrap() {
        out.extend(chunk);
    }
    out
}

#[test]
fn wav_decodes_to_the_exact_samples() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("a.wav");
    let samples = sine(44_100, 2, 440.0, 0, 10_000);
    write_wav(&path, 44_100, 2, &samples);

    let mut d = Decoder::open(&path).unwrap();
    assert_eq!(
        (d.sample_rate(), d.channels(), d.total_frames()),
        (44_100, 2, Some(10_000))
    );
    assert_eq!(to_i16(&decode_all(&mut d)), samples);
}

#[test]
fn flac_decodes_to_the_exact_samples() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("a.flac");
    let samples = sine(48_000, 1, 1000.0, 0, 12_345);
    write_flac(&path, 48_000, 1, &samples);

    let mut d = Decoder::open(&path).unwrap();
    assert_eq!((d.sample_rate(), d.channels()), (48_000, 1));
    assert_eq!(to_i16(&decode_all(&mut d)), samples);
}

#[test]
fn garbage_and_missing_files_are_open_errors() {
    let dir = tempfile::tempdir().unwrap();
    let bad = dir.path().join("bad.mp3");
    std::fs::write(&bad, b"definitely not audio").unwrap();
    assert!(matches!(Decoder::open(&bad), Err(AudioError::Open { .. })));
    assert!(matches!(
        Decoder::open(&dir.path().join("missing.flac")),
        Err(AudioError::Open { .. })
    ));
}

#[test]
fn truncated_flac_returns_what_it_can_without_panicking() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("cut.flac");
    write_flac(&path, 44_100, 1, &sine(44_100, 1, 440.0, 0, 44_100));
    let bytes = std::fs::read(&path).unwrap();
    std::fs::write(&path, &bytes[..bytes.len() / 2]).unwrap();

    let mut d = Decoder::open(&path).unwrap();
    let mut frames = 0;
    while let Ok(Some(c)) = d.next_chunk() {
        frames += c.len();
    }
    assert!(frames > 0 && frames < 44_100);
}

#[test]
fn seek_lands_on_the_exact_frame() {
    let dir = tempfile::tempdir().unwrap();
    for name in ["ramp.wav", "ramp.flac"] {
        let path = dir.path().join(name);
        let samples = ramp(30_000);
        if name.ends_with(".wav") {
            write_wav(&path, 8_000, 1, &samples);
        } else {
            write_flac(&path, 8_000, 1, &samples);
        }
        let mut d = Decoder::open(&path).unwrap();
        for target in [12_000u64, 4_321, 0, 29_999] {
            d.seek(target).unwrap();
            let first = d.next_chunk().unwrap().unwrap()[0];
            assert_eq!(to_i16(&[first])[0] as u64, target, "{name} seek to {target}");
        }
    }
}
