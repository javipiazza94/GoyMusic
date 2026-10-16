//! Saves the queue and records play history every time a new track starts playing, so a
//! crash or kill loses at most the position within the current track (spec §5, §6).
//! It also drops from the queue tracks that left the library, after each library update.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use goymusic_audio::player::Player;
use goymusic_core::events::{Event, PlaybackState};
use goymusic_core::TrackId;
use goymusic_library::db::Db;
use goymusic_library::error::LibraryError;
use goymusic_library::{history, queries, state};

const STOP_CHECK: Duration = Duration::from_millis(100);

pub(crate) struct Coordinator {
    stop: Arc<AtomicBool>,
    thread: Mutex<Option<JoinHandle<()>>>,
}

impl Coordinator {
    pub(crate) fn start(db: Db, events: Receiver<Event>, player: Arc<Mutex<Option<Player>>>) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let stopping = stop.clone();
        let thread = std::thread::Builder::new()
            .name("goymusic-coordinator".into())
            .spawn(move || run(&db, &events, &player, &stopping))
            .expect("spawn coordinator thread");
        Self {
            stop,
            thread: Mutex::new(Some(thread)),
        }
    }

    pub(crate) fn stop(&self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(thread) = self.thread.lock().expect("coordinator poisoned").take() {
            let _ = thread.join();
        }
    }
}

fn run(db: &Db, events: &Receiver<Event>, player: &Mutex<Option<Player>>, stop: &AtomicBool) {
    let mut last: Option<TrackId> = None;
    while !stop.load(Ordering::SeqCst) {
        let event = match events.recv_timeout(STOP_CHECK) {
            Ok(event) => event,
            Err(RecvTimeoutError::Timeout) => continue,
            Err(RecvTimeoutError::Disconnected) => return,
        };
        if event == Event::LibraryUpdated {
            drop_vanished_from_queue(db, player);
            continue;
        }
        let Event::PlayerStateChanged {
            state: PlaybackState::Playing,
            track: Some(track),
        } = event
        else {
            continue;
        };
        if last == Some(track) {
            continue;
        }
        last = Some(track);
        if let Err(e) = history::record_play(db, track, now_secs()) {
            tracing::warn!(error = %e, "could not record play history");
        }
        let snapshot = player.lock().expect("player poisoned").as_ref().and_then(Player::state);
        if let Some(snapshot) = snapshot {
            if let Err(e) = state::save_queue(db, &snapshot.queue, snapshot.position_ms) {
                tracing::warn!(error = %e, "could not save the queue");
            }
        }
    }
}

/// Queued tracks that left the library (folder removed, file deleted) are removed from the
/// queue, so the queue the UI shows and the indexes it sends back stay the core's own.
fn drop_vanished_from_queue(db: &Db, player: &Mutex<Option<Player>>) {
    let guard = player.lock().expect("player poisoned");
    let Some(player) = guard.as_ref() else {
        return;
    };
    let Some(snapshot) = player.state() else {
        return;
    };
    let user = snapshot.queue.user_queue();
    for (index, id) in user.iter().enumerate().rev() {
        if matches!(queries::track(db, *id), Err(LibraryError::NotFound { .. })) {
            player.queue_remove(index);
        }
    }
}

fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as i64)
}
