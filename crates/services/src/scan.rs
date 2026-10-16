//! Background scanning. Requests from the UI, the watcher and the polling fallback are
//! coalesced: however many arrive during a scan, one more scan follows it.

use std::sync::mpsc::{channel, Sender};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use goymusic_core::events::EventBus;
use goymusic_library::db::Db;
use goymusic_library::scanner;

#[derive(Default)]
struct Status {
    pending: bool,
    running: bool,
    stopped: bool,
}

pub(crate) struct ScanWorker {
    requests: Mutex<Option<Sender<()>>>,
    status: Arc<(Mutex<Status>, Condvar)>,
    thread: Mutex<Option<JoinHandle<()>>>,
}

impl ScanWorker {
    pub(crate) fn start(mut db: Db, events: Arc<EventBus>) -> Self {
        let (tx, rx) = channel::<()>();
        let status: Arc<(Mutex<Status>, Condvar)> = Arc::default();
        let shared = status.clone();
        let thread = std::thread::Builder::new()
            .name("goymusic-scan".into())
            .spawn(move || {
                while rx.recv().is_ok() {
                    while rx.try_recv().is_ok() {}
                    {
                        let mut s = shared.0.lock().expect("scan status poisoned");
                        if s.stopped {
                            break;
                        }
                        s.pending = false;
                        s.running = true;
                    }
                    match scanner::scan(&mut db, &events) {
                        Ok(report) if !report.errors.is_empty() => {
                            tracing::warn!(errors = report.errors.len(), "scan finished with errors")
                        }
                        Ok(_) => {}
                        Err(e) => tracing::warn!(error = %e, "scan failed"),
                    }
                    let mut s = shared.0.lock().expect("scan status poisoned");
                    s.running = false;
                    shared.1.notify_all();
                }
            })
            .expect("spawn scan thread");
        Self {
            requests: Mutex::new(Some(tx)),
            status,
            thread: Mutex::new(Some(thread)),
        }
    }

    pub(crate) fn request(&self) {
        if let Some(tx) = self.requests.lock().expect("scan requests poisoned").as_ref() {
            self.status.0.lock().expect("scan status poisoned").pending = true;
            let _ = tx.send(());
        }
    }

    /// Request function for other threads (watcher, polling); it outlives no one.
    pub(crate) fn trigger(self: &Arc<Self>) -> Arc<dyn Fn() + Send + Sync> {
        let worker = Arc::downgrade(self);
        Arc::new(move || {
            if let Some(worker) = worker.upgrade() {
                worker.request();
            }
        })
    }

    pub(crate) fn wait_idle(&self, timeout: Duration) -> bool {
        let deadline = Instant::now() + timeout;
        let mut s = self.status.0.lock().expect("scan status poisoned");
        while s.pending || s.running {
            let left = deadline.saturating_duration_since(Instant::now());
            if left.is_zero() {
                return false;
            }
            s = self.status.1.wait_timeout(s, left).expect("scan status poisoned").0;
        }
        true
    }

    /// Lets the current scan finish, drops queued requests and joins the thread.
    pub(crate) fn stop(&self) {
        self.status.0.lock().expect("scan status poisoned").stopped = true;
        self.requests.lock().expect("scan requests poisoned").take();
        if let Some(thread) = self.thread.lock().expect("scan thread poisoned").take() {
            let _ = thread.join();
        }
        let mut s = self.status.0.lock().expect("scan status poisoned");
        s.pending = false;
        self.status.1.notify_all();
    }
}
