use std::path::Path;

use goymusic_core::model::Folder;
use goymusic_core::FolderId;

use crate::db::Db;
use crate::error::{LibraryError, Result};
use crate::maintenance;
use rusqlite::TransactionBehavior;

/// Adds a library folder. Rejects folders equal to, inside, or containing an existing one,
/// so no file is ever indexed twice.
pub fn add(db: &Db, path: &Path) -> Result<Folder> {
    let canonical = std::fs::canonicalize(path)?;
    let text = canonical
        .to_str()
        .ok_or_else(|| LibraryError::NonUtf8Path(canonical.display().to_string()))?
        .to_owned();
    for existing in list(db)? {
        let existing = Path::new(&existing.path);
        if canonical.starts_with(existing) || existing.starts_with(&canonical) {
            return Err(LibraryError::FolderOverlap(text));
        }
    }
    db.conn().execute("INSERT INTO folders(path) VALUES (?1)", [&text])?;
    Ok(Folder {
        id: FolderId(db.conn().last_insert_rowid()),
        path: text,
    })
}

pub fn list(db: &Db) -> Result<Vec<Folder>> {
    let mut stmt = db.conn().prepare("SELECT id, path FROM folders ORDER BY path")?;
    let folders = stmt
        .query_map([], |r| {
            Ok(Folder {
                id: FolderId(r.get(0)?),
                path: r.get(1)?,
            })
        })?
        .collect::<rusqlite::Result<_>>()?;
    Ok(folders)
}

/// Removes a folder and every track indexed from it.
pub fn remove(db: &mut Db, id: FolderId) -> Result<()> {
    let tx = db
        .conn_mut()
        .transaction_with_behavior(TransactionBehavior::Immediate)?;
    let track_ids: Vec<i64> = {
        let mut stmt = tx.prepare("SELECT id FROM tracks WHERE folder_id = ?1")?;
        let ids = stmt.query_map([id.0], |r| r.get(0))?.collect::<rusqlite::Result<_>>()?;
        ids
    };
    maintenance::delete_tracks(&tx, &track_ids)?;
    if tx.execute("DELETE FROM folders WHERE id = ?1", [id.0])? == 0 {
        return Err(LibraryError::NotFound {
            kind: "folder",
            id: id.0,
        });
    }
    maintenance::cleanup_orphans(&tx)?;
    tx.commit()?;
    Ok(())
}
