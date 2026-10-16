//! Folder watching (spec §6): inotify through notify, a 2 s quiet period before rescanning,
//! and a periodic rescan whenever some folder cannot be watched.

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use serde::Serialize;

const STOP_CHECK: Duration = Duration::from_millis(100);

/// Shown in the settings: `Polling` means changes are picked up every 10 minutes only.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(tag = "mode", rename_all = "camelCase")]
pub enum WatchStatus {
    Live,
    Polling { reason: String },
}

type Trigger = Arc<dyn Fn() + Send + Sync>;

pub(crate) struct FolderWatcher {
    watcher: Mutex<Option<RecommendedWatcher>>,
    shared: Arc<Shared>,
    threads: Mutex<Vec<JoinHandle<()>>>,
}

struct Shared {
    status: Mutex<WatchStatus>,
    stop: AtomicBool,
    trigger: Trigger,
    poll_every: Duration,
    polling: AtomicBool,
    poller: Mutex<Option<JoinHandle<()>>>,
}

impl FolderWatcher {
    pub(crate) fn start(folders: &[String], debounce: Duration, poll_every: Duration, trigger: Trigger) -> Self {
        let shared = Arc::new(Shared {
            status: Mutex::new(WatchStatus::Live),
            stop: AtomicBool::new(false),
            trigger,
            poll_every,
            polling: AtomicBool::new(false),
            poller: Mutex::new(None),
        });
        let (tx, rx) = channel();
        let watcher = match notify::recommended_watcher(tx) {
            Ok(w) => Some(w),
            Err(e) => {
                shared.fall_back(e.to_string());
                None
            }
        };
        let debouncer = {
            let shared = shared.clone();
            std::thread::Builder::new()
                .name("goymusic-watch".into())
                .spawn(move || debounce_loop(&rx, debounce, &shared))
                .expect("spawn watch thread")
        };
        let this = Self {
            watcher: Mutex::new(watcher),
            shared,
            threads: Mutex::new(vec![debouncer]),
        };
        for folder in folders {
            this.watch(Path::new(folder));
        }
        this
    }

    pub(crate) fn watch(&self, path: &Path) {
        if let Some(watcher) = self.watcher.lock().expect("watcher poisoned").as_mut() {
            if let Err(e) = watcher.watch(path, RecursiveMode::Recursive) {
                tracing::warn!(path = %path.display(), error = %e, "cannot watch folder; polling instead");
                self.shared.fall_back(format!("{}: {e}", path.display()));
            }
        }
    }

    pub(crate) fn unwatch(&self, path: &Path) {
        if let Some(watcher) = self.watcher.lock().expect("watcher poisoned").as_mut() {
            let _ = watcher.unwatch(path);
        }
    }

    pub(crate) fn status(&self) -> WatchStatus {
        self.shared.status.lock().expect("watch status poisoned").clone()
    }

    pub(crate) fn stop(&self) {
        self.shared.stop.store(true, Ordering::SeqCst);
        self.watcher.lock().expect("watcher poisoned").take();
        let mut threads: Vec<_> = self.threads.lock().expect("watch threads poisoned").drain(..).collect();
        threads.extend(self.shared.poller.lock().expect("poller poisoned").take());
        for thread in threads {
            let _ = thread.join();
        }
    }
}

impl Shared {
    /// Switches to periodic rescans; the first failure's reason is the one reported.
    fn fall_back(self: &Arc<Self>, reason: String) {
        {
            let mut status = self.status.lock().expect("watch status poisoned");
            if *status == WatchStatus::Live {
                *status = WatchStatus::Polling { reason };
            }
        }
        if self.polling.swap(true, Ordering::SeqCst) {
            return;
        }
        let shared = self.clone();
        let poller = std::thread::Builder::new()
            .name("goymusic-poll".into())
            .spawn(move || {
                let mut next = Instant::now() + shared.poll_every;
                while !shared.stop.load(Ordering::SeqCst) {
                    std::thread::sleep(STOP_CHECK.min(shared.poll_every));
                    if Instant::now() >= next {
                        (shared.trigger)();
                        next = Instant::now() + shared.poll_every;
                    }
                }
            })
            .expect("spawn poll thread");
        *self.poller.lock().expect("poller poisoned") = Some(poller);
    }
}

/// Requests one scan once `debounce` passes with no relevant change, however long the burst
/// of changes (copying a whole album) lasts.
fn debounce_loop(events: &Receiver<notify::Result<notify::Event>>, debounce: Duration, shared: &Arc<Shared>) {
    let mut deadline: Option<Instant> = None;
    while !shared.stop.load(Ordering::SeqCst) {
        match events.recv_timeout(STOP_CHECK) {
            // reads (our own scans, playback, covers) are not changes
            Ok(Ok(event)) if matches!(event.kind, EventKind::Access(_)) => {}
            Ok(Ok(_)) => deadline = Some(Instant::now() + debounce),
            Ok(Err(e)) => {
                tracing::warn!(error = %e, "folder watcher error; polling instead");
                shared.fall_back(e.to_string());
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return,
        }
        if deadline.is_some_and(|d| Instant::now() >= d) {
            deadline = None;
            (shared.trigger)();
        }
    }
}
