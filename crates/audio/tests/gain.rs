mod common;

use common::{sine, write_wav};
use goymusic_audio::gain::{apply, parse_number, volume_to_amplitude, ReplayGain};
use lofty::config::WriteOptions;
use lofty::prelude::*;
use lofty::tag::{ItemKey, Tag, TagType};

#[test]
fn parses_replaygain_tag_values() {
    assert_eq!(parse_number("-6.48 dB"), Some(-6.48));
    assert_eq!(parse_number(" +1.5 db "), Some(1.5));
    assert_eq!(parse_number("0.988547"), Some(0.988547));
    assert_eq!(parse_number("loud"), None);
    assert_eq!(parse_number("NaN dB"), None);
}

#[test]
fn factor_follows_gain_and_respects_peak() {
    let quieter = ReplayGain {
        track_gain_db: Some(-6.0),
        track_peak: None,
    };
    assert!((quieter.factor() - 0.501).abs() < 0.001);
    let louder = ReplayGain {
        track_gain_db: Some(6.0),
        track_peak: Some(0.8),
    };
    assert_eq!(louder.factor(), 1.25, "+6 dB would clip a 0.8 peak, capped at 1/0.8");
    assert_eq!(ReplayGain::default().factor(), 1.0);
}

#[test]
fn reads_replaygain_from_tags() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("rg.wav");
    write_wav(&path, 8_000, 1, &sine(8_000, 1, 440.0, 0, 800));
    let mut tag = Tag::new(TagType::Id3v2);
    tag.insert_text(ItemKey::ReplayGainTrackGain, "-7.25 dB".into());
    tag.insert_text(ItemKey::ReplayGainTrackPeak, "0.95".into());
    tag.save_to_path(&path, WriteOptions::default()).unwrap();

    let rg = ReplayGain::read(&path);
    assert_eq!(
        rg,
        ReplayGain {
            track_gain_db: Some(-7.25),
            track_peak: Some(0.95)
        }
    );
    assert_eq!(ReplayGain::read(&dir.path().join("missing.wav")), ReplayGain::default());
}

#[test]
fn volume_curve_and_clamping() {
    assert_eq!(volume_to_amplitude(1.0), 1.0);
    assert_eq!(volume_to_amplitude(0.5), 0.125);
    assert_eq!(volume_to_amplitude(-3.0), 0.0);
    assert_eq!(volume_to_amplitude(7.0), 1.0);
    let mut samples = [0.5, -0.9];
    apply(&mut samples, 2.0);
    assert_eq!(samples, [1.0, -1.0]);
}
