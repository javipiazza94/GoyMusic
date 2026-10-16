//! Desktop notification when a new track starts playing, with its cover (spec §8).

use std::path::Path;
use std::sync::mpsc::Receiver;
use std::sync::Arc;
use std::thread::JoinHandle;

use goymusic_core::events::{Event, PlaybackState};
use goymusic_core::model::Track;
use goymusic_core::TrackId;
use notify_rust::{Hint, Notification};

use crate::control::SharedControl;

/// Shows one notification; errors are reported, never fatal.
pub trait Notifier: Send + 'static {
    fn show(&mut self, summary: &str, body: &str, image: Option<&Path>) -> Result<(), String>;
}

/// The freedesktop notification server (GNOME Shell). Each notification replaces the last
/// one instead of piling up.
#[derive(Default)]
pub struct DesktopNotifier {
    last: Option<u32>,
}

/// Icon name installed by the packages (`.deb` / AppImage).
const APP_ICON: &str = "goymusic";

impl Notifier for DesktopNotifier {
    fn show(&mut self, summary: &str, body: &str, image: Option<&Path>) -> Result<(), String> {
        let mut n = Notification::new();
        n.appname("GoyMusic")
            .summary(summary)
            .body(body)
            .icon(APP_ICON)
            .hint(Hint::Category("x-gnome.music".into()));
        if let Some(image) = image.and_then(Path::to_str) {
            n.image_path(image);
        }
        if let Some(id) = self.last {
            n.id(id);
        }
        let handle = n.show().map_err(|e| e.to_string())?;
        self.last = Some(handle.id());
        Ok(())
    }
}

/// `(title, "Artists — Album")`, leaving out whatever the tags do not have.
pub fn text_for(track: &Track) -> (String, String) {
    let artists = track.artists.join(", ");
    let body = match (artists.is_empty(), &track.album) {
        (false, Some(album)) => format!("{artists} — {album}"),
        (false, None) => artists,
        (true, Some(album)) => album.clone(),
        (true, None) => String::new(),
    };
    (track.title.clone(), body)
}

/// Decides which events deserve a notification: a track that starts playing and was not the
/// last one notified, while the main window is not in front of the user.
pub struct TrackNotifications<N: Notifier> {
    notifier: N,
    control: SharedControl,
    focused: Arc<dyn Fn() -> bool + Send + Sync>,
    last: Option<TrackId>,
}

impl<N: Notifier> TrackNotifications<N> {
    pub fn new(notifier: N, control: SharedControl, focused: Arc<dyn Fn() -> bool + Send + Sync>) -> Self {
        Self {
            notifier,
            control,
            focused,
            last: None,
        }
    }

    pub fn on_event(&mut self, event: &Event) {
        let Event::PlayerStateChanged {
            state: PlaybackState::Playing,
            track: Some(id),
        } = event
        else {
            return;
        };
        if self.last == Some(*id) || (self.focused)() {
            return;
        }
        let Some(track) = self.control.view().and_then(|v| v.track).filter(|t| t.id == *id) else {
            return;
        };
        // marked even if showing fails: a missing daemon must not be retried on every event
        self.last = Some(*id);
        let (summary, body) = text_for(&track);
        let art = track.album_id.and_then(|album| self.control.art_path(album));
        if let Err(e) = self.notifier.show(&summary, &body, art.as_deref()) {
            tracing::warn!(error = %e, "could not show the track notification");
        }
    }
}

/// Runs the notifications on their own thread, fed by the core's events.
pub fn spawn<N: Notifier>(mut notifications: TrackNotifications<N>, events: Receiver<Event>) -> JoinHandle<()> {
    std::thread::Builder::new()
        .name("notifications".into())
        .spawn(move || {
            while let Ok(event) = events.recv() {
                notifications.on_event(&event);
            }
        })
        .expect("spawn the notifications thread")
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Mutex};

    use goymusic_core::events::{Event, PlaybackState};
    use goymusic_core::model::Track;
    use goymusic_core::{AlbumId, TrackId};

    use super::*;
    use crate::control::fake::RecordingControl;

    fn track(id: i64, title: &str, album: Option<&str>, artists: &[&str]) -> Track {
        Track {
            id: TrackId(id),
            path: format!("/m/{id}.flac"),
            title: title.into(),
            artists: artists.iter().map(|a| a.to_string()).collect(),
            album: album.map(Into::into),
            album_id: album.map(|_| AlbumId(3)),
            album_artist: None,
            track_no: None,
            disc_no: None,
            year: None,
            genre: None,
            duration_ms: 1_000,
        }
    }

    /// (summary, body, image) of each notification shown.
    type Shown = Vec<(String, String, Option<PathBuf>)>;

    #[derive(Clone, Default)]
    struct Recorder {
        shown: Arc<Mutex<Shown>>,
        fail: bool,
    }

    impl Notifier for Recorder {
        fn show(&mut self, summary: &str, body: &str, image: Option<&Path>) -> Result<(), String> {
            self.shown
                .lock()
                .unwrap()
                .push((summary.into(), body.into(), image.map(Path::to_path_buf)));
            if self.fail {
                Err("no notification daemon".into())
            } else {
                Ok(())
            }
        }
    }

    fn playing(id: i64) -> Event {
        Event::PlayerStateChanged {
            state: PlaybackState::Playing,
            track: Some(TrackId(id)),
        }
    }

    fn setup(
        fail: bool,
        focused: bool,
    ) -> (
        TrackNotifications<Recorder>,
        Recorder,
        Arc<RecordingControl>,
        Arc<AtomicBool>,
    ) {
        let recorder = Recorder {
            fail,
            ..Recorder::default()
        };
        let mut view = crate::control::fake::empty_view();
        view.track = Some(track(7, "Faro", Some("Cartografía"), &["Ana"]));
        let control = Arc::new(RecordingControl {
            art: Some(PathBuf::from("/art/3-600.webp")),
            ..RecordingControl::with_view(view)
        });
        let is_focused = Arc::new(AtomicBool::new(focused));
        let f = is_focused.clone();
        let notifications = TrackNotifications::new(
            recorder.clone(),
            control.clone(),
            Arc::new(move || f.load(Ordering::SeqCst)),
        );
        (notifications, recorder, control, is_focused)
    }

    fn set_track(control: &RecordingControl, t: Track) {
        control.view.lock().unwrap().as_mut().unwrap().track = Some(t);
    }

    #[test]
    fn text_for_tagged_and_untagged_tracks() {
        assert_eq!(
            text_for(&track(1, "Faro", Some("Cartografía"), &["Ana", "Bea"])),
            ("Faro".into(), "Ana, Bea — Cartografía".into())
        );
        assert_eq!(
            text_for(&track(1, "Faro", None, &["Ana"])),
            ("Faro".into(), "Ana".into())
        );
        assert_eq!(
            text_for(&track(1, "Grabación 1", None, &[])),
            ("Grabación 1".into(), String::new())
        );
    }

    #[test]
    fn only_the_track_that_starts_playing_is_notified() {
        let (mut n, recorder, control, _) = setup(false, false);
        n.on_event(&Event::PlayerStateChanged {
            state: PlaybackState::Loading,
            track: Some(TrackId(7)),
        });
        n.on_event(&playing(7));
        n.on_event(&Event::PlayerStateChanged {
            state: PlaybackState::Paused,
            track: Some(TrackId(7)),
        });
        n.on_event(&playing(7));
        set_track(&control, track(8, "Vigilia", None, &[]));
        n.on_event(&playing(8));
        let shown = recorder.shown.lock().unwrap().clone();
        assert_eq!(
            shown.iter().map(|s| s.0.as_str()).collect::<Vec<_>>(),
            ["Faro", "Vigilia"]
        );
        assert_eq!(shown[0].2.as_deref(), Some(Path::new("/art/3-600.webp")));
        assert_eq!(shown[1].2, None, "a track without album has no cover");
    }

    #[test]
    fn nothing_is_notified_while_the_window_is_focused() {
        let (mut n, recorder, _, focused) = setup(false, true);
        n.on_event(&playing(7));
        assert!(recorder.shown.lock().unwrap().is_empty());
        focused.store(false, Ordering::SeqCst);
        n.on_event(&playing(7));
        assert_eq!(
            recorder.shown.lock().unwrap().len(),
            1,
            "the track is notified once the window loses focus"
        );
    }

    #[test]
    fn a_failing_notifier_does_not_stop_the_event_loop() {
        let (mut n, recorder, control, _) = setup(true, false);
        n.on_event(&playing(7));
        set_track(&control, track(8, "Vigilia", None, &[]));
        n.on_event(&playing(8));
        assert_eq!(recorder.shown.lock().unwrap().len(), 2);
    }
}
