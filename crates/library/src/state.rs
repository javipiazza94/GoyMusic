use goymusic_core::queue::Queue;
use rusqlite::{params, OptionalExtension};

use crate::db::Db;
use crate::error::Result;

/// Stores the queue and playback position (one row, overwritten on every save).
pub fn save_queue(db: &Db, queue: &Queue, position_ms: u64) -> Result<()> {
    let snapshot = serde_json::to_string(queue)?;
    db.conn().execute(
        "INSERT INTO queue_state (id, snapshot, position_ms) VALUES (1, ?1, ?2)
         ON CONFLICT(id) DO UPDATE SET snapshot = excluded.snapshot, position_ms = excluded.position_ms",
        params![snapshot, position_ms as i64],
    )?;
    Ok(())
}

pub fn load_queue(db: &Db) -> Result<Option<(Queue, u64)>> {
    let row: Option<(String, i64)> = db
        .conn()
        .query_row("SELECT snapshot, position_ms FROM queue_state WHERE id = 1", [], |r| {
            Ok((r.get(0)?, r.get(1)?))
        })
        .optional()?;
    row.map(|(snapshot, position)| Ok((serde_json::from_str(&snapshot)?, position as u64)))
        .transpose()
}

pub fn set_setting(db: &Db, key: &str, value: &str) -> Result<()> {
    db.conn().execute(
        "INSERT INTO settings (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )?;
    Ok(())
}

pub fn setting(db: &Db, key: &str) -> Result<Option<String>> {
    Ok(db
        .conn()
        .query_row("SELECT value FROM settings WHERE key = ?1", [key], |r| r.get(0))
        .optional()?)
}
