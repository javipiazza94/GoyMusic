//! What the desktop integrations (MPRIS, notifications, tray) need from the player. `Services`
//! implements it; tests use [`fake::RecordingControl`], so the integrations run without audio.

use std::path::PathBuf;
use std::sync::Arc;

use goymusic_core::events::PlaybackState;
use goymusic_core::queue::RepeatMode;
use goymusic_core::AlbumId;
use goymusic_library::art::ArtSize;
use goymusic_services::{PlayerView, Services};

pub trait PlayerControl: Send + Sync + 'static {
    /// `None` once the player is gone (shutdown).
    fn view(&self) -> Option<PlayerView>;
    /// The large cached cover of `album`, when it has one.
    fn art_path(&self, album: AlbumId) -> Option<PathBuf>;
    fn play(&self);
    fn pause(&self);
    fn next(&self);
    fn previous(&self);
    fn seek_to(&self, position_ms: u64);
    fn set_volume(&self, volume: f32);
    fn set_shuffle(&self, on: bool);
    fn set_repeat(&self, mode: RepeatMode);

    fn play_pause(&self) {
        match self.view().map(|v| v.state) {
            Some(PlaybackState::Playing) => self.pause(),
            _ => self.play(),
        }
    }
}

impl PlayerControl for Services {
    fn view(&self) -> Option<PlayerView> {
        self.player_view().ok()
    }
    fn art_path(&self, album: AlbumId) -> Option<PathBuf> {
        self.album_art(album, ArtSize::Large).ok().flatten()
    }
    fn play(&self) {
        self.resume();
    }
    fn pause(&self) {
        Services::pause(self);
    }
    fn next(&self) {
        Services::next(self);
    }
    fn previous(&self) {
        Services::previous(self);
    }
    fn seek_to(&self, position_ms: u64) {
        self.seek(position_ms);
    }
    fn set_volume(&self, volume: f32) {
        Services::set_volume(self, volume);
    }
    fn set_shuffle(&self, on: bool) {
        Services::set_shuffle(self, on);
    }
    fn set_repeat(&self, mode: RepeatMode) {
        Services::set_repeat(self, mode);
    }
}

pub type SharedControl = Arc<dyn PlayerControl>;

/// A player double for tests: a fixed view and a log of the calls it received.
#[doc(hidden)]
pub mod fake {
    use std::sync::Mutex;

    use super::*;

    #[derive(Default)]
    pub struct RecordingControl {
        pub view: Mutex<Option<PlayerView>>,
        pub art: Option<PathBuf>,
        pub calls: Mutex<Vec<String>>,
    }

    impl RecordingControl {
        pub fn with_view(view: PlayerView) -> Self {
            Self {
                view: Mutex::new(Some(view)),
                ..Self::default()
            }
        }
        fn record(&self, call: impl Into<String>) {
            self.calls.lock().unwrap().push(call.into());
        }
        pub fn calls(&self) -> Vec<String> {
            self.calls.lock().unwrap().clone()
        }
    }

    impl PlayerControl for RecordingControl {
        fn view(&self) -> Option<PlayerView> {
            self.view.lock().unwrap().clone()
        }
        fn art_path(&self, _album: AlbumId) -> Option<PathBuf> {
            self.art.clone()
        }
        fn play(&self) {
            self.record("play");
        }
        fn pause(&self) {
            self.record("pause");
        }
        fn next(&self) {
            self.record("next");
        }
        fn previous(&self) {
            self.record("previous");
        }
        fn seek_to(&self, position_ms: u64) {
            self.record(format!("seek {position_ms}"));
        }
        fn set_volume(&self, volume: f32) {
            self.record(format!("volume {volume}"));
        }
        fn set_shuffle(&self, on: bool) {
            self.record(format!("shuffle {on}"));
        }
        fn set_repeat(&self, mode: RepeatMode) {
            self.record(format!("repeat {mode:?}"));
        }
    }
}
