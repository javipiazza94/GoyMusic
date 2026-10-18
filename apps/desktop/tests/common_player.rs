// Shared by the MPRIS tests (included with #[path]).
use goymusic_core::events::PlaybackState;
use goymusic_core::model::Track;
use goymusic_core::queue::RepeatMode;
use goymusic_core::TrackId;
use goymusic_services::PlayerView;

pub fn playing(title: &str) -> PlayerView {
    PlayerView {
        state: PlaybackState::Playing,
        track: Some(Track {
            id: TrackId(7),
            path: "/m/7.flac".into(),
            title: title.into(),
            artists: vec!["Ana".into()],
            album: None,
            album_id: None,
            album_artist: None,
            track_no: None,
            disc_no: None,
            year: None,
            genre: None,
            duration_ms: 200_000,
        }),
        position_ms: 1_000,
        volume: 0.8,
        replaygain: false,
        shuffle: false,
        repeat: RepeatMode::Off,
        user_queue: vec![],
        up_next: vec![],
    }
}
