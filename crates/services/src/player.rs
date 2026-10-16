use std::path::{Path, PathBuf};
use std::sync::Mutex;

use goymusic_audio::engine::Resolver;
use goymusic_library::db::Db;
use goymusic_library::queries;

use crate::error::Result;

/// Track id → file, answered from the audio thread's own connection so a long scan or UI
/// query never delays the start of the next track.
pub(crate) fn resolver(db_path: &Path) -> Result<Resolver> {
    let db = Mutex::new(Db::open(db_path)?);
    Ok(Box::new(move |id| {
        let db = db.lock().ok()?;
        queries::track(&db, id).ok().map(|t| PathBuf::from(t.path))
    }))
}
