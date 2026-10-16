use goymusic_core::model::{Playlist, Track};
use goymusic_core::queue::{Queue, RepeatMode};
use goymusic_core::{AlbumId, PlaylistId, TrackId};

#[test]
fn model_serializes_in_camel_case() {
    let track = Track {
        id: TrackId(7),
        path: "/m/a.flac".into(),
        title: "A".into(),
        artists: vec!["X".into()],
        album: Some("Al".into()),
        album_id: Some(AlbumId(3)),
        album_artist: None,
        track_no: Some(1),
        disc_no: None,
        year: Some(2001),
        genre: None,
        duration_ms: 1234,
    };
    let v = serde_json::to_value(&track).unwrap();
    assert_eq!(v["durationMs"], 1234);
    assert_eq!(v["albumId"], 3);
    assert_eq!(v["trackNo"], 1);
    assert_eq!(v["id"], 7); // ids stay plain numbers
    let p = serde_json::to_value(Playlist {
        id: PlaylistId(1),
        name: "p".into(),
        folder_id: None,
        track_count: 2,
    })
    .unwrap();
    assert_eq!(p["trackCount"], 2);
    assert!(p.get("folderId").is_some());
}

#[test]
fn saved_queues_keep_their_format() {
    // queue_state rows written before this change must still load
    let q: Queue = serde_json::from_str(&serde_json::to_string(&Queue::new()).unwrap()).unwrap();
    assert_eq!(q, Queue::new());
    assert_eq!(serde_json::to_value(RepeatMode::All).unwrap(), "all");
}
