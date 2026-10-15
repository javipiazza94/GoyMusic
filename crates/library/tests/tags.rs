mod common;

use common::{write_tagged_wav, write_wav, Meta};
use goymusic_library::error::LibraryError;
use goymusic_library::tags::read_tags;

#[test]
fn reads_tags_and_duration() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("a.wav");
    let meta = Meta {
        album: Some("OK Computer"),
        album_artist: Some("Radiohead"),
        track: Some(3),
        ..Meta::new("Subterranean", "Radiohead")
    };
    write_tagged_wav(&path, 1.0, &meta);

    let tags = read_tags(&path).unwrap();
    assert_eq!(tags.title.as_deref(), Some("Subterranean"));
    assert_eq!(tags.artists, vec!["Radiohead".to_owned()]);
    assert_eq!(tags.album.as_deref(), Some("OK Computer"));
    assert_eq!(tags.album_artist.as_deref(), Some("Radiohead"));
    assert_eq!(tags.track_no, Some(3));
    assert!(
        (950..=1050).contains(&tags.duration_ms),
        "duration {}",
        tags.duration_ms
    );
}

#[test]
fn untagged_file_has_duration_but_no_title() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("plain.wav");
    write_wav(&path, 0.5);
    let tags = read_tags(&path).unwrap();
    assert_eq!(tags.title, None);
    assert!(tags.artists.is_empty());
    assert!(tags.duration_ms > 400);
}

#[test]
fn garbage_file_is_a_tags_error() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("bad.mp3");
    std::fs::write(&path, b"definitely not audio").unwrap();
    assert!(matches!(read_tags(&path), Err(LibraryError::Tags { .. })));
}
