#![allow(dead_code)] // each test binary uses a different subset

use std::f32::consts::PI;
use std::path::Path;

use lofty::config::WriteOptions;
use lofty::id3::v2::{Frame, Id3v2Tag, UniqueFileIdentifierFrame};
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
    pub year: Option<i32>,
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
            year: None,
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
    if let Some(year) = meta.year {
        tag.insert_text(ItemKey::RecordingDate, year.to_string());
    }
    // The MusicBrainz recording id is an ID3v2 UFID frame (binary), not a text frame, so
    // `insert_text` would drop it. Write it through the concrete ID3v2 tag.
    if let Some(id) = meta.mb_recording_id {
        let mut id3 = Id3v2Tag::from(tag);
        let ufid = UniqueFileIdentifierFrame::new("http://musicbrainz.org".to_string(), id.as_bytes().to_vec());
        id3.insert(Frame::UniqueFileIdentifier(ufid));
        id3.save_to_path(path, WriteOptions::default()).unwrap();
    } else {
        tag.save_to_path(path, WriteOptions::default()).unwrap();
    }
}

/// Writes a solid-colour PNG of `width`×`height`.
pub fn write_png(path: &Path, width: u32, height: u32) {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).unwrap();
    }
    image::RgbImage::from_pixel(width, height, image::Rgb([200, 40, 90]))
        .save(path)
        .unwrap();
}

pub fn png_bytes(width: u32, height: u32) -> Vec<u8> {
    let mut out = std::io::Cursor::new(Vec::new());
    image::RgbImage::from_pixel(width, height, image::Rgb([20, 120, 200]))
        .write_to(&mut out, image::ImageFormat::Png)
        .unwrap();
    out.into_inner()
}

/// Adds a front-cover picture to the file's existing tag.
pub fn embed_cover(audio: &Path, png: Vec<u8>) {
    use lofty::picture::{MimeType, Picture, PictureType};
    let mut tagged = lofty::read_from_path(audio).unwrap();
    let tag = tagged.primary_tag_mut().expect("file has a tag");
    tag.push_picture(Picture::new_unchecked(
        PictureType::CoverFront,
        Some(MimeType::Png),
        None,
        png,
    ));
    tag.save_to_path(audio, WriteOptions::default()).unwrap();
}
