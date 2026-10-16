//! MPRIS (`org.mpris.MediaPlayer2.goymusic`): multimedia keys, GNOME's media controls and
//! `playerctl` drive the player; core events keep the published state current (spec §8).
//!
//! `mpris_server::Player` is single-threaded, so it lives on its own thread with its own
//! executor; core events reach it through a channel.

use std::cell::Cell;
use std::path::Path;
use std::rc::Rc;
use std::sync::mpsc::Receiver;
use std::thread::JoinHandle;

use goymusic_core::events::{Event, PlaybackState};
use goymusic_core::model::Track;
use goymusic_core::queue::RepeatMode;
use mpris_server::{LoopStatus, Metadata, PlaybackStatus, Player, Time, TrackId as MprisTrackId};

use crate::control::SharedControl;

pub const BUS_NAME_SUFFIX: &str = "goymusic";
/// Basename of the installed `.desktop` file (without the extension).
pub const DESKTOP_ENTRY: &str = "goymusic";

/// What MPRIS can ask of the app window.
pub struct MprisHooks {
    pub raise: Box<dyn Fn() + Send + Sync>,
    pub quit: Box<dyn Fn() + Send + Sync>,
}

pub fn track_id(track: &Track) -> MprisTrackId {
    MprisTrackId::try_from(format!("/org/mpris/MediaPlayer2/Track/{}", track.id.0)).unwrap_or(MprisTrackId::NO_TRACK)
}

pub fn metadata(track: &Track, art: Option<&Path>) -> Metadata {
    let mut m = Metadata::new();
    m.set_trackid(Some(track_id(track)));
    m.set_title(Some(track.title.clone()));
    if !track.artists.is_empty() {
        m.set_artist(Some(track.artists.clone()));
    }
    m.set_album(track.album.clone());
    if let Some(artist) = &track.album_artist {
        m.set_album_artist(Some([artist.clone()]));
    }
    m.set_length(Some(Time::from_millis(track.duration_ms as i64)));
    if let Some(art) = art {
        m.set_art_url(Some(format!("file://{}", art.display())));
    }
    m
}

pub fn no_track_metadata() -> Metadata {
    let mut m = Metadata::new();
    m.set_trackid(Some(MprisTrackId::NO_TRACK));
    m
}

pub fn playback_status(state: PlaybackState) -> PlaybackStatus {
    match state {
        PlaybackState::Playing | PlaybackState::Loading => PlaybackStatus::Playing,
        PlaybackState::Paused => PlaybackStatus::Paused,
        PlaybackState::Idle | PlaybackState::Ended => PlaybackStatus::Stopped,
    }
}

pub fn loop_status(mode: RepeatMode) -> LoopStatus {
    match mode {
        RepeatMode::Off => LoopStatus::None,
        RepeatMode::All => LoopStatus::Playlist,
        RepeatMode::One => LoopStatus::Track,
    }
}

pub fn repeat_mode(status: LoopStatus) -> RepeatMode {
    match status {
        LoopStatus::None => RepeatMode::Off,
        LoopStatus::Playlist => RepeatMode::All,
        LoopStatus::Track => RepeatMode::One,
    }
}

/// Starts the MPRIS server on its own thread. If the session bus is unavailable it logs a
/// warning and the thread ends: the player keeps working without it.
pub fn spawn(control: SharedControl, events: Receiver<Event>, hooks: MprisHooks) -> JoinHandle<()> {
    let (tx, rx) = async_channel::unbounded();
    // bridges the core's blocking channel into the executor
    std::thread::Builder::new()
        .name("mpris-events".into())
        .spawn(move || {
            while let Ok(event) = events.recv() {
                if tx.send_blocking(event).is_err() {
                    return;
                }
            }
        })
        .expect("spawn the MPRIS event bridge");
    std::thread::Builder::new()
        .name("mpris".into())
        .spawn(move || futures_lite::future::block_on(serve(control, rx, hooks)))
        .expect("spawn the MPRIS thread")
}

async fn serve(control: SharedControl, events: async_channel::Receiver<Event>, hooks: MprisHooks) {
    let player = match Player::builder(BUS_NAME_SUFFIX)
        .identity("GoyMusic")
        .desktop_entry(DESKTOP_ENTRY)
        .can_raise(true)
        .can_quit(true)
        .can_control(true)
        .can_seek(true)
        .build()
        .await
    {
        Ok(player) => player,
        Err(e) => {
            tracing::warn!(error = %e, "MPRIS unavailable: multimedia keys and GNOME controls will not work");
            return;
        }
    };
    // a seek requested over MPRIS is announced (Seeked) once the core reports the new position
    let pending_seek = Rc::new(Cell::new(false));
    connect(&player, &control, hooks, &pending_seek);
    sync(&player, &control).await;
    let updates = async {
        while let Ok(event) = events.recv().await {
            match event {
                Event::PlayerStateChanged { .. } | Event::QueueChanged => sync(&player, &control).await,
                Event::PositionChanged { position_ms, .. } => {
                    let position = Time::from_millis(position_ms as i64);
                    player.set_position(position);
                    if pending_seek.replace(false) {
                        if let Err(e) = player.seeked(position).await {
                            tracing::debug!(error = %e, "MPRIS Seeked signal failed");
                        }
                    }
                }
                _ => {}
            }
        }
    };
    futures_lite::future::or(player.run(), updates).await;
}

fn connect(player: &Player, control: &SharedControl, hooks: MprisHooks, pending_seek: &Rc<Cell<bool>>) {
    let c = control.clone();
    player.connect_play_pause(move |_| c.play_pause());
    let c = control.clone();
    player.connect_play(move |_| c.play());
    let c = control.clone();
    player.connect_pause(move |_| c.pause());
    let c = control.clone();
    player.connect_stop(move |_| c.pause());
    let c = control.clone();
    player.connect_next(move |_| c.next());
    let c = control.clone();
    player.connect_previous(move |_| c.previous());
    let (c, pending) = (control.clone(), pending_seek.clone());
    player.connect_seek(move |p, offset| {
        let target = (p.position().as_millis() + offset.as_millis()).max(0) as u64;
        pending.set(true);
        c.seek_to(target);
    });
    let (c, pending) = (control.clone(), pending_seek.clone());
    player.connect_set_position(move |p, id, position| {
        // ignore requests for a track that is no longer the current one (MPRIS spec)
        if *id == p.metadata().trackid().unwrap_or(MprisTrackId::NO_TRACK) && !position.is_negative() {
            pending.set(true);
            c.seek_to(position.as_millis() as u64);
        }
    });
    let c = control.clone();
    player.connect_set_volume(move |_, volume| c.set_volume(volume.clamp(0.0, 1.0) as f32));
    let c = control.clone();
    player.connect_set_shuffle(move |_, on| c.set_shuffle(on));
    let c = control.clone();
    player.connect_set_loop_status(move |_, status| c.set_repeat(repeat_mode(status)));
    let MprisHooks { raise, quit } = hooks;
    player.connect_raise(move |_| raise());
    player.connect_quit(move |_| quit());
}

/// Publishes the player's current state; each setter emits PropertiesChanged only on change.
async fn sync(player: &Player, control: &SharedControl) {
    let Some(view) = control.view() else {
        return;
    };
    let has_track = view.track.is_some();
    let metadata = match &view.track {
        Some(track) => metadata(track, track.album_id.and_then(|a| control.art_path(a)).as_deref()),
        None => no_track_metadata(),
    };
    let results = [
        player.set_playback_status(playback_status(view.state)).await,
        player.set_metadata(metadata).await,
        player.set_loop_status(loop_status(view.repeat)).await,
        player.set_shuffle(view.shuffle).await,
        player.set_volume(f64::from(view.volume)).await,
        player.set_can_play(has_track).await,
        player.set_can_pause(has_track).await,
        player.set_can_seek(has_track).await,
        player.set_can_go_next(has_track).await,
        player.set_can_go_previous(has_track).await,
    ];
    player.set_position(Time::from_millis(view.position_ms as i64));
    if let Some(e) = results.into_iter().find_map(Result::err) {
        tracing::debug!(error = %e, "MPRIS property update failed");
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use goymusic_core::events::PlaybackState;
    use goymusic_core::model::Track;
    use goymusic_core::queue::RepeatMode;
    use goymusic_core::{AlbumId, TrackId};
    use mpris_server::{LoopStatus, PlaybackStatus, Time, TrackId as MprisTrackId};

    use super::*;

    fn track(album: bool) -> Track {
        Track {
            id: TrackId(7),
            path: "/m/faro.flac".into(),
            title: "Faro".into(),
            artists: if album { vec!["Ana".into()] } else { vec![] },
            album: album.then(|| "Cartografía".into()),
            album_id: album.then_some(AlbumId(3)),
            album_artist: album.then(|| "Ana".into()),
            track_no: Some(1),
            disc_no: None,
            year: None,
            genre: None,
            duration_ms: 200_000,
        }
    }

    #[test]
    fn metadata_maps_a_tagged_track() {
        let m = metadata(&track(true), Some(Path::new("/cache/art/3-600.webp")));
        assert_eq!(m.title(), Some("Faro"));
        assert_eq!(m.artist(), Some(vec!["Ana".to_string()]));
        assert_eq!(m.album(), Some("Cartografía"));
        assert_eq!(m.album_artist(), Some(vec!["Ana".to_string()]));
        assert_eq!(m.length(), Some(Time::from_millis(200_000)));
        assert_eq!(
            m.trackid(),
            Some(MprisTrackId::try_from("/org/mpris/MediaPlayer2/Track/7").unwrap())
        );
        assert_eq!(m.art_url().as_deref(), Some("file:///cache/art/3-600.webp"));
    }

    #[test]
    fn metadata_for_an_untagged_track_is_minimal() {
        let m = metadata(&track(false), None);
        assert_eq!(m.title(), Some("Faro"));
        assert_eq!(m.album(), None);
        assert_eq!(m.artist(), None);
        assert_eq!(m.art_url(), None);
    }

    #[test]
    fn no_track_has_the_no_track_id() {
        assert_eq!(no_track_metadata().trackid(), Some(MprisTrackId::NO_TRACK));
    }

    #[test]
    fn states_map_to_mpris() {
        assert_eq!(playback_status(PlaybackState::Idle), PlaybackStatus::Stopped);
        assert_eq!(playback_status(PlaybackState::Loading), PlaybackStatus::Playing);
        assert_eq!(playback_status(PlaybackState::Playing), PlaybackStatus::Playing);
        assert_eq!(playback_status(PlaybackState::Paused), PlaybackStatus::Paused);
        assert_eq!(playback_status(PlaybackState::Ended), PlaybackStatus::Stopped);
    }

    #[test]
    fn loop_status_round_trips() {
        for (mode, status) in [
            (RepeatMode::Off, LoopStatus::None),
            (RepeatMode::All, LoopStatus::Playlist),
            (RepeatMode::One, LoopStatus::Track),
        ] {
            assert_eq!(loop_status(mode), status);
            assert_eq!(repeat_mode(status), mode);
        }
    }
}
