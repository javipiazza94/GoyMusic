use goymusic_core::model::Track;
use goymusic_core::{AlbumId, TrackId};

#[test]
fn ids_serialize_as_plain_numbers() {
    assert_eq!(serde_json::to_string(&TrackId(5)).unwrap(), "5");
    assert_eq!(serde_json::from_str::<AlbumId>("9").unwrap(), AlbumId(9));
}

#[test]
fn track_roundtrips_through_json() {
    let track = Track {
        id: TrackId(1),
        path: "/music/a.flac".into(),
        title: "Song".into(),
        artists: vec!["Artist".into()],
        album: Some("Album".into()),
        album_id: Some(AlbumId(2)),
        album_artist: None,
        track_no: Some(3),
        disc_no: None,
        year: Some(1997),
        genre: None,
        duration_ms: 1234,
    };
    let json = serde_json::to_string(&track).unwrap();
    assert_eq!(serde_json::from_str::<Track>(&json).unwrap(), track);
}
