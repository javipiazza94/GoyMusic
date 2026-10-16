//! `art://localhost/album/{id}/{200|600}`: album cover thumbnails for the UI, served from
//! the cache without base64 (spec §6).

use std::sync::mpsc::{channel, Sender};
use std::sync::{Arc, Mutex};

use goymusic_core::AlbumId;
use goymusic_library::art::ArtSize;
use goymusic_services::Services;
use tauri::http::{header, Response, StatusCode};
use tauri::UriSchemeResponder;

/// `/album/{id}/{px}` → album and size; anything else is not ours.
pub fn parse(path: &str) -> Option<(AlbumId, ArtSize)> {
    let mut parts = path.strip_prefix("/album/")?.split('/');
    let id = parts.next()?.parse().ok()?;
    let size = ArtSize::from_px(parts.next()?.parse().ok()?)?;
    parts.next().is_none().then_some((AlbumId(id), size))
}

/// A fixed set of worker threads: a grid of hundreds of albums queues its requests instead of
/// spawning a thread each, and never runs on the main (UI) thread.
pub struct ArtPool {
    jobs: Mutex<Sender<Job>>,
}

type Job = Box<dyn FnOnce() + Send>;

impl ArtPool {
    pub fn new(workers: usize) -> Self {
        let (tx, rx) = channel::<Job>();
        let rx = Arc::new(Mutex::new(rx));
        for i in 0..workers.max(1) {
            let rx = rx.clone();
            std::thread::Builder::new()
                .name(format!("goymusic-art-{i}"))
                .spawn(move || loop {
                    let job = rx.lock().expect("art queue poisoned").recv();
                    match job {
                        Ok(job) => job(),
                        Err(_) => return,
                    }
                })
                .expect("spawn art worker");
        }
        Self { jobs: Mutex::new(tx) }
    }

    /// One worker per core, at most 4: covers are CPU-bound and the UI needs some too.
    pub fn sized_for_this_machine() -> Self {
        Self::new(std::thread::available_parallelism().map_or(2, |n| n.get().min(4)))
    }

    pub fn run(&self, job: impl FnOnce() + Send + 'static) {
        let _ = self.jobs.lock().expect("art queue poisoned").send(Box::new(job));
    }
}

/// The core is not up yet (requests can arrive before setup finishes).
pub fn unavailable() -> Response<Vec<u8>> {
    Response::builder()
        .status(StatusCode::SERVICE_UNAVAILABLE)
        .body(Vec::new())
        .expect("static response")
}

pub fn respond(pool: &ArtPool, services: Arc<Services>, path: String, responder: UriSchemeResponder) {
    pool.run(move || {
        let response = match parse(&path).map(|(id, size)| services.album_art(id, size)) {
            Some(Ok(Some(file))) => match std::fs::read(&file) {
                Ok(bytes) => Response::builder()
                    .status(StatusCode::OK)
                    .header(header::CONTENT_TYPE, "image/webp")
                    .header(header::CACHE_CONTROL, "max-age=86400")
                    .body(bytes),
                Err(e) => error(e.to_string()),
            },
            Some(Ok(None)) | None => Response::builder().status(StatusCode::NOT_FOUND).body(Vec::new()),
            Some(Err(e)) => error(e.to_string()),
        };
        responder.respond(response.expect("valid art response"));
    });
}

fn error(message: String) -> tauri::http::Result<Response<Vec<u8>>> {
    tracing::warn!(%message, "album art failed");
    Response::builder()
        .status(StatusCode::INTERNAL_SERVER_ERROR)
        .body(Vec::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn album_paths_are_parsed() {
        assert_eq!(parse("/album/12/200"), Some((AlbumId(12), ArtSize::Small)));
        assert_eq!(parse("/album/12/600"), Some((AlbumId(12), ArtSize::Large)));
    }

    #[test]
    fn other_paths_are_rejected() {
        for path in [
            "/album/x/200",
            "/album/12/300",
            "/track/1/200",
            "",
            "/album/12",
            "/album/12/200/extra",
        ] {
            assert_eq!(parse(path), None, "{path}");
        }
    }
}

#[cfg(test)]
mod pool_tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::time::Duration;

    use super::ArtPool;

    #[test]
    fn the_pool_bounds_concurrent_jobs_and_runs_them_all() {
        let pool = ArtPool::new(2);
        let running = Arc::new(AtomicUsize::new(0));
        let peak = Arc::new(AtomicUsize::new(0));
        let (done_tx, done_rx) = std::sync::mpsc::channel();
        for _ in 0..6 {
            let (running, peak, done_tx) = (running.clone(), peak.clone(), done_tx.clone());
            pool.run(move || {
                let now = running.fetch_add(1, Ordering::SeqCst) + 1;
                peak.fetch_max(now, Ordering::SeqCst);
                std::thread::sleep(Duration::from_millis(50));
                running.fetch_sub(1, Ordering::SeqCst);
                done_tx.send(()).unwrap();
            });
        }
        for _ in 0..6 {
            done_rx.recv_timeout(Duration::from_secs(5)).expect("every job runs");
        }
        assert_eq!(peak.load(Ordering::SeqCst), 2);
    }
}
