mod common;

use std::path::Path;
use std::time::{Duration, Instant};

use common::{album_fixture, start, wait_for, LONG};
use goymusic_core::events::{Event, PlaybackState};
use goymusic_core::{AlbumId, TrackId};
use goymusic_library::db::Db;
use goymusic_library::state;
use goymusic_services::{ErrorCode, PlayContext, Services};

const WAIT: Duration = Duration::from_secs(5);

struct Library {
    services: Services,
    album: AlbumId,
    tracks: Vec<TrackId>,
}

fn library(dir: &Path) -> Library {
    let music = dir.join("music");
    album_fixture(&music, "First", "Ana", 3);
    let services = start(dir);
    services.add_folder(&music).unwrap();
    assert!(services.wait_idle(LONG));
    reopen(services, dir)
}

/// Builds the fixture view of an already-scanned library.
fn reopen(services: Services, _dir: &Path) -> Library {
    let album = services.albums().unwrap()[0].id;
    let tracks = services.album(album).unwrap().tracks.iter().map(|t| t.id).collect();
    Library {
        services,
        album,
        tracks,
    }
}

fn playing(track: TrackId) -> impl Fn(&Event) -> bool {
    move |e| {
        *e == Event::PlayerStateChanged {
            state: PlaybackState::Playing,
            track: Some(track),
        }
    }
}

/// Polls `f` until it returns true or `WAIT` passes.
fn eventually(f: impl Fn() -> bool) -> bool {
    let deadline = Instant::now() + WAIT;
    while Instant::now() < deadline {
        if f() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    false
}

#[test]
fn playing_an_album_starts_its_first_track() {
    let dir = tempfile::tempdir().unwrap();
    let lib = library(dir.path());
    let events = lib.services.events().subscribe();
    lib.services.play(PlayContext::Album(lib.album), 0).unwrap();
    wait_for(&events, WAIT, playing(lib.tracks[0]));
}

#[test]
fn every_context_kind_resolves() {
    let dir = tempfile::tempdir().unwrap();
    let lib = library(dir.path());
    let s = &lib.services;
    let events = s.events().subscribe();
    let ana = s.artists().unwrap()[0].id;
    s.play(PlayContext::Artist(ana), 1).unwrap();
    wait_for(&events, WAIT, playing(lib.tracks[1]));
    let p = s.create_playlist("P").unwrap();
    s.add_to_playlist(p.id, &[lib.tracks[2], lib.tracks[0]]).unwrap();
    s.play(PlayContext::Playlist(p.id), 1).unwrap();
    wait_for(&events, WAIT, playing(lib.tracks[0]));
    s.like(lib.tracks[1]).unwrap();
    s.play(PlayContext::Liked, 0).unwrap();
    wait_for(&events, WAIT, playing(lib.tracks[1]));
    s.play(PlayContext::Tracks(vec![lib.tracks[2]]), 0).unwrap();
    wait_for(&events, WAIT, playing(lib.tracks[2]));
}

#[test]
fn invalid_contexts_are_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let lib = library(dir.path());
    assert_eq!(
        lib.services.play(PlayContext::Liked, 0).unwrap_err().code,
        ErrorCode::InvalidInput
    );
    assert_eq!(
        lib.services.play(PlayContext::Album(lib.album), 3).unwrap_err().code,
        ErrorCode::InvalidInput
    );
}

#[test]
fn player_view_shows_track_queue_and_settings() {
    let dir = tempfile::tempdir().unwrap();
    let lib = library(dir.path());
    let events = lib.services.events().subscribe();
    lib.services.play(PlayContext::Album(lib.album), 0).unwrap();
    wait_for(&events, WAIT, playing(lib.tracks[0]));
    lib.services.queue_add(vec![lib.tracks[2]]);
    let view = lib.services.player_view().unwrap();
    assert_eq!(view.state, PlaybackState::Playing);
    assert_eq!(view.track.unwrap().id, lib.tracks[0]);
    let ids = |ts: &[goymusic_core::model::Track]| ts.iter().map(|t| t.id).collect::<Vec<_>>();
    assert_eq!(ids(&view.user_queue), [lib.tracks[2]]);
    assert_eq!(ids(&view.up_next), [lib.tracks[1], lib.tracks[2]]);
    assert_eq!(view.volume, 1.0);
    assert!(!view.replaygain && !view.shuffle);
}

#[test]
fn queue_survives_a_restart_paused_at_its_position() {
    let dir = tempfile::tempdir().unwrap();
    let lib = library(dir.path());
    let events = lib.services.events().subscribe();
    lib.services.play(PlayContext::Album(lib.album), 1).unwrap();
    wait_for(&events, WAIT, playing(lib.tracks[1]));
    lib.services.seek(1500);
    lib.services.pause();
    lib.services.shutdown();
    drop(lib.services);

    let services = start(dir.path());
    let view = services.player_view().unwrap();
    assert_eq!(view.state, PlaybackState::Paused);
    assert_eq!(view.track.unwrap().id, lib.tracks[1]);
    assert_eq!(view.position_ms, 1500);
    assert_eq!(view.up_next.iter().map(|t| t.id).collect::<Vec<_>>(), [lib.tracks[2]]);
}

#[test]
fn track_change_saves_the_queue_without_shutdown() {
    let dir = tempfile::tempdir().unwrap();
    let lib = library(dir.path());
    let events = lib.services.events().subscribe();
    lib.services.play(PlayContext::Album(lib.album), 0).unwrap();
    wait_for(&events, WAIT, playing(lib.tracks[0]));
    lib.services.next();
    wait_for(&events, WAIT, playing(lib.tracks[1]));
    let db_path = common::config(dir.path()).db_path;
    std::mem::forget(lib.services); // a kill: no shutdown, nothing saved on exit
    let db = Db::open(&db_path).unwrap();
    assert!(eventually(|| {
        state::load_queue(&db)
            .unwrap()
            .is_some_and(|(q, _)| q.current() == Some(lib.tracks[1]))
    }));
}

#[test]
fn volume_and_replaygain_survive_a_restart() {
    let dir = tempfile::tempdir().unwrap();
    let services = start(dir.path());
    services.set_volume(0.4);
    services.set_replaygain(true);
    drop(services);
    let services = start(dir.path());
    let view = services.player_view().unwrap();
    assert_eq!(view.volume, 0.4);
    assert!(view.replaygain);
    assert!(services.settings().unwrap().replaygain);
}

#[test]
fn track_changes_are_recorded_in_history() {
    let dir = tempfile::tempdir().unwrap();
    let lib = library(dir.path());
    let events = lib.services.events().subscribe();
    lib.services.play(PlayContext::Album(lib.album), 0).unwrap();
    wait_for(&events, WAIT, playing(lib.tracks[0]));
    assert!(eventually(|| lib
        .services
        .recent_albums(5)
        .unwrap()
        .iter()
        .any(|a| a.id == lib.album)));
}

#[test]
fn restore_survives_a_track_that_left_the_library() {
    let dir = tempfile::tempdir().unwrap();
    let lib = library(dir.path());
    let events = lib.services.events().subscribe();
    lib.services.play(PlayContext::Album(lib.album), 1).unwrap();
    wait_for(&events, WAIT, playing(lib.tracks[1]));
    lib.services.pause();
    let gone = lib.services.track(lib.tracks[1]).unwrap().path;
    drop(lib.services);
    std::fs::remove_file(gone).unwrap();

    let services = start(dir.path());
    assert!(eventually(|| {
        services.player_view().is_ok_and(|v| {
            matches!(
                v.state,
                PlaybackState::Paused | PlaybackState::Idle | PlaybackState::Ended
            )
        })
    }));
    assert!(services.wait_idle(LONG));
    services.player_view().unwrap();
}
