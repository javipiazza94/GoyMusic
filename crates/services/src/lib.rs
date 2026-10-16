//! GoyMusic services: the library, playlists, search and player behind one transport-agnostic
//! facade (spec §3, §4). The Tauri app adapts it to commands and events today; a server
//! adapter can reuse it unchanged.

mod config;
mod error;
mod library;
mod player;
mod playlists;
mod scan;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use goymusic_audio::player::Player;
use goymusic_audio::sink::SinkFactory;
use goymusic_core::events::EventBus;
use goymusic_library::db::Db;
use goymusic_library::folders;

pub use config::ServicesConfig;
pub use error::{ErrorCode, Result, ServiceError};
pub use library::{AlbumDetail, ArtistDetail};
pub use playlists::PlaylistDetail;

use scan::ScanWorker;

pub struct Services {
    config: ServicesConfig,
    /// Connection for UI commands. Scans, the audio resolver and cover generation each have
    /// their own, so none of them waits on another (SQLite WAL allows that).
    db: Mutex<Db>,
    art_db: Mutex<Db>,
    events: Arc<EventBus>,
    scans: Arc<ScanWorker>,
    player: Mutex<Option<Player>>,
    stopped: AtomicBool,
}

impl Services {
    pub fn start(config: ServicesConfig, sink: SinkFactory) -> Result<Services> {
        let db = Db::open(&config.db_path)?;
        let events = Arc::new(EventBus::new());
        let scans = Arc::new(ScanWorker::start(Db::open(&config.db_path)?, events.clone()));
        let player = Player::spawn(sink, player::resolver(&config.db_path)?, events.clone());
        let has_folders = !folders::list(&db)?.is_empty();
        let services = Services {
            art_db: Mutex::new(Db::open(&config.db_path)?),
            db: Mutex::new(db),
            config,
            events,
            scans,
            player: Mutex::new(Some(player)),
            stopped: AtomicBool::new(false),
        };
        if has_folders {
            // pick up what changed on disk while the app was closed
            services.scans.request();
        }
        Ok(services)
    }

    pub fn events(&self) -> &Arc<EventBus> {
        &self.events
    }

    /// `true` once no scan is running or queued; `false` if `timeout` passed first.
    pub fn wait_idle(&self, timeout: Duration) -> bool {
        self.scans.wait_idle(timeout)
    }

    /// Stops background work and the player. Safe to call more than once.
    pub fn shutdown(&self) {
        if self.stopped.swap(true, Ordering::SeqCst) {
            return;
        }
        self.scans.stop();
        self.player.lock().expect("player poisoned").take();
    }

    fn db(&self) -> MutexGuard<'_, Db> {
        self.db.lock().expect("database poisoned")
    }
}

impl Drop for Services {
    fn drop(&mut self) {
        self.shutdown();
    }
}
