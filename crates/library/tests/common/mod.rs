#![allow(dead_code)] // each test binary uses a different subset

use std::f32::consts::PI;
use std::path::Path;

use lofty::config::WriteOptions;
use lofty::prelude::*;
use lofty::tag::{ItemKey, Tag, TagType};

const RATE: u32 = 8_000;

/// Writes a mono 8 kHz sine wave. Generated audio keeps copyrighted files out of the repo.
pub fn write_wav(path: &Path, seconds: f32) {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).unwrap();
    }
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: RATE,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(path, spec).unwrap();
    for i in 0..(RATE as f32 * seconds) as u32 {
        let sample = (i as f32 * 440.0 * 2.0 * PI / RATE as f32).sin() * f32::from(i16::MAX) * 0.2;
        writer.write_sample(sample as i16).unwrap();
    }
    writer.finalize().unwrap();
}

pub struct Meta<'a> {
    pub title: &'a str,
    pub artist: &'a str,
    pub album: Option<&'a str>,
    pub album_artist: Option<&'a str>,
    pub track: Option<u32>,
    pub mb_recording_id: Option<&'a str>,
}

impl<'a> Meta<'a> {
    pub fn new(title: &'a str, artist: &'a str) -> Self {
        Self {
            title,
            artist,
            album: None,
            album_artist: None,
            track: None,
            mb_recording_id: None,
        }
    }
}

pub fn write_tagged_wav(path: &Path, seconds: f32, meta: &Meta) {
    write_wav(path, seconds);
    let mut tag = Tag::new(TagType::Id3v2);
    tag.insert_text(ItemKey::TrackTitle, meta.title.into());
    tag.insert_text(ItemKey::TrackArtist, meta.artist.into());
    if let Some(album) = meta.album {
        tag.insert_text(ItemKey::AlbumTitle, album.into());
    }
    if let Some(album_artist) = meta.album_artist {
        tag.insert_text(ItemKey::AlbumArtist, album_artist.into());
    }
    if let Some(track) = meta.track {
        tag.insert_text(ItemKey::TrackNumber, track.to_string());
    }
    if let Some(id) = meta.mb_recording_id {
        tag.insert_text(ItemKey::MusicBrainzRecordingId, id.into());
    }
    tag.save_to_path(path, WriteOptions::default()).unwrap();
}
